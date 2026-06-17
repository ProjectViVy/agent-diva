mod authority_boundary_guard;

#[test]
fn governance_direct_write_guard_only_allows_laputa_owned_authority_paths() {
    authority_boundary_guard::assert_authority_boundaries();
}
