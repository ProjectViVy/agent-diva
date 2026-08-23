mod common;

#[path = "common/assert.rs"]
mod authority_assert;

#[test]
fn governance_direct_write_guard_only_allows_laputa_owned_authority_paths() {
    authority_assert::assert_authority_boundaries();
}
