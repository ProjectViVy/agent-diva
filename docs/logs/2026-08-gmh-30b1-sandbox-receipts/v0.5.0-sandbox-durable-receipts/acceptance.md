# Acceptance

1. Request a command requiring human approval.
2. Approve once and verify the durable aggregate reaches `consumed` before the
   waiting call reports approval.
3. Inspect serialized durable state and verify command and reason are absent.
4. Cancel a scope and verify its Pending aggregate becomes `revoked` while the
   waiter reports cancellation.
5. Confirm existing HTTP-facing request and response serialization tests remain
   unchanged.
