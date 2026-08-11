# Summary

Implemented CTX-C2 typed context budgeting. Context inputs and provider-ready
tool schemas now have explicit budget layers, actual token estimates, eviction
policies, and content-free assembly reports. Recall is omitted under its layer
or total pressure while required working-memory and current-turn constraints
remain present.

The legacy history pre-check no longer reports configured system headroom as
consumed tokens. Automatic macro compaction and the 50-message failure fallback
are recorded as `macro_compaction` and `legacy_count_cap` decisions.
