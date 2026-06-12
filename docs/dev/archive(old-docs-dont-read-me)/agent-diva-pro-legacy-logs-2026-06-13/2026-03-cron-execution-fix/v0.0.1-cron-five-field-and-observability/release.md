# Release

## Method
- No package publishing required.
- Change is source-level and can be released with normal repository deployment pipeline.

## Rollout Notes
- Restart gateway process to load updated cron scheduling logic.
- Existing jobs with 5-field cron expressions will work after restart because next run is recomputed on service start.
