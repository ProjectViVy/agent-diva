# agent-diva 代码审查预案与遗留臃肿/死代码清理提案汇总

## 1. 提案背景与目的

随着 `agent-diva` 项目的持续快速演进（包括 Laputa 数据库 Typed Memory clean-break、HITL 审批重构 GMH-30..33、AutoDream E0..E7 闭环等），代码库中积累了一部分为了兼容旧版本实现而保留的包装层、存根工具（Stub tools）、双轨 SSE/通道，以及在多个 Crate 中为了规避编译器警告而添加的 `#[allow(dead_code)]` 冗余代码。

本审查预案严格遵循 **“不修改任何生产代码”** 的原则，全盘梳理 `agent-diva` 仓库中的遗留与死代码，并形成结构化、可落地的清理提案文档集合，为后续版本的瘦身与代码质量提升提供明确的重构蓝图。

---

## 2. 审查发现核心模块与提案索引

本次审计覆盖 `agent-diva` 全部 17 个 Rust Crate 及 Tauri GUI 模块，共识别出 **5 大类主要瘦身与清理目标**：

| 编号 | 提案文档 | 审计对象 | 核心痛点 / 残留现象 | 预计瘦身收益 |
| :--- | :--- | :--- | :--- | :--- |
| **01** | [`01-legacy-memory-boundary-proposal.md`](./01-legacy-memory-boundary-proposal.md) | `agent-diva-agent/src/memory_boundary.rs`<br>`agent-diva-core/src/config/schema.rs` | 在 Laputa Typed Memory 已成为唯一权威存储的情况下，仍然保留 `MemoryAuthorityMode::Legacy`/`Shadow` 枚举分支及 `legacy: Arc<MemoryManager>` 成员与兼容Prefetch逻辑。 | 消除 300+ 行双轨记忆机制代码，完全剥离已废弃的旧版文件 MemoryManager 依赖。 |
| **02** | [`02-stub-planning-tools-proposal.md`](./02-stub-planning-tools-proposal.md) | `agent-diva-agent/src/planning/tools.rs`<br>`agent-diva-agent/src/tool_assembly.rs` | Agent 端仍然注册已不可用的存根工具（如 `PlanApproveTool`），仅用于返回 "Unavailable" 错误提示。 | 彻底清理 Agent 工具箱中的失效存根，简化工具装配（Assembly）与 Prompt 上下文开销。 |
| **03** | [`03-gui-dual-approval-channel-proposal.md`](./03-gui-dual-approval-channel-proposal.md) | `agent-diva-gui/src/App.vue`<br>`agent-diva-gui/src-tauri/src/` | 前端 App.vue 同时监听旧版 `command-approval-requested` SSE 与统一 `approval-event` SSE，且维持两套状态对象 `commandApprovals` 与 `unifiedApprovals`。 | 消灭审批事件丢失隐患，统一前端审批数据流，减少前端 150+ 行双通道维护逻辑。 |
| **04** | [`04-dead-code-suppression-cleanup-proposal.md`](./04-dead-code-suppression-cleanup-proposal.md) | `agent-diva-channels`, `agent-diva-gui`, `agent-diva-core`, `agent-diva-sandbox` 等 10+ Crate | 存在 40 余处显式标记 `#[allow(dead_code)]` 的未受保护死函数/死字段（如 DingTalk, Telegram, Nextcloud 通道中未使用的结构体）。 | 清理无用字段与结构体定义，恢复 Rust compiler 对死代码的精准检测告警。 |
| **05** | [`05-autodream-migration-legacy-cleanup-proposal.md`](./05-autodream-migration-legacy-cleanup-proposal.md) | `agent-diva-autodream/src/service.rs`<br>`agent-diva-migration/` | `AutoDreamFailureCode::LegacyIncomplete` 历史格式兼容处理与 `agent-diva-migration` 中早期 v0.1/v0.2 逻辑残留。 | 统一历史数据失败边界，收口 Migration 工具包职责范围。 |

---

## 3. 清理提案实施原则

1. **分阶段无缝推进**：按提案 01 -> 02 -> 03 -> 04 -> 05 顺序分 Slice 实施，每个 Slice 独立提交 Commit，确保不影响主线功能。
2. **测试与 Clean-Break 验证门禁**：每个清理 Slice 必须通过 `just laputa-clean-break-check` 与 `just ci`，确保无隐式行为退化。
3. **彻底删除而非隐式标记**：拒绝使用 `#[allow(dead_code)]` 或注释来掩盖死代码，确认无调用的逻辑直接移除。
