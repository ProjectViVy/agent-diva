# Summary

Sandbox command approvals can now use the shared durable governance
coordinator. Human decisions are persisted before waiters are released,
approve-once receipts are consumed first, session grants expire after five
minutes, and global rules are written only after a durable Rule receipt.

The ledger record remains payload-free: command text, cwd, and reason stay in
the process-local pending request only.
