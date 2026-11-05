# Progress

## Done
- Phase 1: scaffold (Cargo workspace, core crate, CLI crate, Tauri v2 shell, Vite+React+TS+Tailwind, icons, CI workflow).

## In progress
- Phase 2: scanner + rules engine + tests.

## Next
- Phases 3–7.

## Known issues
- The local build container has no WebKitGTK dev packages, so the Tauri shell cannot be fully built here; it is type-checked in CI (`app` job).
