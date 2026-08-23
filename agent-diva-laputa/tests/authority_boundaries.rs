mod common;

#[path = "common/assert.rs"]
mod authority_assert;

#[test]
fn authority_boundaries_enforce_hard_runtime_write_and_read_edges() {
    authority_assert::assert_authority_boundaries();
}
