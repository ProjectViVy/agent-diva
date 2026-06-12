# Summary

- Relaxed global config validation so incomplete enabled channel credentials no longer block config loading or gateway startup.
- Added runtime channel readiness checks so invalid enabled channels are skipped with warnings instead of crashing the backend.
- Aligned outbound channel subscription with runnable channels only, preventing sends to channels that were enabled but skipped.

# Impact

- Gateway availability is now decoupled from partial channel setup.
- Channel status semantics remain intact: enabled but not ready channels still surface as needing setup.
