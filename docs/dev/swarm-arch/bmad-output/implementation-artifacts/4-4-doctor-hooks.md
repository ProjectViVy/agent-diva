---
story_key: 4-4-doctor-hooks
story_id: "4.4"
epic: 4
status: done
generated: "2026-03-30T18:00:00+08:00"
sources:
  - _bmad-output/planning-artifacts/epics.md
  - _bmad-output/planning-artifacts/architecture.md
  - _bmad-output/planning-artifacts/prd.md
  - agent-diva/agent-diva-cli/src/main.rs
  - agent-diva/agent-diva-cli/src/cli_runtime.rs
---

# Story 4.4：诊断（doctor）扩展挂点 — 蜂群 / 大脑皮层能力摘要

Status: done

## Story

作为 **开发者 / 集成方**,  
我希望 **`doctor` 或等价 CLI 能输出一块与蜂群 / 大脑皮层相关的诊断摘要**（含能力注册数量、皮层状态或错误摘要等至少一类）,  
以便 **满足 FR18（可扩展自检挂点）并在实现上体现 NFR-R2（内部调试与用户 transcript 分轨）的精神**。

## Acceptance Criteria

1. **Given** 仓库内已有或规划中的 `doctor` 入口（当前主线为 `agent-diva config doctor`，含 `--json`）  
   **When** 通过 **明确标志、子命令或 JSON 字段扩展** 请求「蜂群 / 皮层」诊断块  
   **Then** 输出中须包含 **以下至少一项**：大脑皮层相关状态摘要、已注册能力数量或列表摘要、与能力/manifest 相关的错误摘要（与 Story **1.6** 占位注册表或后续真相源对齐，以实现为准）

2. **And** **NFR-R2**：内部 trace、逐步调试详情 **不得** 默认写入用户聊天 transcript；本 story 交付的 doctor 输出 **仅限 CLI / 结构化 status** 路径，与聊天持久化 **明确分离**（若在实现中新增字段，须在代码或文档中注明「非用户消息流」）

3. **And** 扩展方式为 **挂点式**：允许后续 Epic 继续追加段落，而 **不** 要求一次覆盖全部未来诊断项（与 PRD **FR18**「可扩展挂点而非一次做全」一致）

## Tasks / Subtasks

- [x] **对齐入口与命名**（AC: #1、#3）  
  - [x] 阅读 `agent-diva-cli`：`config doctor`、`doctor_report`、`StatusDoctorSummary`；决议 **新增 flag**（如 `--swarm` / `--cortex`）或 **嵌套子命令**，并在 `docs/user-guide/commands.md` 或等价处 **写清调用方式**  
  - [x] JSON 模式（`--json`）下：为消费方（含 GUI status）预留 **版本化、白名单友好** 的字段子对象（与 NFR-I2 演进策略一致）

- [x] **接入能力 / 皮层数据源**（AC: #1）  
  - [x] 从 **Story 1.6** 占位 `CapabilityRegistry`（或当前进程内可查询 API）读取 **注册数量、id 摘要、最近校验错误摘要**（若无运行时实例，可 **文档化**「无活跃 gateway 时返回 `n/a` + 原因」，仍须有一条可测路径）  
  - [x] 大脑皮层 **开/关** 或简化模式状态：与 **Epic 1** 契约（gateway / 设置真相源）对齐；若本 story 实现时仅有部分字段可用，**最小实现** 为打印 **可解析占位 + TODO 链到 architecture**

- [x] **人类可读与机器可读双轨**（AC: #2）  
  - [x] 终端块：开发者向（结构化标题 + 简短行），与头脑风暴结论 **「doctor 默认开发者向；可选 `--friendly`」** 可分期：本 story **至少** 保证默认块清晰、可复制  
  - [x] **确认** 无任何 doctor 路径把上述内容 **默认追加** 到用户会话 transcript

- [x] **测试与回归**（AC: #1–#3）  
  - [x] 扩展 `agent-diva-cli` 测试（参考 `tests/config_commands.rs`）：**至少一个** 用例断言新块或 JSON 键存在且稳定  
  - [x] `cargo clippy -p agent-diva-cli -- -D warnings`

### Review Findings

- [x] [Review][Patch] 合并 `run_status` 与 `run_config_doctor` 中重复的 Swarm/cortex 终端输出逻辑 — 已提取 `print_swarm_cortex_doctor_human`（`agent-diva-cli/src/main.rs`）

- [x] [Review][Defer] [agent-diva/agent-diva-cli/src/cli_runtime.rs:829] — deferred, pre-existing：生产路径 `doctor_report` 始终传入 `registry: None`，进程内能力计数/校验错误摘要需在 gateway 或持有 `PlaceholderCapabilityRegistry` 的宿主进程中显式传入 `swarm_cortex_doctor_section`；AC 已允许无活跃实例时使用 `unavailable` + 说明。

## Dev Notes

### FR18 / NFR-R2 要点

| 主题 | 要求 | 来源 |
|------|------|------|
| FR18 | 系统或工具链支持 **基本自检/诊断输出**；随 diva 主线 `doctor` 演进，**可扩展挂点** | `prd.md` |
| NFR-R2 | 内部 trace 与用户 transcript **默认分轨**；调试内容 **不** 默认混入用户可见记录 | `prd.md` |
| Epic 4 目标 | 文档与诊断（FR17–FR18）；本 story 专注 **doctor 蜂群块** | `epics.md` — Epic 4 |

### 与现有 CLI 的关系

- **现状**：`agent-diva config doctor` 已做配置校验与 readiness；`cli_runtime.rs` 中 `doctor_report` 与 `status --json` 共用摘要结构。  
- **本 story**：在 **不破坏** 现有退出码语义的前提下，**增量** 增加「蜂群 / 皮层 / 能力」段落或 JSON 子树。

### 与 Story 1.6 的衔接

- **1.6** 占位注册表职责含：**供 doctor（Story 4.4）拉取数量/摘要**（见 `1-6-capability-v0-validation.md`）。  
- 若 4.4 实现时 1.6 尚未合并：可 **feature-gate** 或 **stub 返回**「registry 未接入」，但须在 PR 中列出 **对接点类型**（trait / 函数），避免重复造轮子。

### 禁止事项

- 不将 doctor 的调试块 **默认** 写入聊天历史或用户可见「单条消息」流（违反 NFR-R2 精神）。  
- 不在本 story 内要求 **完整** 蜂群编排 UI 或 Meta 依赖（遵守 **ADR-A**：swarm 编排 crate 不依赖 `agent-diva-meta`；doctor 扩展落在 **CLI / gateway 边界** 即可）。  
- 避免一次性实现「全量诊断平台」；**挂点 + 一项可验收摘要** 即满足本故事范围。

### Project Structure Notes

- CLI 工作区：`d:\newspace\agent-diva\agent-diva-cli\`（以本机为准）。  
- 规划产物：`d:\newspace\_bmad-output\planning-artifacts\`。

### Testing Requirements

- 至少 **1** 个 CLI 集成或单元级测试覆盖 **新诊断块或 JSON 字段**。  
- 无需 Playwright E2E，除非团队已将 `config doctor` 纳入 GUI 契约测试（现有 GUI 仅消费 **status JSON** 中的 `doctor` 摘要时，扩展字段须 **向后兼容**）。

### References

- `epics.md` — Epic 4, Story 4.4  
- `prd.md` — FR18, NFR-R2  
- `architecture.md` — 文档与诊断（FR17–FR18）、RunTelemetrySnapshot 与 NFR-R2 表述  
- `_bmad-output/implementation-artifacts/1-6-capability-v0-validation.md` — 占位注册表与 doctor 消费约定  
- `agent-diva/agent-diva-cli/src/main.rs`、`cli_runtime.rs` — `doctor_report` 与 `run_config_doctor`

## Dev Agent Record

### Agent Model Used

Cursor Agent（GPT-5.1 系实现）

### Debug Log References

（无）

### Completion Notes List

- 在 `cli_runtime` 增加 `SwarmCortexDoctorV1`（`schema_version: 1`、`channel: cli_diagnostics`）及 `swarm_cortex_doctor_section(config, Option<&PlaceholderCapabilityRegistry>)`；独立 CLI 无进程内 registry 时 `capabilities.source = unavailable` 并附说明；传入 registry 时走 Story 1.6 的 `summary()`。
- `config doctor` 与 `status` 共用 `StatusArgs` 扩展：`--swarm` + 可见别名 `--cortex`；仅显式指定时填充 `swarm_cortex`（JSON 用 `skip_serializing_if` 保持向后兼容）。
- 皮层：`cortex.state = n/a`，`gateway_bind` 来自 `config.gateway`，注释与文档标明 NFR-R2（非用户 transcript）。
- GUI `get_config_status` 继续调用 `collect_status_report(&runtime, false)`，不扩大 status 载荷。
- 文档：`docs/user-guide/commands.md`；测试：`cli_runtime` 单元测试 + `config_doctor_json_swarm_includes_versioned_swarm_cortex_block`。

### File List

- `agent-diva/agent-diva-cli/src/cli_runtime.rs`
- `agent-diva/agent-diva-cli/src/main.rs`
- `agent-diva/agent-diva-cli/tests/config_commands.rs`
- `agent-diva/agent-diva-gui/src-tauri/src/commands.rs`
- `agent-diva/docs/user-guide/commands.md`
- `_bmad-output/implementation-artifacts/sprint-status.yaml`

### Change Log

- 2026-03-30：Story 4.4 — doctor/status `--swarm` 挂点、`swarm_cortex` JSON v1、能力 registry 对接与测试文档（Dev story 完工 → `review`）。

---

_Context: Ultimate BMad Method story context — 对应 FR18、NFR-R2 与 Epic 4 Story 4.4。_
