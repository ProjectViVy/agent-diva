# v0.4.1 — DN-2A verification

Environment: Linux VM, Node v24.19.0, pnpm 10.33.2, happy-dom vitest.

- `pnpm exec vitest run src/api/vivy/client.test.ts
  src/state/vivy-chat.test.ts src/components/ChatView.test.ts` —
  56 tests pass (9 new: fixture-shape image params, oversized-frame
  rejection with zero mutations, unsupported-MIME draft preservation,
  preset armed-before-turn ordering, mismatch-blocked readback,
  ambiguous-write readback reconciliation, serialized concurrent sends,
  file-picker chip/emit cycle, pick-time reject + oversized-frame block
  with draft intact).
- `just gui-test` — 462 tests, 56 files, all pass.
- `just gui-build` — vue-tsc + vite build green.
- Server contract pinned to source, not assumed:
  `internal/attachment/attachment.go` (MaxCount=4, MaxBytes=5MiB,
  whitelist + content sniff), `internal/rpc/control.go`
  (`attachmentsFromParams`, SupportsImages gate fails open for unknown
  models), `vivy-bridge` `c_input` (serialized frame ≤ 4 MiB).

Environment limits: no Tauri runtime on this VM — the native
`vivy_call` image + permission smoke with a real image-capable model is
recorded as owner-acceptance work; unsupported models surface the exact
server error rather than a simulated success.
