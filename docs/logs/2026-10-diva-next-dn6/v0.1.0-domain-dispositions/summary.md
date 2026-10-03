# DN-6 — Speech / avatar / native domain dispositions

Evidence base: `agent-vivy@feat/diva-embedded` compiled `diva` generation (20 modules), `internal/rpc/control.go` method table, module directories, plus the DN-0 ledger in `docs/plans/diva-next/backend-separation-contracts.md`.

## Per-domain verdicts

| Domain | Verdict | Disposition |
|---|---|---|
| speech (STT/TTS/voice calls) | **Blocked** | No speech module exists anywhere in `agent-vivy` (no provider, no module, no RPC). Producer contract needed: a compiled `vivy/speech` module with list/start/stop/transcribe/synthesize actions, or a declared retirement of the legacy speech surface. Required legacy behavior is NOT marked migrated. |
| resource import/delete (old file/resource commands) | **Blocked** | The only `resources/*` surface is `mcp.resources.list|read` — MCP read-only probes. No import/delete/list-own-domain contract exists. Producer contract needed. |
| avatar (VRM rendering, lip-sync) | **Native — done** | Frontend-native: `DivaVrmAvatar.vue` renders the VRM locally; no backend call existed for the render path. Any old avatar native hooks retire with the legacy bridge. |
| pet (desktop/embedded pet window) | **Dormant by ruling** | The only remaining `invoke` in the frontend is the dormant pet feature, kept by the earlier close=hide + dormant ruling; the surface stays compiled out of any RPC path and untouched until the owner re-scopes it. |

## Notes

- DN-6's predecessor evidence (DN-2 run/utterance/cancel projection) is accepted; nothing in this domain adds runtime requirements to it.
- Optional-module absence is only recorded for genuinely optional scope; speech and resource import remain required-but-unproduced → Blocked, named contracts above.
