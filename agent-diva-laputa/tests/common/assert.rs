use super::common::{scan_forbidden_access, ForbiddenPattern};

const FORBIDDEN_PATTERNS: &[ForbiddenPattern] = &[
    ForbiddenPattern::new(".laputa/sections"),
    ForbiddenPattern::new(".laputa/changelog"),
    ForbiddenPattern::new(".laputa/audit"),
    ForbiddenPattern::new(".laputa/rollback"),
    ForbiddenPattern::new("MEMORY.md"),
    ForbiddenPattern::new("IDENTITY.md"),
    ForbiddenPattern::new("SOUL.md"),
];

const STATIC_SCAN_ROOTS: &[&str] = &[
    "agent-diva-agent/src",
    "agent-diva-autodream/src",
    "agent-diva-manager/src",
    "agent-diva-cli/src",
    "agent-diva-gui/src-tauri/src",
];

const WRITE_CALL_MARKERS: &[&str] = &[
    "fs::write(",
    "std::fs::write(",
    "tokio::fs::write(",
    "File::create(",
    "OpenOptions::new()",
    "atomic_write(",
    "atomic_write_json(",
];

const READ_CALL_MARKERS: &[&str] = &[
    "fs::read_to_string(",
    "std::fs::read_to_string(",
    "fs::read(",
    "std::fs::read(",
    "tokio::fs::read_to_string(",
    "tokio::fs::read(",
    "file::open(",
];

pub fn assert_authority_boundaries() {
    let repo_root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root");

    let write_violations = scan_forbidden_access(
        repo_root,
        STATIC_SCAN_ROOTS,
        WRITE_CALL_MARKERS,
        FORBIDDEN_PATTERNS,
        &[],
    );
    assert!(
        write_violations.is_empty(),
        "unauthorized durable authority writes found:\n{}",
        write_violations.join("\n")
    );

    let read_violations = scan_forbidden_access(
        repo_root,
        &["agent-diva-agent/src", "agent-diva-autodream/src"],
        READ_CALL_MARKERS,
        FORBIDDEN_PATTERNS,
        &[],
    );
    assert!(
        read_violations.is_empty(),
        "unauthorized runtime authority reads found:\n{}",
        read_violations.join("\n")
    );
}
