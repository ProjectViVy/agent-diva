# agent-diva 代码审查提案全量验收标准 (Acceptance Criteria)

## 1. 提案完备性验收

本审查预案交付物为提案文档集合（位于 `docs/logs/2026-08-05-code-audit-proposal/v0.1.0-code-audit-proposal/`），需满足以下验收标准：

- [x] **零代码破坏**：生产 Rust/Vue 代码未被修改，所有审计发现均转化为结构化提案文档。
- [x] **全仓库覆盖**：涵盖 17 个 Rust Crate 及 Tauri/Vue 桌面前端。
- [x] **精准度与可落地性**：每项审计发现均标明精确的文件路径、行号范围、成因与拟定清理方案。

---

## 2. 详细提案检查清单

| 提案 ID | 审查范围 | 重点对象 | 提案文档 | 状态 |
| :--- | :--- | :--- | :--- | :--- |
| **01** | Agent Core & Memory | `governance_adapter.rs`, `CutoverMemoryProvider`, `NagTracker` 等 8 项 | [`01-legacy-memory-boundary-proposal.md`](./01-legacy-memory-boundary-proposal.md) | 已就绪 |
| **02** | Tools & Agent Assembly | `MessageTool`, 4 个旧 Plan Tools, `PlanApproveTool`, `wtf.rs` | [`02-stub-planning-tools-proposal.md`](./02-stub-planning-tools-proposal.md) | 已就绪 |
| **03** | GUI & Manager | 16 个未用 Tauri Command, 双 SSE 通道, 废弃组件/备份, 10 个 API 路由 | [`03-gui-dual-approval-channel-proposal.md`](./03-gui-dual-approval-channel-proposal.md) | 已就绪 |
| **04** | Channels & Providers | `email.rs` 重复解析, `telegram.rs` 死任务, 各通道无用 Payload 字段 | [`04-dead-code-suppression-cleanup-proposal.md`](./04-dead-code-suppression-cleanup-proposal.md) | 已就绪 |
| **05** | Migration, Sandbox & Files | Migration 1389 行未挂载代码, 沙箱 `is_wsl()` 及 `WRITE_RESTRICTED`, 文件 Hook | [`05-autodream-migration-legacy-cleanup-proposal.md`](./05-autodream-migration-legacy-cleanup-proposal.md) | 已就绪 |
