//! Path helpers: sanitizing folder names, collision-free names, no-overwrite
//! renames, and "is this a safe folder to clean" checks.

use crate::error::{Error, Result};
use std::io;
use std::path::{Component, Path, PathBuf};

const WINDOWS_RESERVED: &[&str] = &[
    "con", "prn", "aux", "nul", "com1", "com2", "com3", "com4", "com5", "com6", "com7", "com8",
    "com9", "lpt1", "lpt2", "lpt3", "lpt4", "lpt5", "lpt6", "lpt7", "lpt8", "lpt9",
];

/// Turn arbitrary text into a single safe folder name on every OS.
/// Replaces `\ / : * ? " < > |` and control chars, strips trailing dots/spaces,
/// avoids Windows reserved names and `.`/`..`.
pub fn sanitize_component(name: &str) -> String {
    let mut out: String = name
        .chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '-',
            c if c.is_control() => '-',
            c => c,
        })
        .collect();
    out = out.trim().trim_end_matches(['.', ' ']).to_string();
    while out.starts_with('.') {
        out.remove(0);
    }
    if out.is_empty() {
        out = "Other".into();
    }
    let stem = out.split('.').next().unwrap_or("").to_lowercase();
    if WINDOWS_RESERVED.contains(&stem.as_str()) {
        out = format!("_{out}");
    }
    // Keep folder names reasonably short (Windows path budget).
    if out.chars().count() > 80 {
        out = out
            .chars()
            .take(80)
            .collect::<String>()
            .trim_end()
            .to_string();
    }
    out
}

/// Sanitize a user-entered relative sub-path like `Work/Taxes 2024`.
/// Each component is sanitized; empty, `.` and `..` components are dropped,
/// so the result can never escape the folder it's joined to.
pub fn sanitize_relative(path: &str) -> PathBuf {
    let mut out = PathBuf::new();
    for part in path.split(['/', '\\']) {
        let t = part.trim();
        if t.is_empty() || t == "." || t == ".." {
            continue;
        }
        out.push(sanitize_component(t));
    }
    out
}

/// Split `report.tar.gz` into (`report`, `.tar.gz`), `photo.jpg` into (`photo`, `.jpg`),
/// `.env`-style or extensionless names into (name, "").
pub fn split_name(file_name: &str) -> (String, String) {
    let lower = file_name.to_lowercase();
    for double in [".tar.gz", ".tar.bz2", ".tar.xz", ".tar.zst"] {
        if lower.ends_with(double) && file_name.len() > double.len() {
            let cut = file_name.len() - double.len();
            return (file_name[..cut].to_string(), file_name[cut..].to_string());
        }
    }
    match file_name.rfind('.') {
        Some(0) | None => (file_name.to_string(), String::new()),
        Some(i) => (file_name[..i].to_string(), file_name[i..].to_string()),
    }
}

/// `name (n).ext`
pub fn numbered_name(file_name: &str, n: u32) -> String {
    if n == 0 {
        return file_name.to_string();
    }
    let (stem, ext) = split_name(file_name);
    format!("{stem} ({n}){ext}")
}

/// Does anything (file, dir, broken symlink) exist at `p`?
pub fn exists_any(p: &Path) -> bool {
    std::fs::symlink_metadata(p).is_ok()
}

/// First free path `dir/name`, `dir/name (1)`, `dir/name (2)`, …
/// `taken` lets callers reserve names not yet on disk (e.g. within one plan).
pub fn unique_destination(dir: &Path, file_name: &str, taken: &dyn Fn(&Path) -> bool) -> PathBuf {
    let mut n = 0u32;
    loop {
        let candidate = dir.join(numbered_name(file_name, n));
        if !exists_any(&candidate) && !taken(&candidate) {
            return candidate;
        }
        n += 1;
    }
}

/// Is `child` lexically inside `root` (or equal)? Both should be absolute and normalized.
pub fn is_within(root: &Path, child: &Path) -> bool {
    if child
        .components()
        .any(|c| matches!(c, Component::ParentDir))
    {
        return false;
    }
    child.starts_with(root)
}

/// Second half of the hard-link fallback. If the old name can't be removed, remove
/// the new link again (the data is still at `from`), so the file isn't left in two places.
#[cfg(unix)]
#[allow(dead_code)]
fn unlink_after_link(from: &Path, to: &Path) -> io::Result<()> {
    match std::fs::remove_file(from) {
        Ok(()) => Ok(()),
        Err(e) => {
            let _ = std::fs::remove_file(to);
            Err(e)
        }
    }
}

/// Rename `from` to `to`, failing with `AlreadyExists` instead of overwriting.
///
/// `std::fs::rename` silently replaces an existing destination on every OS, which
/// would destroy a file. We use the OS no-replace primitives instead.
pub fn rename_noreplace(from: &Path, to: &Path) -> io::Result<()> {
    imp::rename_noreplace(from, to)
}

#[cfg(target_os = "linux")]
mod imp {
    use super::*;
    use std::ffi::CString;
    use std::os::unix::ffi::OsStrExt;

    pub fn rename_noreplace(from: &Path, to: &Path) -> io::Result<()> {
        let f = CString::new(from.as_os_str().as_bytes())?;
        let t = CString::new(to.as_os_str().as_bytes())?;
        // SAFETY: valid NUL-terminated strings; AT_FDCWD for relative/absolute paths.
        let r = unsafe {
            libc::renameat2(
                libc::AT_FDCWD,
                f.as_ptr(),
                libc::AT_FDCWD,
                t.as_ptr(),
                libc::RENAME_NOREPLACE,
            )
        };
        if r == 0 {
            return Ok(());
        }
        let err = io::Error::last_os_error();
        match err.raw_os_error() {
            // Filesystem or kernel doesn't support RENAME_NOREPLACE (e.g. some FUSE/NFS).
            Some(libc::EINVAL) | Some(libc::ENOSYS) | Some(libc::ENOTSUP) => fallback(from, to),
            _ => Err(err),
        }
    }

    /// Best effort: hard-link (atomic, fails if exists) then unlink the old name.
    /// If hard links aren't supported either, check-then-rename.
    fn fallback(from: &Path, to: &Path) -> io::Result<()> {
        match std::fs::hard_link(from, to) {
            Ok(()) => unlink_after_link(from, to),
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => Err(e),
            Err(_) => {
                if exists_any(to) {
                    return Err(io::Error::new(
                        io::ErrorKind::AlreadyExists,
                        "destination exists",
                    ));
                }
                std::fs::rename(from, to)
            }
        }
    }
}

#[cfg(target_os = "macos")]
mod imp {
    use super::*;
    use std::ffi::CString;
    use std::os::unix::ffi::OsStrExt;

    extern "C" {
        fn renamex_np(
            from: *const libc::c_char,
            to: *const libc::c_char,
            flags: libc::c_uint,
        ) -> libc::c_int;
    }
    const RENAME_EXCL: libc::c_uint = 0x0000_0004;

    pub fn rename_noreplace(from: &Path, to: &Path) -> io::Result<()> {
        let f = CString::new(from.as_os_str().as_bytes())?;
        let t = CString::new(to.as_os_str().as_bytes())?;
        // SAFETY: valid NUL-terminated strings.
        let r = unsafe { renamex_np(f.as_ptr(), t.as_ptr(), RENAME_EXCL) };
        if r == 0 {
            return Ok(());
        }
        let err = io::Error::last_os_error();
        match err.raw_os_error() {
            Some(libc::ENOTSUP) | Some(libc::EINVAL) => {
                if exists_any(to) {
                    return Err(io::Error::new(
                        io::ErrorKind::AlreadyExists,
                        "destination exists",
                    ));
                }
                std::fs::rename(from, to)
            }
            _ => Err(err),
        }
    }
}

#[cfg(all(unix, not(any(target_os = "linux", target_os = "macos"))))]
mod imp {
    use super::*;
    pub fn rename_noreplace(from: &Path, to: &Path) -> io::Result<()> {
        match std::fs::hard_link(from, to) {
            Ok(()) => unlink_after_link(from, to),
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => Err(e),
            Err(_) => {
                if exists_any(to) {
                    return Err(io::Error::new(
                        io::ErrorKind::AlreadyExists,
                        "destination exists",
                    ));
                }
                std::fs::rename(from, to)
            }
        }
    }
}

#[cfg(windows)]
mod imp {
    use super::*;
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::MoveFileExW;

    pub fn rename_noreplace(from: &Path, to: &Path) -> io::Result<()> {
        let f = wide(&verbatim(from));
        let t = wide(&verbatim(to));
        // Flags = 0: no MOVEFILE_REPLACE_EXISTING (fails if `to` exists) and no
        // MOVEFILE_COPY_ALLOWED (never silently copy+delete across drives).
        // SAFETY: valid NUL-terminated UTF-16 strings.
        let ok = unsafe { MoveFileExW(f.as_ptr(), t.as_ptr(), 0) };
        if ok != 0 {
            Ok(())
        } else {
            let err = io::Error::last_os_error();
            // ERROR_ALREADY_EXISTS (183) / ERROR_FILE_EXISTS (80)
            match err.raw_os_error() {
                Some(183) | Some(80) => Err(io::Error::new(io::ErrorKind::AlreadyExists, err)),
                _ => Err(err),
            }
        }
    }

    fn wide(p: &Path) -> Vec<u16> {
        p.as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect()
    }

    /// `C:\x` -> `\\?\C:\x`, `\\server\share\x` -> `\\?\UNC\server\share\x`, so paths
    /// longer than 260 characters work. Paths must already be absolute.
    pub fn verbatim(p: &Path) -> PathBuf {
        let s = p.as_os_str().to_string_lossy();
        if s.starts_with(r"\\?\") || !p.is_absolute() {
            return p.to_path_buf();
        }
        let s = s.replace('/', "\\");
        if let Some(rest) = s.strip_prefix(r"\\") {
            PathBuf::from(format!(r"\\?\UNC\{rest}"))
        } else {
            PathBuf::from(format!(r"\\?\{s}"))
        }
    }
}

/// Canonicalize a folder the user picked and refuse folders that are dangerous to
/// reorganize (filesystem roots, OS/program folders, the home folder itself).
pub fn check_root(folder: &Path) -> Result<PathBuf> {
    let md = std::fs::metadata(folder)
        .map_err(|e| Error::Unsafe(format!("Can't open folder {}: {e}", folder.display())))?;
    if !md.is_dir() {
        return Err(Error::Unsafe(format!(
            "{} is not a folder",
            folder.display()
        )));
    }
    let root = dunce::canonicalize(folder)?;
    if root.parent().is_none() {
        return Err(Error::Unsafe(
            "Refusing to clean the top level of a drive. Pick a folder like Downloads.".into(),
        ));
    }
    if let Some(home) = dirs::home_dir().and_then(|h| dunce::canonicalize(h).ok()) {
        if root == home {
            return Err(Error::Unsafe(
                "Refusing to clean your whole home folder. Pick a folder inside it, like Downloads.".into(),
            ));
        }
    }
    let lower = root.to_string_lossy().to_lowercase().replace('\\', "/");
    let blocked_prefixes: &[&str] = if cfg!(windows) {
        &[
            "c:/windows",
            "c:/program files",
            "c:/program files (x86)",
            "c:/programdata",
        ]
    } else if cfg!(target_os = "macos") {
        &[
            "/system",
            "/library",
            "/applications",
            "/usr",
            "/bin",
            "/sbin",
            "/etc",
            "/private/etc",
            "/opt",
        ]
    } else {
        &[
            "/usr", "/bin", "/sbin", "/etc", "/lib", "/lib64", "/boot", "/proc", "/sys", "/dev",
            "/var", "/opt", "/snap", "/run",
        ]
    };
    for b in blocked_prefixes {
        if lower == *b || lower.starts_with(&format!("{b}/")) {
            return Err(Error::Unsafe(format!(
                "Refusing to reorganize a system folder ({}).",
                root.display()
            )));
        }
    }
    // Windows: also refuse anything under the real %WINDIR% / %ProgramFiles%.
    for var in ["WINDIR", "ProgramFiles", "ProgramFiles(x86)", "ProgramData"] {
        if let Some(v) = std::env::var_os(var) {
            if let Ok(sys) = dunce::canonicalize(v) {
                if root.starts_with(&sys) {
                    return Err(Error::Unsafe(format!(
                        "Refusing to reorganize a system folder ({}).",
                        root.display()
                    )));
                }
            }
        }
    }
    Ok(root)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn sanitize_examples() {
        assert_eq!(sanitize_component("College/Study"), "College-Study");
        assert_eq!(sanitize_component("a:b*c?"), "a-b-c-");
        assert_eq!(sanitize_component("  Trailing. . "), "Trailing");
        assert_eq!(sanitize_component("CON"), "_CON");
        assert_eq!(sanitize_component(".."), "Other");
        assert_eq!(sanitize_component("बिल और रसीद"), "बिल और रसीद");
        assert_eq!(
            sanitize_relative("../../etc/passwd"),
            PathBuf::from("etc").join("passwd")
        );
        assert_eq!(
            sanitize_relative("Work\\Taxes 2024/"),
            PathBuf::from("Work").join("Taxes 2024")
        );
    }

    #[test]
    fn split_and_number() {
        assert_eq!(split_name("a.pdf"), ("a".into(), ".pdf".into()));
        assert_eq!(split_name("a.tar.gz"), ("a".into(), ".tar.gz".into()));
        assert_eq!(split_name(".bashrc"), (".bashrc".into(), "".into()));
        assert_eq!(split_name("README"), ("README".into(), "".into()));
        assert_eq!(numbered_name("photo.jpg", 1), "photo (1).jpg");
        assert_eq!(numbered_name("backup.tar.gz", 2), "backup (2).tar.gz");
        assert_eq!(numbered_name("रसीद.pdf", 1), "रसीद (1).pdf");
    }

    #[test]
    fn unique_destination_skips_existing() {
        let d = tempfile::tempdir().unwrap();
        fs::write(d.path().join("a.pdf"), "x").unwrap();
        fs::write(d.path().join("a (1).pdf"), "x").unwrap();
        let p = unique_destination(d.path(), "a.pdf", &|_| false);
        assert_eq!(p.file_name().unwrap(), "a (2).pdf");
        let reserved = d.path().join("a (2).pdf");
        let p = unique_destination(d.path(), "a.pdf", &|c| c == reserved);
        assert_eq!(p.file_name().unwrap(), "a (3).pdf");
    }

    #[test]
    fn rename_noreplace_never_overwrites() {
        let d = tempfile::tempdir().unwrap();
        let a = d.path().join("a.txt");
        let b = d.path().join("b.txt");
        fs::write(&a, "AAA").unwrap();
        fs::write(&b, "BBB").unwrap();
        let err = rename_noreplace(&a, &b).unwrap_err();
        assert_eq!(err.kind(), io::ErrorKind::AlreadyExists);
        assert_eq!(fs::read_to_string(&a).unwrap(), "AAA");
        assert_eq!(fs::read_to_string(&b).unwrap(), "BBB");
        let c = d.path().join("c.txt");
        rename_noreplace(&a, &c).unwrap();
        assert!(!a.exists());
        assert_eq!(fs::read_to_string(&c).unwrap(), "AAA");
    }

    #[test]
    fn is_within_rejects_escape() {
        let root = Path::new("/x/root");
        assert!(is_within(root, Path::new("/x/root/a/b")));
        assert!(!is_within(root, Path::new("/x/root/../etc")));
        assert!(!is_within(root, Path::new("/x/rootkit/a")));
    }

    #[test]
    fn check_root_refuses_fs_root() {
        let r = if cfg!(windows) {
            Path::new("C:\\")
        } else {
            Path::new("/")
        };
        assert!(check_root(r).is_err());
        let d = tempfile::tempdir().unwrap();
        assert!(check_root(d.path()).is_ok());
        assert!(check_root(&d.path().join("missing")).is_err());
    }
}
