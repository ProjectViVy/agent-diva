# DN-1 v0.1.0 — Verification

| Command | Result |
|---|---|
| `pnpm --dir agent-diva-gui test` | 70 files / 507 tests pass |
| `pnpm --dir agent-diva-gui build` | vue-tsc + vite clean |
| `cargo test --workspace` (src-tauri) | all pass |
| `cargo test -p vivy-bridge` with `DIVA_VIVY_RUNTIME` + `DIVA_VIVY_CONFIG` | 7/7 incl. `real_artifact_dn1_rpc_smoke` |
| `cargo clippy --all-targets -- -D warnings` | clean |

Real-artifact smoke transcript (staged linux/amd64 artifact,
generationId 322a9cbe…):

1. `initialize` → `{protocol_version:"vivy.rpc.v1", capabilities:[…]}`
   (non-empty set asserted).
2. `session/create "dn1-smoke"` → session id; `session/list` contains it;
   `session/get` → `{session, messages}` shape.
3. `run/subscribe run_missing` → `{subscription_id:"sub_…", after_seq:0}`
   (subscribe does not check run existence).
4. `run/cancel run_missing` → RPC error envelope `code:-32004`
   preserved intact through the bridge.
5. `approval/list`, `question/list` → arrays.
6. `session/delete` → `{deleted:true}`; `VivyShutdown` → ok.

Adverse-case coverage (unit tests):

- duplicate/late events: seq ≤ cursor dropped.
- out-of-order: buffered, cursor never advances across a gap; drain on fill.
- bridge gap/lost: all runs flagged needsResync; lost → connection lost.
- terminal-only answer: `answer(runId)` returns terminalSummary w/o deltas.
- two windows: idempotent replay; snapshot authoritative over a second
  window's approval decision.
- timeout after mutation: `unknownOutcome` flag, exactly one wire call.
