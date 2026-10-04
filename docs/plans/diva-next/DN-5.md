> 2026-10-03 scope amendment: [DN-C2](p0-design.md) and [index.md](index.md)
> take precedence over historical P0-D1 host/speech/import/acceptance premises.
> Tasks/evidence below retain their original scope and artifact pins. New
> closure work is not proved by historical implementation or acceptance.

# DN-5 — Tauri thin shell and vivy-bridge Implementation Plan

> **For agentic workers:** Use `superpowers:executing-plans` after plan review and implementation authorization; no delegation is implied. Read both this plan and the shared design before execution.

**Spec:** [p0-design.md](p0-design.md), revision P0-D1. **State/dependencies:** [index.md](index.md) is authoritative.
**Baselines:** DIVA `d96e396d1641a5e7636e30e1f6cdbd0e597b62c8`; VIVY `5347032d8f18a047b67e85761c0dcc48728bee7c`.
**Global constraints:** one VIVY authority; no legacy Manager fallback; no production-data testing; do not change approved scope to close a gate. New paths below are proposed. Record actual commands/results and scoped commits; never claim mocks as live acceptance.

**Goal:** A native process owns the DLL independently of its windows and exits cleanly.
**Architecture:** Restore only a small Tauri shell and native bridge; never restore the retired Rust workspace/business crates.
**Tech stack:** Vue/TypeScript, Tauri/Rust, Go VIVY as applicable to this Story.

## Files and contracts

Proposed DIVA paths: `agent-diva-gui/src-tauri/{Cargo.toml,build.rs,tauri.conf.json}`, `agent-diva-gui/src-tauri/src/{main.rs,lib.rs,lifecycle.rs}`, `agent-diva-gui/src-tauri/crates/vivy-bridge/{Cargo.toml,src/lib.rs}`, `agent-diva-gui/src-tauri/tests/bridge_lifecycle.rs`, `agent-diva-gui/src/platform/desktop-host.ts`. Modify existing `agent-diva-gui/package.json`, `agent-diva-gui/pnpm-lock.yaml`, `justfile`, `.github/workflows/ci.yml`. Old src-tauri in history is reference, not a copy source.

Consumes accepted DN-L DLL/header/manifest plus ABI. Produces Tauri command `vivy_call(request)` and event `vivy:event`. The Rust-owned worker calls VivyPollEvents; the frontend does not spawn additional FFI pollers. Bridge connection/gap status travels in the event envelope, separate from unchanged VIVY notification payloads. Native commands are a separate small allowlist justified by DN-0.

## Ordered tasks

- [ ] Create the minimal Tauri v2 crate/build/package closure and bridge crate. Pin compatible dependencies; verify native prerequisites from DN-0. Bundle only the matched DLL/header metadata and VIVY resources, resolving from the application's resource directory, never CWD or an arbitrary library search path.
- [ ] Add bridge tests for ABI mismatch, missing DLL, malformed/null output, UTF-8 failure and allocation cleanup. Wrap returned pointers in a single-owner Rust guard and keep the library resident until process termination.
- [ ] Implement native startup and the one-worker event drain. Use blocking workers for synchronous FFI, keep the UI executor unblocked, preserve RPC error envelopes and reject calls during closing. The shell must never interpret business method names into legacy operations.
- [ ] Add lifecycle tests: close/reopen window during streaming and pending approval; two windows; duplicate launch/data-directory conflict; exit with active call/poll/run; startup failure. Wire Close to hide/tray and explicit Quit to the ordered DN-L shutdown. Restore from VIVY state when a new window is ready.
- [ ] Add scoped cargo/GUI/package commands to justfile/CI. Retain @tauri-apps APIs needed for the new thin shell; remove only retired business consumers. Do not assert zero Rust/Cargo as the boundary gate.

## Verification and handoff

`cargo test --manifest-path agent-diva-gui/src-tauri/Cargo.toml`; `cargo clippy --manifest-path agent-diva-gui/src-tauri/Cargo.toml --all-targets -- -D warnings`; `pnpm --dir agent-diva-gui build`. Proposed `pnpm --dir agent-diva-gui tauri build` is run only after adding that script. On the target, inspect the installed bundle, execute real DLL startup/close/reopen/quit, and verify process exit and no unrelated process termination. No gateway-port test is needed for a gatewayless product. Fatal DLL failure is a process failure; do not promise isolated backend restart inside the same process.

Return package hash, dependency graph, bridge error tests and native lifecycle transcript. Review focus: loader path hijacking, UI-thread blocking, no-window event loss, late calls during shutdown, freeing memory after unloading.
