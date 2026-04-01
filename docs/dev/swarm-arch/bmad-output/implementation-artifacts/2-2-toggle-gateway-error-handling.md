---
story_key: 2-2-toggle-gateway-error-handling
story_id: "2.2"
epic: 2
status: done
language: zh-CN
nfr:
  - NFR-R1
nfr_r1:
  rollback_or_explicit_error: true
  no_unknown_toggle_state: true
depends_on:
  - story_key: 2-1-cortex-toggle-ui
    story_id: "2.1"
generated: "2026-03-30T12:00:00+08:00"
sources:
  - _bmad-output/planning-artifacts/epics.md
  - _bmad-output/planning-artifacts/architecture.md
  - _bmad-output/planning-artifacts/ux-design-specification.md
  - _bmad-output/planning-artifacts/prd.md
---

# Story 2.2：开关与 gateway 错误处理

Status: done

**依赖：** 须先完成 Story **2.1**（`2-1-cortex-toggle-ui`）— `CortexToggle` 与 Story 1.3 契约调用路径可用。  
**非功能：** **NFR-R1** — 切换失败须 **可恢复**（回滚至上一稳定态 **或** 显式错误），**禁止** 静默进入未知模式；本故事落实 **回滚/显错** 与 **禁止「未知开/关」中间态**。

## Story

As a **用户**,  
I want **同步失败时看到明确错误而非静默错误状态**,  
So that **符合 NFR-R1**。

（产品叙述，与 `epics.md` 一致。）

## Acceptance Criteria

1. **Given** 可模拟 **gateway 拒绝** 或 **超时**（测试桩、mock invoke、或可控故障注入，实现任选一种并文档化）  
   **When** 用户在聊天主页切换 **大脑皮层**（`CortexToggle`，Story 2.1）  
   **Then** 界面展示 **可读的错误提示**（i18n key + 非泄露内部的友好文案；与现有 Tauri 错误形态一致）  
   **And** **UI 状态回滚** 至切换 **前** 的 **上一稳定开/关**，**或** 在无法确认时 **保持** 该稳定态 —— **不得** 呈现与后端真相源不一致的「已切换成功」假象（对齐 **FR14**）

2. **And** 系统 **不进入**「未知开/关」中间态：任意时刻，用户可见的开关语义须与 **可解释的** 状态集合一致（开 / 关 / 进行中可取消 / 失败且已回滚或保持），**禁止** 长时间停留在无法判定 on/off 的模糊 UI（对齐 **NFR-R1** 与 UX「切换可信」）

3. **And** Rust / gateway 侧在切换失败路径记录 **tracing** **error** 级别 span（或项目等价约定），便于排障，且 **不** 将内部 trace **默认写入** 用户 transcript（**NFR-R2** 精神，与架构一致）

## Tasks / Subtasks

- [x] **契约与故障语义冻结**（AC: #1–#2）  
  - [x] 对照 Story **1.3** 的 set/query 契约：明确 **成功**、**可恢复失败**（回滚/保持）、**超时** 的返回或事件形态  
  - [x] 在实现说明或组件注释中写明：**乐观更新** 若存在，**必须** 与统一回滚策略一致；否则采用 **以后端确认为准**（见 `architecture.md` — State Management Patterns）

- [x] **前端：CortexToggle 错误与回滚**（AC: #1–#2）  
  - [x] invoke 失败或超时：toast 或行内错误（与 UX 规格 **非阻塞**、**可重试** 一致）  
  - [x] **同步失败须回滚 UI** 至上一稳定态并说明原因（UX 规格 CortexToggle 错误策略）  
  - [x] 加载中态有明确起止，避免结束后仍停留在 indeterminate「半开半关」视觉

- [x] **后端 / Tauri：错误传播与观测**（AC: #1、#3）  
  - [x] gateway 拒绝或超时时返回 **稳定错误类型**（与现有 `Result` / 字符串错误约定一致），**不** 半更新持久化皮层状态  
  - [x] 失败分支打 **tracing** error span（字段含 session / 操作 id 等与 `DESIGN_SUPPLEMENT` / 架构可观测约定可对齐的子集）

- [x] **测试**（AC: #1–#2）  
  - [x] 至少 **一条** 自动化用例：**模拟 gateway 失败** 时 UI（或契约层若先做 Rust 测）表现为 **显式错误 + 状态未错误提交**  
  - [x] 若有 E2E 能力：覆盖「切换 → 失败 → 开关与后端一致」；否则以 **组件测 / mock invoke** 验收并记在 Dev Agent Record

- [x] **验证**  
  - [x] 手动：断网或桩失败下切换，确认 **无未知态**、提示可读、可重试  
  - [x] `cargo clippy` / 前端 lint 与相关测试通过（范围含本 story 改动）

## Dev Notes

### Epic 2 上下文

Epic 2 目标：用户在主聊天界面 **操作并理解** 大脑皮层开关，启用时看到过程反馈；满足 **NFR-P1/P2**、**NFR-A1**、**UX-DR1/DR3**。本故事专注 **同步失败时的可信行为**，是 **2.1** 的自然延伸，**不** 实现过程条（属 **2.3**）。

### 架构合规（NFR-R1）

| 主题 | 要求 | 来源 |
|------|------|------|
| 真相源 | 皮层状态权威在 **Rust（gateway）**；失败时 **不** 让 GUI 长期持有与后端分叉的「已切换」 | `architecture.md` — 运行时真相源、API & Communication / 错误 |
| 切换失败 | **显式错误 + 回滚到上一稳定态**；**tracing** error span | `architecture.md` — Process Patterns / Error Handling |
| 状态模式 | 乐观更新 **仅当** 已有统一回滚；否则 **以后端确认为准** | `architecture.md` — State Management Patterns |
| 用户可见错误 | i18n key，**不** 直接堆栈进 toast | `architecture.md` — Format Patterns / API Response |

### UX 对齐

- **CortexToggle** 错误：**非阻塞**；同步失败 **必须回滚 UI** 并说明原因（**NFR-R1**）。见 `ux-design-specification.md` 组件错误策略与 Phase 1 表（`CortexToggle` + 错误回滚）。

### 禁止事项

- 静默失败、无提示的「假成功」切换。  
- 长期 indeterminate 开关，使用户无法判断当前是开还是关。  
- 在本 story 内实现完整 **ProcessFeedbackStrip**（**2.3**）或 RunTelemetry（**2.4**）。

### Project Structure Notes

- 工作区根以本机为准：`d:\newspace\`。  
- GUI 变更预期在 `agent-diva-gui` 的聊天相关视图与 `CortexToggle`（Story 2.1 落地路径）；Tauri 命令与 gateway 与 Story 1.3 一致。

### Testing Requirements

- 至少 **1** 条可自动化测试证明 **失败 + 回滚/保持稳定态**。  
- 全链路 E2E 可选；无则必须在 Dev Agent Record 说明覆盖方式。

### References

- `epics.md` — Epic 2, Story 2.2  
- `architecture.md` — NFR-R1、错误处理、状态管理、GUI↔Rust  
- `ux-design-specification.md` — CortexToggle、错误与回滚  
- `prd.md` — FR1、FR4、FR12–FR14、NFR-R1  
- `2-1-cortex-toggle-ui`（实现后）— UI 入口与 invoke 绑定

## Dev Agent Record

### Agent Model Used

Cursor 内联代理（Composer）

### Debug Log References

（无）

### Implementation Plan

- Tauri：`cortex_sync` 单点钩，`set_cortex_enabled` / `toggle_cortex` 先同步再写入 `CortexRuntime`；失败不 emit。环境变量 `AGENT_DIVA_TEST_CORTEX_SYNC_FAIL=1` 模拟拒绝。  
- GUI：以后端确认为准（无乐观翻转）+ `showAppToast` + `get_cortex_state` 重同步；稳定码 `cortex_sync_rejected` 映射 i18n。  
- 契约：`docs/swarm-cortex-contract-v0.md` 增补故障语义与注入说明。  
- 附带修复：`neuro_overview` 对 `SwarmRunFinished` / `SwarmRunCapped` 的 match 穷尽；`NervousSystemView.vue` 未使用 `Event` 导入（否则 `vue-tsc` 失败）。

### Completion Notes List

- ✅ AC1–3：`tracing::error` 在注入失败路径；Vitest mock invoke 验证 UI 不变 + toast；Rust `apply_cortex_target_after_sync` 单测验证失败不突变 runtime。  
- ✅ 无 E2E：以组件测 + Rust 单测 + 手测说明（设置 `AGENT_DIVA_TEST_CORTEX_SYNC_FAIL=1` 后点开关应 toast 且状态不变）满足验收。  
- ✅ `cargo test`（`agent-diva-gui` src-tauri）、`cargo clippy -D warnings`（gui tauri + swarm）、`npm test`、`npm run build` 已通过。

### File List

- `agent-diva/agent-diva-gui/src-tauri/src/cortex_sync.rs`（新增）
- `agent-diva/agent-diva-gui/src-tauri/src/lib.rs`
- `agent-diva/agent-diva-gui/src-tauri/src/commands.rs`
- `agent-diva/agent-diva-gui/src/api/cortex.ts`
- `agent-diva/agent-diva-gui/src/components/CortexToggle.vue`
- `agent-diva/agent-diva-gui/src/components/CortexToggle.spec.ts`（新增）
- `agent-diva/agent-diva-gui/src/locales/zh.ts`
- `agent-diva/agent-diva-gui/src/locales/en.ts`
- `agent-diva/agent-diva-gui/src/components/neuro/NervousSystemView.vue`
- `agent-diva/agent-diva-swarm/src/cortex.rs`
- `agent-diva/agent-diva-swarm/src/neuro_overview.rs`
- `docs/swarm-cortex-contract-v0.md`
- `_bmad-output/implementation-artifacts/sprint-status.yaml`

### Change Log

- **2026-03-31：** Story 2.2 实现 gateway 同步钩、CortexToggle 错误/toast/后端优先 UI、契约文档与自动化测试；sprint 状态 → review。
- **2026-03-31：** 代码审查修复：`AGENT_DIVA_TEST_CORTEX_SYNC_FAIL` 仅在 `cfg(test)` / `debug_assertions` 下生效；契约文档同步；审查项勾选；sprint 状态 → done。

### Review Findings

- [x] [Review][Patch] 将 `AGENT_DIVA_TEST_CORTEX_SYNC_FAIL` 故障注入限制在 `cfg(any(test, debug_assertions))`（或等价 feature），避免 **release** 二进制因环境变量永久拒绝皮层切换、降低误触/滥用面 [agent-diva/agent-diva-gui/src-tauri/src/cortex_sync.rs] — 已于代码审查后修复

---

_上下文：BMad Method 故事上下文 — `bmad-create-story` 于 2026-03-30 生成；正文为 **简体中文**。_
