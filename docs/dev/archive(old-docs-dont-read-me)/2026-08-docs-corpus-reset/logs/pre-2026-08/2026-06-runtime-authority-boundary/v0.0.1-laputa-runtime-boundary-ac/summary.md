# Summary

Added explicit runtime authority boundary acceptance criteria to the EVO-DIVA epic plan.

Changes:
- Story 5.1 now forbids `ContextBuilder`, subagent identity assembly, and runtime prompt assembly from directly treating legacy authority files as prompt authority after Laputa is available.
- Story 6.1 now states migrated legacy files are migration input or backup only.
- Story 6.3 now requires direct-read guard tests for runtime prompt assembly.

Impact:
- Planning-only change.
- No production code behavior changed in this iteration.
