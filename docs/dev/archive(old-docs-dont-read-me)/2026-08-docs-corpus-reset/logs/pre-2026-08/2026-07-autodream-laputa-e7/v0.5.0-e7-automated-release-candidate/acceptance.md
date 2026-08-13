# Acceptance

Automated E7 acceptance is complete:

- the complete product lifecycle passes without an external provider;
- typed Memory is the tested authority path;
- apply and rollback are idempotent, recoverable and correlated;
- Ask/Mask/cron/background/subagent policy boundaries fail closed;
- health and governance diagnostics contain no Memory payload;
- clean-break, GUI tests/build and Tauri check pass.

G2D+ remains the separate final human-observation gate and is not represented
as complete by this document.
