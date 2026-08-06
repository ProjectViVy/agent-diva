# Summary — GA-MEM-PARITY Wave 6（真机前收口）

- 版本：`v0.0.10-wave6-pre-g2d`
- 日期：2026-08-07
- 类型：代码级缺口修复（3 切片 + 1 docs）
- 前置：`v0.0.9-wave5-consolidation-and-gc`

## 做了什么

Wave 6 修复 Wave 0–5 之后遗留的三个代码级缺口，为 G2D+ 真机桌面验收扫清障碍：

### S1 laputa：memory_list superseded 过滤 — `2572a478`

`memory_list` 之前不过滤 superseded 记录（search 和 startup 已修），用户调用
`memory_list` 会看到已被替换的旧记忆。

- `typed_provider.rs` `memory_list`：增加 `superseded_target_ids()` 调用 +
  `.filter()` 排除，与 `memory_search` 完全同模式。
- `tests/wave5_acceptance.rs`：修正 S3 中记录的 known gap assertion（从
  "assert IS in list" 改回 "assert NOT in list"）。

### S2 laputa：F4 同会话热注入 — `56dd15d5`

`memory_add` 写入 SQLite 后，`startup_markdown` 缓存仍是 `open()` 时的旧值，
直到重启 provider 才刷新。

- `startup_markdown` 字段类型从 `Option<String>` 改为
  `std::sync::RwLock<Option<String>>`（因 `system_prompt_block` 是同步 `&self`）。
- 提取 `render_startup_index(store, l1_budget) -> Option<String>` 辅助函数。
- 新增 `refresh_startup_markdown(&self)` async 方法。
- `memory_add` Applied 后调用 `refresh_startup_markdown()`。
- `f4_memory_add_visible_in_same_session_startup` 测试验证同会话可见。

### S3 core + laputa + agent：B9 soft evidence advisory — `c2e17e5e`

L0_MEMORY_POLICY 公理 1 要求 "persist facts confirmed by tool results or
explicit user statements"。完整强制改动面大；本 Wave 做软校验子集。

- `MemoryCrudOutcome::Applied` 新增 `evidence_advisory: Option<String>`：
  `None` = 有 evidence；`Some(text)` = advisory（存储但无工具验证）。
- `MemoryAddRequest` 新增 `evidence_refs: Vec<EvidenceRef>`（serde default
  向后兼容）。
- `typed_provider::memory_add` 传递 evidence_refs 到 MemoryRecord，空时设
  advisory + info 日志。
- 全 workspace Applied 构造/析构更新（core/laputa/agent/autodream 8 个文件）。
- `wave6_tests` 两个用例验证有/无 evidence 的 advisory 语义。

### S4 docs 收口 — 本提交

- `TODOLIST.md`：Wave 6 状态 + 切片勾选 + 延期项标记。
- `memory-write-paths-contract.md`：删除已修 known gap；补 F4 refresh 语义
  + B9 evidence advisory 契约。
- 四件套文档。

## 影响范围

- 核心类型变更：`MemoryCrudOutcome::Applied`（新增字段）、
  `MemoryAddRequest`（新增字段）——所有调用点已更新。
- 行为变更：`memory_list` 不再返回 superseded 记录；`system_prompt_block`
  在 CRUD 写入后即时反映新内容。
- 向后兼容：`evidence_refs` 和 `evidence_advisory` 都使用 serde default，
  旧序列化数据可正常反序列化。
