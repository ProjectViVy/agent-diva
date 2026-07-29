# G0 Characterization Contract Matrix

| Contract | Executable evidence | Frozen behavior |
| --- | --- | --- |
| Normal Agent turn | `characterization_normal_turn_emits_final_response_without_error` | final response without error |
| Manager routes | `g0_runtime_route_contract_matches_fixture` | health/405/404 fixture |
| Plan SSE | `g0_turn_plan_sse_contract_matches_fixture` | typed readiness projection |
| GUI boundary | `src/api/capabilities.test.ts` | DEFERRED/REMOVED have no transport |
| Memory exclusion | `renders_applied_laputa_sections_and_excludes_unapplied_proposals` | only applied authority reaches prompt |
| Plan revision/restart | `report_projection_uses_canonical_store_for_revision_approval` | stale revision denied; execution restores |
| Sandbox lifecycle | `approval_coordinator` tests | exact once/scope/timeout/cancel behavior |
| Sandbox retry | `test_on_failure_retry_requires_cached_approval` | retry needs cached approval |
| Todo concurrency | `test_concurrent_create_and_*_do_not_drop_items` | rewrites preserve records |
| Background registration | `test_tool_assembly_enqueue_background_task_with_run_store` | durable store required |
| Supervised metadata | `subagent_run_handler::tests` | routing/trace/parent/budget parses |
| Mentle rebuild | `tool_rebuilds`, Mentle/default suites | active provider prompt survives; retired L2 stays absent |

A red or missing row blocks the affected G1 integration. StepFun external E2E
remains credential-blocked and is not reported as passing.
