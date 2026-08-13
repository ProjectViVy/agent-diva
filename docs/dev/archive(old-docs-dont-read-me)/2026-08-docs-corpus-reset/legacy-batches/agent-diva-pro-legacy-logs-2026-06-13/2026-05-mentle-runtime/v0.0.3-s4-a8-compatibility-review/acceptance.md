# Acceptance

## Criteria

- S4-A8 checks S4-A2 through S4-A6 against the Sprint 3 adapter/runtime
  baseline.
- `MemtleToolkit::tool_definitions()` remains the only dynamic tool definition
  source.
- `MemtleToolkit::call_json(name, args).await` remains the only generic adapter
  execution path.
- Toolkit transport and payload failures remain exposed as
  `ToolError::ExecutionFailed`.
- Invalid dynamic definitions are skipped individually and do not deactivate an
  otherwise-open runtime.
- Prompt routing remains anchored on actual `memtle_status` availability.

## Result

Accepted. No adapter/runtime compatibility break was found.
