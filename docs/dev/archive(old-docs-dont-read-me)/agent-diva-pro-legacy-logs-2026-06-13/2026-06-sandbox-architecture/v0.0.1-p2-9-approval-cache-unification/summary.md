# Summary

- Refactored `SandboxManager` to expose high-level approval cache operations instead of requiring orchestrator callers to lock the shared store directly.
- Switched `ToolOrchestrator` to use manager-owned approval cache methods for cached lookup and one-time approval consumption.
- Added temporary `From` bridges between `ExecApprovalRequirement` and `ApprovalRequirement` with TODO notes for the later type unification epic.

# Impact

- Internal sandbox approval cache access is now centralized in `manager.rs`.
- Public runtime behavior is unchanged.
