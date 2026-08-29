# HQ-02 Acceptance

1. Run the dispatcher tests and confirm one canonical session never exceeds one active executor and preserves FIFO order.
2. Confirm two different canonical session keys overlap at the executor barrier.
3. Hold one running request plus the configured waiters; confirm queue-full and timeout requests do not enter the execution future.
4. Confirm Stop cancels only the running token, while Reset cancels the running token and all waiters.
5. Send turns with different approval policy or mask metadata and confirm each tool/subagent surface uses its own captured snapshot.
6. Confirm existing Bus/direct characterization remains green and production Bus is still globally serialized pending HQ-03 request correlation.
