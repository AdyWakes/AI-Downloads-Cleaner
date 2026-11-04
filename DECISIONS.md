# Decisions log

Choices made while building autonomously. Format: **what** — why — alternatives.

1. **Layout: root `package.json` + `src/` (React) + `src-tauri/` (shell) + `crates/tidydrop-core` + `crates/tidydrop-cli` in one Cargo workspace.** — Standard Tauri layout so `tauri-action` works with zero config; core logic is shared by app and CLI. — Alt: separate `app/` dir (more config for tauri-action).
2. **`default-members` excludes the Tauri shell.** — `cargo test` at the repo root works without WebKitGTK installed. CI lints the shell in a separate job with system deps. — Alt: require webkit everywhere.
3. **Tailwind v3 (PostCSS) instead of v4.** — Mature, config-file based, fewer surprises. — Alt: v4 + `@tailwindcss/vite`.
4. **Ollama client uses `ureq` 2 with no TLS features.** — Only talks to localhost over HTTP; no TLS stack needed, small binary. — Alt: reqwest/async.
