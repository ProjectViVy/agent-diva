---
story_key: 6-3-doctor-capability-registry-wiring
story_id: "6.3"
epic: 6
status: done
generated: "2026-03-31T20:30:00+08:00"
sources:
  - _bmad-output/planning-artifacts/epics.md
  - _bmad-output/implementation-artifacts/deferred-work.md
---

# Story 6.3：Doctor 与真实 Capability 注册表接线

详见 `epics.md` **Epic 6 / Story 6.3** 全文 AC。  
**关闭：** `deferred-work.md` 中 Story **4-4** doctor 与 registry 的评审挂起项。

## Story

As a **开发者**,  
I want **`doctor`（或等价）在 gateway/宿主上下文中可接收真实 `CapabilityRegistry`（或等价）并打印非占位计数/错误摘要**,  
So that **FR18 与 Shannon-类「可诊断」P0 成立（关闭 Story 4.4 评审 deferred）**。

## Acceptance Criteria

- **Given** 运行中的 manager 或 CLI 子命令可访问与 GUI 提交 **同源** 的 registry  
- **When** 执行带蜂群/能力标志的 doctor  
- **Then** 输出 **注册能力数或校验错误摘要** 中至少一项为 **真实值**  
- **And** NFR-R2：内部 trace **不** 默认混入用户 transcript

## Tasks / Subtasks

- [x] 将 swarm doctor 块构建迁入 `agent-diva-agent`（`swarm_doctor` + 工作区 manifest 解析路径）
- [x] Gateway：进程内 `Arc<PlaceholderCapabilityRegistry>`，启动时加载 `{workspace}/.agent-diva/capability-manifest.json`
- [x] HTTP：`GET /api/diagnostics/swarm-doctor`、`POST /api/capabilities/manifest`（校验、替换 registry、持久化 JSON）
- [x] CLI：`config doctor` / `status` 在无进程内 registry 时读取同一工作区 manifest 文件
- [x] GUI：校验通过后先持久化工作区 JSON 再替换进程内 registry；写盘/配置加载失败时 `ok: false`（`message`）
- [x] 单测：`agent-diva-agent` swarm_doctor；`agent-diva-cli` workspace manifest；`agent-diva-manager` AppState 路由测试桩

## Dev Notes

- 与 GUI **分离进程** 的 gateway 无法共享内存；同源定义为 **同一 workspace 下** `.agent-diva/capability-manifest.json` + gateway 进程内 registry。
- `swarm_cortex` 仍标记 `channel: cli_diagnostics`，满足 NFR-R2。

## Dev Agent Record

### Implementation Plan

- 抽出 `swarm_cortex_doctor_section` / `swarm_cortex_doctor_section_for_diagnostics` 至 `agent-diva-swarm` 的兄弟 crate `agent-diva-agent::swarm_doctor`，避免 `agent-diva-manager` 依赖 `agent-diva-cli`。
- 新增 `capability::persist` 统一路径常量与读写。

### Debug Log

- （无）

### Completion Notes

- ✅ Story 6.3：gateway doctor 与 GUI/CLI 通过工作区 JSON + HTTP 对齐 capability 诊断；关闭 deferred 4-4 doctor/registry 项。

## File List

- `agent-diva/agent-diva-agent/src/swarm_doctor.rs`（新）
- `agent-diva/agent-diva-agent/src/capability/persist.rs`（新）
- `agent-diva/agent-diva-agent/src/capability/mod.rs`
- `agent-diva/agent-diva-agent/src/lib.rs`
- `agent-diva/agent-diva-cli/src/cli_runtime.rs`
- `agent-diva/agent-diva-cli/tests/config_commands.rs`
- `agent-diva/agent-diva-manager/src/runtime.rs`
- `agent-diva/agent-diva-manager/src/runtime/bootstrap.rs`
- `agent-diva/agent-diva-manager/src/runtime/task_runtime.rs`
- `agent-diva/agent-diva-manager/src/state.rs`
- `agent-diva/agent-diva-manager/src/manager.rs`
- `agent-diva/agent-diva-manager/src/handlers.rs`
- `agent-diva/agent-diva-manager/src/server.rs`
- `agent-diva/agent-diva-gui/src-tauri/src/capability_commands.rs`
- `_bmad-output/implementation-artifacts/deferred-work.md`

## Change Log

- 2026-03-31：实现 Story 6.3 doctor ↔ capability registry 接线（agent + manager + CLI + GUI + deferred 收口）。
- 2026-03-31：code review 批量修复 — bootstrap 失败记入共享状态，HTTP doctor 用 `swarm_cortex_doctor_for_gateway` 在空 registry 时返回 `manifest_file_invalid`；POST/GUI 先写盘再换 registry；`server.rs` 增加 swarm-doctor / manifest 路由集成测。

### Review Findings

- [x] [Review][Patch] 启动时 workspace manifest 校验失败时 HTTP doctor 无真实错误摘要 — **已修复**：`capability_manifest_bootstrap_error` + `swarm_cortex_doctor_for_gateway`（`agent-diva-agent` / `manager` / `bootstrap` / `AppState`）。
- [x] [Review][Patch] GUI 写盘静默失败仍 `ok: true` — **已修复**：先 `persist_capability_manifest_json` 再 `replace_with_manifest`；失败时 `ok: false` + `message`（`capability_commands.rs`；前端 `desktop.ts` 类型补充 `message`）。
- [x] [Review][Patch] Gateway POST 写盘失败仍 `status: ok` — **已修复**：先持久化再换 registry；持久化失败返回 `status: error`；成功 POST 清除 bootstrap 错误（`handlers.rs`）。
- [x] [Review][Patch] 缺少 swarm-doctor / manifest 的 axum 集成测 — **已修复**：`server.rs` 中 `get_swarm_doctor_reflects_app_state_registry`、`post_capability_manifest_persists_and_updates_registry`。
- [x] [Review][Defer] 仅编辑磁盘 `.agent-diva/capability-manifest.json` 而不 POST、不重启 gateway 时，HTTP doctor 与 CLI 读文件路径可能长期不一致 — 进程分离下的已知模型，可文档化而非本 story 必改。

## Status

done
