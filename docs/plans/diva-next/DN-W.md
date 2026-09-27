# DN-W — 拆线：immediate removal of the legacy Rust backend

- **Epic:** A (reordered by user directive 2026-09-27: wire-cut is phase 1) · **Requirements:** R-2, R-8 · **Outcome:** the new branch contains no Rust workspace, no Tauri shell, and no Rust/Manager build path; the Vue frontend still typechecks, tests, and builds in browser mode.
- **Authoritative design:** issue #13 DN-P1 + user directive (deletion happens first, before consumer replacement) · **Baseline:** `0fd005a1` · **Status:** Done on `devin/1790495412-diva-next-planning` · **Index:** [index.md](index.md)
- **Note:** DN-P1 originally sequenced deletion last (DN-8) to preserve the working reference; the user reordered it to first. The old code remains reachable in git history / old releases — the retirement conditions in issue §3 still bind what DN-1+ must provide.

## What was removed

- All 16 root Rust crates: `agent-diva-{agent,autodream,channels,cli,core,e2e,files,laputa,manager,migration,neuron,providers,sandbox,service,tooling,tools}`.
- `agent-diva-gui/src-tauri/` (Tauri shell, embedded Manager, commands, tray, lifecycle).
- `Cargo.toml`, `Cargo.lock`, `rustfmt.toml`, `clippy.toml`, `deny.toml`, `contrib/` (systemd/launchd units), all of `scripts/` (Rust/Tauri packaging, e2e, clean-break gates, model update scripts).
- `justfile` rewritten: GUI-only recipes (`gui-test`, `gui-build`, `gui-dev`, `ci` = test+build).
- `.github/workflows/ci.yml` rewritten: single `gui-check` job (pnpm install → `pnpm test` → `pnpm build` on ubuntu).
- `agent-diva-gui/package.json`: `tauri`/`bundle:prepare` scripts and `@tauri-apps/cli` removed; `pnpm-lock.yaml`/`package-lock.json` regenerated.

## What was deliberately kept

- `@tauri-apps/api` + `@tauri-apps/plugin-opener` npm deps: frontend-only client packages keeping call sites typed and test mocks intact; every call now fails at runtime because no Tauri runtime exists. Removal happens per-domain in DN-1..DN-6 (tracked in TODOLIST `DEAD-INVOKE-SEAMS`).
- `workspace/masks`: runtime prompt data, candidate migration source for DN-7 — not backend code.
- Test fixture `approval_contract_v1.json` relocated to `src/api/fixtures/` (was imported from the deleted `agent-diva-manager` crate).
- Governance docs (AGENTS.md, CLAUDE.md, README bodies) still describe the Rust workspace — stale by design pending governance review (TODOLIST `GOVERNANCE-DOCS-STALE`); branch banners added to both READMEs.

## Verification evidence

- `pnpm --dir agent-diva-gui test`: 68 files / 485 tests pass (was 68/475 on baseline: 3 suites previously imported the deleted crate fixture and now use the relocated copy).
- `pnpm --dir agent-diva-gui build`: vue-tsc + Vite build passes.
- `git ls-files | grep -E '\.rs$|Cargo'` → no Rust sources or manifests remain.

## Consequences for downstream Stories

- DN-0's inventory now describes a deleted codebase — read it from git history at `0fd005a1`, not the working tree. The classification/contract record is still required.
- DN-5/DN-8 shrink: no Rust build closure remains to delete; DN-8 becomes final packaging/boundary-gate + acceptance.
- `isTauri()`/`isTauriRuntime()` guards now always evaluate false in the shipped artifact; browser paths are the only paths.
