# G0 Characterization Contract Matrix

G0 combines new cross-surface fixtures with focused tests already located beside
the owning implementation. G1 must preserve this matrix or intentionally revise
the corresponding product decision.

| Contract | Owning test evidence | Expected current behavior |
| --- | --- | --- |
| Normal Agent turn | `characterization_normal_turn_emits_final_response_without_error` | final response is emitted without an error event |
| Manager route status | `g0_runtime_route_contract_matches_fixture` | health, method-not-allowed, and not-found statuses match the frozen fixture |
| Plan SSE event surface | `g0_turn_plan_sse_contract_matches_fixture` | plan readiness remains a typed SSE projection |
| GUI capability boundary | `src/api/capabilities.test.ts` | MANAGER/LOCAL are explicit; DEFERRED/REMOVED have no transport |
| Pending Memory exclusion | `renders_applied_laputa_sections_and_excludes_unapplied_proposals` | only applied authority enters the default prompt; pending proposals are excluded |
| Revision-bound Plan approval | `report_projection_uses_canonical_store_for_revision_approval` and report-store approval tests | stale or mismatched revision cannot authorize execution |
| Plan restart restoration | `agent-diva-manager planning_service` restart test | approved execution and materialized work restore from the canonical store |
| Sandbox approve-once | `approve_once_resumes_exact_request_and_is_consumed` | exact request resumes once and the receipt is consumed |
| Sandbox session scope | `session_approval_is_scoped_by_session_command_and_cwd` | a session grant does not widen command or cwd scope |
| Sandbox timeout/cancel | `timeout_and_cancel_leave_no_pending_requests` | waiters terminate and no pending request remains |
| Sandbox retry | `test_on_failure_retry_requires_cached_approval` | retry outside isolation requires matching cached approval |
| Concurrent approval | approval coordinator and planning CAS tests | first valid decision wins; stale decisions cannot execute |
| Todo concurrent rewrite | `test_concurrent_create_and_update_do_not_drop_items`, `test_concurrent_create_and_archive_do_not_drop_items` | create/update/archive interleavings do not lose records |
| Background registration | `test_tool_assembly_enqueue_background_task_with_run_store` | production assembly exposes enqueue only with its durable run store |
| Supervised context | `test_parse_context_from_metadata` plus Manager runtime bootstrap tests | routing, trace, parent run, and budget metadata survive handoff |

## Failure interpretation

- A red row blocks G1 for that domain.
- A missing test name or fixture is a G0 evidence defect, not permission to infer
  behavior from production code.
- Credential-only external E2E, including StepFun, remains explicitly blocked and
  cannot be reported as passing.
