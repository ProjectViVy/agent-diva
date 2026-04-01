---
story_key: 4-2-skills-capability-ui
story_id: "4.2"
epic: 4
status: done
generated: "2026-03-30T12:00:00+08:00"
sources:
  - _bmad-output/planning-artifacts/epics.md
  - _bmad-output/planning-artifacts/prd.md
  - _bmad-output/planning-artifacts/architecture.md
  - _bmad-output/planning-artifacts/ux-design-specification.md
  - _bmad-output/implementation-artifacts/1-6-capability-v0-validation.md
---

# Story 4.2：Skills/能力包管理与校验反馈（UI）

Status: done

## Story

作为一名 **进阶用户**，  
我希望 **在设置或既定入口管理 skills/能力并看到校验错误**，  
以便 **FR10、FR11 在用户侧闭环**。

## 前置依赖

- **必须先完成 Story 1.6**（`1-6-capability-v0-validation.md`）：Rust 侧 v0 manifest 校验、**可机读错误**（字段级/文件级）、占位注册表与 **serde 可序列化** 消费契约已就绪。  
- 本故事 **消费** 1.6 的错误与成功摘要形态；不得在 UI 层重新发明与 1.6 不一致的「成功列表/状态」语义。

## Acceptance Criteria

1. **Given** 用户提交或编辑能力声明，且入口与 **现有 Settings（设置）** 对齐（沿用 PRD **FR10**「既有或随 MVP 提供的入口」与 agent-diva 当前设置能力衔接）  
   **When** 校验失败  
   **Then** 展示 **明确错误**，粒度为 **字段级或文件级**（与 `ux-design-specification.md` Form Patterns — 能力 manifest 及 Story 1.6 机读 `location_kind` / `path` 一致）

2. **Given** 同上  
   **When** 校验成功  
   **Then** **列表或状态** 更新，且与 **Story 1.6** 占位注册表可查询的摘要（条数、id 列表等）**一致** — 数据以 **invoke/DTO 透传** 或 thin map 为准，与 `architecture.md` API 命名惯例一致

3. **And** 设置页表单遵循 UX：**标签上置、校验行内、保存按钮右下**；**不** 引入第二套表单规范（`ux-design-specification.md` — 设置与 Provider）

4. **And** 错误反馈遵循全局反馈模式：**非阻塞** 为主、**可重试**；持久错误须可被辅助技术感知（与 UX Accessibility 节一致，避免无文案仅靠颜色）

## Tasks / Subtasks

- [x] **确认 1.6 契约**（AC: #1–#2）  
  - [x] 阅读并实现（或对接）Tauri `invoke`：提交/重载 manifest 的 command 与 **1.6 错误 JSON 结构**（`code`、`message`、`location_kind`、`path` 等）一一映射  
  - [x] 文件级错误（整份 JSON 无法解析）与字段级错误在 UI 上有 **区分展示**（标题/图标或分组列表）

- [x] **Settings 对齐与路由**（AC: #1、#3）  
  - [x] 在 **现有设置架构** 下增加或挂载「skills/能力包」区块；文案与 i18n key 与产品术语一致  
  - [x] 不新建与全局设置冲突的平行「第二设置中心」

- [x] **成功态 UI**（AC: #2）  
  - [x] 展示与 1.6 一致的 **能力列表或状态摘要**（实现以 1.6 暴露字段为准）  
  - [x] 空态：一句原因 + 主行动（若 UX 空态模式适用）

- [x] **验证**（AC: #1–#4）  
  - [x] 组件级：Vue Test Utils（或仓库既定）覆盖「多错误列表渲染」「文件级单条错误」  
  - [x] 至少一条 **冒烟 E2E**（若仓库已有 Playwright）：设置路径进入 → 触发失败/成功各一（可用 fixture manifest）— **本仓库未配置 Playwright，本条按故事条件跳过**；已用 Vitest + `@vue/test-utils` 覆盖错误展示组件。

### Review Findings

- [x] [Review][Patch] `submit_capability_manifest_json` 先 `clear()` 再 `register()`：若 `register` 返回 `Err`（如写锁中毒），注册表已被清空且未写入新 manifest，用户上一次成功提交的能力会丢失；与 AC#4「可重试」及 Dev Notes「成功时再替换」的语义不一致。建议在 `PlaceholderCapabilityRegistry` 提供单次写锁内的原子替换（例如先构建新 `HashMap` 再整体 swap），或仅在成功路径提交替换。 [`agent-diva/agent-diva-gui/src-tauri/src/capability_commands.rs:27-40`] — **已修复（2026-03-31）**：`PlaceholderCapabilityRegistry::replace_with_manifest` + command 改用单次写锁整体替换。

- [x] [Review][Defer] Vitest 仅覆盖 `CapabilityManifestErrorsDisplay`，未覆盖 `CapabilityManifestPanel` 的提交、`refreshSummary`、与 `invoke` 的交互；属测试深度缺口，非 AC 明文阻塞。 [`agent-diva/agent-diva-gui/src/components/settings/CapabilityManifestPanel.vue`] — deferred, pre-existing

- [x] [Review][Defer] `PlaceholderCapabilityRegistry::clear` 在写锁失败时静默跳过；与本次「clear + register」组合叠加时，可能出现旧条目未清掉或状态难诊断（与 Story 1.6 review 已记的 poisoned `RwLock` 行为一致）。 [`agent-diva/agent-diva-agent/src/capability/registry.rs:94-98`] — deferred, pre-existing

## Dev Notes

### FR10 / FR11 要点

| 主题 | 要求 | 来源 |
|------|------|------|
| **FR10** | 用户可通过 **既有或随 MVP 提供的入口** 管理 **skills 或能力包**（与 agent-diva 当前设置衔接） | `prd.md` |
| **FR11** | 系统 **加载并校验** 用户能力声明（v0 字段子集），**错误时可见反馈** — 本故事落实 **UI 反馈**；校验核心在 **1.6** | `prd.md`、`epics.md` Coverage |
| 结构映射 | 能力与 manifest（FR10–11）落点：`agent-diva-agent` skills 路径 + registry；UI 仅调用边界 API | `architecture.md` |

### 与 Story 1.6 的衔接

- **字段级 / 文件级**：1.6 产出结构化错误；4.2 **按 `location_kind` + `path` 映射** 到行内或列表，**禁止** 仅展示原始堆栈或非结构化字符串作为唯一反馈。  
- **成功一致性**：列表/计数与 **1.6 占位注册表** 查询结果同源（经 IPC）；若发现偏差，优先修正 DTO 或 1.6 暴露面，而非在 UI 硬编码第二条真相源。

### Settings 与 UX 对齐

- **表单**：`ux-design-specification.md` — 设置与 Provider；能力 manifest 错误 **字段级或文件级**，**显式** 与 Epic 4 Story 4.2 对齐（本故事即该条）。  
- **按钮层级**：保存为主行动时沿用现有主按钮样式；取消/返回为次要。

### 禁止与范围

- 不在本故事重复实现 **1.6 校验逻辑**（Rust）；不在 UI 解析 manifest 替代服务端校验。  
- 不引入 **swarm → meta** 违反 ADR-A 的依赖；GUI 变更限于 Tauri/Vue 现有栈。

### References

- `epics.md` — Epic 4、Story 4.2、故事依赖（**4.2 依赖 1.6**）  
- `1-6-capability-v0-validation.md` — 机读错误模型、Epic 4 消费契约、占位注册表  
- `prd.md` — FR10、FR11  
- `architecture.md` — FR10–FR11、能力与 manifest 映射、API 命名  
- `ux-design-specification.md` — Form Patterns、Feedback Patterns、Accessibility  

## Dev Agent Record

### Agent Model Used

Cursor Composer（bmad-dev-story，2026-03-30）

### Debug Log References

- 为使 `agent-diva-gui` 依赖 `agent-diva-agent` 能通过 `cargo check`，补齐了 `agent-diva-swarm` 中缺失的 `mod process_events`、`ProcessEventPipeline` 的 `Debug` 派生移除，以及 `agent_loop` 中 `ProcessEventFlushGuard` 改为持有 `Arc` 以避免与 `&mut self` 冲突（见 File List）。

### Completion Notes List

- Tauri：`submit_capability_manifest_json` / `get_capability_registry_summary`，直接调用 `agent-diva-agent::capability` 的解析、校验与 `PlaceholderCapabilityRegistry`；成功时 **`replace_with_manifest` 单次写锁整体替换**（设置内 JSON 为当前真相源；避免 clear+register 失败导致空表）。
- 返回体 `CapabilityManifestSubmitResult`：`ok` + 可选 `summary`（`RegistrySummary`）/ `errors`（与 1.6 一致的 `Vec<CapabilityManifestError>`），避免仅用字符串承载结构化校验失败。
- 设置 → 技能页挂载 `CapabilityManifestPanel`：标签上置、错误行内（`CapabilityManifestErrorsDisplay` 分组 file/field）、主按钮右下；`role="alert"` / `aria-live` 与文案并用。
- 前端：`npm run test`（Vitest）3 条用例覆盖多错误、单文件级、多字段级；`npm run build`、`cargo clippy -p agent-diva-gui -D warnings` 已通过。
- **E2E**：仓库无 Playwright，故事子任务在「若已有」前提下跳过；如需冒烟可后续加 Playwright 依赖与一条设置流。

### File List

- `agent-diva/agent-diva-gui/src-tauri/Cargo.toml`（增加 `agent-diva-agent`）
- `agent-diva/agent-diva-agent/src/capability/registry.rs`（`replace_with_manifest`）
- `agent-diva/agent-diva-gui/src-tauri/src/capability_commands.rs`（新建）
- `agent-diva/agent-diva-gui/src-tauri/src/lib.rs`
- `agent-diva/agent-diva-gui/src/api/desktop.ts`
- `agent-diva/agent-diva-gui/src/components/settings/CapabilityManifestPanel.vue`（新建）
- `agent-diva/agent-diva-gui/src/components/settings/CapabilityManifestErrorsDisplay.vue`（新建）
- `agent-diva/agent-diva-gui/src/components/settings/CapabilityManifestErrorsDisplay.spec.ts`（新建）
- `agent-diva/agent-diva-gui/src/components/settings/SkillsSettings.vue`
- `agent-diva/agent-diva-gui/src/locales/zh.ts`
- `agent-diva/agent-diva-gui/src/locales/en.ts`
- `agent-diva/agent-diva-gui/package.json`
- `agent-diva/agent-diva-gui/vite.config.ts`
- `agent-diva/agent-diva-swarm/src/lib.rs`
- `agent-diva/agent-diva-swarm/src/process_events.rs`
- `agent-diva/agent-diva-agent/src/agent_loop/loop_turn.rs`
- `_bmad-output/implementation-artifacts/sprint-status.yaml`

## Change Log

- 2026-03-31：代码审查 patch — `PlaceholderCapabilityRegistry::replace_with_manifest` + Tauri 提交路径，避免 clear+register 失败导致注册表被清空。
- 2026-03-30：实现 Story 4.2（能力 manifest UI + Tauri IPC + Vitest）；修复 swarm/agent 编译阻塞项以便链接 `agent-diva-agent`。

---

_Context: BMad 故事上下文 — 与 `1-1-swarm-crate-workspace.md` / `1-6-capability-v0-validation.md` 模板对齐；Epic 4 Story 4.2，简体中文。_
