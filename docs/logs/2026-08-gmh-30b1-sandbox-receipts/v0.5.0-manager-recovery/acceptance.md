# Acceptance

1. Leave a CommandExecute request Pending and restart the recovery coordinator.
2. Verify it becomes Revoked and no command is executed or rebroadcast.
3. Leave a session-grant aggregate Allowed but unconsumed and repeat recovery.
4. Verify it becomes Revoked.
5. Run Manager command-approval route tests and confirm HTTP JSON behavior is
   unchanged.
