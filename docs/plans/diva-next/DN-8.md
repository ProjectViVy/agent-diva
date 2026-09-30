# DN-8 — New build contract and final acceptance

- **Rescoped 2026-09-27:** the legacy backend deletion happened in DN-W (user-ordered phase 1). This Story now covers only the new build contract, boundary gate, and final acceptance.

- **Epic:** D · **Requirements:** R-1, R-2, R-8 · **Outcome:** clean-checkout build of the new product builds only the new Tauri/vivy-bridge Rust closure and bundles no old business runtime binaries; installed product runs on only the new backend; real browser/desktop/model and migration scenarios pass.
- **Authoritative design:** issue #13 DN-P1 §6–§8 · **Baseline:** `0fd005a1` · **Status:** see index — final release gate · **Predecessors:** DN-M, DN-7 · **Index:** [index.md](index.md)
- **Files:** `agent-diva-gui/package.json` + lock, `justfile`, `.github/workflows/ci.yml`, README and repo guidance (AGENTS.md/AGENTS-ARCH.MD/CLAUDE.md/LAPUTA.md governance rewrite per TODOLIST `GOVERNANCE-DOCS-STALE`). **Proposed gate:** `scripts/ci/check_vivy_backend_boundary.py` (new). **Escalate:** any DN-0 parity item still unresolved — this Story cannot close with open parity blockers.

## Prerequisites / contracts

- Consumes: accepted DN-M/DN-7 outcomes; complete DN-0 parity list; pinned VIVY Generation acceptance.
- Old release/commit reference preserved before source retirement; legacy backend is not duplicated under a new active directory.

## Tasks

- [ ] Verify DN-M removed legacy business invoke/listen seams; retain @tauri-apps dependencies used by the new transport and native shell.
- [ ] Update governance docs (AGENTS.md, AGENTS-ARCH.MD, CLAUDE.md, LAPUTA.md, README bodies) to the post-wire-cut reality.
- [ ] Establish the new product build/package contract (Vue + Tauri thin shell + pinned VIVY shared Generation) per the DN-5 host choice; wire CI release jobs to it.
- [ ] Add `scripts/ci/check_vivy_backend_boundary.py` (scoped dependency/source/package check rejecting the removed backend closure; historical docs/fixtures are not violations). Verify the actual dependency graph and packaged contents, not just string searches.
- [ ] Validate the selected DIVA artifact against the complete DN-0 parity list; remove development-only transitional entry points.

## Verification

- Clean-checkout product build: only the approved thin-shell Cargo closure, no old business runtime binaries bundled.
- Installed-product run on the new backend only; real browser/desktop/model scenarios; migration scenario.
- The former `laputa-clean-break-check` / `cognitive-clean-break-check` semantics are carried into the new boundary gate where still applicable (the old check scripts were deleted with `scripts/` in DN-W).
- Report measured build time/disk use if measured; do not invent improvement targets.

## Evidence to supervisor

Build log evidence, packaged-content inspection, parity-list sign-off, gate CI results.

## Notes

A successful Vue bundle build alone does not close this Story.

## P0-D1 scope amendment

The shared-library desktop core is accepted by DN-P and the semantic frontend migration by DN-M. Reuse their boundary gates; do not create duplicate scripts. DN-8 still checks full-product parity, DN-7 import, release packaging and governance documentation. All current state is in index.md. This plan does not authorize automatic release, publication or external tracker changes.
