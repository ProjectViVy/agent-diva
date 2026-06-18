# Acceptance

1. Start the manager/runtime and verify a cron job with payload kind `notebook_monthly_report` exists.
2. Trigger a monthly report from Notebook GUI and confirm the manager creates `reports/monthly/{YYYY-MM}.md`.
3. Confirm the generated monthly report contains:
   - `period: monthly`
   - `source: daily_aggregate`
   - `daily_inputs_count`
   - `missing_daily_dates_count`
4. Force a monthly generation failure with no daily inputs/session fallback and confirm `reports/monthly/{YYYY-MM}.error.json` is written with `attempts`.
5. Re-run with valid inputs and confirm the monthly markdown is generated and the error marker is removed.
