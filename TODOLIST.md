# TODOLIST

项目级待办与已完成缺陷/差距/未竟工作追踪。

## Open

_(No stale open items remain.)_

## Backlog

- [ ] **Production cron clock abstraction** — `runtime.rs:274` uses a hardcoded clock path; needs abstraction for testability and portability.
  - Related files: `agent-diva-manager/src/runtime.rs`
- [ ] **Audit GUI i18n** — 3 Vue components use hardcoded English strings without `useI18n`; should be wired to the i18n system.
  - Related files: `agent-diva-gui/src/components/` (affected components TBD)
- [ ] **UX-DR-3/4/7** — Deferred UX gaps identified during sprint review (design review items 3, 4, 7).
  - Context: Sprint closure review; specifics to be elaborated when picked up.

## Done

- [x] **GUI 预算状态改动后的全局验证仍受既有阻塞影响** — clippy/tests now pass after Wave 0 CI fixes; fmt-check and GUI embedded gateway health check stable.
  - Related files: `agent-diva-core/src/lib.rs`, `agent-diva-gui/src-tauri/src/embedded_server.rs`
- [x] **Workspace 全局验证门禁阻塞** — Resolved by Wave 0 CI cleanup; `just check` and `just test` pass on default workspace config.
  - Related files: `agent-diva-gui/src-tauri/src/notebook.rs`, `agent-diva-agent/src/agent_loop.rs`, `agent-diva-agent/src/subagent.rs`
- [x] **Claude 规则镜像同步** — 重写 `CLAUDE.md`，使其与当前 `AGENTS.md` 的工作区结构、并行锁机制、验证规则、提交协议和中文通信要求保持一致
  - 修复范围：`CLAUDE.md`
  - 期望行为：Claude 会话读取仓库说明时，不再使用过时 crate 列表、错误路径或缺失的并行协作规则
- [x] **Codex 并行互斥锁机制** — 新增根级 `LOCK.md` 作为并行会话互斥文件，并把锁获取/释放/过期接管流程写入 `AGENTS.md`
  - 修复范围：`LOCK.md`, `AGENTS.md`
  - 期望行为：并行 Codex/Cursor/人工会话在写入前先登记锁范围、心跳和工作区边界，避免覆盖同一批文件
- [x] **Plan mode 运行时生效修复** — GUI `execMode = 'plan'` 已随 `send_message` 传到 Manager；AgentLoop 接入 workspace-local planning store、Plan mode 工具限制、active plan context 注入和 planning/todo 工具注册；GUI PlanningView 依赖的 Tauri commands/nav 已注册。
  - 修复范围：`agent-diva-gui/src/components/ChatView.vue`, `agent-diva-gui/src/App.vue`, `agent-diva-agent/src/agent_loop.rs`, `agent-diva-agent/src/tool_assembly.rs`, `agent-diva-agent/src/tool_config/mod.rs`, `agent-diva-manager/src/handlers.rs`, `agent-diva-manager/src/manager.rs`, `agent-diva-gui/src-tauri/src/commands.rs`。
  - 验证记录：`docs/logs/2026-06-plan-mode-runtime/v0.0.1-plan-mode-runtime-wiring/verification.md`。
- [x] **月度报告产品化** — 月报已接入 `notebook-monthly` 触发链路、AutoDream 共享生成器、失败 error marker attempt 计数、manager 启动时自动安装的月报 cron 调度，以及 GUI 统一的 manager 触发入口（`monthly.rs`, `service.rs`, `runtime.rs`, `bootstrap.rs`, `commands.rs`）
- [x] **日报聚合与会话回退** — AutoDream 日/周触发器生成真实报告文件；月度从日报合成，缺失日期回退到会话摘要
- [x] **并行状态工作区隔离** — 规则已写入 `AGENTS.md`（`parallel-state-worktree-isolation`），通过 git worktree/branch 执行
- [x] **GUI vitest 修复** — `SubAgentPanel.test.ts`（vue-i18n）、`DivaPetView.test.ts`（ChevronDown mock），全套件通过
- [x] **workspace rustfmt 漂移修复** — 重格式化 `memory_boundary.rs`，`just fmt-check` 通过
- [x] **Mentle release-gate 测试升级** — 静态过滤测试替换为启用运行时治理边界覆盖（Story 6.5）
- [x] **2026-06 待办清理** — 关闭过期 Open 项；确认 `epic6-release-gate` 含权威边界测试；治理守卫不再豁免 `notebook.rs`
- [x] **Story 5.1 Laputa 会话压缩持久化** — Laputa 为默认 MemoryProvider 时，会话压缩须持久化或显式失败，不可静默推进 `last_consolidated`
- [x] **Story 5.1 移除 MemoryManager 回退** — Laputa 初始化失败时不可恢复旧版 `MEMORY.md`/`HISTORY.md` 权威写入
- [x] **Story 5.1 子代理权威走 MemoryProvider 注入边界** — 禁止直接实例化 `LaputaMemoryProvider`（`subagent.rs`）
- [x] **Story 5.2 Mentle 治理排除** — 治理 prompt 不再注入 Mentle 召回/路由引导（`context.rs`）
- [x] **Story 5.2 Mentle 启用时回归覆盖** — 治理组装依赖 Mentle 运行时状态时测试须失败（`mentle_governance_boundaries.rs`）
- [x] **Story 5.3 压缩证据在提案边界拒绝** — Laputa 提案创建/编辑拒绝纯压缩证据（`proposals.rs`）
- [x] **Epic 4 Notebook 前后端联动修复** — 畸形文件跳过、`source_run_id` 修正、会话证据附加、预览模态清理、Evolution 深链导航（`notebook.rs`, `NotebookView.vue`, `EvolutionView.vue`）
- [x] **Story 4.4 `session_hits` 编译修复** — Notebook 测试调用点适配新签名
- [x] **Story 6.1 Laputa 迁移写锁** — 迁移提交前获取 proposals 写锁（`migration.rs`）
- [x] **Story 6.1 `state.json` 字段保留** — 迁移合并 schema 版本到现有 state，非覆盖
- [x] **Story 6.1 根级遗留文件发现** — 迁移扫描根目录 `MEMORY.md`/`HISTORY.md`，不仅限 `memory/` 子目录
- [x] **Story 6.1 `BOOTSTRAP.md` 不再作为运行时权威** — 迁移后 prompt 组装停止注入 BOOTSTRAP 内容（`context.rs`）
- [x] **Story 6.1 schema 版本从 state 读取** — 读 API 返回迁移后版本，非硬编码 `1.0.0`
- [x] **Story 6.4 治理直写守卫移除白名单** — 文件白名单替换为共享权威边界守卫（`direct_write_guard.rs`）
- [x] **Story 6.4 治理证明循环调用权威守卫** — `governance_proof_loop` 调用共享边界守卫（`governance_proof_loop.rs`）
- [x] **Story 5.2 验证阻塞修复** — 压缩次要证据标记路径 + Laputa 缺失 migration/apply API 补全
- [x] **agent-diva-laputa clippy 清理** — `too_many_arguments`、`manual_inspect` 等已修
- [x] **agent-diva-sandbox 编译修复** — 添加 `fs2` 锁，修正 `bool`/`&bool` 匹配（`exec_policy.rs`, `macos.rs`）
- [x] **agent-diva-sandbox 全目标验证** — 移除未用测试导入，修正 shell 注入断言（`manager.rs`, `macos.rs`）
- [x] **agent-diva-gui Rust 编译修复** — 改用 `state.client`，启用 Tauri `macos-private-api`（`commands.rs`, `tauri.conf.json`）
- [x] **agent-diva-manager skill 服务测试** — 测试注入临时 builtin-skill fixture（`skill_service.rs`）
- [x] **agent-diva-agent clippy 清理** — 移除冗余导入、无谓借用、字段重赋值（agent_loop, context, compaction tests）
- [x] **agent-diva-agent mask/runtime 警告清理** — 警告产生导入已随 clippy 清理移除
- [x] **Story 2.4 GUI 验证阻塞** — 清理 `vue-tsc` 错误，修复 i18n 插件、ChevronDown mock、mood 断言
- [x] **GUI 构建阻塞** — 移除 settings 组件残留未用导入，修正 `TodoCard.vue` API 路径
- [x] **Story 2.4/3.1 阻塞拆分** — Laputa lint、sandbox 编译已确认解决，剩余项拆为独立 TODO
- [x] **agent-diva-manager AutoDream 错误匹配** — 映射 `InputCollection`/`ProposalPersistence` 变体到现有错误类（`handlers/autodream.rs`）
- [x] **Story 2.2 Inbox 行为补全** — `ProposalInbox.vue` 含过滤器、键盘导航、批量操作、加载/空态（`ProposalInbox.vue`）
- [x] **GUI 剪贴板图片粘贴** — commit `53bc086`：`handlePaste` 捕获 → `uploadFile` 上传 → 图片预览 → 多图支持
- [x] **2026-06-11 sandbox clippy 清理** — 12 个 lint 错误修复（`windows.rs`, `orchestrator.rs`），`cargo test` 99 通过
- [x] **2026-06-11 sandbox 审计整改** — 10 项关闭（2C+4H+3M+1P3）：shell 注入防护、沙箱不可用 fail-closed、guardian 默认收紧、解释器别名禁令、受保护路径扩展、非零退出码、审批缓存统一、orchestrator 解耦、feature gates、安全策略桥接
