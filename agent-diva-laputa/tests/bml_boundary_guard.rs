mod authority_boundary_guard;

use std::path::Path;

use authority_boundary_guard::{scan_forbidden_access, ForbiddenPattern};

const BML_WRITE_METHODS: &[&str] = &[
    ".put(",
    ".put_governed(",
    ".import_records(",
    ".rollback_governed(",
    ".gc_session_scoped(",
    ".gc_stale_session_scoped(",
    ".rollback_canonical_identity(",
];

const SCAN_ROOT: &str = "agent-diva-laputa/src";

// File-level exemptions: components that are legitimately part of the BML
// layer or its allowed write paths. An empty snippet exempts the whole file.
const FILE_ALLOWLIST: &[&str] = &[
    // Storage core itself (internal self-calls, e.g. backup during open).
    "agent-diva-laputa/src/typed_store.rs",
    // Composition facade; the only governance-side component that legally
    // calls BML write APIs (crud_store.put for CRUD and session GC).
    "agent-diva-laputa/src/typed_provider.rs",
    // Record adaptation layer owned by BML.
    "agent-diva-laputa/src/memory_records.rs",
    // Migration tooling (allowed write path per the BML boundary contract).
    "agent-diva-laputa/src/migration.rs",
];

const SNIPPET_ALLOWLIST: &[(&str, &str)] = &[
    // Test helper writing a supersedes tombstone into a TempDir-backed store.
    (
        "agent-diva-laputa/src/service.rs",
        ".put(tombstone, metadata.store_revision, none)",
    ),
];

#[test]
fn governance_modules_must_not_write_bml_directly() {
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("workspace root");

    let violations = scan_forbidden_access(
        repo_root,
        &[SCAN_ROOT],
        BML_WRITE_METHODS,
        &[ForbiddenPattern::new("")],
        FILE_ALLOWLIST
            .iter()
            .map(|path| (*path, ""))
            .chain(SNIPPET_ALLOWLIST.iter().copied())
            .collect::<Vec<_>>()
            .as_slice(),
    );
    assert!(
        violations.is_empty(),
        "governance modules must not call BML write APIs directly:\n{}",
        violations.join("\n")
    );
}
