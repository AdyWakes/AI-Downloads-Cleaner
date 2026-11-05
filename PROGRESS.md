# Progress

## Done
- Phase 1: scaffold (Cargo workspace, core crate, CLI crate, Tauri v2 shell, Vite+React+TS+Tailwind, icons, CI workflow).
- Phase 2: config (categories/rules JSON), rules engine (Unicode word matching, regex keywords, Hindi keywords), scanner (skip partial/recent/hidden/system/in-use/symlink, companion `.part` detection), stats + duplicate detection, path helpers (sanitize, collision names, no-overwrite rename, long-path verbatim on Windows, refuse system folders). 24 tests.

- Phase 3: plan (overrides recomputed server-side, can't escape root), apply (re-checks every file right before moving, no-overwrite rename, collision numbering, refuses symlink/junction destinations, fsynced intent line before each move), JSONL journal in app data dir, undo (works after restart, never overwrites, retryable partial undo, crash recovery, removes only empty folders it created). Integration tests incl. full messy-folder round trip, long paths >260 chars, Hindi/Japanese/emoji names. 36 tests.

- Phase 5 (core part, done before UI so commands could use it): Ollama client, snippets, pipeline.
- Phase 4: Tauri commands (state holds the plan; UI only sends overrides), React UI: home/folder pick, AI badge + Ollama guide, stats (size, largest, duplicates, skipped), preview (untick, change category, custom folder, by-month toggle, filter), apply progress, before/after animation, undo, history, settings (folders, rules, AI, general), dark/light. Browser mock backend for `npm run dev` demos. Screenshots in docs/screenshots.

- Phase 6: `tidydrop-cli` (scan, plan [--json], clean [--yes] [--only CAT] [--by-month] [--no-ai], undo [--id], history, ai-status, config path|show|reset, global --home). Shares config + undo history with the app. Integration test drives the real binary.

- Phase 7: release workflow (tauri-action draft release for Windows/macOS universal/Linux + CLI archives), README, CONTRIBUTING, LICENSE (MIT).

- Final: full test run (36 core unit + 10 e2e + 2 CLI + 4 UI tests green), `npm run build`, release CLI build, safety self-review (fixes in DECISIONS #23), manual `build-installers.yml` workflow, MORNING.md.

## In progress
- Nothing. Waiting on your manual Windows test (checklist in MORNING.md).

## Next
- Run the real desktop app on Windows (never launched in this container) and fix what you find.
- Record the demo GIF (docs/demo.gif) and replace the README placeholder.
- Tune default keywords using real misfiles from the preview.
- Code signing (Windows/macOS) before a public release.

## Known issues
- The desktop window was never launched in this container (no WebKitGTK). The shell compiles and passes clippy in CI (`app` job), and the UI was tested against the browser mock.
- AI tier tested only against a fake Ollama server.
- Undo moves back whatever file now has the recorded name, even if you replaced it with a different file (never lost, but may be surprising).
- Installers are not code-signed.
- No failing tests.
