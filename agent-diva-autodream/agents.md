# agent-diva-autodream

## OVERVIEW

Manual AutoDream run lifecycle: collect inputs, run reflection, produce reports/rhythm/metrics, and emit proposals. Durable writes must go through Laputa governance; this crate only drafts.

## WHERE TO LOOK

| Concern | File(s) |
|---|---|
| Service entry point | `src/service.rs` (`AutoDreamService`, `ManualRunTriggerRequest`, `AutoDreamRunStatus`) |
| Worker / stage orchestration | `src/worker.rs` (`AutoDreamWorker`, `AutoDreamWorkerConfig`, `AutoDreamWorkerOutcome`) |
| Input collection | `src/inputs.rs` (`AutoDreamInputCollector`) |
| Reflection engine | `src/reflection.rs` (`SkillReflectionEngine`, `SkillReflectionInput`) |
| Report writing | `src/reports.rs` (`AutoDreamReportWriter`, `RhythmReportWriteRequest`) |
| Rhythm reports | `src/rhythm.rs` (`AutoDreamRhythmReportGenerator`) |
| Metrics | `src/metrics.rs` (`AutoDreamMetrics`) |
| Curation | `src/curation.rs` |
| Storage layout | `src/layout.rs` |
| Error type | `src/error.rs` (`AutoDreamError`) |

## CONVENTIONS

- A manual run is triggered via `ManualRunTriggerRequest` and executed by `AutoDreamWorker`.
- Reports are written as drafts/proposals; authority changes are handed to `agent-diva-laputa`.
- Monthly reports and rhythm generation are separate concerns but share the same proposal boundary.

## ANTI-PATTERNS

- **Do not** write `MEMORY.md`, identity files, or authority store directly.
- **Do not** bypass the proposal/apply boundary when suggesting persona/memory changes.
- Do not run reflection against live memory without `RecallFeedback` plumbing.

## NOTES

- `just e7-recovery-drills` and `just e7-vertical-e2e` exercise AutoDream + Laputa integration paths.
