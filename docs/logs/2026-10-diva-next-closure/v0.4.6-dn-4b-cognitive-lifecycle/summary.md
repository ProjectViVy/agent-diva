# v0.4.6 — DN-4B primary FrozenCore + safe cognitive lifecycle (agent-vivy)

## Scope
Primary model input carries the session-frozen FrozenCore v2; cognitive
workflow admission verifies persisted pins against the bound composition;
the wake loop settles terminal outcomes into durable block reasons;
embedded lifecycle owner starts/stops cognition exactly once.

## Changes (agent-vivy `feat/dn-closure-wave1`)

- `internal/runtime/cognitive_binding.go` — `CognitiveBinding` gains
  `Primary CognitivePrimaryPreparer` + `Resolve`; `workflowNodes` decodes
  `laputaevolution.Input` from persisted input and refuses scope /
  destination / strategy-digest mismatch plus mission-revision drift via
  `RunBinding.CheckMissionRevision`.
- `internal/runtime/workflow_service.go` — all three workflowNodes call
  sites (dedupe, new-commit, recovery) surface the pin-verification error.
- `internal/runtime/service.go` — inside `runWithAdmissionGate`,
  `Primary.Prepare(ctx, PrimaryContextInput{SessionID, RunID, WorkspaceID,
  BudgetBytes})` gates admission before `buildPromptSnapshot`; frozen
  text+digest feed `PromptInput`.
- `internal/runtime/prompt_state.go` — `PromptInput` gains FrozenText /
  FrozenDigest (must travel together); `composeAuthoritativeInstruction`
  inserts frozen after persona, before code-mode/frame/mask; payload gains
  `Frozen *PersonaSnapshot{Source:"garden/frozen-core", Revision:"v2"}`.
- `internal/storage/masks.go` — `RunPromptPayload.Frozen`.
- `internal/modules/diva-cognitive/factory.go` — `Prepare` validates
  `frozen.Validate()` (explicit v1/corrupt reject) and errors on
  budget overflow instead of silent truncation.
- `internal/runtime/cognitive_service.go` — `cognitiveState.Blocked` with
  reasons cancelled / unknown_outcome / attempts_exhausted /
  unavailable; settle switch classifies terminal runs
  (`cognitiveRunBlocked` reads engine `recovery_required` + node
  `outcome_unknown`); auto wakes fenced on Blocked, manual trigger clears;
  live dedupe adopt instead of Attempt++; capture provider skips
  `sess_cognitive_supervisor`; `CognitiveLoopActive()`.
- `internal/app/app.go` — binding wires `Primary: cognitiveBundle,
  Resolve: cognitiveBundle.ResolveBinding`; `StartEmbeddedServices`
  starts the cognitive loop (idempotent); `Close` stops it before drains.
- Tests: `internal/runtime/cognitive_primary_test.go` (4 named tests),
  `internal/app/embedded_cognitive_lifecycle_test.go` (1 named test);
  trusted fixtures now carry `cognitiveFixtureRunBinding` so persisted
  pins match the bound composition.

## Rulings
- FrozenCore persistence needs no extra VIVY call: `ReadFrozen` is
  get-or-capture (INSERT OR IGNORE, first capture wins, session-frozen).
  Fresh sessions self-bind; fork = new session id = new snapshot;
  rewind = same session = same snapshot.
- `Blocked` is only cleared by the deliberate manual `TriggerCognitive`;
  policy disable/enable never clears unknown state.

## Pending owner acceptance
All DN-4B claims await owner acceptance; green tests ≠ acceptance.
