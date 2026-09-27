# DN-8 — Dependency deletion, new build contract, final acceptance

- **Epic:** D · **Requirements:** R-1, R-2, R-8 · **Outcome:** clean-checkout build of the new product invokes no Rust/Cargo and bundles no old runtime binaries; installed product runs on only the new backend; real browser/desktop/model and migration scenarios pass.
- **Authoritative design:** issue #13 DN-P1 §6–§8 · **Baseline:** `0fd005a1` · **Status:** Planned — final retirement/release gate, not preliminary cleanup · **Predecessors:** DN-6, DN-7 · **Index:** [index.md](index.md)
- **Files:** `Cargo.toml`, `Cargo.lock`, all legacy root Rust crates, `agent-diva-gui/src-tauri/`, `agent-diva-gui/package.json` + lock, `justfile`, `scripts/make-diva.ps1`, `scripts/ci/prepare_gui_bundle.py`, `.github/workflows/ci.yml`, README and repo guidance. **Proposed gate:** `scripts/ci/check_vivy_backend_boundary.py`. **Escalate:** any DN-0 parity item still unresolved — this Story cannot close with open parity blockers.

## Prerequisites / contracts

- Consumes: accepted DN-6/DN-7 outcomes; complete DN-0 parity list; pinned VIVY Generation acceptance.
- Old release/commit reference preserved before source retirement; legacy backend is not duplicated under a new active directory.

## Tasks

- [ ] Preserve the old release/commit reference.
- [ ] Remove old Manager/agent/core/provider/channel/tool/sandbox/CLI/service/memory/AutoDream/migration backend dependencies and retired Tauri code from the new product tree/build.
- [ ] Remove frontend invoke/listen business calls, Manager HTTP/SSE endpoints, direct-provider fallbacks; remaining native bridge classified through DN-5 only.
- [ ] Replace Cargo/Tauri-dependent product build jobs/installers with the Vue + Go-host + pinned-VIVY path; update build guidance and required gates without silently weakening existing gates while old code is active.
- [ ] Add `check_vivy_backend_boundary.py` (scoped dependency/source/package check rejecting the removed backend closure; historical docs/fixtures are not violations). Verify the actual dependency graph and packaged contents, not just string searches.
- [ ] Validate the selected DIVA artifact against the complete DN-0 parity list; remove development-only transitional entry points.

## Verification

- Clean-checkout product build: no Rust/Cargo invocation, no old runtime binaries bundled.
- Installed-product run on the new backend only; real browser/desktop/model scenarios; migration scenario.
- `just laputa-clean-break-check` / `just cognitive-clean-break-check` semantics carried into the new gate where still applicable.
- Report measured build time/disk use if measured; do not invent improvement targets.

## Evidence to supervisor

Build log evidence, packaged-content inspection, parity-list sign-off, gate CI results.

## Notes

A successful Vue bundle build alone does not close this Story.
