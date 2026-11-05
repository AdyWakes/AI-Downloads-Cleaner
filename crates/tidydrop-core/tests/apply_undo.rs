//! End-to-end safety tests: plan -> apply -> undo on synthetic messy folders.

use filetime::FileTime;
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use tidydrop_core::apply::apply_plan;
use tidydrop_core::config::Config;
use tidydrop_core::journal::{self, Entry, JournalStatus, JournalWriter};
use tidydrop_core::plan::{build_plan, classify_with_rules, ItemOverride, Plan};
use tidydrop_core::{scanner, undo};

const OLD: i64 = 1_650_000_000; // April 2022

fn write_old(p: &Path, content: &[u8]) {
    if let Some(parent) = p.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(p, content).unwrap();
    filetime::set_file_mtime(p, FileTime::from_unix_time(OLD, 0)).unwrap();
}

fn cfg() -> Config {
    let mut c = Config::default();
    // Windows also checks creation time, which tests can't backdate.
    if cfg!(windows) {
        c.skip_recent_secs = 0;
    }
    c
}

/// Everything under `root`: relative path -> (content, mtime). Directories as "<dir>".
fn snapshot(root: &Path) -> BTreeMap<String, (Vec<u8>, i64)> {
    fn walk(root: &Path, dir: &Path, out: &mut BTreeMap<String, (Vec<u8>, i64)>) {
        for e in fs::read_dir(dir).unwrap().flatten() {
            let p = e.path();
            let rel = p
                .strip_prefix(root)
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/");
            let md = fs::symlink_metadata(&p).unwrap();
            if md.is_dir() {
                out.insert(format!("{rel}/"), (b"<dir>".to_vec(), 0));
                walk(root, &p, out);
            } else {
                let mtime = FileTime::from_last_modification_time(&md).unix_seconds();
                out.insert(rel, (fs::read(&p).unwrap(), mtime));
            }
        }
    }
    let mut out = BTreeMap::new();
    walk(root, root, &mut out);
    out
}

fn make_plan(root: &Path, c: &Config, by_month: bool) -> Plan {
    let scan = scanner::scan(root, c).unwrap();
    let cls = classify_with_rules(&scan.files, c);
    build_plan(&scan, &cls, c, by_month)
}

fn messy_folder(root: &Path) {
    let names = [
        "Amazon_Invoice_2024.pdf",
        "GST bill March.pdf",
        "receipt (2).jpg",
        "Unit 3 Lecture Notes.pdf",
        "DBMS_Assignment2.docx",
        "Screenshot 2024-05-01 101010.png",
        "Screen Shot 2023-01-01 at 10.00.00.png",
        "IMG_20240101_101010.jpg",
        "holiday.HEIC",
        "vlc-3.0.20-win64.exe",
        "Setup.msi",
        "photos.zip",
        "backup.tar.gz",
        "movie.mkv",
        "song.mp3",
        "voice note.m4a",
        "budget.xlsx",
        "data.csv",
        "deck.pptx",
        "letter.docx",
        "readme.txt",
        "script.py",
        "index.html",
        "random.pdf",
        "report.pdf",
        "mystery.xyz",
        "NO_EXTENSION",
        "बिजली का बिल.pdf",
        "हिंदी कहानी.pdf",
        "日本語のファイル.txt",
        "emoji 🎉 party.png",
        "spaces   and   tabs.docx",
        "UPPER.PDF",
        "dup.bin",
        "dup copy.bin",
    ];
    for (i, n) in names.iter().enumerate() {
        write_old(&root.join(n), format!("content of {n} #{i}").as_bytes());
    }
    // Things that must never move
    write_old(&root.join(".hidden"), b"hidden");
    write_old(&root.join("desktop.ini"), b"[.ShellClassInfo]");
    write_old(&root.join("big.iso.crdownload"), b"partial");
    write_old(&root.join("firefox.zip"), b"");
    write_old(&root.join("firefox.zip.part"), b"partial");
    // Existing user folders with content, including a category folder name
    write_old(&root.join("My Stuff").join("keep.pdf"), b"keep");
    write_old(
        &root.join("PDFs").join("report.pdf"),
        b"pre-existing report, different content",
    );
}

#[test]
fn undo_fully_restores_a_messy_folder() {
    let d = tempfile::tempdir().unwrap();
    let jdir = tempfile::tempdir().unwrap();
    let root = d.path().join("Downloads");
    fs::create_dir(&root).unwrap();
    messy_folder(&root);
    let c = cfg();
    let before = snapshot(&root);

    let plan = make_plan(&root, &c, true);
    assert_eq!(
        plan.items.len(),
        35,
        "{:#?}",
        plan.items.iter().map(|i| &i.name).collect::<Vec<_>>()
    );
    // Nothing moved by planning.
    assert_eq!(snapshot(&root), before);

    let report = apply_plan(&plan, &c, jdir.path(), &mut |_| {}).unwrap();
    assert_eq!(
        report.moved.len(),
        35,
        "failed: {:?} skipped: {:?}",
        report.failed,
        report.skipped
    );
    assert!(report.failed.is_empty());

    // Spot-check destinations
    let month = tidydrop_core::plan::month_folder(OLD);
    assert!(root
        .join("Bills & Receipts")
        .join(&month)
        .join("Amazon_Invoice_2024.pdf")
        .exists());
    assert!(root
        .join("Bills & Receipts")
        .join(&month)
        .join("बिजली का बिल.pdf")
        .exists());
    assert!(root
        .join("College & Study")
        .join(&month)
        .join("Unit 3 Lecture Notes.pdf")
        .exists());
    assert!(root
        .join("Screenshots")
        .join(&month)
        .join("Screen Shot 2023-01-01 at 10.00.00.png")
        .exists());
    assert!(root
        .join("Installers")
        .join(&month)
        .join("vlc-3.0.20-win64.exe")
        .exists());
    // The pre-existing PDFs/report.pdf is untouched; ours went into the month folder.
    assert_eq!(
        fs::read(root.join("PDFs").join("report.pdf")).unwrap(),
        b"pre-existing report, different content"
    );
    assert!(root.join("PDFs").join(&month).join("report.pdf").exists());
    // Protected files stayed put
    for n in [
        ".hidden",
        "desktop.ini",
        "big.iso.crdownload",
        "firefox.zip",
        "firefox.zip.part",
    ] {
        assert!(root.join(n).exists(), "{n} must not move");
    }
    assert!(root.join("My Stuff").join("keep.pdf").exists());
    // mtimes are preserved by a move
    let md = fs::metadata(root.join("Audio").join(&month).join("song.mp3")).unwrap();
    assert_eq!(
        FileTime::from_last_modification_time(&md).unix_seconds(),
        OLD
    );

    // "Restart": only the journal directory on disk is used.
    let list = journal::list(jdir.path()).unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].status, JournalStatus::Applied);
    assert_eq!(list[0].moved, 35);

    let u = undo::undo_last(jdir.path()).unwrap();
    assert_eq!(u.restored.len(), 35);
    assert!(u.problems.is_empty(), "{:?}", u.problems);
    assert!(u.restored.iter().all(|r| !r.renamed));

    assert_eq!(
        snapshot(&root),
        before,
        "undo must restore the folder exactly"
    );
    assert_eq!(
        journal::list(jdir.path()).unwrap()[0].status,
        JournalStatus::Undone
    );
    assert!(
        undo::undo_last(jdir.path()).is_err(),
        "nothing left to undo"
    );
}

#[test]
fn collisions_get_numbered_and_nothing_is_overwritten() {
    let d = tempfile::tempdir().unwrap();
    let j = tempfile::tempdir().unwrap();
    let root = d.path();
    write_old(&root.join("photo.jpg"), b"new photo");
    write_old(&root.join("Images").join("photo.jpg"), b"old photo");
    write_old(&root.join("Images").join("photo (1).jpg"), b"older photo");
    write_old(&root.join("backup.tar.gz"), b"new archive");
    write_old(&root.join("Archives").join("backup.tar.gz"), b"old archive");
    let c = cfg();
    let plan = make_plan(root, &c, false);
    let r = apply_plan(&plan, &c, j.path(), &mut |_| {}).unwrap();
    assert_eq!(r.moved.len(), 2);
    assert_eq!(
        fs::read(root.join("Images").join("photo.jpg")).unwrap(),
        b"old photo"
    );
    assert_eq!(
        fs::read(root.join("Images").join("photo (1).jpg")).unwrap(),
        b"older photo"
    );
    assert_eq!(
        fs::read(root.join("Images").join("photo (2).jpg")).unwrap(),
        b"new photo"
    );
    assert_eq!(
        fs::read(root.join("Archives").join("backup (1).tar.gz")).unwrap(),
        b"new archive"
    );
    undo::undo_last(j.path()).unwrap();
    assert_eq!(fs::read(root.join("photo.jpg")).unwrap(), b"new photo");
    assert_eq!(
        fs::read(root.join("Images").join("photo.jpg")).unwrap(),
        b"old photo"
    );
}

#[test]
fn files_changed_or_removed_after_preview_are_skipped() {
    let d = tempfile::tempdir().unwrap();
    let j = tempfile::tempdir().unwrap();
    let root = d.path();
    write_old(&root.join("a.pdf"), b"a");
    write_old(&root.join("b.pdf"), b"b");
    write_old(&root.join("c.pdf"), b"c");
    let c = cfg();
    let plan = make_plan(root, &c, false);
    // After preview: a.pdf grows, b.pdf disappears, c.pdf unchanged
    write_old(&root.join("a.pdf"), b"a, but longer now");
    fs::rename(
        root.join("b.pdf"),
        d.path()
            .join("..")
            .join(format!("b-{}.pdf", std::process::id())),
    )
    .ok();
    let r = apply_plan(&plan, &c, j.path(), &mut |_| {}).unwrap();
    assert_eq!(r.moved.len(), 1);
    assert_eq!(r.skipped.len(), 2, "{:?}", r.skipped);
    assert!(root.join("a.pdf").exists());
    assert!(root.join("PDFs").join("c.pdf").exists());
}

#[test]
fn unticked_and_overridden_items() {
    let d = tempfile::tempdir().unwrap();
    let j = tempfile::tempdir().unwrap();
    let root = d.path();
    write_old(&root.join("keep-me.pdf"), b"1");
    write_old(&root.join("move-me.pdf"), b"2");
    write_old(&root.join("custom.pdf"), b"3");
    let c = cfg();
    let mut plan = make_plan(root, &c, false);
    let id = |n: &str| plan.items.iter().find(|i| i.name == n).unwrap().id;
    let ov = vec![
        ItemOverride {
            id: id("keep-me.pdf"),
            enabled: Some(false),
            ..Default::default()
        },
        ItemOverride {
            id: id("move-me.pdf"),
            category: Some("Bills & Receipts".into()),
            ..Default::default()
        },
        ItemOverride {
            id: id("custom.pdf"),
            custom_folder: Some("Work/Taxes 2024".into()),
            ..Default::default()
        },
    ];
    plan.apply_overrides(&ov, &c);
    let r = apply_plan(&plan, &c, j.path(), &mut |_| {}).unwrap();
    assert_eq!(r.moved.len(), 2);
    assert!(root.join("keep-me.pdf").exists());
    assert!(root.join("Bills & Receipts").join("move-me.pdf").exists());
    assert!(root
        .join("Work")
        .join("Taxes 2024")
        .join("custom.pdf")
        .exists());
    undo::undo_last(j.path()).unwrap();
    assert!(
        !root.join("Work").exists(),
        "created folders are removed when empty"
    );
}

#[test]
fn undo_never_overwrites_a_file_that_reappeared() {
    let d = tempfile::tempdir().unwrap();
    let j = tempfile::tempdir().unwrap();
    let root = d.path();
    write_old(&root.join("doc.pdf"), b"original");
    let c = cfg();
    let plan = make_plan(root, &c, false);
    apply_plan(&plan, &c, j.path(), &mut |_| {}).unwrap();
    // The user downloads a new doc.pdf before undoing
    fs::write(root.join("doc.pdf"), b"newer download").unwrap();
    let u = undo::undo_last(j.path()).unwrap();
    assert_eq!(u.restored.len(), 1);
    assert!(u.restored[0].renamed);
    assert_eq!(fs::read(root.join("doc.pdf")).unwrap(), b"newer download");
    assert_eq!(fs::read(root.join("doc (1).pdf")).unwrap(), b"original");
}

#[test]
fn undo_reports_missing_files_and_can_be_retried() {
    let d = tempfile::tempdir().unwrap();
    let j = tempfile::tempdir().unwrap();
    let root = d.path();
    write_old(&root.join("a.mp3"), b"a");
    write_old(&root.join("b.mp3"), b"b");
    let c = cfg();
    let plan = make_plan(root, &c, false);
    apply_plan(&plan, &c, j.path(), &mut |_| {}).unwrap();
    // User moves b.mp3 somewhere else
    let elsewhere = tempfile::tempdir().unwrap();
    fs::rename(
        root.join("Audio").join("b.mp3"),
        elsewhere.path().join("b.mp3"),
    )
    .unwrap();
    let u = undo::undo_last(j.path()).unwrap();
    assert_eq!(u.restored.len(), 1);
    assert_eq!(u.problems.len(), 1);
    assert_eq!(
        journal::list(j.path()).unwrap()[0].status,
        JournalStatus::PartiallyUndone
    );
    // Put it back and retry: finishes the job without touching a.mp3 again
    fs::create_dir_all(root.join("Audio")).unwrap();
    fs::rename(
        elsewhere.path().join("b.mp3"),
        root.join("Audio").join("b.mp3"),
    )
    .unwrap();
    let u2 = undo::undo_last(j.path()).unwrap();
    assert_eq!(u2.restored.len(), 1);
    assert!(root.join("a.mp3").exists() && root.join("b.mp3").exists());
    assert_eq!(
        journal::list(j.path()).unwrap()[0].status,
        JournalStatus::Undone
    );
}

#[test]
fn crash_between_intent_and_done_is_recoverable() {
    let d = tempfile::tempdir().unwrap();
    let j = tempfile::tempdir().unwrap();
    let root = dunce::canonicalize(d.path()).unwrap();
    // Simulate: header + dir + intent for x (rename happened, no done line),
    // intent for y (rename did not happen), then a truncated line.
    fs::create_dir(root.join("PDFs")).unwrap();
    write_old(&root.join("PDFs").join("x.pdf"), b"x");
    write_old(&root.join("y.pdf"), b"y");
    let mut w = JournalWriter::create(j.path(), "20240101-000000-000000001").unwrap();
    w.write(
        &Entry::Header {
            version: 1,
            id: w.id.clone(),
            root: root.clone(),
            created_at: "t".into(),
            organize_by_month: false,
            planned: 2,
        },
        true,
    )
    .unwrap();
    w.write(
        &Entry::DirCreated {
            path: root.join("PDFs"),
        },
        true,
    )
    .unwrap();
    w.write(
        &Entry::Intent {
            seq: 1,
            from: root.join("x.pdf"),
            to: root.join("PDFs").join("x.pdf"),
            size: 1,
        },
        true,
    )
    .unwrap();
    w.write(
        &Entry::Intent {
            seq: 2,
            from: root.join("y.pdf"),
            to: root.join("PDFs").join("y.pdf"),
            size: 1,
        },
        true,
    )
    .unwrap();
    drop(w);
    let jp = j.path().join("20240101-000000-000000001.jsonl");
    let mut text = fs::read_to_string(&jp).unwrap();
    text.push_str("{\"type\":\"done\",\"se");
    fs::write(&jp, text).unwrap();

    let s = journal::list(j.path()).unwrap();
    assert_eq!(s[0].status, JournalStatus::Interrupted);
    assert_eq!(s[0].moved, 1);
    let u = undo::undo_last(j.path()).unwrap();
    assert_eq!(u.restored.len(), 1);
    assert_eq!(fs::read(root.join("x.pdf")).unwrap(), b"x");
    assert_eq!(fs::read(root.join("y.pdf")).unwrap(), b"y");
    assert!(!root.join("PDFs").exists());
}

#[test]
fn long_and_unicode_names_roundtrip() {
    let d = tempfile::tempdir().unwrap();
    let j = tempfile::tempdir().unwrap();
    // Deep root so the full destination path exceeds 260 chars (Windows MAX_PATH).
    let mut root = d.path().to_path_buf();
    for i in 0..4 {
        root.push(format!(
            "very long folder name number {i} with padding padding padding"
        ));
    }
    fs::create_dir_all(&root).unwrap();
    let long_name = format!("{} रसीद invoice.pdf", "x".repeat(150));
    write_old(&root.join(&long_name), b"long");
    write_old(&root.join("नमस्ते दुनिया.docx"), b"hindi");
    let c = cfg();
    let before = snapshot(&root);
    let plan = make_plan(&root, &c, true);
    let r = apply_plan(&plan, &c, j.path(), &mut |_| {}).unwrap();
    assert_eq!(r.moved.len(), 2, "{:?}", r.failed);
    assert!(r.moved.iter().any(|m| m.to.to_string_lossy().len() > 260));
    undo::undo_last(j.path()).unwrap();
    assert_eq!(snapshot(&root), before);
}

#[cfg(unix)]
#[test]
fn refuses_to_move_through_symlinked_category_folder() {
    let d = tempfile::tempdir().unwrap();
    let outside = tempfile::tempdir().unwrap();
    let j = tempfile::tempdir().unwrap();
    let root = d.path();
    std::os::unix::fs::symlink(outside.path(), root.join("PDFs")).unwrap();
    write_old(&root.join("a.pdf"), b"a");
    let c = cfg();
    let plan = make_plan(root, &c, false);
    let r = apply_plan(&plan, &c, j.path(), &mut |_| {}).unwrap();
    assert_eq!(r.moved.len(), 0);
    assert_eq!(r.failed.len(), 1);
    assert!(root.join("a.pdf").exists());
    assert_eq!(fs::read_dir(outside.path()).unwrap().count(), 0);
}

#[test]
fn second_cleanup_and_undo_order() {
    let d = tempfile::tempdir().unwrap();
    let j = tempfile::tempdir().unwrap();
    let root = d.path();
    write_old(&root.join("one.mp3"), b"1");
    let c = cfg();
    apply_plan(&make_plan(root, &c, false), &c, j.path(), &mut |_| {}).unwrap();
    write_old(&root.join("two.mp3"), b"2");
    apply_plan(&make_plan(root, &c, false), &c, j.path(), &mut |_| {}).unwrap();
    // Undo last only restores two.mp3
    undo::undo_last(j.path()).unwrap();
    assert!(root.join("two.mp3").exists());
    assert!(root.join("Audio").join("one.mp3").exists());
    undo::undo_last(j.path()).unwrap();
    assert!(root.join("one.mp3").exists());
    assert!(!root.join("Audio").exists());
}
