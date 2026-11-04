# Decisions log

Choices made while building autonomously. Format: **what** — why — alternatives.

1. **Layout: root `package.json` + `src/` (React) + `src-tauri/` (shell) + `crates/tidydrop-core` + `crates/tidydrop-cli` in one Cargo workspace.** — Standard Tauri layout so `tauri-action` works with zero config; core logic is shared by app and CLI. — Alt: separate `app/` dir (more config for tauri-action).
2. **`default-members` excludes the Tauri shell.** — `cargo test` at the repo root works without WebKitGTK installed. CI lints the shell in a separate job with system deps. — Alt: require webkit everywhere.
3. **Tailwind v3 (PostCSS) instead of v4.** — Mature, config-file based, fewer surprises. — Alt: v4 + `@tailwindcss/vite`.
4. **Ollama client uses `ureq` 2 with no TLS features.** — Only talks to localhost over HTTP; no TLS stack needed, small binary. — Alt: reqwest/async.
5. **Only top-level files are touched; sub-folders are never entered or moved.** — Folders are the user's (or our) existing organisation; recursing is how cleaners wreck things. — Alt: recursive mode (possible future flag).
6. **"College/Study" category stores into folder `College & Study`.** — `/` is illegal in folder names. Display name kept as requested. — Alt: `College-Study`.
7. **Recent-file check uses mtime everywhere, plus creation time on Windows only.** — On Windows a copied/downloaded file keeps an old mtime but gets a fresh ctime; on Linux btime can't be backdated in tests and isn't meaningful for downloads. Threshold default 120 s, configurable.
8. **"In use" detection is Windows-only (exclusive open).** — macOS/Linux have no mandatory locks; renaming an open file there is harmless. Recent-modification check catches active downloads.
9. **`file.zip` is skipped if `file.zip.part` / `.crdownload` etc. exists next to it.** — Firefox creates the final file as a 0-byte placeholder during download.
10. **Refuse to clean drive roots, the home folder itself, and OS folders.** Project folders (`.git`, `package.json` …) only produce a warning.
11. **No-overwrite renames via OS primitives** (`renameat2(RENAME_NOREPLACE)` on Linux, `renamex_np(RENAME_EXCL)` on macOS, `MoveFileExW` without REPLACE/COPY flags on Windows). — `std::fs::rename` silently overwrites. Cross-drive moves are refused rather than copy+delete.
12. **Duplicates: size → 64 KiB pre-hash → full BLAKE3.** Cloud-only (OneDrive online-only) files are never read, so scanning doesn't trigger downloads.
13. **Rule keywords match whole words** (Unicode word segmentation + letter/digit split) so `bill` doesn't hit `billboard`. `re:` prefix for regex.
