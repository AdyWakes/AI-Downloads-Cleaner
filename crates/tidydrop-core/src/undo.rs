//! Undo a cleanup from its journal. Works after restarts: everything needed is on disk.

use crate::apply::sorted_deepest_first;
use crate::error::{Error, Result};
use crate::journal::{self, Entry, JournalWriter};
use crate::paths::{exists_any, rename_noreplace, unique_destination};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Restored {
    pub from: PathBuf,
    pub to: PathBuf,
    /// True when the original name was taken and the file came back as `name (1).ext`.
    pub renamed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Problem {
    pub path: PathBuf,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UndoReport {
    pub journal_id: String,
    pub restored: Vec<Restored>,
    pub problems: Vec<Problem>,
    pub folders_removed: usize,
}

/// Undo the most recent cleanup that still has something to undo.
pub fn undo_last(journal_dir: &Path) -> Result<UndoReport> {
    let j = journal::latest_undoable(journal_dir)?
        .ok_or_else(|| Error::Journal("nothing to undo".into()))?;
    undo_journal(&j.path)
}

pub fn undo_by_id(journal_dir: &Path, id: &str) -> Result<UndoReport> {
    if id.contains(['/', '\\']) || id.contains("..") {
        return Err(Error::Journal("bad journal id".into()));
    }
    let p = journal_dir.join(format!("{id}.jsonl"));
    if !p.exists() {
        return Err(Error::Journal(format!("no cleanup with id {id}")));
    }
    undo_journal(&p)
}

/// Move every file back, newest move first. Never overwrites: if something now
/// sits at the original location the file is restored next to it as `name (1).ext`.
/// Safe to run again after a partial undo; already-restored moves are skipped.
pub fn undo_journal(path: &Path) -> Result<UndoReport> {
    let entries = journal::read_entries(path)?;
    let parsed = journal::parse(&entries);
    let mut w = JournalWriter::open_append(path)?;
    w.write(
        &Entry::UndoStarted {
            at: chrono::Local::now().to_rfc3339(),
        },
        true,
    )?;

    let mut report = UndoReport {
        journal_id: parsed.id.clone(),
        restored: vec![],
        problems: vec![],
        folders_removed: 0,
    };

    for m in parsed.moves.iter().rev() {
        if m.restored || (m.failed && !m.done) {
            continue;
        }
        let moved = m.done || (exists_any(&m.to) && !exists_any(&m.from));
        if !moved {
            continue; // crashed before the rename happened: file never left
        }
        match fs::symlink_metadata(&m.to) {
            // If we only *inferred* the move (crash before the `done` line), insist on the same size.
            Ok(md) if md.is_file() && (m.done || md.len() == m.size) => {}
            Ok(_) => {
                report.problems.push(Problem {
                    path: m.to.clone(),
                    reason: "something other than the file is there now".into(),
                });
                continue;
            }
            Err(_) => {
                report.problems.push(Problem {
                    path: m.to.clone(),
                    reason:
                        "file is no longer where tidydrop put it (moved, renamed or deleted since)"
                            .into(),
                });
                w.write(
                    &Entry::RestoreFailed {
                        seq: m.seq,
                        error: "missing".into(),
                    },
                    false,
                )?;
                continue;
            }
        }
        let Some(orig_dir) = m.from.parent() else {
            continue;
        };
        if !orig_dir.is_dir() {
            report.problems.push(Problem {
                path: m.to.clone(),
                reason: format!("original folder {} no longer exists", orig_dir.display()),
            });
            continue;
        }
        let name = m
            .from
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let mut tried = std::collections::HashSet::new();
        let mut done = None;
        for _ in 0..50 {
            let target = if !exists_any(&m.from) && !tried.contains(&m.from) {
                m.from.clone()
            } else {
                unique_destination(orig_dir, &name, &|p| tried.contains(p))
            };
            tried.insert(target.clone());
            match rename_noreplace(&m.to, &target) {
                Ok(()) => {
                    done = Some(Ok(target));
                    break;
                }
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(e) => {
                    done = Some(Err(e));
                    break;
                }
            }
        }
        match done {
            Some(Ok(target)) => {
                w.write(
                    &Entry::Restored {
                        seq: m.seq,
                        to: target.clone(),
                    },
                    false,
                )?;
                report.restored.push(Restored {
                    renamed: target != m.from,
                    from: m.to.clone(),
                    to: target,
                });
            }
            Some(Err(e)) => {
                w.write(
                    &Entry::RestoreFailed {
                        seq: m.seq,
                        error: e.to_string(),
                    },
                    false,
                )?;
                report.problems.push(Problem {
                    path: m.to.clone(),
                    reason: e.to_string(),
                });
            }
            None => report.problems.push(Problem {
                path: m.to.clone(),
                reason: "could not find a free name".into(),
            }),
        }
    }

    // Remove folders this cleanup created, only if they are now empty.
    for d in sorted_deepest_first(parsed.dirs_created.clone()) {
        if d.starts_with(&parsed.root) && d != parsed.root && fs::remove_dir(&d).is_ok() {
            report.folders_removed += 1;
        }
    }
    w.write(
        &Entry::UndoFinished {
            at: chrono::Local::now().to_rfc3339(),
            restored: report.restored.len(),
            problems: report.problems.len(),
        },
        true,
    )?;
    Ok(report)
}
