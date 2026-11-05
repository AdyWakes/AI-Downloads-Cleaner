//! Execute a plan: journaled, no-overwrite moves. Never deletes, never copies.

use crate::config::Config;
use crate::error::{Error, Result};
use crate::journal::{new_id, Entry, JournalWriter};
use crate::paths::{self, is_within, rename_noreplace};
use crate::plan::Plan;
use crate::scanner::{self, SkipReason};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::io;
use std::path::{Component, Path, PathBuf};
use std::time::SystemTime;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Moved {
    pub id: usize,
    pub from: PathBuf,
    pub to: PathBuf,
    pub category: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotMoved {
    pub id: usize,
    pub name: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApplyReport {
    pub journal_id: String,
    pub moved: Vec<Moved>,
    /// Files left in place on purpose (changed since preview, now in use, …).
    pub skipped: Vec<NotMoved>,
    /// Files that hit an error. They are still in their original place.
    pub failed: Vec<NotMoved>,
}

#[derive(Debug, Clone, Copy, Serialize)]
pub struct Progress {
    pub done: usize,
    pub total: usize,
}

/// Create `root/rel` one component at a time, refusing to walk through symlinks
/// (which could point outside the folder) and journaling each folder we create.
fn ensure_dir(
    root: &Path,
    rel: &Path,
    journal: &mut JournalWriter,
    created: &mut HashSet<PathBuf>,
) -> Result<PathBuf> {
    let mut cur = root.to_path_buf();
    for c in rel.components() {
        match c {
            Component::Normal(part) => cur.push(part),
            _ => {
                return Err(Error::Unsafe(format!(
                    "bad destination folder {}",
                    rel.display()
                )))
            }
        }
        match fs::symlink_metadata(&cur) {
            Ok(md) if md.file_type().is_symlink() => {
                return Err(Error::Unsafe(format!(
                    "{} is a link; refusing to move files through it",
                    cur.display()
                )))
            }
            Ok(md) if md.is_dir() => {
                #[cfg(windows)]
                {
                    use std::os::windows::fs::MetadataExt;
                    if md.file_attributes() & 0x400 != 0 {
                        return Err(Error::Unsafe(format!(
                            "{} is a junction; refusing",
                            cur.display()
                        )));
                    }
                }
            }
            Ok(_) => {
                return Err(Error::Other(format!(
                    "a file named '{}' is in the way of the destination folder",
                    cur.file_name().unwrap_or_default().to_string_lossy()
                )))
            }
            Err(e) if e.kind() == io::ErrorKind::NotFound => {
                // Journal before creating so undo knows about it even after a crash.
                journal.write(&Entry::DirCreated { path: cur.clone() }, true)?;
                match fs::create_dir(&cur) {
                    Ok(()) => {
                        created.insert(cur.clone());
                    }
                    Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {}
                    Err(e) => return Err(e.into()),
                }
            }
            Err(e) => return Err(e.into()),
        }
    }
    Ok(cur)
}

/// Move every enabled item of `plan`. Each file is re-checked right before it is moved.
pub fn apply_plan(
    plan: &Plan,
    config: &Config,
    journal_dir: &Path,
    progress: &mut dyn FnMut(Progress),
) -> Result<ApplyReport> {
    // The root must still be the same safe folder.
    let root = paths::check_root(&plan.root)?;
    if root != plan.root {
        return Err(Error::Unsafe(
            "the folder changed location since the preview".into(),
        ));
    }
    let items: Vec<_> = plan.items.iter().filter(|i| i.enabled).collect();
    let id = new_id();
    let mut journal = JournalWriter::create(journal_dir, &id)?;
    journal.write(
        &Entry::Header {
            version: crate::journal::JOURNAL_VERSION,
            id: id.clone(),
            root: root.clone(),
            created_at: chrono::Local::now().to_rfc3339(),
            organize_by_month: plan.organize_by_month,
            planned: items.len(),
        },
        true,
    )?;

    let mut report = ApplyReport {
        journal_id: id,
        moved: vec![],
        skipped: vec![],
        failed: vec![],
    };
    let mut created_dirs = HashSet::new();
    let total = items.len();
    let now = SystemTime::now();
    let mut seq = 0usize;
    let siblings = scanner::sibling_names(&root);

    for (n, item) in items.iter().enumerate() {
        progress(Progress { done: n, total });
        let not = |reason: String| NotMoved {
            id: item.id,
            name: item.name.clone(),
            reason,
        };

        // 1. Source must be directly inside the root.
        if item.source.parent() != Some(root.as_path())
            || item.source.file_name().and_then(|n| n.to_str()) != Some(item.name.as_str())
        {
            report
                .skipped
                .push(not("not directly inside the chosen folder".into()));
            continue;
        }
        // 2. Re-run every skip check against the file as it is *now*.
        let current = match scanner::check_file(&item.source, config, &siblings, now) {
            Ok(f) => f,
            Err(s) => {
                let why = if s.reason == SkipReason::Unreadable {
                    "file is gone or unreadable".to_string()
                } else {
                    s.reason.describe().to_string()
                };
                report.skipped.push(not(why));
                continue;
            }
        };
        if current.size != item.size || current.modified != item.modified {
            report.skipped.push(not("changed since the preview".into()));
            continue;
        }
        // 3. Destination folder, recomputed from the plan's relative path and kept inside root.
        let rel = paths::sanitize_relative(&item.dest_rel);
        if rel.components().next().is_none() {
            report.skipped.push(not("no destination".into()));
            continue;
        }
        let dest_dir = match ensure_dir(&root, &rel, &mut journal, &mut created_dirs) {
            Ok(d) => d,
            Err(e) => {
                report.failed.push(not(e.to_string()));
                continue;
            }
        };
        if !is_within(&root, &dest_dir) {
            report
                .failed
                .push(not("destination outside the folder".into()));
            continue;
        }
        // 4. Pick a free name and move without ever overwriting.
        let mut attempt = 0u32;
        let mut tried: HashSet<PathBuf> = HashSet::new();
        let result = loop {
            let target = paths::unique_destination(&dest_dir, &item.name, &|p| tried.contains(p));
            tried.insert(target.clone());
            seq += 1;
            journal.write(
                &Entry::Intent {
                    seq,
                    from: item.source.clone(),
                    to: target.clone(),
                    size: item.size,
                },
                true,
            )?;
            match rename_noreplace(&item.source, &target) {
                Ok(()) => {
                    journal.write(&Entry::Done { seq }, false)?;
                    break Ok(target);
                }
                Err(e) if e.kind() == io::ErrorKind::AlreadyExists && attempt < 50 => {
                    // Someone created that name between our check and the rename. Try the next one.
                    journal.write(
                        &Entry::Failed {
                            seq,
                            error: "name taken, retrying".into(),
                        },
                        false,
                    )?;
                    attempt += 1;
                }
                Err(e) => {
                    journal.write(
                        &Entry::Failed {
                            seq,
                            error: e.to_string(),
                        },
                        false,
                    )?;
                    break Err(e);
                }
            }
        };
        match result {
            Ok(to) => report.moved.push(Moved {
                id: item.id,
                from: item.source.clone(),
                to,
                category: item.category.clone(),
            }),
            Err(e) => {
                let msg = if is_cross_device(&e) {
                    "destination is on a different drive".to_string()
                } else {
                    e.to_string()
                };
                report.failed.push(not(msg));
            }
        }
    }
    progress(Progress { done: total, total });
    // Remove any folder we created but didn't end up using (all moves into it failed).
    for d in sorted_deepest_first(created_dirs.into_iter().collect()) {
        let _ = fs::remove_dir(&d); // only succeeds if empty
    }
    journal.write(
        &Entry::Finished {
            moved: report.moved.len(),
            failed: report.failed.len(),
            skipped: report.skipped.len(),
        },
        true,
    )?;
    Ok(report)
}

fn is_cross_device(e: &io::Error) -> bool {
    #[cfg(unix)]
    {
        e.raw_os_error() == Some(libc::EXDEV)
    }
    #[cfg(windows)]
    {
        e.raw_os_error() == Some(17) // ERROR_NOT_SAME_DEVICE
    }
}

pub(crate) fn sorted_deepest_first(mut dirs: Vec<PathBuf>) -> Vec<PathBuf> {
    dirs.sort_by_key(|d| std::cmp::Reverse(d.components().count()));
    dirs
}
