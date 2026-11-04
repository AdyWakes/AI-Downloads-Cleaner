use std::fs;
use std::path::Path;
use std::process::Command;

fn cli(home: &Path, args: &[&str]) -> (bool, String, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_tidydrop-cli"))
        .arg("--home")
        .arg(home)
        .args(args)
        .output()
        .unwrap();
    (
        out.status.success(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

#[test]
fn plan_clean_undo_roundtrip() {
    let home = tempfile::tempdir().unwrap();
    let dl = tempfile::tempdir().unwrap();
    // No AI (no Ollama in CI) and no recent-file skipping for freshly written test files.
    let mut cfg: serde_json::Value = serde_json::to_value(tidydrop_core_default()).unwrap();
    cfg["skip_recent_secs"] = 0.into();
    cfg["ai"]["enabled"] = false.into();
    fs::write(home.path().join("config.json"), cfg.to_string()).unwrap();
    for n in ["invoice.pdf", "setup.exe", "song.mp3", "रसीद.jpg"] {
        fs::write(dl.path().join(n), n).unwrap();
    }
    let d = dl.path().to_str().unwrap();

    let (ok, out, err) = cli(home.path(), &["plan", d]);
    assert!(ok, "{err}");
    assert!(out.contains("invoice.pdf  ->  Bills & Receipts/"), "{out}");
    assert!(
        dl.path().join("invoice.pdf").exists(),
        "plan must not move anything"
    );

    let (ok, out, _) = cli(home.path(), &["plan", d, "--json"]);
    assert!(ok);
    let v: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(v["plan"]["items"].as_array().unwrap().len(), 4);

    let (ok, out, err) = cli(home.path(), &["clean", d, "--yes", "--only", "Installers"]);
    assert!(ok, "{err}");
    assert!(out.contains("Moved 1 files"), "{out}");
    assert!(dl.path().join("Installers").join("setup.exe").exists());
    assert!(dl.path().join("invoice.pdf").exists());

    let (ok, out, _) = cli(home.path(), &["history"]);
    assert!(ok && out.contains("applied"), "{out}");

    let (ok, out, err) = cli(home.path(), &["undo"]);
    assert!(ok, "{err}");
    assert!(out.contains("Restored 1 files"), "{out}");
    assert!(dl.path().join("setup.exe").exists());
    assert!(!dl.path().join("Installers").exists());

    let (ok, _, err) = cli(home.path(), &["undo"]);
    assert!(!ok && err.contains("nothing to undo"), "{err}");
}

#[test]
fn refuses_unknown_category_and_bad_folder() {
    let home = tempfile::tempdir().unwrap();
    let dl = tempfile::tempdir().unwrap();
    let (ok, _, err) = cli(
        home.path(),
        &[
            "plan",
            dl.path().to_str().unwrap(),
            "--only",
            "Nope",
            "--no-ai",
        ],
    );
    assert!(!ok && err.contains("unknown category"));
    let root = if cfg!(windows) { "C:\\" } else { "/" };
    let (ok, _, err) = cli(home.path(), &["clean", root, "--yes", "--no-ai"]);
    assert!(!ok && err.contains("Refusing"), "{err}");
}

fn tidydrop_core_default() -> serde_json::Value {
    let out = Command::new(env!("CARGO_BIN_EXE_tidydrop-cli"))
        .args(["--home", "/nonexistent-tidydrop-home", "config", "show"])
        .output()
        .unwrap();
    serde_json::from_str(&String::from_utf8_lossy(&out.stdout)).unwrap()
}
