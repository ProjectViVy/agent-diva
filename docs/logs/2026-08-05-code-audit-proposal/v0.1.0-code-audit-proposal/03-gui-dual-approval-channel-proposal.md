# 提案 03：清理 GUI 废弃 Tauri Command、前端双通道与 Manager 冗余路由

## 1. 残留代码现状与定位

经过对 `agent-diva-gui` (Vue 前端与 Tauri `src-tauri`) 以及 `agent-diva-manager` (HTTP 服务器) 的深入审计，梳理出以下问题：

### 1.1 16 个 Vue 前端 0 调用的废弃 Tauri Command
- **源码位置**: `agent-diva-gui/src-tauri/src/commands.rs` & `lib.rs` (L316-L469)
- **发现清单**:
  1. `greet`: 脚手架遗留。
  2. `get_cron_job` (单数): 前端统一使用 `get_cron_jobs` (复数)。
  3. `get_plan` (单数): 前端统一使用 `get_plans` (复数)。
  4. `delete_plan_todo` / `restore_plan_todo`: 细粒度 Todo 操作已废弃。
  5. `get_plan_reports` / `approve_plan_report`: 旧版 Plan 报告，已被 Laputa Proposal 替代。
  6. `get_active_plan_execution` / `get_execution_todos` / `update_execution_todo`: 旧版执行跟踪。
  7. `get_command_approvals` / `resolve_command_approval` / `start_command_approval_stream`: 旧版单通道审批命令，已被统一治理 `list_approvals` / `decide_approval` / `start_approval_stream` 替代。
  8. `uninstall_gateway`: 已废弃网关卸载命令。
  9. `tail_logs`: 日志流命令，前端采用分段拉取。
  10. `get_runtime_info`: 前端使用 `check_health`。
  11. `get_service_status` / `install_service` / `uninstall_service` / `start_service` / `stop_service`: Windows 服务管理命令（Vue 0 调用）。

### 1.2 前端双通道审批与废弃组件/备份文件
- **相关源码**:
  - `agent-diva-gui/src/App.vue`: L454-520 (`commandApprovals` 响应式状态), L2011-2018 (旧版 SSE 事件监听 `'command-approval-requested'`)。
  - `src/components/ApprovalBanner.vue`: 仅用于旧通道回退渲染的旧组件。
  - `src/components/GatewayControlPanel.vue`: 7 行无引用的桩组件。
  - `src/components/MaskSwitcher.vue` & `src/composables/useMask.ts`: Mock 阶段废弃文件。
  - **3 个源代码备份文件**: `NormalMode.vue.backup`, `NormalMode.vue.before-refactor`, `DivaVrmAvatar.vue.bak`。

### 1.3 `agent-diva-manager` 10 个冗余 HTTP 路由
- **文件位置**: `agent-diva-manager/src/handlers/command_approvals.rs` & `planning.rs`
- **残留路由**:
  - `/api/command-approvals` 3 个路由 (已被统一 `/api/approvals` 替代)。
  - `/api/plan-reports` 与 `/api/plan-executions` 7 个路由 (前端调用次数全为 0)。

---

## 2. 拟定的重构与瘦身方案

### 2.1 变更内容 [DELETE & MERGE]
1. 从 `commands.rs` 和 `lib.rs` 的 `generate_handler!` 中移除 16 个未用 Tauri Command。
2. 彻底清理 `App.vue` 中的旧 `command-approval` SSE 监听与状态，删除 `ApprovalBanner.vue`、`GatewayControlPanel.vue`、`MaskSwitcher.vue` 以及 3 个物理备份文件。
3. 从 `agent-diva-manager` 的 `server.rs` 中清理废弃的 10 个 HTTP 路由及关联 Handler。

---

## 3. 收益与风险评估
- **预期收益**：消灭前端双通道 SSE 竞争与重复渲染风险；大幅瘦身 Tauri IPC 接口面（减少 16 个 Command）；清理后端 10 个废弃 API。
- **风险分析**：中等。需运行 `cd agent-diva-gui && npm run test:unit && npm run build` 进行前端冒烟断言。
