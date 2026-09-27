# Summary — DIVA Next backend wire-cut (DN-W)

User directive: phase 1 of the DIVA Next migration is wire-cutting — delete the
legacy `Vue -> Tauri commands -> Rust Manager HTTP/SSE -> Rust Agent` backend on
the migration branch immediately, ahead of the issue's delete-after-parity order.

Removed:

- All 16 root Rust crates and `agent-diva-gui/src-tauri/`
- `Cargo.toml` / `Cargo.lock` / `rustfmt.toml` / `clippy.toml` / `deny.toml`
- `contrib/` (systemd/launchd units) and all of `scripts/` (Rust/Tauri packaging, e2e, clean-break gates, model update helpers)
- `justfile` rewritten to GUI-only recipes; `.github/workflows/ci.yml` rewritten to a single `pnpm test` + `pnpm build` job
- `agent-diva-gui/package.json`: `tauri`/`bundle:prepare` scripts and `@tauri-apps/cli` removed; locks regenerated

Kept: `@tauri-apps/api`/`plugin-opener` client packages as dead seams (fail at runtime; per-domain removal in DN-1..DN-6), `workspace/masks` (migration data), test fixture `approval_contract_v1.json` relocated into `src/api/__fixtures__/`, README branch banners, TODOLIST entries for stale governance docs and remaining seams.

Planning package updated: new `DN-W.md` (Done), index DAG/decision log updated, DN-8 rescoped to build contract + boundary gate + acceptance.
