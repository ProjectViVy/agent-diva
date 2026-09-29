# DN-5 — Non-Rust desktop host and lifecycle

- **Epic:** C · **Requirements:** R-6 · **Outcome:** packaged cold start, duplicate launch, occupied port, backend crash/restart, window/tray behavior, clean exit without orphan/foreign-process termination — on the frozen initial platform.
- **Authoritative design:** issue #13 DN-P1 §3–§4 (Desktop rules) · **Baseline:** `0fd005a1` · **Status:** Blocked on the host probe inside this Story · **Predecessor:** DN-2 · **Index:** [index.md](index.md)
- **Reference only:** `src-tauri/src/{lib,embedded_server,gateway_status,shutdown_manager,tray,process_utils}.rs`, `src/features/diva-pet/components/DesktopPetOverlay.vue`, `desktop-pet.html`, `embedded-pet.html`. **Proposed:** `desktop/` host code, `src/platform/desktop-host.ts` seam. **Escalate:** no probed shell satisfies the required capabilities — blocks desktop release, not browser work.

## Prerequisites / contracts

- Frozen acceptance platform + probe checklist from DN-0.
- Host may launch/attach a pinned VIVY artifact or explicit compatible instance; must never implement a second backend or own competing config/database; never terminate an unrelated process to reclaim a port; HTTP(S) origin policy proven without wildcard bypasses.

## Tasks

- [ ] Run the bounded host probe: transparency, click-through, always-on-top, tray, multiwindow, mic/audio, HTTP-origin/bootstrap behavior on the initial OS.
- [ ] Record the concrete host/framework choice and exact launch/readiness API; do not assume a shell supports the capabilities.
- [ ] Implement launch/attach: ownership tracking, endpoint discovery, readiness, version mismatch, crash, bounded graceful shutdown.
- [ ] Keep host calls limited to native presentation/lifecycle; all domain operations go through VIVY.
- [ ] Prove the desktop path no longer starts `embedded_server` or a second backend.

## Verification

- Packaged cold start; duplicate launch; occupied port; backend crash/restart; window/tray behavior; clean exit.
- No Rust/Cargo step in the desktop path.

## Evidence to supervisor

Probe result matrix, host choice record, packaged-lifecycle test results.
