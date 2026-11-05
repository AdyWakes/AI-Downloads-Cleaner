//! Folder statistics and duplicate detection (by content hash). Duplicates are
//! only *reported*; tidydrop never deletes them.

use crate::scanner::{FileEntry, ScanResult};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileBrief {
    pub name: String,
    pub path: PathBuf,
    pub size: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DuplicateGroup {
    pub hash: String,
    pub size: u64,
    pub files: Vec<FileBrief>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Stats {
    pub file_count: usize,
    pub total_size: u64,
    pub skipped_count: usize,
    pub largest: Vec<FileBrief>,
    pub duplicates: Vec<DuplicateGroup>,
    /// Bytes that could be freed if all but one copy of each duplicate were removed (info only).
    pub duplicate_waste: u64,
}

fn brief(f: &FileEntry) -> FileBrief {
    FileBrief {
        name: f.name.clone(),
        path: f.path.clone(),
        size: f.size,
    }
}

/// Hash up to `limit` bytes (None = whole file).
fn hash_file(path: &std::path::Path, limit: Option<u64>) -> Option<String> {
    let f = File::open(path).ok()?;
    let mut reader: Box<dyn Read> = match limit {
        Some(l) => Box::new(f.take(l)),
        None => Box::new(f),
    };
    let mut h = blake3::Hasher::new();
    let mut buf = vec![0u8; 256 * 1024];
    loop {
        let n = reader.read(&mut buf).ok()?;
        if n == 0 {
            break;
        }
        h.update(&buf[..n]);
    }
    Some(h.finalize().to_hex().to_string())
}

/// Find byte-identical files. Group by size, then a 64 KiB pre-hash, then a full hash.
/// Empty files and cloud-only placeholders are ignored (reading them would download them).
pub fn find_duplicates(files: &[FileEntry]) -> Vec<DuplicateGroup> {
    let mut by_size: HashMap<u64, Vec<&FileEntry>> = HashMap::new();
    for f in files.iter().filter(|f| f.size > 0 && !f.cloud_only) {
        by_size.entry(f.size).or_default().push(f);
    }
    let mut groups = Vec::new();
    for (size, same) in by_size.into_iter().filter(|(_, v)| v.len() > 1) {
        let mut by_pre: HashMap<String, Vec<&FileEntry>> = HashMap::new();
        for f in same {
            if let Some(h) = hash_file(&f.path, Some(64 * 1024)) {
                by_pre.entry(h).or_default().push(f);
            }
        }
        for (_, cands) in by_pre.into_iter().filter(|(_, v)| v.len() > 1) {
            let mut by_full: HashMap<String, Vec<&FileEntry>> = HashMap::new();
            for f in cands {
                let h = if size <= 64 * 1024 {
                    hash_file(&f.path, Some(64 * 1024))
                } else {
                    hash_file(&f.path, None)
                };
                if let Some(h) = h {
                    by_full.entry(h).or_default().push(f);
                }
            }
            for (hash, fs) in by_full.into_iter().filter(|(_, v)| v.len() > 1) {
                let mut files: Vec<FileBrief> = fs.into_iter().map(brief).collect();
                files.sort_by(|a, b| a.name.cmp(&b.name));
                groups.push(DuplicateGroup { hash, size, files });
            }
        }
    }
    groups.sort_by(|a, b| {
        (b.size * b.files.len() as u64)
            .cmp(&(a.size * a.files.len() as u64))
            .then(a.hash.cmp(&b.hash))
    });
    groups
}

pub fn compute(scan: &ScanResult) -> Stats {
    let mut largest: Vec<FileBrief> = scan.files.iter().map(brief).collect();
    largest.sort_by(|a, b| b.size.cmp(&a.size).then(a.name.cmp(&b.name)));
    largest.truncate(10);
    let duplicates = find_duplicates(&scan.files);
    let duplicate_waste = duplicates
        .iter()
        .map(|g| g.size * (g.files.len() as u64 - 1))
        .sum();
    Stats {
        file_count: scan.files.len(),
        total_size: scan.files.iter().map(|f| f.size).sum(),
        skipped_count: scan.skipped.len(),
        largest,
        duplicates,
        duplicate_waste,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use std::fs;

    #[test]
    fn finds_duplicates_and_largest() {
        let d = tempfile::tempdir().unwrap();
        let w = |n: &str, c: &[u8]| fs::write(d.path().join(n), c).unwrap();
        let big = vec![7u8; 200_000];
        let mut big2 = big.clone();
        big2[150_000] = 8; // same size + same first 64K, different content
        w("a.bin", &big);
        w("a copy.bin", &big);
        w("b.bin", &big2);
        w("x.txt", b"hello");
        w("x (1).txt", b"hello");
        w("y.txt", b"world");
        w("empty1.txt", b"");
        w("empty2.txt", b"");
        let mut c = Config::default();
        c.skip_recent_secs = 0;
        let scan = crate::scanner::scan(d.path(), &c).unwrap();
        let s = compute(&scan);
        assert_eq!(s.file_count, 8);
        assert_eq!(s.largest[0].size, 200_000);
        assert_eq!(s.duplicates.len(), 2);
        assert_eq!(s.duplicates[0].files.len(), 2);
        assert_eq!(s.duplicates[0].files[0].name, "a copy.bin");
        assert_eq!(
            s.duplicates[1]
                .files
                .iter()
                .map(|f| f.name.as_str())
                .collect::<Vec<_>>(),
            vec!["x (1).txt", "x.txt"]
        );
        assert_eq!(s.duplicate_waste, 200_005);
        // nothing was touched
        assert_eq!(fs::read_dir(d.path()).unwrap().count(), 8);
    }
}
