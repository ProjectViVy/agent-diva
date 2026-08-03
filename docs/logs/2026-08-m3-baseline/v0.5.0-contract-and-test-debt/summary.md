# M3 baseline summary

The M3 baseline now freezes the existing Command approval HTTP/SSE wire shape in a
versioned JSON fixture. Rust 1.94 all-target test lints in `agent-diva-core` and
`agent-diva-manager` are repaired without changing production behavior. The Manager
log-range regression now exercises a dedicated audit directory without `AppState`
initialization events contaminating the time-window assertion.

This stage does not implement GMH-30B2 and does not change approval runtime semantics,
HTTP paths, JSON fields, Tauri commands, or SSE event names.
