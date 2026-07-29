# Acceptance

- AgentLoop remains the sole public turn entry.
- The existing sandbox/tool registry remains the sole execution path.
- Policy phase is captured once and refreshed before tool execution.
- Runtime prompt contracts expose stable IDs and versions without logging text.
- Compaction and consolidation prompts are owned by their subsystems.
- English and legacy Chinese Plan headings validate in Core and GUI.
- Existing user-visible Chinese copy and stored Plan JSON remain unchanged.
- Admission, context preparation, sampling/stream collection, tool execution,
  and final persistence/events have explicit internal owners.
- Cancellation is checked before executor entry and while collecting provider
  stream events.
- Focused AgentLoop, Clippy, full workspace, CLI, and gateway command gates pass.

Deferred acceptance:

- G1.6 coordinator slimming from 865 lines toward the ~500-line review target.
- Real desktop GUI Plan approval smoke for the completed full G1 cut.
- Before that real-desktop milestone, remind the user with environment, steps,
  expected observations, and diagnostics to retain.
