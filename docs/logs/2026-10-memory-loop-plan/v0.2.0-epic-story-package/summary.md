# Epic–Story verification plan package

Date: 2026-10-10 (Asia/Shanghai). Owner requested a detailed Superpowers
handoff package using Epics and Stories instead of time-of-day scheduling.

Replaced the single active plan with five Epics and thirteen independently
readable Stories. Added requirements, test/evidence contracts, an execution
runbook, 28-case acceptance matrix, metadata mirror and a package checker.
Each Story includes scope, precise files, inputs/outputs, tests/assertions,
steps, commands, dependency gates and handoff expectations.

The package README alone owns live Story status. The old file and parent
index link to it. Backlog mappings use Story IDs. Base effort is 10.5 person-days
(the prior ten plus 0.5 for reusable fixtures/evidence checks); the shared
repair reserve remains 3–5 days. Corrected Garden console install instructions
to use its existing pnpm lock.

Product code and dependency pins remain unchanged. No runtime verification,
model calls, Windows runs, PR, branch push or release occurred.
