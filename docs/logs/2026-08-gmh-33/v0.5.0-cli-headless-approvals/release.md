# Release

This update is delivered as one focused CLI commit and is not pushed or deployed.

Operator-facing additions:

- `agent-diva approvals list [--status pending] [--session ...] [--json]`
- `agent-diva approvals review [--session ...]`
- `agent-diva approvals decide <request-id> --version <n> --decision <choice>`
- `agent-diva approvals cancel <request-id> --version <n>`
- `agent-diva agent --message ... --approval-mode fail|queue [--json]`

Rollback is the revert of the focused GMH-33 commit. The approval ledger schema,
Manager HTTP contract, and existing GUI/Tauri wire shapes are unchanged.
