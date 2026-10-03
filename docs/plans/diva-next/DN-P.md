> 2026-10-03 scope amendment: [DN-C1](p0-design.md) and [index.md](index.md)
> take precedence over historical P0-D1 host/speech/import/acceptance premises.
> Tasks/evidence below retain their original scope and artifact pins. New
> closure work is not proved by historical implementation or acceptance.

# DN-P — P0-A packaged acceptance Implementation Plan

> **For agentic workers:** Use `superpowers:executing-plans` after plan review and implementation authorization; no delegation is implied. Read both this plan and the shared design before execution.

**Spec:** [p0-design.md](p0-design.md), revision P0-D1. **State/dependencies:** [index.md](index.md) is authoritative.
**Baselines:** DIVA `d96e396d1641a5e7636e30e1f6cdbd0e597b62c8`; VIVY `5347032d8f18a047b67e85761c0dcc48728bee7c`.
**Global constraints:** one VIVY authority; no legacy Manager fallback; no production-data testing; do not change approved scope to close a gate. New paths below are proposed. Record actual commands/results and scoped commits; never claim mocks as live acceptance.

**Goal:** Close the shared-library/desktop-core P0 only with installed-product evidence.
**Architecture:** Exercise the same DLL and projection the user will run, across window and process lifetimes.
**Tech stack:** Vue/TypeScript, Tauri/Rust, Go VIVY as applicable to this Story.

## Files and contracts

Consumes DN-2 accepted core chain (including DN-5 package and DN-L artifact provenance). Proposed `scripts/ci/check_vivy_backend_boundary.py`, `docs/logs/diva-p0-acceptance/{summary,verification,acceptance}.md`; modify `.github/workflows/ci.yml` for the frozen target. This is the minimal P0-A package gate, not full DN-8 release.

## Ordered tasks

- [ ] Build/install from clean pinned DIVA/VIVY inputs. Compare shipped DLL Generation/ABI/hash with DN-L evidence; fail mismatches, missing files and executable-only substitution.
- [ ] Execute deterministic full chain: chat → stream → approval → continue → cancel a second run → close/reopen main window → recover completed and pending state → explicit Quit. Repeat with active polling, pending approval and cancellation during close. Inspect final persisted statuses using the backend, not UI assertions alone.
- [ ] Restart the whole application with the same temporary data root and verify history/pending interaction recovery according to the backend contract. No process-level hot-reload or crash containment is claimed.
- [ ] Execute the live-model safe-tool chain and record model/provider, platform, artifact hashes, observed output and side effects. Unavailable credentials/native host keep this gate open.
- [ ] Implement the boundary check over Rust dependency graph, loader/resource manifest and actual package contents: permit Tauri/vivy-bridge, reject old business crates/Manager executable/extra domain database. No whole-tree ban on Rust or Tauri names.

## Verification

Proposed command after implementation: `python scripts/ci/check_vivy_backend_boundary.py`; native package build from DN-5; deterministic and live scenario logs. Expected: installed artifact alone supplies the selected DLL and one VIVY authority; all lifecycle cases pass. Record actual startup/stream/exit timing without invented limits. Keep P0-A open for any missing mandatory case even if GUI tests are green.

Review focus: terminal state before Journal close, blocked approval on reopen, different artifact loaded at installation, duplicate run on replay, hidden native failure. Return a criterion-by-criterion acceptance record. P0-B and DN-8 remain independently open.
