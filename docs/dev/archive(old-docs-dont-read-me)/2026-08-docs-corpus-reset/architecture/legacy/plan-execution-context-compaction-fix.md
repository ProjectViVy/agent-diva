# Plan Execution Context Compaction Remediation Plan

## Status

Proposed. This document records a code-change plan only; no runtime behavior is changed by this document.

## Problem

The approval UI defaults execution context policy to `Compact`, but the current execution-start branch only takes the final six in-memory history messages for the first provider request. It neither creates a compaction summary nor records an execution boundary. The full exploration history remains in the session and is available again on later execution turns. `Clear` is likewise request-local: it clears the local history vector without defining a durable execution-context boundary.

This makes the policy label misleading: neither policy establishes a stable, auditable context transition from planning to execution.

## Target Semantics

Session transcripts remain immutable audit history. “Clear” and “Compact” control the model-visible context for the entire execution session, not deletion of persisted chat records.

| Policy | First execution turn | Later execution turns |
| --- | --- | --- |
| Retain | Existing history plus approved plan | Same full history behavior |
| Compact (default) | A quality-checked summary of pre-execution context, approved plan, and execution messages | The same stored summary, approved plan, and only messages after the execution boundary |
| Clear | Approved plan and execution messages only | Approved plan and only messages after the execution boundary |

## Design

1. Extend the execution-session record with an immutable context boundary that identifies the first execution message, and use the existing `compacted_context` field only for the approved summary. Persist both as part of plan approval/startup; do not overload session-wide `last_compacted`, which belongs to generic budget compaction.
2. At the transition into execution, snapshot the pre-execution history. For `Compact`, run the existing LLM summarizer with the same quality gate and store its result in `ExecutionSession.compacted_context`. For `Clear`, store no summary. For `Retain`, do not create a boundary filter.
3. If default compaction cannot produce a usable summary, do not silently fall back to retained context. Keep the execution session pending/blocked, expose an actionable error, and allow an explicit retry or explicit user selection of `Clear`/`Retain`.
4. Replace the one-shot `history.split_off(..6)` and request-local `history.clear()` branches with one shared execution-context builder. It must apply the stored boundary on every execution turn, including overflow retry/rebuild paths.
5. Build provider messages in this order: system prompt; approved-plan system note; optional persisted compacted-context boundary and summary; post-boundary execution history; current turn. The planning transcript must not re-enter through normal `Session::get_history` while an execution boundary is active.
6. Preserve generic token-budget compaction as an independent safety mechanism for post-boundary execution messages. Its persisted `last_compacted` and summary chain must still be injected after the execution-context policy has selected the correct history window.
7. Emit structured observability for context-policy selection, summary creation/failure, boundary index, and model-visible history count. Do not log raw compacted content.

## Implementation Sequence

1. Define the execution-context boundary model and migration in `agent-diva-core/src/planning/report.rs` and `report_store.rs`; expose it through the manager/Tauri projections.
2. Add a focused execution-context service in `agent-diva-agent` that snapshots, summarizes, persists, and reconstructs the execution-visible history. Reuse `ContextCompactor` internals only where their session-wide assumptions are valid.
3. Replace execution-start-only trimming in `agent-diva-agent/src/agent_loop/loop_turn.rs`; use the new service before every provider request and reactive retry rebuild.
4. Make the GUI approval result distinguish successful initialization, initialization failure, and retryable blocked execution. Keep `Compact` as the explicit default.
5. Add deterministic integration tests with a recording provider and persisted execution session fixture.

## Acceptance Criteria

- Given a plan with more than six exploratory messages, when approved with default `Compact`, then the first provider request receives an execution-context summary and no raw exploratory messages.
- Given subsequent execution turns, when the same execution remains active, then raw pre-boundary messages never reappear in provider requests.
- Given `Clear`, when execution starts and continues, then the provider receives no exploration summary or raw exploration messages, while the stored session transcript remains intact.
- Given `Retain`, when execution starts, then existing history remains model-visible unchanged.
- Given summary generation failure under `Compact`, when execution starts, then no implementation provider request is sent with silently retained context and the user can retry or select another policy.
- Given reactive/budget compaction after execution starts, when the request is rebuilt, then the execution boundary and approved plan remain present and generic compaction does not reintroduce pre-execution history.
- Given a restart, when an active execution session is reloaded, then its boundary and compacted summary produce the same model-visible context.

## Validation Plan

Run focused core persistence tests, agent-loop execution-context integration tests, and the existing compaction ordering/retry suite. Then run `just fmt-check`, `just check`, and `just test`; record unrelated existing failures separately rather than weakening the new assertions.

## Non-Goals

- Deleting or mutating the historical session transcript.
- Changing generic compaction thresholds or model-token estimation.
- Reworking unrelated plan approval, TODO materialization, or UI layout behavior.
