# Release Test Reliability

Three load-sensitive release-gate failures were corrected:

- the QQ invalid-session mock now keeps the socket alive until opcode 9 is observable;
- the CLI logging test executes Cargo's already-built binary instead of nesting Cargo;
- the embedded desktop gateway test observes startup explicitly, bypasses external
  proxies for loopback, and accepts the current degraded-health HTTP contract.
