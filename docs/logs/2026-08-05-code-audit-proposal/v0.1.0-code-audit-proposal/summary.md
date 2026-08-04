# agent-diva 全盘代码审查预案与遗留臃肿/死代码清理提案汇总（全量审查版）

## 1. 审查背景与目的

基于 4 路并行深度审查子代理（Agent Core & Memory, Channels & Providers, GUI & Manager, Tools Sandbox & Storage）对 `agent-diva` 全仓库 17 个 Rust Crate 及 Vue/Tauri 桌面前端的代码检索与链路推演，本报告汇总了当前代码库中所有**因兼容旧实现导致的膨胀代码、未集成的死代码文件、废弃的 Tauri Command/HTTP 路由、未调用的结构体字段与死通道逻辑**。

本预案严格遵循 **“不修改任何生产代码”** 的原则，仅生成结构化、精准到文件与行号的决策提案集合，为后续版本的瘦身与代码质量提升提供直接依据。

---

## 2. 审查发现核心模块与提案索引

| 编号 | 提案文档 | 审计范围 | 核心问题 / 残留现象 | 拟定清理收益 |
| :--- | :--- | :--- | :--- | :--- |
| **01** | [`01-legacy-memory-boundary-proposal.md`](./01-legacy-memory-boundary-proposal.md) | `agent-diva-agent`<br>`agent-diva-core` | ① `governance_adapter.rs` 135 行未调用的旧适配器<br>② `loop_runtime_control.rs` 中硬编码报错的死控制命令<br>③ `memory_boundary.rs` / `recall.rs` 中未清理的 Shadow 影子比对逻辑<br>④ `nag.rs` / `todo_planner.rs` / `security/` 中未调用的孤立结构体与 Stub | 精简约 800+ 行死代码，巩固 Laputa 唯一权威，简化 Agent 循环与控制面。 |
| **02** | [`02-stub-planning-tools-proposal.md`](./02-stub-planning-tools-proposal.md) | `agent-diva-tools`<br>`agent-diva-agent` | ① `agent-diva-tools/src/message.rs` 未注册的 `MessageTool`<br>② `planning/mod.rs` 中 4 个废弃的旧版 Plan Tools (`TodoShowTool` 等)<br>③ `agent-diva-agent/src/planning/tools.rs` 中的存根 `PlanApproveTool`<br>④ `wtf.rs` ASCII Logo 逻辑错置于 Tools Crate | 净化 Built-in Tools 集合，节省 Prompt Token，解耦 CLI 打印逻辑。 |
| **03** | [`03-gui-dual-approval-channel-proposal.md`](./03-gui-dual-approval-channel-proposal.md) | `agent-diva-gui`<br>`agent-diva-manager` | ① `src-tauri` 中 16 个前端零调用的废弃 Tauri Command（如 `greet`, `get_plan_reports`, `tail_logs`, Windows 服务管理等）<br>② `App.vue` / `ChatView.vue` 前端双通道 SSE 监听与旧 `ApprovalBanner`<br>③ `src/` 中 3 个残留的临时备份文件 (`NormalMode.vue.backup` 等)<br>④ `agent-diva-manager` 中 10 个废弃 HTTP 路由 (`/api/command-approvals`, `/api/plan-reports` 等) | 清理前端/后端双通道冗余，消灭 SSE 事件冲突隐患，移除 16 个未用 Command 与 10 个废弃 Endpoint。 |
| **04** | [`04-dead-code-suppression-cleanup-proposal.md`](./04-dead-code-suppression-cleanup-proposal.md) | `agent-diva-channels`<br>`agent-diva-providers` | ① `email.rs` 中 `parse_email` 辅助函数被内联重复手写，导致原函数变成死代码<br>② `telegram.rs` 未调用的 `start_typing` 任务与死函数 `handle_text_message`<br>③ `dingtalk`, `qq`, `feishu`, `discord` 等 10+ 通道结构体中带 `#[allow(dead_code)]` 的未用 Payload 字段<br>④ `whatsapp.rs` 过时的 Python 迁移注释与 Map 冗余转换 | 修复/移除未调用的打字指示器，消除重复解析逻辑，精简通道数据转换。 |
| **05** | [`05-autodream-migration-legacy-cleanup-proposal.md`](./05-autodream-migration-legacy-cleanup-proposal.md) | `agent-diva-migration`<br>`agent-diva-sandbox`<br>`agent-diva-files` | ① **`agent-diva-migration` 源码树中积压的 1389 行未挂载到 `main.rs` 的旧版 Python 迁移模块** (`config_migration.rs`, `memory_migration.rs`, `session_migration.rs`)<br>② `agent-diva-sandbox` 中未调用的 `is_wsl()` 与死常量 `WRITE_RESTRICTED`<br>③ `agent-diva-files` 中带 `#[allow(dead_code)]` 的 `run_extract_metadata` Hook | 物理删除 1389 行未挂载死文件，清理沙箱与文件 Hook 的未用能力。 |

---

## 3. 重构推进原则

1. **零生产中断**：全过程按 Wave 1 至 Wave 5 分切片推进，每个切片必须独立通过 `just fmt-check && just check && just laputa-clean-break-check && just test`。
2. **彻底删除不隐盖**：拒绝使用 `#[allow(dead_code)]` 掩盖问题，对确定无用的代码直接物理删除。
3. **决策先行**：本提案集合仅供用户决策参考，需用户显式批准后方可开启重构。
