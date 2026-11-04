# Progress

## Done
- Phase 1: scaffold (Cargo workspace, core crate, CLI crate, Tauri v2 shell, Vite+React+TS+Tailwind, icons, CI workflow).
- Phase 2: config (categories/rules JSON), rules engine (Unicode word matching, regex keywords, Hindi keywords), scanner (skip partial/recent/hidden/system/in-use/symlink, companion `.part` detection), stats + duplicate detection, path helpers (sanitize, collision names, no-overwrite rename, long-path verbatim on Windows, refuse system folders). 24 tests.

## In progress
- Phase 3: plan/apply/journal/undo.

## Next
- Phases 3–7.

## Known issues
- The local build container has no WebKitGTK dev packages, so the Tauri shell cannot be fully built here; it is type-checked in CI (`app` job).
