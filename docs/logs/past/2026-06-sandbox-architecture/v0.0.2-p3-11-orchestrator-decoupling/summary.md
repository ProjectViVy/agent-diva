# Summary

- Refactored `ToolOrchestrator::run()` into five private steps:
  - `preflight_guardian()`
  - `resolve_approval()`
  - `select_sandbox()`
  - `execute()`
  - `handle_failure()`
- Kept the public `run()` API and execution behavior unchanged.
- Added a small `ApprovalResolution` carrier to keep state flow explicit between steps.

# Impact

- The orchestrator remains functionally equivalent but is easier to review and evolve.
