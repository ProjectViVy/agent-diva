# HQ-04 Acceptance

1. Start concurrent requests in one chat against a retrying provider fixture; confirm every retry
   and final event retains its own request and trace ID.
2. Block one session worker, fill its two default waiter slots, and submit another request; confirm
   `session_queue_full` is returned before the provider observes the rejected request.
3. Let a queued request exceed its configured wait timeout; confirm `session_wait_timeout`, no
   provider call, and no later execution after the running lease is released.
4. Inject a session-worker panic with running and queued requests; confirm all receive
   `session_worker_unavailable`, then submit another request and confirm the worker recovers.
5. While a request is queued in the desktop, confirm the queue badge is visible. When it starts,
   confirm the badge clears. For each terminal code, confirm the placeholder is replaced by a
   localized explanatory system message.
6. Send admission events for another request into the same chat stream; confirm they do not alter
   the active message or busy state.
7. Press Stop for a queued request; confirm the UI explains that queued work is preserved and stays
   attached to the active request. Press Stop for running work and confirm cancellation closes it.
8. Confirm Manager SSE and CLI remote/direct paths preserve queued/running ordering and exact
   session/request/trace correlation through the final response.

HQ-04 is accepted by automated fault-injection and cross-entry regression coverage. Configuration
documentation, final smoke selection, release checklist, and Epic closure remain HQ-05 scope.
