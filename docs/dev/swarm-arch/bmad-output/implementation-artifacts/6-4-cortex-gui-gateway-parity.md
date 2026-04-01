---
story_key: 6-4-cortex-gui-gateway-parity
story_id: "6.4"
epic: 6
status: done
generated: "2026-03-31T20:30:00+08:00"
sources:
  - _bmad-output/planning-artifacts/epics.md
  - _bmad-output/implementation-artifacts/deferred-work.md
---

# Story 6.4：皮层状态 GUI 与 Gateway 权威源一致性

## Story

As a **用户**,  
I want **聊天顶栏皮层开关所反映的状态与 gateway 内 `CortexRuntime` 不产生长期错觉性漂移**,  
So that **FR14 在「可见 UI」维度也成立（关闭 Story 2.3 评审 deferred）**。

## Acceptance Criteria

**Given** 用户仅通过 GUI 或仅通过 API 切换皮层  
**When** 在另一侧查询状态  
**Then** 在文档定义的同步窗口内 **一致**；若不同步须有 **可感知错误/recover**（对齐 Story 2.2 精神）  
**And** 验收用例写入 UX 或实现说明

### 实现说明（同步窗口与恢复）

| 场景 | 行为 |
|------|------|
| 本机 `/api/health` 成功 | `get_cortex_state` / `toggle` / `set_cortex_enabled` 经 HTTP 与 gateway 内 `ProcessEventPipeline` 共用之 `CortexRuntime` 对齐；拒绝时 `invoke` 返回 `cortex_sync_rejected`，前端沿用 Story 2.2 文案。 |
| 网关不可达 | 不阻塞桌面开发：跳过远端握手，仅更新本地 `Arc`；与既有调试/外置后端路径一致。 |
| 仅 API 改皮层 | 用户切回聊天页（`document.visibilitychange` → `visible`）时 `ChatView` 再次 `getCortexState`，条带与顶栏与网关拉齐。 |
| API 契约 | `GET/POST http://localhost:3000/api/swarm/cortex`（body: `{"enabled": bool}`），JSON 与 `CortexState` camelCase 一致。 |

## Tasks / Subtasks

- [x] Gateway：`RuntimeControlCommand` + Manager HTTP `/api/swarm/cortex`
- [x] GUI：`cortex_sync` 在健康检查通过时 POST；`get_cortex_state` 拉取并回写本地 `Arc`
- [x] `ChatView`：可见性恢复时重新拉取皮层状态（API 侧切换后的 recover）
- [x] 单元/路由测试与 `cargo check`

### Review Findings

- [x] [Review][Patch] `ChatView` 挂载时 `getCortexState` 失败将 `cortexLayerOn` 固定为 `true`，在网关实际为关或本地 `Arc` 已为关时会造成与权威源不一致的「假开」展示；与 AC「不产生长期错觉性漂移」冲突。[`agent-diva/agent-diva-gui/src/components/ChatView.vue` 约 189–195 行] — **已修（2026-03-31）：** `cortexLayerOn` 初始 `false`，挂载拉取失败时保持未知态不冒充「开」；成功拉取或 `cortex_toggled` 后再与权威一致。

- [x] [Review][Defer] `server.rs` 集成测仅覆盖 `GET /api/swarm/cortex` 与 mock `SetCortex` 成功路径，未断言 `POST` body（`{"enabled":bool}`）与响应 JSON 与 `CortexState` 的往返；与任务勾选「单元/路由测试」相比仍有余量。[`agent-diva/agent-diva-manager/src/server.rs` 测试模块] — deferred, pre-existing gap

## Dev Notes

- 关闭 `deferred-work.md` 中 Story **2-3** 关于皮层 UI 与 gateway 门控不同步的挂起项（以网关内真相源为准）。
- 注入失败：`AGENT_DIVA_TEST_CORTEX_SYNC_FAIL=1`（仅 test / debug_assertions），与 Story 2.2 一致。

## Dev Agent Record

### Debug Log

- 根因：桌面 `Arc<CortexRuntime>` 与 gateway 进程内实例原为先写本地、无 HTTP 对齐，导致 `ProcessEventPipeline` 与 GUI 漂移。

### Completion Notes

- ✅ 已实现 companion API 与 GUI 双向同步策略；`cargo check -p agent-diva-gui -p agent-diva-manager` 通过（独立 `CARGO_TARGET_DIR`）。

## File List

- `agent-diva/agent-diva-agent/src/runtime_control.rs`
- `agent-diva/agent-diva-agent/src/agent_loop/loop_runtime_control.rs`
- `agent-diva/agent-diva-manager/src/state.rs`
- `agent-diva/agent-diva-manager/src/manager.rs`
- `agent-diva/agent-diva-manager/src/manager/runtime_control.rs`
- `agent-diva/agent-diva-manager/src/handlers.rs`
- `agent-diva/agent-diva-manager/src/server.rs`
- `agent-diva/agent-diva-gui/src-tauri/src/cortex_sync.rs`
- `agent-diva/agent-diva-gui/src-tauri/src/commands.rs`
- `agent-diva/agent-diva-gui/src/components/ChatView.vue`

## Change Log

- 2026-03-31：Story 6.4 — gateway `/api/swarm/cortex`、GUI HTTP 同步、`ChatView` 可见性 recover。
- 2026-03-31：Code review — 1 项待修（ChatView 挂载失败时的默认皮层态）；1 项测试缺口记入 deferred-work。
- 2026-03-31：修复 review patch — `ChatView.vue` 皮层条初始与挂载失败时的默认态。
