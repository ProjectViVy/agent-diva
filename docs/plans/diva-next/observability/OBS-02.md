# OBS-02 — Durable observable model-call lifecycle Implementation Plan

> **For agentic workers:** After specification review and implementation authorization, use `superpowers:executing-plans`. Read this plan and OBS-D1 first. Delegation follows repository instructions, not this heading alone.

**Goal:** Journal each real provider invocation and its usage/finish without changing Eino execution or live-stream persistence ordering.
**Architecture:** Extend the existing model observer; Service commits before emitting or exposing provider chunks.
**Tech stack:** Go, pinned Eino v0.9.13 and pinned provider modules; existing Journal.
**Epic / requirements:** OBS-A / O2
**Specification:** [architecture.md](architecture.md), OBS-D1. **State/evidence:** [parent index](../index.md).
**Immediate inputs:** None beyond OBS-D1 review; backend work starts against inspected VIVY 8fc6bea. Output schemas must be accepted before OBS-03/04; DN-0 later selects the integration pin for native packaging.
**Global constraints:** one VIVY authority; existing Eino execution and Recipe/Generation path; no legacy fallback, production data, synthetic usage, guessed wire adapters or fabricated acceptance. VIVY code changes require its AGENTS and pinned-capability check. DIVA writers claim LOCK.md before edits and serialize shared files.
**Path convention:** VIVY paths are relative to the selected agent-vivy checkout; DIVA paths to agent-diva. New paths explicitly marked proposed. Source baselines are in OBS-D1.

## Files and exact seam

Modify VIVY `internal/runtime/{model_stream_observer.go,model_stream_observer_test.go,service.go,service_test.go,mapper.go,payloads.go,request.go,engine.go,compaction_middleware.go,budget.go,budget_test.go}`, `internal/domain/event.go`, `schemas/events/payloads/{model.request.json,model.usage.json}`; create proposed `schemas/events/payloads/model.call.finished.json` and proposed `internal/runtime/payload_schema_test.go` using the already pinned jsonschema/v6 validator. Update `schemas/events/run-event.schema.json` and `schemas/README.md` for the new event/version. Do not add provider HTTP wrappers for optional bucket presence: inspected scalar-zero buckets stay unknown. Only existing route metadata is consumed; future raw-presence adapter changes require separate evidence. Update the existing event schema registry if it enumerates payload kinds.

Use `modelCallObserver.Begin/Chunk/End` and `modelCallMeta` exactly as [D2](architecture.md#d2--capture-calls-at-the-real-model-boundary). WithTools keeps observer wrapping and route binding. Wrap BaseModel summary primary/fallback as well as ToolCallingChatModel chat routes; copy source/provider/model metadata into bound clones. Produce v3 request/v2 usage/v1 finish schemas plus main/child/summary, setup-failure, cancelled and legacy fixtures.

## Ordered tasks

- [ ] Record Eino capability evidence: ChatModel Generate/Stream/WithTools, callbacks stream Copy timing, pinned OpenAI/Claude usage conversion and SDK retry visibility. Prove the current producer tee is necessary for commit-before-Send; reject callback-only replacement.
- [ ] Add failing `TestObserverLifecycleGenerateAndStream` with exact start → optional usage samples → finish order, unique call IDs, returned errors and persisted-before-forwarded assertions. Add `TestObserverRecordsStreamSetupFailure`, `TestObserverBoundToolsPreservesCallScope` and source attribution across summary/child routes.
- [ ] Move the drive-leg surrogate request into Begin before inner invocation, using actual input, pinned model.GetCommonOptions/bound tools and selected route. Preserve documented text-only digest coverage. StreamOpened sets the mapper marker only after Stream succeeds. Retain v1/v2 parsing only for replay. A start append failure must assert zero provider invocations.
- [ ] Observe Generate once; extend the existing Stream pump, not another tee. Suppress duplicate materialized usage from the mapper. Multiple samples carry the same call ID; separate calls carry separate IDs. Preserve latest usage and usage_present/response_complete at End; finish does not carry duplicate counts. Keep model.completed and existing message/tool projection unchanged. Add bounded per-call usage-only normalization using pinned Eino monotonic merging before event emission; total follows input+output. Test Claude start-input plus output-only delta and OpenAI final usage-only chunk; contradictory counters set normalization_partial instead of complete evidence.
- [ ] Add deterministic tests for EOF/read error, consumer Close, blocked upstream cancellation, approval/question resume, concurrent chunks and persistence failure. Drain/cancel producers before run terminal; use the existing bounded terminal persistence context for caller cancellation. Never append after run closure or publish an uncommitted event.
- [ ] Verify missing usage emits no fake sample, reported zero is retained, reasoning/cache scalar zero remains unknown while positive values become known optional pointer fields; test known-zero/absent wire fixtures separately, and crash/failed finish replay is interrupted. Do not claim SDK-internal retries or auxiliary calls are observed.
- [ ] Test observed call admission before invocation and unchanged tool/retry limits. Add `TestObservedCallSettlementAtMaxEvents`: v3 request/ordinary samples consume MaxEvents; exactly one changed End sample tagged settlement:true plus finish may use the explicit bounded exemption. Assert at most two extra records per admitted call, no provider work admitted by exemptions, terminal closure and no duplicate End. Deduplicate equal samples.
- [ ] Add `TestObservedCallBudgetReplayParity` for new/legacy/mixed parent-child histories and cancellation. Replay applies the same exemptions and charges model work once at v3 requests per run, preserving legacy fallback mode elsewhere. Update BudgetPolicy comments/docs for this explicit quota amendment; no silent MaxEvents bypass.
- [ ] Add `TestModelCallPayloadSchemas` validating actual producer events plus historical v1/v2 request and v1 usage fixtures; assert missing IDs, unknown fields, invalid outcome/presence and unsafe counts fail the closed schema. Validate old and new payload fixtures; commit `feat(runtime): journal observable model call lifecycles`.

## Verification and handoff

VIVY: `go test ./internal/runtime ./internal/domain`, `go test -race ./internal/runtime`; `go test ./internal/runtime -run '^TestModelCallPayloadSchemas$'` for the proposed schema test, then `just ci`. Expected red: baseline lacks actual per-call starts/finish and Generate coverage. Expected green: ordering/cardinality/closed-run invariants hold; historical schemas and existing streaming/barrier/resume tests pass.

Return capability evidence with module paths/versions, exact payload fixtures, commit, commands, route/zero/absence table and producer lifecycle transcript. Review focus: no duplicate accounting path, start failure before provider side effects, cancellation/drain before terminal, model.completed compatibility. Schema changes are not accepted with fixtures alone; actual persisted events must match.
