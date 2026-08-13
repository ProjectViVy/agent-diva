# Plan Approval Card

Implemented the Plan-mode approval experience in the current chat view.

- Added an inline plan approval card with goal, steps, expandable detail preview, execute, and revoke actions.
- Routed revoke through the Vue app, Tauri command, and existing manager plan deletion API.
- Fixed active-plan pointer cleanup when a plan is deleted.
- Fixed an existing duplicate `mask.mode` locale key so the GUI typecheck can complete.

Unrelated working-tree changes were preserved.
