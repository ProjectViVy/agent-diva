# Acceptance

1. Trigger `notebook-daily` and confirm `.agent-diva/autodream/reports/daily/{YYYY-MM-DD}.md` is written with report frontmatter and synthesized content from the day's sessions.
2. Trigger `notebook-weekly` and confirm `.agent-diva/autodream/reports/weekly/{YYYY}-{WW}.md` aggregates daily reports and fills missing days from session fallback data.
3. Trigger monthly Notebook generation and confirm `{workspace}/reports/monthly/{YYYY-MM}.md` contains synthesized monthly sections instead of placeholder text.
4. Delete one expected daily file, regenerate weekly or monthly output, and confirm generation still succeeds with fallback metadata indicating missing daily inputs.
5. Force a monthly generation failure and confirm `{workspace}/reports/monthly/{YYYY-MM}.error.json` is written; regenerate successfully and confirm the stale error marker is removed.
