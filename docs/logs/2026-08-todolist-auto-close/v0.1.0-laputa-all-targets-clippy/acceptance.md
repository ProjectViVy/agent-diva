# Acceptance

- [x] `cargo clippy -p agent-diva-laputa --all-targets -- -D warnings` 无 warning。
- [x] `authority_boundaries` / `direct_write_guard` / `bml_boundary_guard` 行为不变且通过。
- [x] BML 写 API 扫描针仍包含已退役的 `put_governed` / `rollback_governed`。
- [x] 生产 `TypedMemoryStore` 与 schema 未改。
