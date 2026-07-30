# GMH-23C Shadow Recall Acceptance

1. Open `LaputaRecallService` with an exact workspace identity.
2. Execute `recall_shadow` with a canonical `RecallRequest`.
3. Confirm global and exact-session applied records may be selected while
   cross-scope, restricted, tombstoned, superseded, and duplicate records do
   not enter the rendered block.
4. Confirm retrieval failures return a degraded empty outcome with no stale or
   Mentle fallback.
5. Serialize `LaputaRecallMetrics` and confirm it contains no Memory body or
   query text.
6. Confirm the existing production `LaputaMemoryProvider::prefetch` remains a
   no-injection boundary and AgentLoop prompt behavior is unchanged.

No real desktop validation is required because GMH-23C has no user-visible
surface or production runtime activation.
