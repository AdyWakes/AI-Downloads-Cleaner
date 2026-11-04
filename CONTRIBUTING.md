# Contributing to tidydrop

Thanks for helping! tidydrop's users trust it with their files, so the bar for safety is high and the bar for everything else is friendly.

## Ground rules

1. **Never delete, never overwrite.** File operations go through `tidydrop_core::paths::rename_noreplace`. No `fs::remove_file` on user files, no `fs::rename` (it overwrites), no copy-then-delete.
2. **Everything is undoable.** Any new way of moving files must write `intent`/`done` journal entries before/after the move.
3. **Preview before action.** Nothing moves without the user seeing the plan.
4. **Offline.** No network calls except to a loopback Ollama endpoint. No telemetry, no update pings.
5. **All file logic lives in `crates/tidydrop-core`** so the app and CLI behave identically. `src-tauri` stays a thin shell.

## Getting started

```bash
npm ci
cargo test            # core + CLI
npm test              # UI unit tests
npm run typecheck
npm run dev           # UI with a mock backend in your browser
npm run tauri dev     # full desktop app (needs Tauri prerequisites)
```

Before opening a PR:

```bash
cargo fmt --all
cargo clippy -p tidydrop-core -p tidydrop-cli --all-targets -- -D warnings
cargo test
npm run typecheck && npm test
```

## Tests

- Rules: `crates/tidydrop-core/src/rules.rs` (add a line to the tests when you add keywords).
- Safety end-to-end: `crates/tidydrop-core/tests/apply_undo.rs` builds messy folders in temp dirs, applies, undoes, and compares the whole tree byte-for-byte.
- Tests must never touch real user folders — always `tempfile::tempdir()`.

## Adding default keywords / languages

Edit `Config::default()` in `crates/tidydrop-core/src/config.rs`. Keywords match whole words (Unicode-aware), so add the word as people write it in filenames. Add a test in `rules.rs`.

## Code style

- Rust: `rustfmt` defaults, clippy clean.
- TypeScript: strict mode; keep components small; Tailwind for styling.
- Plain, friendly copy in the UI — our users aren't technical.
