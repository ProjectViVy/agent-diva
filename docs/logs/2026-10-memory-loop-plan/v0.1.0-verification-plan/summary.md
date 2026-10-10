# Memory-loop verification planning

Cloned agent-diva at 518a33ef09858ee1bb190579dd7529aceaa15dd6 and inspected
its source lock, Wails entrypoints and historical Windows evidence alongside
agent-vivy 017ec8cc37970b291e04c619990aed00d5403116 and
laputa ff3936f44ff8cf08c12af2cf698c194cfe474fd3.

Delivered a source-grounded verification plan with eight tasks, 28 scenario
classes, real-storage and actual-model-input gates, six process-crash cuts,
Windows acceptance, and a fixed 30-trial live-model evaluation proposal.
Estimate: 10 verification days plus 3–5 repair days, assuming environment
availability. Index owns status; TODOLIST records unverified gaps.

No product code, dependency pins or architectural rules changed.
