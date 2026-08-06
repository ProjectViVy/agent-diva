# Summary — GA-MEM-PARITY Wave 0（诚实与契约）

- 版本：`v0.0.4-wave0-honesty`
- 日期：2026-08-06
- 类型：行为修复 + 契约文档（4 切片，每切片独立提交）
- 前置：`v0.0.1-gap-inventory` / `v0.0.2-decision-freeze` / `v0.0.3-closure-revision`

## 做了什么

### W0-C：sync_turn 状态诚实化（A9/H2）— `679d718d`
- `SyncTurnStatus` 新增 `ProposalCreated` 变体；`Persisted` 语义收窄为
  「权威写入已生效」；trait 文档同步。
- Laputa `sync_turn` 的 proposal 创建成功路径返回 `ProposalCreated`，
  不再谎报 `Persisted`；typed provider 保持委托（即 proposal 路径）。
- consolidation 两处 exhaustive match 将 `ProposalCreated` 视为成功；
  新增回归测试 `sync_turn_proposal_created_is_treated_as_success`。
- 无 serde/wire 波及（纯进程内类型，8 文件引用全在 core/agent/laputa）。

### W0-B：authority_mode 出箱默认统一 Typed（F10）— `b07a0818`
- `Config.memory` 缺失段 `#[serde(default = "legacy_memory_config")]` →
  `#[serde(default)]`（Typed）；enum `#[default]` Legacy → Typed；
  删除死代码 `legacy_memory_config`。
- 测试：`missing_memory_config_defaults_to_typed` 改写 + 新增「memory 段
  存在但字段缺失 → Typed」「显式 legacy 仍生效」。
- `AppState::new` fixture 翻转 Typed；health/server 走 file-first 语义的
  测试显式 opt-in Legacy。
- **行为变更**：无 memory 段配置出箱走 Typed（此前 Legacy）；migration
  不受影响。已写入提交体与本文档。

### W0-A：prompt 假承诺降级（A8）— `deb5754b`
- context.rs 删除「available memory tools」承诺，改为诚实文案：工具面
  未开放、不得声称已记住、任意文件写入不算权威。
- 新增负面回归测试 `prompt_does_not_promise_unavailable_memory_tools`。
- Wave 1 工具落地后恢复工具指引。

### W0-D：写路径分工契约文档（G10/H5/C6）— 本次
- 新建 `docs/architecture/memory-write-paths-contract.md`：Working /
  Long-term / AutoDream / consolidation 分工、优先级（即时 apply 优先、
  proposal 不 imply 权威）、诚实状态语义、tombstone 过滤（G3）、
  无静默降级（F9）。

## 验证

- 每切片 `cargo fmt --check` + `cargo clippy -D warnings`（改动 crate）
  + `cargo test -p <crate>`。
- 全 workspace 测试：core 680 / manager 109 / agent 374 全绿；
  CLI 6 个既有 wiremock 502 失败（`CLI-WIREMOCK-502-PREEXISTING`，
  2026-08-05 已在干净树复现，与本迭代无关）原样复现。

## 影响范围

- core：SyncTurnStatus 变体、config 默认语义。
- laputa：sync_turn 状态上报。
- agent：consolidation 成功臂、prompt 文案。
- manager：测试 fixture 默认语义（生产行为经 config，未变）。
- docs：新增写路径分工契约。
