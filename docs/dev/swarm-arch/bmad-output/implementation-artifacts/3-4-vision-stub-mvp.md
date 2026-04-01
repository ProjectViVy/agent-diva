---
story_key: 3-4-vision-stub-mvp
story_id: "3.4"
epic: 3
status: done
generated: "2026-03-30T12:00:00+08:00"
sources:
  - _bmad-output/planning-artifacts/epics.md
  - _bmad-output/planning-artifacts/prd.md
  - _bmad-output/planning-artifacts/ux-design-specification.md
  - _bmad-output/planning-artifacts/architecture.md
---

# Story 3.4：愿景占位（可选）与 MVP 路径隔离

Status: done

## Story

As a **产品**,  
I want **若存在游戏化/总控台占位，明确标注「后续」且不阻断 MVP**,  
So that **满足 FR16**。

## 覆盖 FR / UX

| 编号 | 说明 |
|------|------|
| **FR16** | 游戏化总控台优先入口、《头脑特工队》式多角色忙碌等 **不属于** MVP 验收；占位须 **标明愿景/后续** 且 **不阻断** FR5–FR7、FR15。 |
| **UX-DR2** | 神经系统与中控台 **概念分坑**；侧栏 **不同图标 + 不同 i18n key**。 |
| **UX-IMPL-2～4** | MVP 首屏为架构图式 + 左右分区；诚实 stub；与 Epic 3 其它故事衔接。 |

## Acceptance Criteria

1. **Given** 实现中若加入 **非 MVP** 入口或占位卡片（游戏化总控台、多角色忙碌愿景等）  
   **When** 用户走 **MVP 路径**（侧栏 → 神经系统 → **BrainOverview**）  
   **Then** **无需** 经过该占位即可完成 **FR5–FR7、FR15** 所要求的核心体验

2. **And** 所有此类占位的 **用户可见文案** 均通过 **vue-i18n** 提供，且文案或键的语义 **明确标明**「愿景 / 后续 / 非 MVP 验收」，**不得** 暗示当前已交付或可验收

3. **And** 占位 **不得** 成为进入神经系统后的 **必经首屏**；默认主路径仍以 Story 3.1 定义的 **BrainOverview** 为准（与 FR16、Epic 3 目标一致）

## Tasks / Subtasks

- [x] **盘点与门禁**（AC: #1–#3）  
  - [x] 列出 `agent-diva-gui` 内神经系统相关路由/子视图；确认 MVP 默认落地页为 **BrainOverview**（或等价实现名），且无强制路由经过愿景区  
  - [x] 若尚无愿景 UI：本 story 可 **仅** 在代码或设计注释中登记「禁止必经」规则 + 预留 i18n 键命名空间（仍须满足 AC：无阻断）

- [x] **i18n 与标签**（AC: #2）  
  - [x] 在 `src/locales/en.ts`、`zh.ts`（或项目现行结构）新增 **专用 key**（示例方向：`nervousSystem.vision.*`、`nervousSystem.futureOnly.*` — 以实现为准），**中英文** 均含「后续 / 非验收 / 愿景」语义  
  - [x] **禁止** 硬编码用户可见中/英文；愿景卡片标题、副标题、辅助说明一律走 key

- [x] **可选 UI：愿景占位**（AC: #1–#3）  
  - [x] 若实现卡片/次要入口：置于 **BrainOverview 之外的** 可选区域（如次要 tab、折叠区、「了解更多」），且 **默认可跳过**  
  - [x] 视觉层级：**弱于** MVP 分区与排障主路径；可用 `UX-IMPL` 中的 **警告/琥珀** 语义表达「beta / 愿景」（与 UX 规格一致，非阻断）

- [x] **验证**（AC: #1–#3）  
  - [x] 手动：侧栏进入神经 → 直达首屏分区视图 → 完成与 FR6/FR7 相关的最小浏览 **不点击** 愿景区仍可结束旅程  
  - [x] 切换语言：愿景相关文案均来自 i18n，且仍明确「非 MVP 验收」

## Dev Notes

### Epic 3 上下文

Epic 3 目标：神经系统全屏视图 **MVP 首屏** 为 **架构图式大脑 + 左右分区**；**真实或诚实 stub** 与排障线索；**不得以游戏化总控台为必经首屏**。Story 3.4 是 **FR16** 的显式门禁故事，可与 3.1–3.3 **并行或殿后** 实施，但合并前须满足上表 AC。

### 架构与实现约束

| 主题 | 要求 | 来源 |
|------|------|------|
| i18n | 用户可见字符串走 `locales/*`；神经系统术语 **键名稳定** | `architecture.md`、`ux-design-specification.md` |
| 错误/提示 | 与现有 Tauri 命令错误形态一致；展示层用 key | `architecture.md` |
| 诚实标注 | stub/愿景与「live」数据区分，不冒充已交付功能 | FR6 精神、UX 规格 |

### 依赖故事

- **建议前置：** Story **3.1**（路由与壳 + BrainOverview 首屏）至少已具备可导航骨架，以便验证「不经愿景即可完成 MVP 路径」。  
- **衔接：** Story **3.2、3.3** 的数据阶段与空错模板 **不得** 被愿景占位劫持为主路径。

### References

- PRD **FR16**：非 MVP 愿景占位须标注且 **不阻断** FR5–FR7、FR15。  
- `epics.md` — Epic 3 / Story 3.4 原文验收标准。  
- `ux-design-specification.md` — 神经系统分期、诚实 stub、侧栏与中台区分（UX-DR2）。

## Dev Agent Record

### Implementation Plan

- 盘点：`NormalMode.vue` 侧栏 `navigateTo('neuro')` → `NervousSystemView` → 主区域首块为 `BrainOverview`；无独立子路由，愿景区块不阻塞该路径。
- 在 `neuro.vision.*` 下新增中英键；`NervousSystemView` 在 `BrainOverview` 下方增加默认收起的 `<details>` 琥珀愿景区；脚本内注释登记 FR16「禁止必经首屏」规则。
- Vitest：`neuroVision.i18n.spec.ts` 校验键与语义；`NervousSystemView.spec.ts` 校验主视图与 i18n 绑定。

### Debug Log

- （无）

### Completion Notes

- ✅ AC1–3：`BrainOverview` 仍为进入神经系统后的首要内容；愿景为下方可选折叠区，默认 `open=false`，无需交互即可浏览 MVP 分区。
- ✅ 愿景文案全部经 `vue-i18n` 的 `neuro.vision.*`，中英均含非 MVP 验收/愿景/后续语义。
- ✅ `agent-diva-gui` 下已执行 `npm test`、`npm run build` 通过。

## File List

- `agent-diva/agent-diva-gui/src/locales/en.ts`
- `agent-diva/agent-diva-gui/src/locales/zh.ts`
- `agent-diva/agent-diva-gui/src/components/neuro/NervousSystemView.vue`
- `agent-diva/agent-diva-gui/src/components/neuro/neuroVision.i18n.spec.ts`
- `agent-diva/agent-diva-gui/src/components/neuro/NervousSystemView.spec.ts`
- `_bmad-output/implementation-artifacts/sprint-status.yaml`

## Change Log

- 2026-03-31：实现 FR16 愿景 i18n、神经系统内可选折叠占位、自动化测试；sprint 状态更新为 review。
