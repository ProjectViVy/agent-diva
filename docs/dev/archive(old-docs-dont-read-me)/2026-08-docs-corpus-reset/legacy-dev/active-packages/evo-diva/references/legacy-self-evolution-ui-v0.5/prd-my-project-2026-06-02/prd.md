---
title: "agent-diva-pro 自进化 + 沙箱 UI v0.5"
status: final
created: "2026-06-02"
updated: "2026-06-02"
project: agent-diva-pro
target_branch: agent-diva-pro
includes: ["自进化 UI", "沙箱 UI", "Plan/Todo 聊天内联"]
---

# agent-diva-pro 自进化 UI v0.5 — 产品需求文档

## 1. 产品愿景

为 agent-diva-pro 构建第一批面向用户的"自进化感知"界面。不是让用户看 AutoDream 有多智能，而是让用户清楚知道：**Diva 想改什么、依据是什么、风险多高、能不能撤回**。

核心原则：
- **Chat 是唯一行动中心** — 所有即时决策、Todo 跟踪在主聊天完成
- **记事本是节律治理入口** — 周期性报告 + 从报告触发的深层决策
- **治理链可见** — 提案 → 审批 → 应用 → 追踪 → 回滚，每一步都有 UI 承载
- **不静默** — 任何 durable change（长期记忆、身份、关系、承诺、SOP）必须可审阅、可追踪

## 2. 当前状态

### 2.1 agent-diva-pro 分支现状

| 资产 | 状态 | 说明 |
|------|------|------|
| agent-diva-gui (Tauri + Vue3) | ✅ 已有 | 9 页面：Chat / Pet / Settings / Console / Neuro / Cron + 子页面 |
| SettingsView 导航框架 | ✅ | dashboard→subview 模式，扩展开关 |
| appDialog / AppToast | ✅ | 通用对话框/通知系统 |
| TokenStatsPanel | ✅ | 完整统计面板 |
| TokenBudget 后端 | ✅ | 阈值计算、告警消息 |
| ConfigEditor | ✅ | JSON 实时编辑/校验/保存 |
| 计划模式骨架 | ⚠️ | pro 分支已有骨架代码 |
| 工具审批弹窗 | ❌ | 完全缺失 |
| 权限→后端联动 | ❌ | permissionMode ref 未通过 IPC 传递 |
| 独立沙箱设置页 | ❌ | SettingsView 9 子页面中无 Sandbox |
| 上下文健康指示器 | ❌ | Chat 界面零指示 |
| SandboxSettings.vue | ⚠️ | 沙箱分支已有，未合入 pro，可作为起点 |

### 2.2 参考文档

| 文档 | 角色 |
|------|------|
| `agent-diva-pro/docs/project-context.md` | 架构/命名/模式规范 |
| `agent-diva/docs/dev/awesomeagents/decisions.md` | 最终产品方向（第 9 节：UI 设计决策） |
| `agent-diva/docs/dev/awesomeagents/pro-ui-audit.md` | pro 分支可复用资产审计 |
| `agent-diva/docs/dev/genericagent/newedge/agent-diva-pro-self-evolution-ui-research.md` | 自进化 UI 完整调研 |
| `agent-diva/docs/dev/genericagent/newedge/ui-design.md` | Card 协议 + Journal 基线 |
| `agent-diva/docs/dev/genericagent/candidate-evidence-journal-audit-design.md` | 治理链规范 |
| `agent-diva-sandbox/docs/dev/sandbox-audit/agent-diva-sandbox-summary.md` | 沙箱审查总结 |
| `agent-diva-sandbox/docs/dev/sandbox-audit/agent-diva-sandbox-migration-plan.md` | 沙箱迁移计划 + GUI 切片 |

## 3. P0 范围

### 3.1 本期交付

P0 仅包含**自进化 UI**。Plan/Todo 降级为 Chat 内联卡片（复用 pro 分支已有骨架），不建独立 PlanningView 页面。

| # | 页面/表面 | 功能 | 形态 |
|---|----------|------|------|
| **P0-1** | Chat — 决策卡片 | Plan 模式生成决策卡（做什么、为什么、风险），用户同意/拒绝 | 增强现有 ChatView |
| **P0-2** | Chat — Todo 卡片 | 通用 Todo 卡片（非仅 Plan），逐项勾掉，适用日常对话各种场景 | 新增内联组件 |
| **P0-3** | Chat — 即时审批 | 会话中轻量确认（"Diva 想执行 X，允许吗？"），不跳转 | 内联按钮卡片 |
| **P0-4** | 记事本（新建） | 日报/周报/月报浏览，从报告触发的 SOP/技能提炼决策 | 新侧边栏页面 |
| **P0-5** | Settings — 自进化 | 自进化开关、触发策略、学习策略、Memory changelog 只读 | 扩展现有 SettingsView |
| **P0-6** | Settings — 沙箱 | 沙箱模式、审批策略、网络访问、保护路径、规则编辑 | 并入 Self Evolution 子页面，参考沙箱分支 SandboxSettings.vue |

### 3.2 本期不做（延后至 P1/P2）

| 项目 | 原因 |
|------|------|
| Plan/Todo 独立 PlanningView 页面 | 降级为 Chat 内联，pro 已有骨架 |
| Inbox 独立审批队列 | 合并入记事本 + Chat 即时审批 |
| 宏观记忆拓扑图 | 非节律报告范畴 |
| 沙箱规则表单化编辑 | 已有 ConfigEditor JSON 编辑可临时替代 |
| Kanban 看板 | 等 Plan 阶段结束后再议 |
| 回滚 UI | P2，先保证审批和应用链路完整 |
| 审计日志可视化 | P2，先保证 changelog 写入和只读 |
| 成本统计（金额） | decisions.md 已明确不做 |
| 沙箱 CLl 审批（一行交互） | GUI 审批卡片替代，CLI 审批不建独立通道 |
| sandbox-verification.md 中的 Phase B trace log | P2 安全增强，非 UI 阻塞 |

## 4. UI 信息架构

### 4.1 侧边栏（NormalMode.vue）

```
Chat          ← 主聊天（增强：决策卡 + Todo 卡 + 即时审批）
  ├─ 决策卡   （Plan 模式生成）
  ├─ Todo 卡  （通用，可勾掉）
  └─ 审批按钮 （即时确认）
记事本        ← 新建：日报/周报/月报 + 节律治理
Journal       ← 保留不动
Pet           ← 保留不动
Settings      ← 扩展：自进化设置子页面
  ├─ General
  ├─ Providers
  ├─ Channels
  ├─ Self Evolution  ← 新建子页面
  │   ├─ 自进化开关
  │   ├─ 触发策略
  │   ├─ Memory Changelog（只读）
  │   └─ 沙箱设置  ← 合并入此分段
  │       ├─ 沙箱模式（DangerFullAccess / ReadOnly / WorkspaceWrite）
  │       ├─ 审批策略（Never / OnFailure / OnRequest / UnlessTrusted）
  │       ├─ 网络访问控制
  │       ├─ 可写根目录
  │       ├─ 保护路径列表
  │       ├─ 拒绝模式（deny_patterns）
  │       └─ 超时设置
  └─ ...
Console       ← 保留
Neuro         ← 保留
Cron          ← 保留
```

### 4.2 Chat 内联卡片协议

所有卡片共用 `UiCard` 协议（源自 ui-design.md），扩展为三类：

```
UiCard {
  id, kind: "decision" | "todo" | "approval",
  status, title, summary, body_markdown,
  actions: UiCardAction[],   // 按钮
  evidence_refs: string[],   // 决策卡特有：证据链接
  risk_level: "low" | "medium" | "high",  // 决策卡特有
  todo_items: TodoItem[],    // Todo 卡特有
  created_at, updated_at
}

TodoItem {
  id, content, status: "pending" | "done",
  completed_at?
}

UiCardAction {
  id, label,
  style: "primary" | "secondary" | "danger" | "quiet",
  payload: string
}
```

### 4.3 记事本页面结构

```
NotebookView.vue
├─ 标签栏：日报 / 周报 / 月报
├─ 报告列表（左栏 280px）
│   └─ 每项：日期 + 标题 + AutoDream 摘要缩略
└─ 报告详情（右栏）
    ├─ 完整 Markdown 渲染
    ├─ 提炼区（底部操作栏）
    │   ├─ "固化为 SOP" 按钮
    │   ├─ "固化为技能" 按钮
    │   └─ "更新长期记忆" 按钮
    └─ 关联候选列表（AutoDream 生成的提案）
```

### 4.4 自进化设置子页面

```
SelfEvolutionSettings.vue
├─ 自进化总开关 (toggle)
├─ 触发策略
│   ├─ AutoDream 节律频率（每日/每周/手动）
│   └─ 触发阈值（会话数/消息数）
├─ 学习策略
│   ├─ 自动合并信任度阈值（0.0-1.0）
│   └─ 需强制确认的类型（身份/关系/承诺/SOP/deprecate）
├─ Memory Changelog（只读列表）
│   ├─ 变更时间线
│   ├─ 变更来源（AutoDream/手动/回滚）
│   └─ 变更内容摘要
```

## 5. 功能需求

### FR-1: 决策卡片

| ID | 需求 | 优先级 |
|----|------|--------|
| FR-1.1 | Plan 模式激活时，Diva 生成结构化决策卡（非纯文本） | P0 |
| FR-1.2 | 决策卡展示：标题、摘要、具体步骤列表、风险评估（low/medium/high） | P0 |
| FR-1.3 | 决策卡附带证据引用（对话片段、文件引用） | P0 |
| FR-1.4 | 用户可点击"同意执行"或"拒绝"，拒绝后可输入原因 | P0 |
| FR-1.5 | "同意执行"后自动生成 Todo 卡片 | P0 |
| FR-1.6 | 决策卡状态变更写入消息历史 | P0 |

### FR-2: Todo 卡片

| ID | 需求 | 优先级 |
|----|------|--------|
| FR-2.1 | Todo 卡片可出现在任何对话上下文中（非仅 Plan 模式） | P0 |
| FR-2.2 | Todo 项逐条显示，带复选框，可点击勾掉 | P0 |
| FR-2.3 | 已完成的 Todo 项显示删除线 + 完成时间 | P0 |
| FR-2.4 | Todo 卡片支持"全部完成"/"取消"操作 | P0 |
| FR-2.5 | Todo 状态变更实时同步到消息历史 | P0 |
| FR-2.6 | 对话结束后 Todo 卡片状态持久化（下次打开同 session 可见） | P0 |

### FR-3: 即时审批（Chat 内联）

| ID | 需求 | 优先级 |
|----|------|--------|
| FR-3.1 | 当 Diva 需要执行敏感操作时，生成审批卡片（非弹窗阻断） | P0 |
| FR-3.2 | 审批卡片显示：操作描述、影响范围、风险等级 | P0 |
| FR-3.3 | 用户点击"允许"/"拒绝"，不跳转页面 | P0 |
| FR-3.4 | 审批结果立即反馈给 agent loop | P0 |
| FR-3.5 | 超时未响应（5 分钟）自动拒绝 | P1 |

### FR-4: 记事本

| ID | 需求 | 优先级 |
|----|------|--------|
| FR-4.1 | 侧边栏新增"记事本"入口（Chat 右侧，Journal 左侧） | P0 |
| FR-4.2 | 三标签切换：日报/周报/月报 | P0 |
| FR-4.3 | 报告列表按时间倒序排列 | P0 |
| FR-4.4 | 每项报告显示：日期、标题、AutoDream 摘要（前 100 字） | P0 |
| FR-4.5 | 点击报告展开完整 Markdown 渲染 | P0 |
| FR-4.6 | 报告底部显示 AutoDream 生成的关联候选提案列表 | P0 |
| FR-4.7 | 用户可从报告提炼区执行：固化为 SOP、固化为技能、更新长期记忆 | P0 |
| FR-4.8 | 固化操作需二次确认弹窗（描述变更内容 + 影响范围） | P0 |
| FR-4.9 | Chat 中 Diva 可在节律触发时提示"记事本有新报告"，用户点击跳转 | P0 |
| FR-4.10 | 报告数据源：`.laputa/rhythm/` 下的日报/周报/月报 Markdown 文件 | P0 |
| FR-4.11 | 候选提案数据源：通过 manager API 获取 | P1 |

### FR-5: 自进化设置

| ID | 需求 | 优先级 |
|----|------|--------|
| FR-5.1 | Settings 下新增"Self Evolution"子页面 | P0 |
| FR-5.2 | 自进化总开关（布尔 toggle，默认关） | P0 |
| FR-5.3 | AutoDream 节律频率选择（每日/每周/手动） | P0 |
| FR-5.4 | 触发阈值设置（会话数/消息数，整数滑块） | P0 |
| FR-5.5 | 自动合并信任度阈值（0.0-1.0 滑块，默认 0.95） | P0 |
| FR-5.6 | 强制确认类型多选（身份/关系/承诺/SOP/deprecate） | P0 |
| FR-5.7 | Memory Changelog 只读列表（时间线、来源、内容摘要） | P0 |
| FR-5.8 | 设置变更通过 invoke 同步到后端 config.json | P0 |
| FR-5.9 | 设置变更后触发 gateway 热重载 | P1 |

### FR-6: 后端 API 扩展

| ID | 需求 | 优先级 |
|----|------|--------|
| FR-6.1 | 新增 `GET /api/notebook/reports?type=daily|weekly|monthly` | P0 |
| FR-6.2 | 新增 `GET /api/notebook/report?id=xxx` | P0 |
| FR-6.3 | 新增 `GET /api/notebook/candidates?report_id=xxx` | P1 |
| FR-6.4 | 新增 `POST /api/cards/{id}/action`（决策卡/审批卡动作） | P0 |
| FR-6.5 | 新增 `POST /api/todos/{id}/check`（Todo 项勾选） | P0 |
| FR-6.6 | 新增 `POST /api/notebook/promote`（固化为 SOP/技能/记忆） | P1 |
| FR-6.7 | 新增 `GET /api/memory/changelog`（Memory changelog 查询） | P0 |
| FR-6.8 | 新增 `POST /api/self-evolution/config`（自进化设置写回） | P0 |
| FR-6.9 | 新增 SSE 事件类型：`notebook.new_report`、`approval.required` | P1 |
| FR-6.10 | 新增 `POST /api/approval/respond`（审批响应） | P0 |

### FR-7: 沙箱设置

| ID | 需求 | 优先级 |
|----|------|--------|
| FR-7.1 | 沙箱模式选择器（DangerFullAccess / ReadOnly / WorkspaceWrite） | P0 |
| FR-7.2 | 审批策略选择器（Never / OnFailure / OnRequest / UnlessTrusted） | P0 |
| FR-7.3 | 网络访问开关 | P0 |
| FR-7.4 | 可写根目录列表（添加/编辑/删除） | P0 |
| FR-7.5 | 保护路径列表（添加/编辑/删除） | P0 |
| FR-7.6 | 拒绝模式列表（deny_patterns，可编辑文本） | P0 |
| FR-7.7 | 工具执行超时设置（秒，整数） | P0 |
| FR-7.8 | 设置变更通过 invoke 同步到 config.json → SandboxConfig | P0 |
| FR-7.9 | 参考沙箱分支 SandboxSettings.vue + sandbox.ts 设计 | P0 |
| FR-7.10 | 沙箱模式变更后提示"需重启 gateway 生效" | P1 |

## 6. 非功能需求

### NFR-1: 性能

| ID | 需求 |
|----|------|
| NFR-1.1 | Chat 内卡片渲染不阻塞消息流滚动，< 50ms 增量渲染 |
| NFR-1.2 | 记事本报告列表加载 < 500ms（报告文件 < 100KB） |
| NFR-1.3 | Memory changelog 列表分页加载，每页 20 条 |

### NFR-2: 可靠性

| ID | 需求 |
|----|------|
| NFR-2.1 | 审批操作幂等——重复点击同一审批按钮不产生重复副作用 |
| NFR-2.2 | Todo 状态变更写入失败时不静默丢失，展示 toast 错误 |
| NFR-2.3 | 记事本报告读取失败时展示占位错误提示，不白屏 |

### NFR-3: 安全

| ID | 需求 |
|----|------|
| NFR-3.1 | GUI 不直接写 MEMORY.md / .laputa 运行时文件——所有写操作通过 manager API |
| NFR-3.2 | 固化为 SOP/技能/记忆的操作写入前必须经过候选→审批链路 |
| NFR-3.3 | Memory changelog 只读，无 GUI 编辑入口 |

### NFR-4: 兼容性

| ID | 需求 |
|----|------|
| NFR-4.1 | 新增 UI 不破坏现有 Chat/Pet/Settings/Console/Neuro/Cron 页面 |
| NFR-4.2 | 新增 API 端点不破坏现有 manager API 路由 |
| NFR-4.3 | UI 组件遵循 agent-diva-pro 现有命名/导入/状态管理规范（project-context.md） |
| NFR-4.4 | i18n 同时覆盖 zh.ts 和 en.ts |

## 7. 用户核心旅程

### 旅程 1: Plan 模式 → Todo 跟踪

```
1. 用户在 Chat 中触发 Plan 模式
   → Diva 生成结构化决策卡：
     "我计划做以下事情：
      1. 创建 agent-diva-pro 的新 Vue 组件
      2. 注册到路由
      3. 编写单元测试
      风险：低 | 预计 3 步完成"
   → 用户点击 [同意执行]

2. 决策卡转为 Todo 卡：
   ☐ 创建 agent-diva-pro 的新 Vue 组件
   ☐ 注册到路由
   ☐ 编写单元测试

3. Diva 每完成一项，该项自动标记 ☑ 完成
   → 用户可见实时进度

4. 全部完成后，Todo 卡片显示"已完成"状态 + 折叠
```

### 旅程 2: 日常对话 Todo

```
1. 用户："帮我整理三个待办：改 README、更新 CHANGELOG、发 PR"
   → Diva 响应的同时生成 Todo 卡片：
     ☐ 改 README
     ☐ 更新 CHANGELOG
     ☐ 发 PR

2. 用户手动点击勾掉已完成项
   → Todo 卡片实时更新

3. 对话结束后，Todo 卡片状态随 session 持久化
```

### 旅程 3: 节律治理 — 记事本

```
1. Chat 中 Diva 提示：
   "📋 本周记事本已生成，点击查看本周报告"
   [查看记事本]

2. 用户点击跳转 → 记事本页面
   → 默认显示"周报"标签，本周报告已展开

3. 报告底部：
   "AutoDream 分析建议：
    - 你本周 3 次提到 Rust workspace 结构 → 建议固化为 SOP
    - 你使用了新的 deploy 命令序列 → 建议创建技能 'deploy-prod'"

4. 用户点击 [固化为 SOP] → 二次确认弹窗：
   "将以下内容固化为 SOP？
   'Rust workspace 结构：crate 按核心/agent/provider/channel/tool 分层'
   影响范围：agent-diva-core 的 SOP 目录"
   [确认] [取消]

5. 用户确认 → SOP 写入 → Memory changelog 追加记录
```

### 旅程 4: 即时审批

```
1. Chat 会话中，Diva 需要执行 delete 操作
   → 聊天流中出现审批卡片：
     "Diva 请求执行：删除 /tmp/test-project/
      影响范围：临时目录，不包含用户代码
      风险：低"
     [允许] [拒绝]

2. 用户点击 [允许] → 卡片变为"已批准"
   → Diva 继续执行

3. 用户点击 [拒绝] → 卡片变为"已拒绝"
   → Diva 收到拒绝信号，不执行
```

## 8. 验收标准

### AC-1: Chat 卡片

- [ ] 用户在 Plan 模式下可看到结构化决策卡（非纯文本 Markdown）
- [ ] 决策卡包含标题、步骤列表、风险等级、证据引用
- [ ] 用户点击"同意执行"后自动生成 Todo 卡片
- [ ] Todo 卡片可出现在任意对话中，不仅 Plan 模式
- [ ] Todo 项可逐条勾掉，已完成显示删除线
- [ ] 审批卡片可即时响应，不阻断消息流

### AC-2: 记事本

- [ ] 侧边栏可见"记事本"入口
- [ ] 日报/周报/月报三标签切换正常
- [ ] 报告列表按时间倒序显示
- [ ] 报告详情 Markdown 渲染正确
- [ ] 从报告可触发"固化为 SOP"/"固化为技能"操作
- [ ] 固化操作有二次确认
- [ ] Chat 中 Diva 可提示"记事本有新报告"并提供跳转

### AC-3: 自进化设置

- [ ] Settings 页面可见"Self Evolution"子页
- [ ] 自进化总开关可切换
- [ ] 触发策略/学习策略参数可调整
- [ ] Memory Changelog 列表正确显示变更记录
- [ ] 设置变更通过 IPC 同步到后端

### AC-5: 沙箱设置

- [ ] Settings 页面 Self Evolution 子页下可见沙箱设置分段
- [ ] 沙箱模式选择器可切换（DangerFullAccess / ReadOnly / WorkspaceWrite）
- [ ] 审批策略选择器可切换（Never / OnFailure / OnRequest / UnlessTrusted）
- [ ] 网络访问/可写根目录/保护路径/拒绝模式各项可编辑
- [ ] 设置变更通过 IPC 写入 config.json SandboxConfig 节
- [ ] 沙箱模式变更后提示重启提醒（非阻断）

### AC-6: 集成验证

- [ ] `just fmt-check` 通过
- [ ] `just check` 通过（clippy -D warnings）
- [ ] `just test` 通过
- [ ] GUI 构建成功（`agent-diva-gui` 编译 + 前端打包）
- [ ] 新增页面在 zh-CN 和 en 下 i18n 覆盖完整
- [ ] 新增 API 端点有对应的集成测试

## 9. 依赖

| 依赖 | 状态 | 说明 |
|------|------|------|
| agent-diva-pro 分支 workspace 稳定 | ✅ | 已有，可编译 |
| agent-diva-gui Tauri 框架 | ✅ | Vue3 + Composition API + Tailwind |
| manager API 路由扩展（server.rs） | ❌ | 需新增 ~10 个端点 |
| agent-diva-core 配置 schema 扩展 | ❌ | 需新增 self_evolution 配置节 |
| .laputa/rhythm/ 报告文件存在 | ⚠️ | 需确认后端 AutoDream 是否已产出报告 |
| agent-diva-agent loop 审批钩子 | ❌ | agent loop 需在工具执行前检查审批状态 |

## 10. 交付计划

> 由 PM 决策：4 周迭代，3 次增量交付。每次交付独立可验收。

### Sprint 1（第 1 周）：Chat 卡片基础

**目标**：Chat 页面内可看到三类卡片，可交互。

| 任务 | 范围 |
|------|------|
| 定义 `UiCard` / `TodoItem` TypeScript 接口 | `api/desktop.ts` |
| 新增后端 DTO + 序列化 | `agent-diva-core` |
| 实现 `DecisionCard.vue` | 决策卡组件 |
| 实现 `TodoCard.vue` | Todo 卡组件（含勾选逻辑） |
| 实现 `ApprovalBanner.vue` | 即时审批内联组件 |
| ChatView.vue 接入卡片渲染 | 在 tool 消息区插入卡片 |
| 新增 manager API：`POST /api/cards/{id}/action` | server.rs |
| 新增 manager API：`POST /api/todos/{id}/check` | server.rs |
| 新增 manager API：`POST /api/approval/respond` | server.rs |
| i18n 覆盖 zh-CN + en | locales/ |

**验收**：
- Plan 模式可生成决策卡 → 同意 → Todo 卡 → 逐项勾掉
- 日常对话 Diva 可主动生成 Todo 卡
- 审批卡片可即时允许/拒绝

### Sprint 2（第 2-3 周）：记事本 + 设置（自进化 + 沙箱）

**目标**：侧边栏新增记事本入口，Settings 新增自进化 + 沙箱子页。

| 任务 | 范围 |
|------|------|
| `NotebookView.vue` + 三标签 + 报告列表 + 详情 | 新建页面 |
| `SelfEvolutionSettings.vue` + 子组件 | 新建子页面 |
| `SandboxSettingsSection.vue`（沙箱分段） | 新建子组件，参考沙箱分支 SandboxSettings.vue |
| NormalMode.vue 侧边栏扩展 | 插入"记事本"入口 |
| SettingsView 路由扩展 | 新增 `self-evolution` 子页 |
| 新增 manager API：报告列表/详情/候选 | server.rs（3 端点） |
| 新增 manager API：changelog 查询 + 设置写回 | server.rs（2 端点） |
| 新增 `agent-diva-core` 配置节：`self_evolution` + `sandbox` | config schema |
| Chat 中"记事本有新报告"提示 + 跳转 | ChatView 联动 |
| i18n 覆盖 | locales/ |

**验收**：
- 记事本可浏览日报/周报/月报
- 从报告可触发 SOP/技能固化
- 自进化设置可调整并持久化
- Memory changelog 只读列表正常

### Sprint 3（第 4 周）：集成 + 打磨 + 验收

**目标**：全线联通，全量测试，发布就绪。

| 任务 | 范围 |
|------|------|
| 决策卡 → Todo 卡 → 记事本固化完整链路联调 | 端到端 |
| SSE 事件：`notebook.new_report`、`approval.required` | server.rs |
| agent loop 审批钩子 | agent-diva-agent |
| 异常场景处理：加载失败、审批超时、写入失败 | 所有组件 |
| 全量 i18n 审核 | zh-CN + en |
| `just ci` 通过 | fmt + clippy + test |
| GUI 构建验证 | `agent-diva-gui` 编译打包 |
| 迭代日志 | `docs/logs/2026-06-self-evolution-ui/v0.5.0-*/` |

**验收**：
- 三个核心旅程端到端通过
- 零 clippy warning
- 零测试回归
- GUI 构建产物可用

---

## 附录 A: 组件清单

| 组件 | 类型 | Sprint |
|------|------|--------|
| `DecisionCard.vue` | 新建 | S1 |
| `TodoCard.vue` | 新建 | S1 |
| `ApprovalBanner.vue` | 新建 | S1 |
| `NotebookView.vue` | 新建 | S2 |
| `SelfEvolutionSettings.vue` | 新建 | S2 |
| `SandboxSettingsSection.vue` | 新建 | S2 |
| `MemoryChangelog.vue` | 新建 | S2 |
| `ChatView.vue` | 修改 | S1 |
| `NormalMode.vue` | 修改 | S2 |
| `SettingsView.vue` | 修改 | S2 |

## 附录 B: API 端点清单

| 端点 | 方法 | Sprint | 用途 |
|------|------|--------|------|
| `/api/cards/{id}/action` | POST | S1 | 决策卡/审批卡动作 |
| `/api/todos/{id}/check` | POST | S1 | Todo 项勾选 |
| `/api/approval/respond` | POST | S1 | 审批响应 |
| `/api/notebook/reports` | GET | S2 | 报告列表 |
| `/api/notebook/report` | GET | S2 | 报告详情 |
| `/api/notebook/candidates` | GET | S2 | 关联候选提案 |
| `/api/notebook/promote` | POST | S2 | 固化操作 |
| `/api/memory/changelog` | GET | S2 | changelog 查询 |
| `/api/self-evolution/config` | POST | S2 | 设置写回 |
| `/api/sandbox/config` | POST | S2 | 沙箱设置写回 |
| SSE: `notebook.new_report` | event | S3 | 新报告推送 |
| SSE: `approval.required` | event | S3 | 审批请求推送 |
