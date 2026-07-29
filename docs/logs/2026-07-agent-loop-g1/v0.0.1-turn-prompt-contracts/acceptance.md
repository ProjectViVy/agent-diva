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
- `process_inbound_message_inner` is 350 lines and only coordinates admission,
  context, iteration/tool, and finalization.
- Attachment/security/vision preparation, tool execution lifecycle, Plan/Soul
  demultiplexing, and persistence each have a single `turn`-module owner.
- Tool failures retain `ToolCallStarted`, `ToolCallFinished(error)`, then
  `FinalResponse` ordering.

Deferred acceptance:

- Real desktop GUI Plan approval smoke for the completed full G1 cut.
- Environment: Windows desktop build with a configured provider, writable
  workspace/session directory, and Manager gateway reachable by the GUI.
- Steps: create a Plan-producing task, approve it and verify execution; repeat
  and reject it; start an effectful tool turn and press stop before execution.
- Observe: one approval card per revision, approve transitions to execution,
  reject leaves no tool side effect, stop emits no late `ToolCallFinished`, and
  the restored session preserves the same Plan/report state after restart.
- Retain diagnostics: Manager and GUI logs, session key, plan id/revision,
  ordered AgentEvent names, tool invocation count, and persisted session JSON
  with secrets redacted.
