mod authority_boundary_guard;

#[test]
fn authority_boundaries_enforce_hard_runtime_write_and_read_edges() {
    authority_boundary_guard::assert_authority_boundaries();
}
