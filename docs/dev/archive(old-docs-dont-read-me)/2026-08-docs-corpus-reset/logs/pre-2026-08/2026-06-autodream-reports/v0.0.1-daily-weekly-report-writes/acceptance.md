# Acceptance

User/product acceptance steps:

1. Trigger or call AutoDream daily report generation with date `2026-06-14`.
2. Confirm `.agent-diva/autodream/reports/daily/2026-06-14.md` exists.
3. Confirm the file contains v1 frontmatter with `period: daily`, `date`, `generated_at`, `generated_by: agent-diva-autodream`, and `schema_version: 1`.
4. Trigger or call weekly report generation with week `2026-W24`.
5. Confirm `.agent-diva/autodream/reports/weekly/2026-W24.md` exists and contains `period: weekly` plus `week: 2026-W24`.
6. Confirm evidence references are present and bounded in markdown.
7. Confirm Report System can read the markdown file as plain filesystem content without creating monthly or authority writes.
