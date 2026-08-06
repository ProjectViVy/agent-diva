# Verification — GA-MEM-PARITY Wave 6

## 验证命令与结果

### S1（memory_list superseded 过滤）

```
cargo fmt -p agent-diva-laputa                    → OK
cargo clippy -p agent-diva-laputa --all-targets -- -D warnings → OK
cargo test -p agent-diva-laputa                   → OK (all tests pass)
```

关键测试：`f7_tombstone_lifecycle_filters_startup_and_search` 断言修正为
`assert!(entries.iter().all(|e| !e.content.contains("falcon")))` 验证 list
不再返回 superseded 记录。

### S2（F4 同会话热注入）

```
cargo fmt -p agent-diva-laputa                    → OK
cargo clippy -p agent-diva-laputa --all-targets -- -D warnings → OK
cargo test -p agent-diva-laputa                   → OK
```

关键测试：`f4_memory_add_visible_in_same_session_startup` — memory_add 后
**不重开 provider**，`system_prompt_block` 立即包含新内容。

### S3（B9 soft evidence advisory）

```
cargo fmt -p agent-diva-core -p agent-diva-laputa -p agent-diva-agent → OK
cargo clippy -p agent-diva-core -p agent-diva-laputa -p agent-diva-agent -p agent-diva-autodream --all-targets -- -D warnings → OK
cargo test -p agent-diva-core memory              → 53 passed
cargo test -p agent-diva-laputa                   → all passed (incl. wave6_tests)
cargo test -p agent-diva-agent                    → all passed
cargo test -p agent-diva-autodream                → 11 passed
```

关键测试：
- `wave6_tests::memory_add_with_evidence_no_advisory` — 传 evidence_refs →
  Applied 无 advisory
- `wave6_tests::memory_add_without_evidence_has_advisory` — 不传 → advisory
  包含 "no evidence_refs"

### 既有问题

`agent-diva-manager` 有 4 个 pre-existing 编译错误
（`BoundedReflectionInput` 缺 `superseded_memory_digests` 字段——Wave 5 S2
引入但未在 manager 测试中更新）。不在 Wave 6 范围内。
