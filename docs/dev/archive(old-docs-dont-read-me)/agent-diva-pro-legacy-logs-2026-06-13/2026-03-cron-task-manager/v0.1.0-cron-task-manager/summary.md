# Summary

- Added a new `定时任务管理` page in `agent-diva-gui` and linked it from the sidebar.
- Exposed cron management through the full stack: `CronService` -> manager commands -> HTTP API -> Tauri commands -> Vue UI.
- Added cron lifecycle actions for list, create, edit, enable/disable, run now, stop current run, and delete.
- Added runtime-facing cron DTOs so the GUI can display current running tasks, computed lifecycle status, next run time, last run result, and task details.
- Added cron runtime tracking and cancellation support in `agent-diva-core`, including delete-while-running handling.
- Fixed several pre-existing GUI TypeScript issues and one pre-existing clippy issue in `agent-diva-agent` so validation can pass.
