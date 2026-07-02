# Acceptance

1. Start the local gateway so `/api/cron/jobs` is available through the Tauri bridge.
2. Open the GUI and confirm the sidebar contains `定时任务管理`.
3. Enter the page and verify:
   - summary cards show total/running/paused/failed counts
   - running tasks are separated from the full task list
   - each task card shows status, schedule, next run, last run, and task details
4. Create a new task from the page and confirm it appears immediately.
5. Edit an existing task and confirm the updated values persist after refresh.
6. Pause and re-enable a task and confirm the status badge changes accordingly.
7. Click `立即执行` and confirm the task enters or passes through the running state and the timestamps refresh.
8. If a task is running, click `停止` and confirm the current run is terminated while the task definition remains.
9. Click `删除` on a stopped task and confirm the card disappears.
10. Click `删除` on a running task and confirm the UI warns that the run will be stopped first, then the task is removed.
