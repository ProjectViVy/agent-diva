# Acceptance

HQ-01 is accepted when:

- One session can hold only one lease and accepted waiters advance in FIFO order.
- Different session keys acquire independently.
- The waiting bound excludes the running lease and rejects the newest overflow arrival.
- Timeout, cancellation, dropped futures, aborted tasks, and failed delivery leave no ghost lease or waiter.
- Session-wide cancellation leaves the running lease untouched and drains only queued work.
- Idle eviction never removes running or queued slots, and close rejects new turns while draining accepted ones.
- No state lock is held across await and the injected clock supports deterministic paused-time tests.
- Core and full workspace gates pass without adding runtime or wire behavior.
