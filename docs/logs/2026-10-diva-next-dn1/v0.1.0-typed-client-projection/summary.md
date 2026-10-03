# DN-1 v0.1.0 — Typed VIVY client + session projection

## What changed

- `agent-diva-gui/src/api/vivy/contracts.ts` (new): wire types mirrored from
  the frozen DN-0 fixture (`fixtures/core-rpc.json`, status captured):
  `BridgeErrorBody`, `VivyCallError` (with `unknownOutcome`),
  `InitializeResult`, session/turn/run/approval/question contracts,
  `RunEvent`, `WireEvent` (vivy notification | bridge gap/lost).
- `agent-diva-gui/src/api/vivy/transport.ts` (new): `VivyTransport`
  interface + `createTauriTransport()`. Sole business-facing import of
  `platform/desktop-host`; `vivy:event` fan-out, close-detach.
- `agent-diva-gui/src/api/vivy/client.ts` (new): `VivyClient.call<T>`,
  typed wrappers (initialize, session CRUD, turn/start, run subscribe/
  get/log/cancel, approval + question list/respond). Preserves bridge
  error envelopes verbatim, never retries; mutations submitted before a
  `timeout`/`transport_lost` set `unknownOutcome` for read-reconcile.
- `agent-diva-gui/src/state/vivy-session.ts` (new):
  `VivySessionProjection` — connection state machine, per-run cursor +
  out-of-order buffer + needsResync, streamText/terminalSummary,
  pending approvals/questions, snapshot-authoritative `applySnapshot`,
  listener-before-snapshot `attach`, `resync()`.
- `agent-diva-gui/src/api/capabilities.ts` (rewritten): 23-entry GUI
  capability ledger keyed to DN-0 dispositions; `negotiateCapabilities`
  derives available/absent/disconnected/failed from the negotiated
  `initialize` capability set; `markCapabilityFailed`. No static MANAGER
  authority, no localStorage domain truth, no fake-success fallback.
- `.gitignore`: negation so `agent-diva-gui/src/state/` survives the
  global `state/` rule.
- `crates/vivy-bridge/tests/ffi.rs`: `real_artifact_dn1_rpc_smoke`
  (env-gated) driving initialize/session CRUD/run subscribe/cancel
  error/approval+question list through the staged DLL.

## Impact range

Frontend VIVY seam + projection only; no product screen rewires old
invoke paths yet (DN-2). Shell crate untouched except one test.

## Fixture version

`fixtures/core-rpc.json` `payload_version: 1`, status `captured`
(DN-0 live transcript, 38 frames + 18 run events).
