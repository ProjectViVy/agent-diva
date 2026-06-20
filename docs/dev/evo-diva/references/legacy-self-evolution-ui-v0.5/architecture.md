---
stepsCompleted: [1, 2, 3, 4, 5, 6, 7, 8]
inputDocuments:
  - "prds/prd-my-project-2026-06-02/prd.md"
  - "ux-designs/ux-my-project-2026-06-02/DESIGN.md"
  - "ux-designs/ux-my-project-2026-06-02/EXPERIENCE.md"
  - "agent-diva-pro/docs/project-context.md"
workflowType: 'architecture'
lastStep: 8
status: 'complete'
completedAt: '2026-06-02'
project_name: 'agent-diva-pro'
user_name: 'Administrator'
date: '2026-06-02'
hard_constraints:
  - "本分支（agent-diva-pro）仅做 GUI，不改后端代码"
  - "架构文档转发主分支开发组，作为前后端合同"
---

# Architecture Decision Document

_本文档记录 `agent-diva-pro` 自进化 + 沙箱 UI 的架构决策。该项目建立在既有 Rust workspace + Vue/Tauri GUI 之上，技术栈锁定，本文档只定义本轮需求对应的增量架构、前后端合同与 AI Agent 实施边界。_

---

## Project Context Analysis

### Requirements Overview

本轮需求聚焦 **6 个 P0 UI 表面 + 1 组后端合同需求**，实际实施范围被硬约束限定为：**pro 分支只做 GUI，不改后端代码**。

| FR 组 | 目标 | 主要落点 | 说明 |
|------|------|----------|------|
| FR-1 | 决策卡片 | `ChatView.vue` + `DecisionCard.vue` | Plan 结果结构化展示、风险可见、同意/拒绝 |
| FR-2 | Todo 卡片 | `TodoCard.vue` | 对话内通用待办，逐项勾选、持久化、自动折叠 |
| FR-3 | 即时审批 | `ApprovalBanner.vue` + SSE | 敏感操作前的轻量审批，不跳转页面 |
| FR-4 | 记事本 | `NotebookView.vue` | 日报/周报/月报浏览与治理动作入口 |
| FR-5 | 自进化设置 | `SelfEvolutionSettings.vue` + `MemoryChangelog.vue` | AutoDream 节律、触发策略、学习策略、变更只读视图 |
| FR-7 | 沙箱设置 | `SandboxSettingsSection.vue` | 模式、审批策略、网络、路径、拒绝模式、超时 |
| FR-6 | 后端合同 | manager/core/agent（主分支实现） | API、SSE、配置 schema、审批链路 |

### Technical Constraints & Dependencies

| 约束 | 详情 |
|------|------|
| 目标仓库 | `agent-diva-pro` |
| 分支约束 | **本分支前端 only，禁止后端修改；后端仅定义合同，不在本分支实现** |
| 前端框架 | Vue 3 Composition API + `<script setup lang="ts">` |
| 状态模式 | props-down / events-up，无 Pinia |
| 样式系统 | Tailwind CSS 3.4 + CSS Custom Properties 四主题 |
| 现有 GUI 壳 | Tauri v2 + 既有 `SettingsView` dashboard/subview 模式 |
| 通知/弹窗 | `AppToastLayer` + `appDialog` 已存在，应复用 |
| Markdown 渲染 | 记事本详情区复用现有 markdown-body 样式 |
| 审批超时 | 默认 5 分钟，由 manager 仲裁，GUI 只显示倒计时 |
| 配置写回 | 统一通过 `invoke()` / manager API，不允许 GUI 直接写运行时文件 |

### Cross-Cutting Concerns

1. **审批链路必须可见**：agent loop → manager → GUI 三层协议，禁止静默执行敏感动作。  
2. **SSE 是实时交互主通道**：审批请求、审批结果、记事本新报告通知通过 `useEventStream()` 统一接入。  
3. **卡片协议要稳定**：Decision / Todo / Approval 三类卡片共用统一 DTO，避免多个 Agent 各自发明消息格式。  
4. **治理链必须可追踪**：提案 → 审批 → 应用 → changelog 只读可查。  
5. **四主题必须天然兼容**：所有视觉 token 通过 CSS 变量，不新增脱离主题系统的硬编码配色。  
6. **前后端职责必须清晰**：本分支实现 GUI；主分支负责 API、SSE、配置 schema 与审批仲裁。

---

## Starter Template Evaluation

**结论：不做 starter/template 评估，直接扩展既有 `agent-diva-pro` workspace。**

理由：

- 项目已存在稳定的 Rust workspace + Vue/Tauri GUI。
- `project-context.md` 已锁定命名、模块组织、错误处理与前端目录结构。
- 本轮需求是 **在既有 GUI 壳内做增量扩展**，不是新建应用。
- 复用现有页面与壳层（`ChatView` / `NormalMode` / `SettingsView`）的成本最低，且符合用户“不做过度设计”的偏好。

因此，架构目标不是“选择新框架”，而是：

- 在既有壳层中插入结构化交互组件；
- 补足 Notebook / Self Evolution / Sandbox UI；
- 把主分支未来实现的后端能力先定义成稳定合同。

---

## Core Architectural Decisions

### ADR-1: UiCard DTO 定义位置

**Decision**: 逻辑上的卡片合同由 manager 层拥有；本分支前端先在 `agent-diva-gui/src/api/desktop.ts` 建立 TypeScript 对应接口，作为消费端合同。主分支正式实现时，Rust 侧 DTO 放在 `agent-diva-manager` 边界内，而非 `core`。

**Rationale**:

- 卡片的直接消费者是 GUI，直接生产者是 manager handler。
- `agent-diva-core` 应保持基础设施/领域基础层定位，不承载面向 GUI 的展示 DTO。
- 当前分支受限于“只做 GUI”，因此前端先定义镜像接口最务实。
- 这允许前端先用 mock / stub 开发，再由主分支补齐真实 API。

**Frontend contract**:

```ts
export interface UiCard {
  id: string
  kind: 'decision' | 'todo' | 'approval'
  status: string
  title: string
  summary: string
  body_markdown?: string
  actions: UiCardAction[]
  evidence_refs?: string[]
  risk_level?: 'low' | 'medium' | 'high'
  todo_items?: TodoItem[]
  created_at: string
  updated_at: string
}

export interface TodoItem {
  id: string
  content: string
  status: 'pending' | 'done'
  completed_at?: string
}

export interface UiCardAction {
  id: string
  label: string
  style: 'primary' | 'secondary' | 'danger' | 'quiet'
  payload: string
}
```

**Affects**: `agent-diva-gui/src/api/desktop.ts`, 主分支 manager DTO 设计。

### ADR-2: 卡片状态持久化策略

**Decision**: 卡片状态以 manager/API 为单一真相源；本分支前端开发阶段可采用临时本地状态或 localStorage 过渡，但目标合同必须是后端持久化。

**Rationale**:

- Todo/审批状态属于会话资产，不能只存在组件内存里。
- 未来 CLI / 多窗口 / 重新打开 session 都需要看到同一状态。
- 用户要求 durable change 可审阅、可追踪，因此状态必须进入历史或持久层。
- 由于本分支不改后端，前端开发时允许“先 mock，后切换到正式端点”。

**Target endpoints (contract only)**:

- `POST /api/cards/{id}/action`
- `POST /api/todos/{id}/check`
- `POST /api/approval/respond`

**Affects**: `DecisionCard.vue`, `TodoCard.vue`, `ApprovalBanner.vue`, 主分支 manager/session 存储。

### ADR-3: SSE 连接管理

**Decision**: 采用原生 `EventSource` + Vue composable `useEventStream()` 作为 GUI 实时事件接入层。

**Rationale**:

- 标准 Web API，简单、成熟、对 Tauri 友好。
- composable 可统一封装连接、断开、重连、事件分发与 mock 模式。
- 前端可先用 mock SSE 开发；主分支实现后只替换 transport，不改组件协议。

**Required events**:

- `notebook.new_report`
- `approval.required`
- `approval.resolved`

**Affects**: `agent-diva-gui/src/utils/useEventStream.ts`, `ChatView.vue`, `NotebookView.vue`。

### ADR-4: 审批状态机与三层协议

**Decision**: 审批协议分为三层，**manager 端负责超时仲裁与幂等**，GUI 负责展示与提交用户决策。

```text
manager → GUI (SSE): approval.required
{ request_id, operation, risk, scope, timeout_seconds, created_at }

GUI → manager (HTTP): POST /api/approval/respond
{ request_id, decision }

manager → GUI (SSE): approval.resolved
{ request_id, decision }
```

**State machine**:

- `pending`
- `approved`
- `rejected`
- `expired`

**Rules**:

- 超时默认 5 分钟，未响应自动 `expired` → `rejected`。
- GUI 仅显示剩余时间，最后 60 秒才显式倒计时。
- 同一 `request_id` 重复提交必须幂等，返回已有结果。
- 审批横幅不阻断消息流，不使用全屏 modal。

**Affects**: 主分支 agent loop / manager；前端 `ApprovalBanner.vue` 与 `useEventStream.ts`。

### ADR-5: ChatView 卡片渲染入口

**Decision**: 扩展现有 tool 消息渲染区，在 `ChatView.vue` 内通过 `v-if / v-else-if` 分叉统一渲染三类卡片，不额外创建新的全局渲染通道。

| toolName | 渲染组件 |
|----------|----------|
| `plan_create` | `DecisionCard` |
| `todo_write` | `TodoCard` |
| `approval_request` | `ApprovalBanner` |

**Rationale**:

- 卡片必须紧贴触发它的上下文消息，不能飘到别处。
- 复用现有消息循环最小改动，降低 UI 回归风险。
- 方便后续新卡片类型继续按分叉链增量追加。
- 符合 UX 文档“Chat 是唯一行动中心”的原则。

**Affects**: `ChatView.vue`, 三个卡片组件。

### ADR-6: 配置 Schema 扩展边界

**Decision**: 配置合同扩展为两个逻辑节：`self_evolution` 与 `sandbox`。本分支前端实现界面与 TS 接口；主分支补齐 config schema、默认值、读取与写回。

```json
{
  "self_evolution": {
    "enabled": false,
    "autodream_frequency": "weekly",
    "trigger_threshold_sessions": 10,
    "trigger_threshold_messages": 100,
    "auto_merge_confidence": 0.95,
    "require_confirmation_for": [
      "identity",
      "relationship",
      "commitment",
      "sop",
      "deprecation"
    ]
  },
  "sandbox": {
    "mode": "WorkspaceWrite",
    "approval_policy": "OnRequest",
    "network_access": false,
    "writable_roots": ["/tmp/agent-diva"],
    "protected_paths": ["~/Documents", "~/.ssh"],
    "deny_patterns": ["rm -rf /", "dd if="],
    "timeout_seconds": 30
  }
}
```

**Rationale**:

- UI 必须围绕稳定 schema 建立，不能让组件自己拼配置结构。
- Self Evolution 与 Sandbox 都是 Settings 页面上的治理配置，但语义不同，必须分节。
- 缺失 key 时应由后端补默认值，避免破坏旧配置。

**Affects**: `SelfEvolutionSettings.vue`, `SandboxSettingsSection.vue`, `MemoryChangelog.vue`, 主分支 core schema 与 manager 配置接口。

---

## Decision Impact Analysis

### Implementation Sequence（前端）

1. `api/desktop.ts`：定义所有 TS 合同接口。  
2. `ChatView.vue`：插入统一卡片渲染入口。  
3. `DecisionCard.vue` / `TodoCard.vue` / `ApprovalBanner.vue`：完成三类对话内联组件。  
4. `NotebookView.vue`：建立报告浏览与治理动作界面。  
5. `SelfEvolutionSettings.vue` + `SandboxSettingsSection.vue` + `MemoryChangelog.vue`：完成治理设置页面。  
6. `useEventStream.ts`：先 mock，后切真实 SSE。  
7. `NormalMode.vue` / `SettingsView.vue` / `locales/*.ts`：补导航、路由与 i18n 收尾。

### Cross-Decision Dependencies

- **ADR-1 是前置**：没有统一 DTO，后续卡片与页面接口无法稳定。  
- **ADR-2 依赖主分支合同**：前端能先做交互，但最终持久化必须切后端。  
- **ADR-3 + ADR-4 共同构成实时审批链路**。  
- **ADR-6 是 Settings 全部表单绑定的基础**。  
- **ADR-5 把交互入口集中**，避免多个 Agent 在不同组件里各自塞 UI。

### Risk Summary

| 风险 | 等级 | 缓解策略 |
|------|------|----------|
| GUI 先行、后端未就绪 | 中 | 所有 API/SSE 先定义成稳定 contract，前端使用 mock/stub 开发 |
| Chat 卡片插入破坏现有布局 | 中 | 严格限制只在 tool 消息区插入，保留其他消息渲染逻辑 |
| 设置页面复杂度增加 | 低 | Self Evolution 单页分组；Sandbox 作为分段嵌入，不新增独立 SettingsView 子体系 |
| 持久化行为不一致 | 中 | 明确 manager 为单一真相源，本地状态仅过渡 |
| 主题回归 | 低 | 所有新样式复用 CSS variables，不自定义新主题系统 |

---

## Implementation Patterns & Consistency Rules

**基础命名、目录结构、导入风格、Rust 错误处理与 Vue 组织方式均以 `project-context.md` 为准。** 本节只补充本轮需求新增的强制模式。

### Pattern 1: ChatView 卡片统一渲染模式

**Conflict point**: 不同 Agent 可能在不同位置插入卡片，导致消息流、间距、状态回写逻辑不一致。

**Rule**: 所有结构化交互卡片都必须经由 `ChatView.vue` 的 tool 消息区统一渲染。

```vue
<template v-for="msg in messages" :key="msg.id">
  <!-- 既有 user / assistant 渲染保持不动 -->

  <template v-if="msg.role === 'tool'">
    <DecisionCard
      v-if="msg.toolName === 'plan_create'"
      :card="parseCard(msg.content)"
      @action="handleCardAction"
    />

    <TodoCard
      v-else-if="msg.toolName === 'todo_write'"
      :card="parseCard(msg.content)"
      @check="handleTodoCheck"
    />

    <ApprovalBanner
      v-else-if="msg.toolName === 'approval_request'"
      :request="parseApproval(msg.content)"
      @respond="handleApprovalRespond"
    />

    <div v-else class="tool-output">{{ msg.content }}</div>
  </template>
</template>
```

**Enforcement**:

- 新卡片类型只能追加在该分叉链末尾；
- 禁止在组件树别处新增第二套卡片渲染入口；
- 事件统一由 `ChatView` 处理，再转发到 `api/desktop.ts`。

### Pattern 2: Settings 表单统一绑定模式

**Conflict point**: Settings 新字段若没有统一绑定方式，容易出现本地状态、invoke 参数、schema 命名各不一致。

**Rule**: 所有治理配置均通过 `api/desktop.ts` 中的 TS interface + `invoke()` 统一读写。

```ts
export interface SelfEvolutionConfig {
  enabled: boolean
  autodream_frequency: 'daily' | 'weekly' | 'manual'
  trigger_threshold_sessions: number
  trigger_threshold_messages: number
  auto_merge_confidence: number
  require_confirmation_for: string[]
}

export interface SandboxConfig {
  mode: 'DangerFullAccess' | 'ReadOnly' | 'WorkspaceWrite'
  approval_policy: 'Never' | 'OnFailure' | 'OnRequest' | 'UnlessTrusted'
  network_access: boolean
  writable_roots: string[]
  protected_paths: string[]
  deny_patterns: string[]
  timeout_seconds: number
}

const config = ref<SelfEvolutionConfig>(await invoke('get_self_evolution_config'))
await invoke('set_self_evolution_config', { config: config.value })
```

**Enforcement**:

- 先扩 interface，再写组件字段；
- 所有设置表单使用 `v-model` 绑定 typed state；
- 任何用户可见保存结果都必须有 toast 反馈；
- Sandbox 模式切换必须额外显示“需重启 gateway 生效”提醒条。

### Pattern 3: Mock-First Real-Time Development

**Conflict point**: 后端 SSE 未就绪时，前端若直接写死真实连接，开发会卡死；若完全不按真实事件结构做，又会导致后续集成返工。

**Rule**: `useEventStream.ts` 必须同时支持 mock 模式与生产模式，事件 shape 保持一致。

**Enforcement**:

- mock 模式下暴露 `mockEmit(eventType, data)`；
- 生产模式下使用 `EventSource`；
- 组件只关心统一的事件回调，不直接依赖 transport。

### All AI Agents MUST

1. 遵循 `project-context.md` 的命名、导入、组件组织规则。  
2. 新 Vue 组件统一使用 `<script setup lang="ts">`。  
3. 所有用户可见文本必须进入 `zh.ts` / `en.ts`，禁止硬编码。  
4. 所有颜色与圆角通过现有 CSS variables / 视觉 token 接入。  
5. 所有错误态必须可见：toast、占位、回退，不允许静默失败。  
6. GUI 不直接写 `MEMORY.md`、`.laputa/rhythm/` 或其他运行时文件。  
7. 本分支只做 GUI；任何后端实现工作都只写成合同或 TODO，不直接改 Rust 服务端代码。

---

## Project Structure & Boundaries

### Complete Project Directory Structure（本轮增量）

```text
agent-diva-pro/agent-diva-gui/src/
├── api/
│   └── desktop.ts                         [MOD] 新增 UiCard / TodoItem / ApprovalRequest /
│                                                 SelfEvolutionConfig / SandboxConfig 等接口
│
├── components/
│   ├── ChatView.vue                       [MOD] 插入卡片渲染分叉与审批事件接线
│   ├── NormalMode.vue                     [MOD] 侧边栏新增“记事本”入口
│   ├── SettingsView.vue                   [MOD] dashboard 新增 Self Evolution 卡片与子页路由
│   ├── DecisionCard.vue                   [NEW] FR-1
│   ├── TodoCard.vue                       [NEW] FR-2
│   ├── ApprovalBanner.vue                 [NEW] FR-3
│   ├── NotebookView.vue                   [NEW] FR-4
│   └── settings/
│       ├── SelfEvolutionSettings.vue      [NEW] FR-5
│       ├── SandboxSettingsSection.vue     [NEW] FR-7
│       └── MemoryChangelog.vue            [NEW] FR-5 子组件
│
├── utils/
│   └── useEventStream.ts                  [NEW] ADR-3 realtime composable
│
└── locales/
    ├── zh.ts                              [MOD] 新增卡片/记事本/设置/沙箱文案
    └── en.ts                              [MOD] 对应英文文案
```

### API Boundaries（合同，主分支提供）

| Endpoint / Event | Method | GUI Consumer | 用途 |
|------------------|--------|--------------|------|
| `/api/cards/{id}/action` | POST | `DecisionCard`, `ApprovalBanner` | 决策同意/拒绝、审批动作 |
| `/api/todos/{id}/check` | POST | `TodoCard` | 勾选/取消勾选 Todo 项 |
| `/api/approval/respond` | POST | `ApprovalBanner` | 提交审批结果 |
| `/api/notebook/reports` | GET | `NotebookView` | 读取日报/周报/月报列表 |
| `/api/notebook/report` | GET | `NotebookView` | 获取单份报告详情 |
| `/api/notebook/candidates` | GET | `NotebookView` | 获取 AutoDream 关联候选提案 |
| `/api/notebook/promote` | POST | `NotebookView` | 固化为 SOP / 技能 / 长期记忆 |
| `/api/memory/changelog` | GET | `MemoryChangelog` | 只读时间线 |
| `/api/self-evolution/config` | POST | `SelfEvolutionSettings` | 保存自进化设置 |
| `/api/sandbox/config` | POST | `SandboxSettingsSection` | 保存沙箱设置 |
| `notebook.new_report` | SSE | `useEventStream` → `ChatView` / `NotebookView` | 新报告通知 |
| `approval.required` | SSE | `useEventStream` → `ApprovalBanner` | 触发审批 |
| `approval.resolved` | SSE | `useEventStream` → `ApprovalBanner` | 审批结果同步 |

### Component Boundaries

```text
ChatView
├── DecisionCard (props: card, emits: action)
├── TodoCard (props: card, emits: check)
├── ApprovalBanner (props: request, emits: respond)
└── useEventStream (approval/notebook 事件分发)

NotebookView
├── 标签栏：日报 / 周报 / 月报
├── 左栏：报告列表（280px）
├── 右栏：Markdown 详情
└── 底部：治理动作栏（SOP / 技能 / 长期记忆）

SelfEvolutionSettings
├── 自进化控制分组
├── 学习策略分组
├── SandboxSettingsSection
└── MemoryChangelog
```

### Data Boundaries

- **GUI 只保存临时交互状态，不直接落地运行时文件。**  
- **治理变更与卡片状态最终归属 manager/API。**  
- **Memory changelog 为只读视图，无编辑入口。**  
- **Notebook 的本地文件来源是 `.laputa/rhythm/`，但 GUI 通过 API 读取，不直接操作该目录。**

### Requirements to Structure Mapping

| FR | 主文件 | 关键依赖 |
|----|--------|----------|
| FR-1 决策卡 | `DecisionCard.vue` | `api/desktop.ts` 中 `UiCard` 接口 |
| FR-2 Todo 卡 | `TodoCard.vue` | `TodoItem` 接口、状态回写 API |
| FR-3 即时审批 | `ApprovalBanner.vue` | `useEventStream.ts`、审批合同 |
| FR-4 记事本 | `NotebookView.vue` | 报告 API、Markdown 样式、治理动作确认弹窗 |
| FR-5 自进化设置 | `SelfEvolutionSettings.vue`, `MemoryChangelog.vue` | `SelfEvolutionConfig` |
| FR-7 沙箱设置 | `SandboxSettingsSection.vue` | `SandboxConfig` |
| FR-6 后端合同 | 主分支 manager/core/agent | 由本文 ADR 与 API 边界定义 |

### Integration Points

**Internal communication**:

```text
User Action → Vue emit → ChatView / Settings container → api/desktop.ts → invoke()/HTTP bridge
```

**External integrations**:

```text
manager SSE → useEventStream → ChatView / NotebookView
manager HTTP/API → desktop.ts typed functions → page/components
```

**Theme / i18n integration**:

- 所有新组件复用现有 CSS variables 与 Tailwind 体系；
- 所有新文本进入 `zh.ts` / `en.ts`；
- 风险颜色必须颜色 + 文字双重表达，满足可访问性底线。

---

## Architecture Validation Results

### Coherence Validation ✅

**Decision Compatibility**: 六个 ADR 互不冲突。GUI 先行、后端补合同的策略与“pro 分支只做 GUI”的约束一致；SSE（ADR-3）与审批状态机（ADR-4）互补；卡片合同（ADR-1）与统一渲染入口（ADR-5）闭环。

**Pattern Consistency**: Chat 卡片统一渲染模式直接支撑 ADR-5；Settings 统一绑定模式直接支撑 ADR-6；mock-first realtime 模式保证在主分支 SSE 未完成前，前端仍可独立推进。

**Structure Alignment**: 项目结构以既有 `agent-diva-gui` 壳为中心，新增文件数量可控，且全部落在已有前端目录约定中，不引入第二套页面系统或主题系统。

### Requirements Coverage Validation ✅

| FR / NFR | 架构支撑 |
|----------|----------|
| FR-1 决策卡 | ADR-1 + ADR-5 + DecisionCard 结构 |
| FR-2 Todo 卡 | ADR-1 + ADR-2 + TodoCard 结构 |
| FR-3 即时审批 | ADR-3 + ADR-4 + ApprovalBanner 结构 |
| FR-4 记事本 | NotebookView 结构 + SSE 通知合同 |
| FR-5 自进化设置 | ADR-6 + SelfEvolutionSettings + MemoryChangelog |
| FR-7 沙箱设置 | ADR-6 + SandboxSettingsSection |
| FR-6 后端 API | API Boundaries 章节作为主分支合同 |
| NFR-1 性能 | 单入口渲染、Notebook 双栏、分页 changelog、mock-first 减少集成阻塞 |
| NFR-2 可靠性 | 幂等审批、失败 toast、回退策略、错误占位 |
| NFR-3 安全 | GUI 不直接写运行时文件、治理动作需审批链、changelog 只读 |
| NFR-4 兼容性 | 既有主题、既有目录、既有 Settings / NormalMode / ChatView 壳层增量扩展 |

### Implementation Readiness Validation ✅

**Decision Completeness**: 六个关键决策均给出边界、理由和影响文件。  
**Structure Completeness**: 新增/修改文件、API 边界、组件边界、数据边界均已明确。  
**Pattern Completeness**: 已覆盖最容易引发冲突的三个领域：消息内联卡片、Settings 绑定、实时事件接入。  

### Gap Analysis Results

| 优先级 | Gap | 处理方式 |
|--------|-----|----------|
| Known | 主分支 SSE / API 未在本分支实现 | 前端先用 mock/stub，按本文合同开发 |
| Minor | Memory changelog 的分页与筛选细节未展开到更细交互层 | 实现阶段按现有列表分页模式补齐 |
| Minor | Notebook 候选提案 UI 细节可在实现前再局部微调 | 不阻塞主结构与合同建立 |

**无 Critical Gap。**

### Architecture Completeness Checklist

**Requirements Analysis**

- [x] Project context thoroughly analyzed
- [x] Scale and complexity assessed
- [x] Technical constraints identified
- [x] Cross-cutting concerns mapped

**Architectural Decisions**

- [x] Critical decisions documented with versions
- [x] Technology stack fully specified
- [x] Integration patterns defined
- [x] Performance considerations addressed

**Implementation Patterns**

- [x] Naming conventions established
- [x] Structure patterns defined
- [x] Communication patterns specified
- [x] Process patterns documented

**Project Structure**

- [x] Complete directory structure defined
- [x] Component boundaries established
- [x] Integration points mapped
- [x] Requirements to structure mapping complete

### Architecture Readiness Assessment

**Overall Status:** READY FOR IMPLEMENTATION

**Confidence Level:** high

**Key Strengths:**

- 完全遵守“pro 分支只做 GUI”的边界；
- Chat / Notebook / Settings 三个核心表面结构清晰；
- 后端合同足以支撑前后端并行；
- 交互卡片、实时事件、治理设置三条主线已打通到可执行粒度。

**Areas for Future Enhancement:**

- `useEventStream` 生产态的指数退避重连细节；
- 前端 E2E 测试基础设施；
- 记事本候选提案与回滚 UI 的二期扩展；
- 卡片渲染性能观测与可视化指标。

### Implementation Handoff

**AI Agent Guidelines:**

- 严格按本文 ADR 与 Pattern 实现；
- 先做 TS 接口，再做组件，再做路由/导航接线；
- 一切后端工作仅写合同/适配层，不直接修改 Rust 后端；
- 所有用户可见文案必须补齐中英文。

**First Implementation Priority:**

1. `agent-diva-gui/src/api/desktop.ts` — 新增全部合同接口。  
2. `ChatView.vue` + 三个卡片组件。  
3. `NotebookView.vue`。  
4. `SelfEvolutionSettings.vue` / `SandboxSettingsSection.vue` / `MemoryChangelog.vue`。  
5. `useEventStream.ts` 从 mock 切到真实 SSE。  

---

## Completion Note

本文档现在可作为：

- `agent-diva-pro` GUI 实施的直接规格；
- 主分支后端团队的前后端合同；
- 后续 Epics / Stories / 实施 Story 的单一架构依据。
