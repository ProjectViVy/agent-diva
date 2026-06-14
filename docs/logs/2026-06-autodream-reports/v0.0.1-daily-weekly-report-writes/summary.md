# Summary

Implemented AutoDream daily and weekly markdown report writes for Story 3.5.

Changes:
- Added `AutoDreamReportWriter` and typed report request/result APIs.
- Added daily and weekly report path helpers under `.agent-diva/autodream/reports`.
- Added v1 markdown frontmatter and bounded evidence references.
- Reused atomic temp-file plus rename writes and added parent-directory sync where supported.
- Added report integration tests for paths, schema, evidence, replacement, unsupported monthly input, traversal rejection, and readback.

Impact:
- AutoDream can produce daily/weekly report files for Report System consumption.
- Report System remains a read-side consumer and does not need AutoDream internals.
