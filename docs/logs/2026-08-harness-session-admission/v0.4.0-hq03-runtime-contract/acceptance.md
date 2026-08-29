# HQ-03 Acceptance

1. Start two sessions against a provider fixture that blocks the first call; confirm the second
   session reaches the provider before the first is released.
2. Submit three turns to one session with the default limits; confirm one runs, two wait in FIFO
   order, and a fourth receives `session_queue_full` before provider/tool/BML work.
3. Stop by the running request ID and confirm its cancellation outcome includes request, trace, and
   session identity. Stop by a queued request ID and confirm `queued_preserved` without reordering.
4. Reset a session with running and queued turns; confirm queued turns receive `session_reset` and
   durable cleanup completes only after the running turn exits.
5. Open two streams for the same chat with different request IDs; confirm each receives only its own
   delta/final/error/admission events and SSE IDs match that request.
6. Omit `session_admission` from an existing config and confirm the frozen defaults apply. Set queue
   depth to zero and confirm one turn may run but none may wait.
7. Confirm session history, Plan/Sandbox/Approval behavior, BML/Persona authorities, and MessageBus
   transport responsibilities are unchanged.

Cross-entry provider fault injection and end-to-end backpressure UX remain the explicit HQ-04 scope.
