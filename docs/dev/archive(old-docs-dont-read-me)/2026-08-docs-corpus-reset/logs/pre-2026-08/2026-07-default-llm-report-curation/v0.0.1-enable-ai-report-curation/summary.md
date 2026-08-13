# Default LLM report curation

## Scope

Enabled LLM curation by default for daily, weekly, and monthly Notebook reports. The existing provider/model inheritance and deterministic fallback behavior are unchanged.

## Impact

New or omitted report-curation configuration now uses the configured default model for report summaries. Operators can opt out with `reports.llm_curation.enabled=false`.
