# Summary

- Moved the primary plan-mode UX from the standalone planning page toward the active chat surface.
- Added structured planning runtime events across `agent-diva-core`, `agent-diva-agent`, `agent-diva-manager`, and the Tauri bridge so the GUI can react to live todo/approval state instead of parsing tool text.
- Added a manager/runtime approval path and wired the GUI to show an inline approval bar plus an execution progress strip in the current chat view.
- Left `PlanningView` and the old plan list APIs intact as a secondary/history surface.
