# Verification — backend wire-cut (DN-W)

Commands run on this branch after deletion:

- `pnpm --dir agent-diva-gui install` — lockfile regenerated, install clean.
- `pnpm --dir agent-diva-gui test` — **68 files / 485 tests pass** (baseline `0fd005a1`: 68 files / 475 tests; 3 suites previously imported `agent-diva-manager` fixtures and now use the relocated copy).
- `pnpm --dir agent-diva-gui build` — `vue-tsc --noEmit` + `vite build` pass.
- `git ls-files | grep -E '\.rs$|Cargo\.(toml|lock)'` — no Rust sources or manifests remain tracked.

Notes:

- `isTauri()` / `isTauriRuntime()` guards are now permanently false in the shipped artifact; the browser path is the only runtime path.
- Remaining `invoke`/`listen` call sites have no live backend; their removal is scheduled per domain in DN-1..DN-6 (TODOLIST `DEAD-INVOKE-SEAMS`).
- Governance docs (AGENTS.md etc.) still describe the Rust workspace — recorded as TODOLIST `GOVERNANCE-DOCS-STALE`, deliberately not rewritten unilaterally.
