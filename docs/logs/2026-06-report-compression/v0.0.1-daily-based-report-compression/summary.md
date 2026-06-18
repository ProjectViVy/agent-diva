# Summary

- Added a shared report parsing and session-digest layer in `agent-diva-core` so higher-level report generation can consume daily markdown files and recover missing dates from session history.
- Implemented AutoDream daily and weekly rhythm generation, including manager-triggered execution for `notebook-daily` and `notebook-weekly`.
- Replaced Notebook monthly placeholder output with daily-based monthly synthesis plus session fallback, metadata emission, and error-marker maintenance.
- Added focused regression coverage for report parsing, AutoDream report generation, manager trigger execution, and Notebook monthly synthesis.
