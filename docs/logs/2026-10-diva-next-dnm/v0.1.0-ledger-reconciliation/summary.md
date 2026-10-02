# DN-M — P0-B ledger reconciliation + static closure gate

## Gate delivered

`scripts/ci/check_legacy_frontend_calls.mjs` — TypeScript-AST allowlist gate over `agent-diva-gui/src/**/*.{ts,vue}`:

- `@tauri-apps/api/core` import/dynamic-import only in `src/platform/desktop-host.ts`; any other `@tauri-apps/*` import only in `desktop-host.ts`/`utils/openExternal.ts`;
- any `invoke()`/`invokeCommand()`/`x.invoke()` outside `desktop-host.ts` rejected (literal or dynamic first arg); inside, only `vivy_call` passes;
- `new EventSource|WebSocket|XMLHttpRequest` and `fetch(` rejected outside the dormant pet feature (Manager HTTP/SSE + direct network bypasses);
- provider endpoint literals rejected outside `utils/welcomeConfig.ts` (the wizard's default-endpoint data constant, not a call site);
- `src/features/diva-pet/**` exempt wholesale (dormant native feature per DN-3/DN-6 ruling);
- negative fixtures: `scripts/ci/fixtures/legacy-calls/*.ts` — `--selftest` requires 6/6 firing (legacy invoke, dynamic invoke, aliased invoke, Manager SSE, provider bypass, dynamic import). Wired into CI `gui-check` + `boundary-gate` jobs.

Current state: **gate clean** — zero unclassified production sites, zero reachable legacy business paths.

## Ledger reconciliation outcome

Reconciled rows against `backend-separation-contracts.md` §4/§6 dispositions:

| Ledger area | Status |
|---|---|
| core session/turn/run/cancel | migrated + live evidence (DN-2) |
| approval/question/plan/work | migrated + live evidence (DN-2) |
| settings/tools/MCP/skills/channels/cron/stats | migrated + persist evidence (DN-3) |
| masks | migrated + live contract evidence (DN-4, this stage) |
| speech (STT/TTS) | **Blocked** — no producer module (DN-6) |
| resource import/delete | **Blocked** — only MCP read probes exist (DN-6) |
| persona/memory/ACTMEM/autodream/notebook/reports | **Blocked** — named missing contracts (DN-4) |
| avatar VRM | native (DN-6) |
| pet | dormant native (DN-6 ruling) |
| 21 dead registrations + orphan events | retire pending owner approval |

## Story status

Per the DN-M plan, blocked required domains keep the Story **Blocked**: the static gate is green but cannot waive missing speech/persona/memory producer contracts. DN-M stays open until DN-4's blocked table resolves or the owner approves retirements.
