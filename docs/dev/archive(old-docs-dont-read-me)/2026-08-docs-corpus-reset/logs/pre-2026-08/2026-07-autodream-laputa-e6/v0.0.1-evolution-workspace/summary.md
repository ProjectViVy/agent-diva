# E6 Evolution Workspace Summary

The Evolution page is now the unified operational surface for the automated
AutoDream–Laputa chain. It displays typed Memory authority health and revision,
AutoDream orchestration phase/attempt/input coverage, governed proposals,
changelog/rollback status, and recent payload-free Recall feedback.

Users can start or cancel a run, open proposals produced by a run, edit proposal
content into a new revision, approve/apply, reject, defer, and rollback without
editing configuration files or invoking an API manually. Editing invalidates
the prior governance revision through the existing backend contract.

The Manager exposes a bounded Recall-feedback read endpoint and Tauri forwards
both that endpoint and detailed health status. No Memory payload is added to
the DTO.
