# Progress

## Done
- Phase 1: scaffold (Cargo workspace, core crate, CLI crate, Tauri v2 shell, Vite+React+TS+Tailwind, icons, CI workflow).
- Phase 2: config (categories/rules JSON), rules engine (Unicode word matching, regex keywords, Hindi keywords), scanner (skip partial/recent/hidden/system/in-use/symlink, companion `.part` detection), stats + duplicate detection, path helpers (sanitize, collision names, no-overwrite rename, long-path verbatim on Windows, refuse system folders). 24 tests.

- Phase 3: plan (overrides recomputed server-side, can't escape root), apply (re-checks every file right before moving, no-overwrite rename, collision numbering, refuses symlink/junction destinations, fsynced intent line before each move), JSONL journal in app data dir, undo (works after restart, never overwrites, retryable partial undo, crash recovery, removes only empty folders it created). Integration tests incl. full messy-folder round trip, long paths >260 chars, Hindi/Japanese/emoji names. 36 tests.

## In progress
- Phase 4: UI.

## Next
- Phases 3–7.

## Known issues
- The local build container has no WebKitGTK dev packages, so the Tauri shell cannot be fully built here; it is type-checked in CI (`app` job).
