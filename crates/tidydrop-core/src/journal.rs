//! Append-only JSON-lines journal of every move, stored in the app data folder
//! (never inside the folder being cleaned).
//!
//! Crash safety: an `intent` line is written and fsynced *before* each rename,
//! and a `done` line after. If the app dies between the two, undo checks the
//! disk to see whether the rename happened. A truncated last line is ignored.

use crate::error::{Error, Result};
use serde::{Deserialize, Serialize};
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

pub const JOURNAL_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Entry {
    Header {
        version: u32,
        id: String,
        root: PathBuf,
        created_at: String,
        organize_by_month: bool,
        planned: usize,
    },
    DirCreated {
        path: PathBuf,
    },
    Intent {
        seq: usize,
        from: PathBuf,
        to: PathBuf,
        size: u64,
    },
    Done {
        seq: usize,
    },
    Failed {
        seq: usize,
        error: String,
    },
    Finished {
        moved: usize,
        failed: usize,
        skipped: usize,
    },
    UndoStarted {
        at: String,
    },
    Restored {
        seq: usize,
        to: PathBuf,
    },
    RestoreFailed {
        seq: usize,
        error: String,
    },
    UndoFinished {
        at: String,
        restored: usize,
        problems: usize,
    },
}

pub struct JournalWriter {
    file: File,
    pub path: PathBuf,
    pub id: String,
}

impl JournalWriter {
    pub fn create(dir: &Path, id: &str) -> Result<JournalWriter> {
        fs::create_dir_all(dir)?;
        let path = dir.join(format!("{id}.jsonl"));
        let file = OpenOptions::new()
            .create_new(true)
            .append(true)
            .open(&path)?;
        Ok(JournalWriter {
            file,
            path,
            id: id.to_string(),
        })
    }

    pub fn open_append(path: &Path) -> Result<JournalWriter> {
        let file = OpenOptions::new().append(true).open(path)?;
        let id = path
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default();
        Ok(JournalWriter {
            file,
            path: path.to_path_buf(),
            id,
        })
    }

    /// Append one entry. `durable` = fsync before returning.
    pub fn write(&mut self, e: &Entry, durable: bool) -> Result<()> {
        let mut line = serde_json::to_string(e)?;
        line.push('\n');
        self.file.write_all(line.as_bytes())?;
        self.file.flush()?;
        if durable {
            self.file.sync_data()?;
        }
        Ok(())
    }
}

/// Read all well-formed entries; a corrupt/truncated line (crash mid-write) is skipped.
pub fn read_entries(path: &Path) -> Result<Vec<Entry>> {
    let f = File::open(path)?;
    let mut out = Vec::new();
    for line in BufReader::new(f).lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => continue,
        };
        if line.trim().is_empty() {
            continue;
        }
        if let Ok(e) = serde_json::from_str::<Entry>(&line) {
            out.push(e);
        }
    }
    if !matches!(out.first(), Some(Entry::Header { .. })) {
        return Err(Error::Journal(format!("{} has no header", path.display())));
    }
    Ok(out)
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum JournalStatus {
    /// Moves were applied (possibly interrupted) and not undone.
    Applied,
    /// Apply was interrupted (no `finished` line).
    Interrupted,
    Undone,
    PartiallyUndone,
    /// Nothing was moved.
    Empty,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JournalSummary {
    pub id: String,
    pub path: PathBuf,
    pub root: PathBuf,
    pub created_at: String,
    pub moved: usize,
    pub restored: usize,
    pub status: JournalStatus,
}

/// One move as reconstructed from the journal.
#[derive(Debug, Clone)]
pub struct MoveRecord {
    pub seq: usize,
    pub from: PathBuf,
    pub to: PathBuf,
    pub size: u64,
    pub done: bool,
    pub failed: bool,
    pub restored: bool,
}

impl MoveRecord {
    /// Did this move actually happen (now or before an undo)?
    pub fn is_effective(&self) -> bool {
        self.done
            || self.restored
            || (!self.failed
                && crate::paths::exists_any(&self.to)
                && !crate::paths::exists_any(&self.from))
    }
}

pub struct Parsed {
    pub id: String,
    pub root: PathBuf,
    pub created_at: String,
    pub moves: Vec<MoveRecord>,
    pub dirs_created: Vec<PathBuf>,
    pub finished: bool,
    pub undo_started: bool,
}

pub fn parse(entries: &[Entry]) -> Parsed {
    let mut p = Parsed {
        id: String::new(),
        root: PathBuf::new(),
        created_at: String::new(),
        moves: Vec::new(),
        dirs_created: Vec::new(),
        finished: false,
        undo_started: false,
    };
    for e in entries {
        match e {
            Entry::Header {
                id,
                root,
                created_at,
                ..
            } => {
                p.id = id.clone();
                p.root = root.clone();
                p.created_at = created_at.clone();
            }
            Entry::DirCreated { path } => p.dirs_created.push(path.clone()),
            Entry::Intent {
                seq,
                from,
                to,
                size,
            } => p.moves.push(MoveRecord {
                seq: *seq,
                from: from.clone(),
                to: to.clone(),
                size: *size,
                done: false,
                failed: false,
                restored: false,
            }),
            Entry::Done { seq } => p
                .moves
                .iter_mut()
                .filter(|m| m.seq == *seq)
                .for_each(|m| m.done = true),
            Entry::Failed { seq, .. } => p
                .moves
                .iter_mut()
                .filter(|m| m.seq == *seq)
                .for_each(|m| m.failed = true),
            Entry::Restored { seq, .. } => p
                .moves
                .iter_mut()
                .filter(|m| m.seq == *seq)
                .for_each(|m| m.restored = true),
            Entry::Finished { .. } => p.finished = true,
            Entry::UndoStarted { .. } => p.undo_started = true,
            Entry::RestoreFailed { .. } | Entry::UndoFinished { .. } => {}
        }
    }
    p
}

pub fn summarize(path: &Path) -> Result<JournalSummary> {
    let entries = read_entries(path)?;
    let p = parse(&entries);
    // A move with an intent but no done/failed line may have happened (crash); count it if the disk says so.
    let moved = p.moves.iter().filter(|m| m.is_effective()).count();
    let restored = p.moves.iter().filter(|m| m.restored).count();
    let status = if moved == 0 {
        JournalStatus::Empty
    } else if restored >= moved {
        JournalStatus::Undone
    } else if restored > 0 || p.undo_started {
        JournalStatus::PartiallyUndone
    } else if !p.finished {
        JournalStatus::Interrupted
    } else {
        JournalStatus::Applied
    };
    Ok(JournalSummary {
        id: p.id,
        path: path.to_path_buf(),
        root: p.root,
        created_at: p.created_at,
        moved,
        restored,
        status,
    })
}

/// All journals, newest first.
pub fn list(dir: &Path) -> Result<Vec<JournalSummary>> {
    let rd = match fs::read_dir(dir) {
        Ok(rd) => rd,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(vec![]),
        Err(e) => return Err(e.into()),
    };
    let mut out: Vec<JournalSummary> = rd
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().map(|e| e == "jsonl").unwrap_or(false))
        .filter_map(|p| summarize(&p).ok())
        .collect();
    // ids start with a sortable timestamp
    out.sort_by(|a, b| b.id.cmp(&a.id));
    Ok(out)
}

/// The most recent journal that still has something to undo.
pub fn latest_undoable(dir: &Path) -> Result<Option<JournalSummary>> {
    Ok(list(dir)?.into_iter().find(|j| {
        matches!(
            j.status,
            JournalStatus::Applied | JournalStatus::Interrupted | JournalStatus::PartiallyUndone
        )
    }))
}

/// New sortable journal id: `20241104-213000-123456789`.
pub fn new_id() -> String {
    let now = chrono::Local::now();
    format!(
        "{}-{:09}",
        now.format("%Y%m%d-%H%M%S"),
        now.timestamp_subsec_nanos()
    )
}
