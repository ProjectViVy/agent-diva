# Original-window interruption and recovery classification

Implementer: one sequential lane. Scope: real subprocess test transport and INOFY Core Storage adapter; no new runtime, backend, dependency pin or automatic retry policy.

Actual App interruption probe waits for a recorded reflect HTTP request whose real response is held, records original Run/window/source, kills only the owned subprocess and waits its real organism lease expiry. Restart uses the same profile and generated composition; observe workflow/get, actual automatic ticks, manual controls and native canonical records. This pre-effect probe does not claim any of the six formal crash cuts or ten required repetitions.

Observed failure: immediate restart correctly refuses the existing 30-second SQLite lease. After actual lease expiry the original workflow projection remains running. Pinned INOFY program.go Run classifies loaded running state through the public RunStore commit, but execution.go commitID restarts ordinal one in generation zero. That raw commit ID is already the original admission ID. Core Storage rightly rejects different content under that ID; launch currently discards the error.

Architecture order: preserve Service.Run/Journal/WorkflowStepStore as the single authority; use pinned INOFY/Eino execution and public RunStore contract; do not parse private checkpoints, mutate cached sources or fabricate outcomes. The missing capability is a distinct durable ID for the recovery classification transition. The smallest host adaptation qualifies only a running-to-recovery_required commit with the writer epoch, preserving all normal/admission commit IDs and original receipts. Same recovery retries must return the original receipt; stale/changed writers still obey Core Storage fences. This does not authorize workflow replay or repair unknown effects.

Acceptance: first reproduce the collision with real SQLite store; then classify durably, replay the same recovery/admission receipts and retain a non-terminal native Run. Rerun real App subprocess crash proof plus directly affected INOFY/recovery/race tests. Preserve all initial test/lease/projection failures. Final CI/source-bound artifact and whole-phase review remain pending.

Follow-up visibility: native recovery_required deliberately stays non-terminal, so the admission controller previously reported active indefinitely. The actual App probe established the recovered state and no replay; the controller regression first returns active. Read only the committed public Core WorkflowStep projection during active-workflow admission checks and expose unknown_outcome while retaining original Run/window/watermark. No effect execution or outcome fabrication is added.

Scope refinement: only later writer epochs receive the recovery-classification namespace. First-writer unknown transitions and all ordinary/admission IDs remain unchanged.
