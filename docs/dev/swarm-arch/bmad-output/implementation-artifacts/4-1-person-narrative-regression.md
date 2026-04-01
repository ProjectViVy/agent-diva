---
story_key: 4-1-person-narrative-regression
story_id: "4.1"
epic: 4
status: done
generated: "2026-03-30T12:00:00+08:00"
sources:
  - _bmad-output/planning-artifacts/epics.md
  - _bmad-output/planning-artifacts/prd.md
  - _bmad-output/planning-artifacts/architecture.md
  - _bmad-output/planning-artifacts/ux-design-specification.md
---

# Story 4.1：Person 单一叙事回归测试 / 走查清单

Status: done

## 故事

As a **产品/QA**,  
I want **可重复的检查单或自动化断言**,  
So that **用户可见界面不出现多机器人并列流（FR8、FR9）**。

## 功能需求锚点

- **FR8：** 系统在用户可见渠道上维持 **单一 Person 叙事线**（不出现多个并列「机器人头像」式独立对话流）。
- **FR9：** 系统在 **大脑皮层启用** 时仍满足 FR8，对内协作 **不** 以多用户可见聊天室形式暴露。

## 验收标准（来自 epics）

**Given** 大脑皮层 **开/关** 各至少一条典型路径  
**When** 执行检查  
**Then** **无** 多个并列「独立 agent 聊天头像」式通道  
**And** 结果记录为测试或手动清单链接

---

## 走查清单（手动）

在每条路径下，观察者仅看 **用户可见 UI**（含侧栏、主对话区、神经/大脑皮层相关视图），勾选下列项。

### 路径 A：大脑皮层 **关闭**

| # | 检查项 | 通过 |
|---|--------|------|
| A1 | 主叙事区仅呈现 **一个** 对外 Person/助手身份（无多列独立「机器人对话」） | ☐ |
| A2 | 无多个可并行输入的「独立 agent 聊天室」式并列线程（头像 + 独立 transcript） | ☐ |
| A3 | 若存在状态/进度提示，仍感觉 **同一 Person** 在说话，而非多个并列 bot | ☐ |

### 路径 B：大脑皮层 **开启**

| # | 检查项 | 通过 |
|---|--------|------|
| B1 | 仍满足 FR8：用户可见渠道 **单一 Person 叙事线** | ☐ |
| B2 | 内部协作、多 agent 活动 **不得** 以「多用户可见聊天室」形式暴露（FR9） | ☐ |
| B3 | 可有进度/阶段/工具信号，但 **不** 引入第二套并列「机器人头像」主对话流 | ☐ |

### 记录

- 执行日期 / 版本 / 构建号：________________  
- 证据：截图或录屏路径，或链接至测试报告：________________

---

## 自动化断言（建议）

实现时按栈选用（E2E / 组件测 / 快照），下列为 **可机器检查** 的表述，与开/关路径各跑至少一次。

### 通用（关 / 开均需）

1. **并列头像/会话计数：** 在用户可见根容器内，代表「独立 agent 对话流」的 DOM 角色或 `data-testid` 集合 **数量 ≤ 1**（或等价：仅一个主 transcript 容器激活）。
2. **禁止多聊天室壳：** 不存在多个并列的、各自含完整输入框 + 消息列表的「聊天室」结构（与 UX 反模式「多聊天机器人头像并列」对齐）。
3. **可访问性/语义（若实现）：** 主对话区 `aria` 或标题不暗示多个并立「助手身份」为对等聊天对象。

### 皮层 **关** 专用

4. 关闭状态下导航典型 MVP 路径（如侧栏 → 神经入口 → 主对话），断言 **不出现** 第二套 agent 流容器（即使为空也不应挂载双轨 UI）。

### 皮层 **开** 专用

5. 开启状态下触发至少一次需对内协作的典型任务，断言：仍仅 **一个** 主 Person 输出通道；内部 trace/子 agent UI **不在**默认可见 transcript 中并列展示（与 NFR-R2 / 架构「单一对外写出口」精神一致，若有单独「诊断/开发者」面板须默认隐藏或非主路径）。

### CI 挂钩

- [x] 将上述断言纳入现有 E2E 或视觉回归任务，并在 PR 描述或 `TESTING.md` 中 **链接本文件** 作为 Story 4.1 结果记录。

---

## Tasks / Subtasks

- [x] 为唯一主对话壳添加 `data-testid`（`person-agent-conversation-stream` / `person-main-transcript` / `person-main-composer`）及用户可见根 `user-visible-app-root`
- [x] 主对话区 `role="log"` 与 i18n `aria-label`（单一助手线程语义，满足可访问性检查项）
- [x] Vitest：`personNarrativeRegression.spec.ts` — 聊天单壳、模拟 Tauri+皮层条仍单壳、神经视图无第二聊天壳、中英 aria
- [x] `agent-diva-gui/TESTING.md` 链接本故事文件；`npm test` / `npm run build` 通过
- [x] `ChatView` 在 jsdom 下 `scrollIntoView` 可选调用，避免未处理 rejection

### Review Findings

- [x] [Review][Decision] 自动化是否在「用户可见根容器」语义下足够 — **已按选项 B 落实**：在 `NormalMode` 整壳的 `user-visible-app-root` 内计数；侧栏切换神经（主区仅 `NervousSystemView`，与聊天 `v-if` 互斥）后根内 0 流壳，再回到聊天后仍为 1；另含整壳 + Tauri 桩下皮层条仍单流。见 `personNarrativeRegression.spec.ts`。

- [x] [Review][Patch] 中文 Vitest 仅断言外壳 `primaryPersonNarrativeAria` — **已补** `mainTranscriptAria` / `mainComposerAria` 断言 [`personNarrativeRegression.spec.ts`]

- [x] [Review][Defer] 分支上 `en.ts` / `zh.ts` 的 git diff 含大量非 Story 4.1 的词条块（cortex、neuro、capabilityManifest 等）— 属并行故事合流所致，非本故事实现缺陷；针对 4.1 的评审以故事 File List 与上述实现为准 — deferred, pre-existing

---

## Dev Agent Record

### Implementation Plan

- 以 Vitest + 组件挂载覆盖故事所述「≤1 并列 agent 流」与「神经非第二聊天室」；全应用 E2E（Playwright）未引入，与当前包一致。
- 皮层开/关：结构不随皮层状态分叉，用「显示皮层条 + 仍单壳」模拟路径 B。

### Debug Log

- Vitest 报 `scrollIntoView is not a function` → `ChatView` 使用 `scrollIntoView?.(...)` 可选调用。

### Completion Notes

- Story 4.1 自动化基线已落地：`src/components/personNarrativeRegression.spec.ts`（含 `NormalMode` 整壳与隔离 `ChatView`/`NervousSystemView` 共 9 条用例）。手动走查表仍见本文档上文表格；请在发版前按路径 A/B 勾选并填「记录」段。

---

## File List

- `agent-diva/agent-diva-gui/src/components/ChatView.vue`
- `agent-diva/agent-diva-gui/src/components/NormalMode.vue`
- `agent-diva/agent-diva-gui/src/locales/en.ts`
- `agent-diva/agent-diva-gui/src/locales/zh.ts`
- `agent-diva/agent-diva-gui/src/components/personNarrativeRegression.spec.ts`（含 `NormalMode` 集成用例）
- `agent-diva/agent-diva-gui/TESTING.md`

---

## Change Log

- 2026-03-31（续）：代码审查选项 B — `NormalMode` + `user-visible-app-root` 集成 Vitest（聊天 / 神经切换 / Tauri 皮层条）；中文 aria 全量断言；故事与 sprint 标为 done。
- 2026-03-31：Story 4.1 — 单一 Person 叙事 DOM 标记、Vitest 回归、`TESTING.md` 与 i18n aria；`ChatView` scrollIntoView 兼容 jsdom。

---

## 依赖与备注

- 与架构中 **PersonOutbox / lease**、**单一对外写出口** 不变式一致；实现细节以 `architecture.md` 与蜂群设计文档为准。
- Epic 4 内 **Story 4.2** 依赖 **Story 1.6**；本故事（4.1）为叙事回归基线，宜在 UI 稳定后重复执行。
