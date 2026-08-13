# GMH-23B Release

No deployment or runtime cutover is performed. The new API is Rust-only and
unregistered. Existing file-first proposal, authority, provider, Manager,
Tauri, GUI, and AgentLoop behavior remains unchanged.

GMH-23C may consume bounded FTS candidates. GMH-23D owns governed apply, and
GMH-24 owns offline import, cutover, rollback, and Mentle clean-break.
