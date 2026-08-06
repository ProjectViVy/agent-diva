# Summary — GA-MEM-PARITY Wave 2（分层与工作记忆）

- 版本：`v0.0.6-wave2-layers-working-memory`
- 日期：2026-08-06
- 类型：分层记忆 + 工作记忆（4 切片，每切片独立提交）
- 前置：`v0.0.5-wave1-memory-tools`

## 做了什么

Wave 2 实现 L1 预算注入 + pointer（B2/B10）、Working checkpoint 工具与注入
（C1–C3）、L0 policy 文本（B1/B8/B9 部分），并承接 G1（distill evidence
扩展）。决策基线：inventory §6 Wave 2 + §10.1 #3（工作记忆=Session store，
易失、非权威）+ §10.3 G1。

### S1 core：working trait 面 + L1 预算配置 — `7b30dc72`
- 新建 `agent-diva-core/src/memory/working.rs`：`WorkingMemoryRequest /
  WorkingMemoryResponse`、`CheckpointWriteRequest{key_info, related_sops,
  content}`、`L0_MEMORY_POLICY`（三条：Action-Verified / 禁止易变状态 /
  最小指针）、`render_checkpoint_block`。
- `MemoryProvider` 加 2 个 async 默认方法：`working_memory_block`（默认空块，
  安静无注入）、`checkpoint_write`（默认 unsupported）。
- `MemoryConfig.l1_index_lines`（serde default 30）；builtin tools gate
  `working_memory`（默认 true，minimal/none/for_subagent false）。
- 测试：默认语义、serde roundtrip、配置缺省/显式。

### S2 laputa：Typed L1 索引 + session checkpoint — `2e70d92c`
- L1 索引：`render_l1_index_line`（首行 ≤80 字符预览 + …）/`render_l1_index_block`
  （指针指引 + 预算截断；空权威/0 预算不注入）；`TypedLaputaMemoryProvider::open`
  默认预算渲染，`open_with_l1_budget` 显式配置；MemoryManager（Legacy）同步。
- checkpoint_write：固定 per-session id 的 session-scoped AppliedAuthority
  记录（`MemoryRecordKind::WorkingMemory` 新增变体，upsert 覆盖）+ audit。
- working_memory_block：按 session 读 checkpoint（supersedes-aware——
  被 tombstone 指向则不再可见）。
- on_session_end：收到真实 session_id 时以独立 supersedes tombstone 记录
  清理（content 空 + supersedes=[target]，过 `validate_at`）。
- distill evidence（G1 承接）：新建写 `skills/<name>/EVIDENCE.md`；覆盖走
  proposal 时 excerpt 带 evidence 摘要；audit 标记 `has_evidence`。
- 测试（tempdir + 真实 store）：L1 有界不注入全文、0 预算、checkpoint 写读
  一致且不进启动渲染、覆盖替换、session end 清理、evidence 文件。

### S3 agent：注入接线 + session 生命周期 — `b1c101f9`
- context.rs：system prompt 注入 `## Memory Management Policy`
  （`L0_MEMORY_POLICY`，位于 memory 块之前）。
- turn/context.rs：`prepare_runtime_context` 每轮注入 working memory 块
  （`inject_working_memory` 辅助，system 后第一块，prefetch 索引移位）。
- agent_loop.rs：loop 退出时按 `SessionManager::list_sessions` 逐会话调用
  `on_session_end`（清理 checkpoint），保留 agent-loop-shutdown rhythm。
- 测试：policy 段存在、working memory 注入位置、空块跳过。

### S4 tools：checkpoint 工具 + distill evidence — `c8111c22`
- `agent-diva-tools/src/update_working_checkpoint.rs`：参数
  key_info/related_sops/content（workspace/session 由装配注入，不向模型
  暴露）；结果三态 JSON；gate `working_memory && memory`。
- ToolAssembly：`working_memory_session` 字段 + `with_working_memory_session`
  builder + 注册；ToolTurnOptions 加 `session_key` 穿透
  `rebuild_tools_for_turn`（admission 绑定真实 session key，其余重建点 None）。
- `memory_distill`：parameters 暴露可选 `evidence`。
- 测试：工具单测（无 provider / 无 session / 无效参数）、装配注册 + session
  绑定、gate deny。

## 验证

- 每切片 `cargo fmt --check` + `cargo clippy -D warnings`（改动 crate）
  + `cargo test -p <crate>`。
- 全 workspace 相关 crate：core 692 / laputa 27+ / agent 383 / tools 109
  全绿；CLI 6 个既有 wiremock 502 失败（`CLI-WIREMOCK-502-PREEXISTING`）
  与本迭代无关。

## 影响范围

- core：working 契约 + L1 渲染 helper + MemoryConfig/gate 配置 +
  MemoryRecordKind::WorkingMemory。
- laputa：Typed L1 渲染 + session checkpoint 三路（写/读/清）+ distill
  evidence。
- agent：policy 注入、turn working 块注入、session-end 会话枚举清理、
  ToolAssembly/loop 接线。
- tools：新 `update_working_checkpoint` 工具 + distill evidence 参数。
- docs：TODOLIST 新建 WAVE2 条目并勾选。
