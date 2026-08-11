# Acceptance

1. Assemble a normal turn and confirm the report totals actual prompt content
   rather than reserving `system_budget_ratio` as consumed tokens.
2. Supply oversized Recall content and confirm it is omitted with
   `layer_soft_limit` or `total_hard_limit`, while Working Memory and the current
   user message remain.
3. Confirm CORE and DEFERRED schemas are reported in separate layers.
4. Force automatic compaction and confirm `macro_compaction` is reported; force
   compaction failure and confirm the 50-message fallback reports
   `legacy_count_cap`.
5. Confirm C1 prefix ordering and cache anchors remain unchanged.
