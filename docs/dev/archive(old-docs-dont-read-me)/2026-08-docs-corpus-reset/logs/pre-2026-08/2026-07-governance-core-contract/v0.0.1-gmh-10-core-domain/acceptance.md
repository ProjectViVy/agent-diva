# GMH-10 Acceptance

1. Confirm `agent_diva_core::governance` exports all GMH-10 public contracts.
2. Confirm Plan, Sandbox, and Memory payload examples validate through the same
   generic `ApprovalRequest<P>` without a shared business payload enum.
3. Confirm unknown values, missing identifiers, invalid expiry, and mismatched
   approve-once receipts fail closed with typed errors.
4. Confirm existing Evolution, Plan approval, and Sandbox approval contracts
   remain unchanged.
5. Confirm the focused and workspace validation commands in `verification.md`
   pass.
