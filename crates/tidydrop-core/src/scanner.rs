//! Scan the top level of a folder and decide which files are safe to touch.
//!
//! Only files directly inside the folder are considered. Sub-folders are left
//! completely alone (they are either the user's own organisation or ours).

use crate::config::Config;
use crate::paths;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs::{self, Metadata};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FileEntry {
    pub path: PathBuf,
    pub name: String,
    pub size: u64,
    /// Unix seconds.
    pub modified: i64,
    /// Lowercase extension without dot.
    pub ext: String,
    /// Cloud placeholder (e.g. OneDrive "online-only"): never read its contents.
    pub cloud_only: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum SkipReason {
    Directory,
    Symlink,
    Hidden,
    System,
    PartialDownload,
    RecentlyModified,
    InUse,
    BadName,
    Unreadable,
}

impl SkipReason {
    pub fn describe(&self) -> &'static str {
        match self {
            SkipReason::Directory => "folder (left as is)",
            SkipReason::Symlink => "shortcut / link",
            SkipReason::Hidden => "hidden file",
            SkipReason::System => "system file",
            SkipReason::PartialDownload => "download still in progress",
            SkipReason::RecentlyModified => "changed in the last few minutes",
            SkipReason::InUse => "open in another program",
            SkipReason::BadName => "filename can't be handled safely",
            SkipReason::Unreadable => "can't read file info",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Skipped {
    pub name: String,
    pub reason: SkipReason,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanResult {
    pub root: PathBuf,
    pub files: Vec<FileEntry>,
    pub skipped: Vec<Skipped>,
    /// Things the UI should warn about (e.g. "this looks like a code project").
    pub warnings: Vec<String>,
}

/// Extensions used by browsers / download managers for incomplete files.
pub const PARTIAL_EXTS: &[&str] = &[
    "crdownload",
    "part",
    "partial",
    "download",
    "tmp",
    "temp",
    "opdownload",
    "!ut",
    "!qb",
    "bc!",
    "aria2",
    "filepart",
    "dtapart",
    "unconfirmed",
    "lock",
];

/// Names that are OS metadata and must never be moved.
pub const SYSTEM_NAMES: &[&str] = &[
    "desktop.ini",
    "thumbs.db",
    "ehthumbs.db",
    ".ds_store",
    "icon\r",
    ".localized",
];

pub fn is_partial_name(name: &str) -> bool {
    let lower = name.to_lowercase();
    if lower.starts_with("~$") || lower.starts_with(".~lock.") || lower.starts_with("unconfirmed ")
    {
        return true;
    }
    match lower.rsplit_once('.') {
        Some((_, ext)) => PARTIAL_EXTS.contains(&ext),
        None => false,
    }
}

/// Is this file changing right now, or was it just finished?
pub fn is_recent(md: &Metadata, threshold: Duration, now: SystemTime) -> bool {
    if threshold.is_zero() {
        return false;
    }
    let mut times = vec![md.modified().ok()];
    // On Windows, files copied/downloaded in keep an old mtime but get a fresh creation time.
    if cfg!(windows) {
        times.push(md.created().ok());
    }
    times
        .into_iter()
        .flatten()
        .any(|t| match now.duration_since(t) {
            Ok(age) => age < threshold,
            // In the future: treat "slightly ahead" (clock skew) as recent, far future as not.
            Err(e) => e.duration() < threshold,
        })
}

#[cfg(windows)]
fn win_attrs(md: &Metadata) -> u32 {
    use std::os::windows::fs::MetadataExt;
    md.file_attributes()
}

#[cfg(windows)]
const FILE_ATTRIBUTE_HIDDEN: u32 = 0x2;
#[cfg(windows)]
const FILE_ATTRIBUTE_SYSTEM: u32 = 0x4;
#[cfg(windows)]
const FILE_ATTRIBUTE_TEMPORARY: u32 = 0x100;
#[cfg(windows)]
const FILE_ATTRIBUTE_REPARSE_POINT: u32 = 0x400;
#[cfg(windows)]
const FILE_ATTRIBUTE_OFFLINE: u32 = 0x1000;
#[cfg(windows)]
const FILE_ATTRIBUTE_RECALL_ON_OPEN: u32 = 0x40000;
#[cfg(windows)]
const FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS: u32 = 0x400000;

fn is_hidden(name: &str, _md: &Metadata) -> bool {
    if name.starts_with('.') {
        return true;
    }
    #[cfg(windows)]
    {
        if win_attrs(_md) & FILE_ATTRIBUTE_HIDDEN != 0 {
            return true;
        }
    }
    #[cfg(target_os = "macos")]
    {
        use std::os::macos::fs::MetadataExt;
        const UF_HIDDEN: u32 = 0x8000;
        if _md.st_flags() & UF_HIDDEN != 0 {
            return true;
        }
    }
    false
}

fn is_system(name: &str, _md: &Metadata) -> bool {
    if SYSTEM_NAMES.contains(&name.to_lowercase().as_str()) {
        return true;
    }
    #[cfg(windows)]
    {
        let a = win_attrs(_md);
        if a & (FILE_ATTRIBUTE_SYSTEM | FILE_ATTRIBUTE_TEMPORARY) != 0 {
            return true;
        }
    }
    false
}

fn is_cloud_only(_md: &Metadata) -> bool {
    #[cfg(windows)]
    {
        let a = win_attrs(_md);
        return a
            & (FILE_ATTRIBUTE_OFFLINE
                | FILE_ATTRIBUTE_RECALL_ON_OPEN
                | FILE_ATTRIBUTE_RECALL_ON_DATA_ACCESS)
            != 0;
    }
    #[allow(unreachable_code)]
    false
}

fn is_link(md: &Metadata) -> bool {
    if md.file_type().is_symlink() {
        return true;
    }
    #[cfg(windows)]
    {
        if win_attrs(md) & FILE_ATTRIBUTE_REPARSE_POINT != 0 {
            return true;
        }
    }
    false
}

/// Is another program holding the file open? (Windows: exclusive open fails.)
/// On macOS/Linux there are no mandatory locks; moving an open file is harmless
/// there, and the "recently modified" rule catches active downloads.
pub fn is_in_use(_path: &Path) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        const GENERIC_READ: u32 = 0x8000_0000;
        let r = fs::OpenOptions::new()
            .access_mode(GENERIC_READ)
            .share_mode(0)
            .open(_path);
        return match r {
            Ok(_) => false,
            // 32 = ERROR_SHARING_VIOLATION, 33 = ERROR_LOCK_VIOLATION
            Err(e) => matches!(e.raw_os_error(), Some(32) | Some(33)),
        };
    }
    #[allow(unreachable_code)]
    false
}

/// Check one top-level path. `Ok(entry)` if it may be moved, `Err(skip)` otherwise.
/// Used at scan time *and again right before each move*.
pub fn check_file(
    path: &Path,
    config: &Config,
    siblings: &HashSet<String>,
    now: SystemTime,
) -> Result<FileEntry, Skipped> {
    let raw_name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let skip = |reason: SkipReason, detail: String| Skipped {
        name: raw_name.clone(),
        reason,
        detail,
    };

    let name = match path.file_name().and_then(|n| n.to_str()) {
        Some(n) => n.to_string(),
        None => return Err(skip(SkipReason::BadName, "not valid Unicode".into())),
    };
    let md = fs::symlink_metadata(path).map_err(|e| skip(SkipReason::Unreadable, e.to_string()))?;
    if is_link(&md) {
        return Err(skip(SkipReason::Symlink, String::new()));
    }
    if md.is_dir() {
        return Err(skip(SkipReason::Directory, String::new()));
    }
    if !md.is_file() {
        return Err(skip(SkipReason::System, "not a regular file".into()));
    }
    if is_system(&name, &md) {
        return Err(skip(SkipReason::System, String::new()));
    }
    if is_hidden(&name, &md) {
        return Err(skip(SkipReason::Hidden, String::new()));
    }
    if is_partial_name(&name) {
        return Err(skip(SkipReason::PartialDownload, String::new()));
    }
    // Firefox/Safari etc. create `file.zip` next to `file.zip.part` while downloading.
    let lower = name.to_lowercase();
    if PARTIAL_EXTS
        .iter()
        .any(|e| siblings.contains(&format!("{lower}.{e}")))
    {
        return Err(skip(
            SkipReason::PartialDownload,
            "a matching partial download exists".into(),
        ));
    }
    if is_recent(&md, Duration::from_secs(config.skip_recent_secs), now) {
        return Err(skip(SkipReason::RecentlyModified, String::new()));
    }
    if is_in_use(path) {
        return Err(skip(SkipReason::InUse, String::new()));
    }
    let modified = md
        .modified()
        .ok()
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    Ok(FileEntry {
        path: path.to_path_buf(),
        ext: crate::rules::extension(&name),
        name,
        size: md.len(),
        modified,
        cloud_only: is_cloud_only(&md),
    })
}

/// Lowercased names of everything directly in `root` (for sibling checks).
pub fn sibling_names(root: &Path) -> HashSet<String> {
    fs::read_dir(root)
        .map(|rd| {
            rd.flatten()
                .map(|e| e.file_name().to_string_lossy().to_lowercase())
                .collect()
        })
        .unwrap_or_default()
}

/// Scan the top level of `folder`. Refuses dangerous folders (see [`paths::check_root`]).
pub fn scan(folder: &Path, config: &Config) -> crate::Result<ScanResult> {
    let root = paths::check_root(folder)?;
    let now = SystemTime::now();
    let siblings = sibling_names(&root);
    let mut files = Vec::new();
    let mut skipped = Vec::new();
    let mut warnings = Vec::new();

    for marker in [".git", "package.json", "Cargo.toml", ".svn", "pom.xml"] {
        if root.join(marker).exists() {
            warnings.push(format!(
                "This folder contains {marker}, so it looks like a project folder. Moving files here may break it."
            ));
            break;
        }
    }

    for entry in fs::read_dir(&root)? {
        let entry = match entry {
            Ok(e) => e,
            Err(e) => {
                skipped.push(Skipped {
                    name: "?".into(),
                    reason: SkipReason::Unreadable,
                    detail: e.to_string(),
                });
                continue;
            }
        };
        match check_file(&entry.path(), config, &siblings, now) {
            Ok(f) => files.push(f),
            Err(s) => {
                // Folders are expected; don't clutter the skipped list with them.
                if s.reason != SkipReason::Directory {
                    skipped.push(s)
                }
            }
        }
    }
    files.sort_by_key(|a| a.name.to_lowercase());
    skipped.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(ScanResult {
        root,
        files,
        skipped,
        warnings,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use filetime::FileTime;

    fn old(p: &Path) {
        let t = FileTime::from_unix_time(1_600_000_000, 0);
        filetime::set_file_mtime(p, t).unwrap();
    }

    fn cfg() -> Config {
        Config::default()
    }

    #[test]
    fn partial_names() {
        for n in [
            "movie.mp4.crdownload",
            "a.zip.part",
            "x.tmp",
            "Unconfirmed 1234.crdownload",
            "~$report.docx",
            "file.PART",
            "big.iso.!qB",
            "f.opdownload",
        ] {
            assert!(is_partial_name(n), "{n}");
        }
        for n in ["report.pdf", "party.mp3", "temple.jpg", "partners.docx"] {
            assert!(!is_partial_name(n), "{n}");
        }
    }

    #[test]
    fn scan_skips_unsafe_things() {
        let d = tempfile::tempdir().unwrap();
        let r = d.path();
        let mk = |n: &str| {
            let p = r.join(n);
            fs::write(&p, n).unwrap();
            old(&p);
            p
        };
        mk("ok.pdf");
        mk("रसीद.pdf");
        mk(".hidden.txt");
        mk("desktop.ini");
        mk("movie.mp4.crdownload");
        mk("song.mp3");
        mk("song.mp3.part"); // makes song.mp3 a companion of a partial download
        fs::create_dir(r.join("Existing Folder")).unwrap();
        fs::write(r.join("Existing Folder").join("inner.pdf"), "x").unwrap();
        fs::write(r.join("fresh.docx"), "new").unwrap(); // mtime = now -> recent

        let mut c = cfg();
        if cfg!(windows) {
            // Windows also checks creation time, which tests can't backdate.
            c.skip_recent_secs = 0;
            fs::remove_file(r.join("fresh.docx")).unwrap();
        }
        let res = scan(r, &c).unwrap();
        let names: Vec<_> = res.files.iter().map(|f| f.name.as_str()).collect();
        assert_eq!(names, vec!["ok.pdf", "रसीद.pdf"]);
        let reason = |n: &str| res.skipped.iter().find(|s| s.name == n).map(|s| s.reason);
        assert_eq!(reason(".hidden.txt"), Some(SkipReason::Hidden));
        assert_eq!(reason("desktop.ini"), Some(SkipReason::System));
        assert_eq!(
            reason("movie.mp4.crdownload"),
            Some(SkipReason::PartialDownload)
        );
        assert_eq!(reason("song.mp3"), Some(SkipReason::PartialDownload));
        assert_eq!(reason("song.mp3.part"), Some(SkipReason::PartialDownload));
        if !cfg!(windows) {
            assert_eq!(reason("fresh.docx"), Some(SkipReason::RecentlyModified));
        }
        // sub-folder contents never appear
        assert!(!names.contains(&"inner.pdf"));
    }

    #[cfg(unix)]
    #[test]
    fn scan_skips_symlinks() {
        let d = tempfile::tempdir().unwrap();
        let target = d.path().join("real.pdf");
        fs::write(&target, "x").unwrap();
        old(&target);
        std::os::unix::fs::symlink(&target, d.path().join("link.pdf")).unwrap();
        let res = scan(d.path(), &cfg()).unwrap();
        assert_eq!(res.files.len(), 1);
        assert_eq!(res.skipped[0].reason, SkipReason::Symlink);
    }

    #[test]
    fn recent_threshold_zero_disables() {
        let d = tempfile::tempdir().unwrap();
        fs::write(d.path().join("fresh.txt"), "x").unwrap();
        let mut c = cfg();
        c.skip_recent_secs = 0;
        assert_eq!(scan(d.path(), &c).unwrap().files.len(), 1);
    }

    #[test]
    fn warns_on_project_folder() {
        let d = tempfile::tempdir().unwrap();
        fs::create_dir(d.path().join(".git")).unwrap();
        assert_eq!(scan(d.path(), &cfg()).unwrap().warnings.len(), 1);
    }

    #[cfg(windows)]
    #[test]
    fn in_use_detected_on_windows() {
        use std::os::windows::fs::OpenOptionsExt;
        let d = tempfile::tempdir().unwrap();
        let p = d.path().join("open.docx");
        fs::write(&p, "x").unwrap();
        let _h = fs::OpenOptions::new()
            .read(true)
            .share_mode(0)
            .open(&p)
            .unwrap();
        assert!(is_in_use(&p));
    }
}
