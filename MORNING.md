# Good morning ☀️ — tidydrop status

## 10-line summary
1. All 7 phases are done and pushed to `claude/zen-johnson-dl58p6` (one commit per phase, plus a self-review commit).
2. `crates/tidydrop-core` holds all file logic: scanner, rules, stats/duplicates, plan, journaled apply, undo, Ollama AI tier.
3. `tidydrop-cli` and the Tauri v2 app (`src-tauri`) both call the core and share settings + undo history.
4. React + TS + Tailwind UI: folder pick → stats → preview (untick / change folder / custom folder / by month) → apply → before/after animation → undo. There's also History, Settings (folders, rules, AI, general), dark/light, and an Ollama setup guide.
5. Safety: only moves, never deletes; no-overwrite OS renames; `(1)` numbering; fsynced journal entry before every move; undo works after a restart and never overwrites.
6. Skips partial downloads, files changed in the last 2 min, in-use (Windows), hidden/system files, symlinks/junctions and sub-folders, and refuses drive roots, the home folder and system folders.
7. AI: talks only to Ollama on localhost, only for ambiguous files, strict JSON is validated, and anything that fails falls back to rules. PDF/docx/txt snippets are capped at 400 characters.
8. Tests: 36 core unit + 10 end-to-end safety tests (including "undo restores a 35-file messy folder byte-for-byte", long paths > 260 chars, Hindi/Japanese/emoji, crash recovery) + 2 CLI tests + 4 UI tests.
9. CI (`ci.yml`) is green on Windows, macOS and Linux, including a real Tauri shell build. `release.yml` builds draft-release installers on a `v*` tag. I did not tag anything.
10. I also added `build-installers.yml` (manual, artifacts only) and ran it for Windows. It succeeded: the `tidydrop-windows` artifact (8.9 MB, `.msi` + setup `.exe` + `tidydrop-cli.exe`) is at https://github.com/AdyWakes/AI-Downloads-Cleaner/actions/runs/41924127536 and includes all the code.

## What works (verified)
- `cargo test` is green locally, and CI is green on Windows, macOS and Linux for every commit from Phase 6 on (including the real Tauri shell build).
- The Windows `.msi`/`.exe` installers were built successfully by GitHub Actions.
- `npm run typecheck`, `npm test`, `npm run build`.
- The UI flow was clicked through end-to-end in headless Chromium against the mock backend, in light and dark mode (screenshots in `docs/screenshots/`).
- `cargo build --release -p tidydrop-cli` (≈4 MB binary).

## What's weak / not verified
- **I never ran the actual desktop window.** This container has no WebKitGTK, so I only type-checked and clippy'd the Tauri crate locally (with stub pkg-config files). CI builds it for real, but nobody has clicked through the real app yet. **Your Windows test is the first real run.**
- The AI tier was tested against a fake Ollama server, not a real model. Real 3B models may answer slowly (~5–20 s per batch of 8) or pick odd categories. Validation stops it from breaking anything, but quality is unmeasured.
- Undo restores a file even if you edited it after the cleanup (correct), and also if you *replaced* it with a different file of the same name (it gets moved back to Downloads; not lost, just moved).
- The Windows "in use" check is a quick exclusive-open probe. Antivirus or the search indexer can briefly make a file look in-use, so it gets skipped (safe, but surprising).
- Installers aren't code-signed (SmartScreen / Gatekeeper warnings). There's no auto-update.
- The demo GIF is a placeholder: the README uses screenshots of the browser mock.
- The rule keyword list is opinionated (e.g. `order`, `ticket`, `notes`, `lab` are broad words). Expect some misfiles in the preview; that's why the preview exists.

## How to test on your Windows laptop

### Get the build
Option A (no toolchain): GitHub → **Actions** → **Build installers (no release)** → latest run → download the `tidydrop-windows` artifact (contains `.msi`, `-setup.exe`, `tidydrop-cli.exe`). If the run failed, click **Run workflow** with `windows` on this branch, or use Option B.

Option B (build yourself): install Rust (https://rustup.rs), Node 22, and the Tauri prerequisites (WebView2 is already on Win 10/11, plus the "Desktop development with C++" Visual Studio Build Tools). Then:
```powershell
git clone https://github.com/AdyWakes/AI-Downloads-Cleaner; cd AI-Downloads-Cleaner
git checkout claude/zen-johnson-dl58p6
npm ci
npm run tauri dev
```

### Prepare a safe test folder
```powershell
robocopy "$env:USERPROFILE\Downloads" "$env:USERPROFILE\Desktop\dl-test" /E /COPY:DAT /DCOPY:T
```
(`/COPY:DAT` keeps timestamps. Because copied files get a fresh *creation* time, set **Settings → General → Skip files changed in the last … minutes** to `0` for this test, or wait 2 minutes.)

### Manual checklist
1. [ ] App opens; header shows **AI: off** (or **on** if Ollama is installed). Home shows your real Downloads path.
2. [ ] Click **Change…** and pick `Desktop\dl-test`.
3. [ ] **Scan folder** → stats: file count and total size look right; **Largest files** is sensible; duplicates (if any) are listed; **Files left alone** expands and gives reasons.
4. [ ] Try to pick `C:\` or `C:\Users\<you>` → you get a clear "Refusing…" error, nothing scanned.
5. [ ] **Review the plan** → spot-check 20 files: is the folder sensible and the reason understandable? Note bad guesses (they help tune the rules).
6. [ ] Untick one file, change one file's folder via the dropdown, set one to **Custom folder…** `Test/Sub`, and toggle **Organize by month** on, then off.
7. [ ] Before applying, start a big download in the browser into `dl-test` (or put a file named `x.zip.crdownload` there) and open a `.docx` in Word.
8. [ ] **Clean up N files** → progress, then the before/after animation. In Explorer: files are in category folders; the unticked file, the `.crdownload`, the open `.docx`, `desktop.ini` and existing sub-folders are untouched.
9. [ ] If a name already existed in a category folder, the moved file is `name (1).ext` and the original is unchanged.
10. [ ] Hindi/emoji filenames (create `बिजली का बिल.pdf` and `🎉 party.png` first if you have none) moved correctly; the Hindi bill went to **Bills & Receipts**.
11. [ ] **Close the app completely.** Reopen it → Home shows the "Last cleanup … Undo it" banner. Click it.
12. [ ] Compare against the original: a count check, then a full diff:
    ```powershell
    (Get-ChildItem "$env:USERPROFILE\Desktop\dl-test" -Recurse -File).Count
    (Get-ChildItem "$env:USERPROFILE\Downloads" -Recurse -File).Count
    robocopy "$env:USERPROFILE\Downloads" "$env:USERPROFILE\Desktop\dl-test" /L /E /NJH /NP /FP
    ```
    The robocopy `/L` (list-only) run should report 0 files to copy. Category folders created by tidydrop should be gone.
13. [ ] Do another cleanup, then before undoing create a new file in `dl-test` with the same name as one that moved. Undo → your new file is untouched and the old one comes back as `name (1).ext`.
14. [ ] **History** lists both cleanups with their status.
15. [ ] Settings: add a keyword (e.g. `aadhaar` → Bills & Receipts), save, rescan, check that it applies. **Reset to defaults** works. The dark/light toggle works and persists after a restart.
16. [ ] (Optional AI) Install Ollama, run `ollama pull llama3.2:3b`, click the badge → **Check again** → **AI: on**. Rescan: unclear files (`scan001.pdf`, `document(3).pdf`) show a purple **✨ AI** tag with a reason. Stop Ollama mid-scan → the scan still finishes using rules.
17. [ ] CLI, from the artifact folder:
    ```powershell
    .\tidydrop-cli.exe plan "$env:USERPROFILE\Desktop\dl-test" --no-ai
    .\tidydrop-cli.exe clean "$env:USERPROFILE\Desktop\dl-test" --no-ai --only Installers
    .\tidydrop-cli.exe history
    .\tidydrop-cli.exe undo
    ```
18. [ ] Long path: create a deep folder chain inside `dl-test` totalling > 260 characters, put a file with a 150-character name at the top level of `dl-test`, then clean and undo. It moves and comes back.

Where things live: settings `%APPDATA%\tidydrop\config.json`; undo journals `%APPDATA%\tidydrop\journals\*.jsonl` (plain JSON lines, readable if anything goes wrong).

See `PROGRESS.md` for status and `DECISIONS.md` for every judgement call I made.
