---
title: "Harness V1.2 GUI 最小化更新计划"
date: 2026-06-27
version: 1.2
status: draft
author: Sisyphus
parent: agent-diva-pro/docs/prds/prd-harness-v1.2/prd.md
---

# Harness V1.2 GUI 最小化更新计划

## 0. 设计哲学

**原则：没有 UI 就会不完整的功能，添加最小化 UI；已有功能的后端变更，复用现有 UI 版块。**

- **不新增独立页面**：所有配置项嵌入现有 Settings 子版块
- **不新增导航项**：不增加左侧 sidebar 或顶部 tab
- **复用现有组件**：checkbox、slider、select、input 直接使用现有样式
- **后端优先**：UI 只是配置的「显示器」和「编辑器」，不做复杂交互
- **增量暴露**：V1.2 新增的配置项，在现有对应版块中增加字段

---

## 1. GUI 现有结构

```
NormalMode.vue (主界面)
├── ChatView.vue (聊天区域)
├── SettingsView.vue (设置面板)
│   ├── SettingsDashboard.vue (概览)
│   ├── GeneralSettings.vue (通用)
│   ├── ProvidersSettings.vue (Provider)
│   ├── ChannelsSettings.vue (Channel)
│   ├── SkillsSettings.vue (Skill)
│   ├── McpSettings.vue (MCP)
│   ├── NetworkSettings.vue (网络)
│   ├── LanguageSettings.vue (语言)
│   ├── audit/ (审计日志)
│   └── AboutSettings.vue (关于)
├── CronTaskManagementView.vue (Cron 任务)
├── GatewayControlPanel.vue (Gateway 控制)
└── ConsoleView.vue (控制台)
```

---

## 2. V1.2 必须的最小化 UI

### 2.1 概览：需要 UI 暴露的功能

| Epic | Story | 现有 UI 版块 | 新增 UI 元素 | 必要性 |
|------|-------|-------------|-------------|--------|
| Epic 1 | E1-S1 Always skill injection 扫描 | SkillsSettings.vue | 安全配置：severity threshold + action (summary-only/block) | **必须** – 用户需要配置安全策略 |
| Epic 2 | E2-S3 Config 热重载 | GeneralSettings.vue | 热重载开关：enable/disable + 轮询间隔 | **必须** – 用户需要控制热重载行为 |
| Epic 2 | E2-C3 Cron HTTP API 鉴权 | CronTaskManagementView.vue | Cron 任务 owner 字段显示 + token 配置提示 | **必须** – 安全相关 |
| Epic 3 | E3-S9 Error 分类 | — | 无 UI 变更（纯后端） | 不需要 |
| Epic 6 | E6-S3 Token 账本 | ChatView.vue | 聊天界面显示 token 使用量（可选，最小化） | **可选** – 用户可见的闭环 |
| Epic 7 | E7-S4 工具集配置 | McpSettings.vue 或新增 Tools 区域 | 工具集选择：core/file/shell/web/browser/code | **必须** – 用户需要启用/禁用工具 |
| Epic 8 | E8-S8 MCP 热重载 | McpSettings.vue | MCP server 重新加载按钮 | **必须** – 用户需要手动触发重载 |
| Epic 9 | E9-S1 debug bundle | GeneralSettings.vue 或 About | 导出 debug bundle 按钮 | **必须** – 用户需要故障排查 |
| Epic 9 | E9-S6 日志查询 API | audit/ 或新增 Logs 区域 | 日志查询：时间范围 + 事件类型过滤 | **必须** – 可观测性的用户入口 |
| Epic 10 | E10-S6 Guardian 模式 | GeneralSettings.vue | Guardian 模式选择：strict/balanced/permissive | **必须** – 用户需要配置安全级别 |
| Epic 10 | E10-S11 Heartbeat 节律 | GeneralSettings.vue | Heartbeat 配置：启用/禁用 + 节律模式 | **可选** – 高级用户 |
| Epic 5 | E5-S1 ChannelAuthPolicy | ChannelsSettings.vue | Channel 安全：allowlist 管理 + 默认 deny 提示 | **必须** – 安全策略配置 |

### 2.2 不做 UI 的功能

以下功能**不需要** GUI，保持纯后端实现：

| 功能 | 原因 |
|------|------|
| Error 分类系统 (E10-S8~S10) | 后端错误处理，用户不可见 |
| Retry 策略 (E3-S3, E10-S9) | 后端自动行为，无需配置 |
| Token 账本内部计算 (E6-S3) | 后端统计，可选暴露 |
| Provider fallback (E3-S10) | 后端自动，无需用户干预 |
| Compaction (E6-S2) | 后端自动行为 |
| MCP 并发锁修复 (E8-S2) | 纯后端 |
| 僵尸进程修复 (E8-S3) | 纯后端 |
| 结构化日志完善 (E9-S4) | 后端日志格式 |
| 审计日志持久化 (E9-S5) | 后端存储 |
| Rate limiter (E10-S5) | 后端限流 |
| 工具超时 (E10-S3) | 后端默认 60s，无需配置 |
| RuntimeTodo (Epic 11) | V1.2 无独立 Todo 页面 |
| SupervisedRun (Epic 12) | V1.2 无独立任务管理页面 |
| Workspace 切换 (Epic 13) | V1.2 仅 CLI；GUI 延后 |

---

## 3. 具体 UI 变更

### 3.1 SkillsSettings.vue — Skill 安全配置

**位置：** 现有 SkillsSettings.vue 内新增「安全策略」区块

**新增元素：**

```vue
<!-- 在 SkillsSettings.vue 内 -->
<SettingsSection title="Skill Security">
  <SettingsRow>
    <label>Injection Scan Threshold</label>
    <select v-model="skillSecurity.scanThreshold">
      <option value="low">Low</option>
      <option value="medium">Medium</option>
      <option value="high">High</option>
    </select>
  </SettingsRow>

  <SettingsRow>
    <label>High Severity Action</label>
    <select v-model="skillSecurity.highSeverityAction">
      <option value="summary-only">Summary Only</option>
      <option value="block">Block</option>
    </select>
  </SettingsRow>

  <SettingsRow>
    <label>Trust Workspace Skills</label>
    <input type="checkbox" v-model="skillSecurity.trustWorkspace" />
  </SettingsRow>
</SettingsSection>
```

**对应后端：** Epic 1 E1-S1, E1-S3

---

### 3.2 GeneralSettings.vue — 热重载 + Guardian + Heartbeat + Debug Bundle

**位置：** 现有 GeneralSettings.vue 内新增区块

#### 3.2.1 Config 热重载

```vue
<SettingsSection title="Config Reload">
  <SettingsRow>
    <label>Enable Hot Reload</label>
    <input type="checkbox" v-model="configReload.enabled" />
  </SettingsRow>

  <SettingsRow v-if="configReload.enabled">
    <label>Poll Interval (seconds)</label>
    <input type="number" v-model="configReload.interval" min="1" max="60" />
  </SettingsRow>
</SettingsSection>
```

**对应后端：** Epic 2 E2-S3, E2-S4

#### 3.2.2 Guardian 安全模式

```vue
<SettingsSection title="Guardian">
  <SettingsRow>
    <label>Approval Mode</label>
    <select v-model="guardian.mode">
      <option value="strict">Strict (Always Ask)</option>
      <option value="balanced">Balanced (Known Safe Auto-Approve)</option>
      <option value="permissive">Permissive (Auto-Approve Unless Dangerous)</option>
    </select>
  </SettingsRow>

  <SettingsRow>
    <label>Auto-Approve Known Safe</label>
    <input type="checkbox" v-model="guardian.autoApproveKnownSafe" />
  </SettingsRow>
</SettingsSection>
```

**对应后端：** Epic 10 E10-S6, E10-S7

#### 3.2.3 Heartbeat 配置（可选，高级）

```vue
<SettingsSection title="Heartbeat" v-if="showAdvanced">
  <SettingsRow>
    <label>Enable Heartbeat</label>
    <input type="checkbox" v-model="heartbeat.enabled" />
  </SettingsRow>

  <SettingsRow>
    <label>Adaptive Rhythm</label>
    <input type="checkbox" v-model="heartbeat.adaptive" />
    <small>Adjust cadence based on presence state</small>
  </SettingsRow>
</SettingsSection>
```

**对应后端：** Epic 10 E10-S11~S13

#### 3.2.4 Debug Bundle 导出

```vue
<SettingsSection title="Diagnostics">
  <SettingsRow>
    <button @click="exportDebugBundle">Export Debug Bundle</button>
    <small>Export logs, config, and system state for troubleshooting</small>
  </SettingsRow>
</SettingsSection>
```

**对应后端：** Epic 9 E9-S1

---

### 3.3 McpSettings.vue — 工具集配置 + MCP 重载

**位置：** 现有 McpSettings.vue 内

#### 3.3.1 工具集配置

```vue
<SettingsSection title="Tool Sets">
  <SettingsRow>
    <label>Enabled Tool Sets</label>
    <div class="toolset-checkboxes">
      <label><input type="checkbox" v-model="toolSets.core" /> Core</label>
      <label><input type="checkbox" v-model="toolSets.file" /> File</label>
      <label><input type="checkbox" v-model="toolSets.shell" /> Shell</label>
      <label><input type="checkbox" v-model="toolSets.web" /> Web</label>
      <label><input type="checkbox" v-model="toolSets.browser" /> Browser</label>
      <label><input type="checkbox" v-model="toolSets.code" /> Code</label>
    </div>
  </SettingsRow>
</SettingsSection>
```

**对应后端：** Epic 7 E7-S4

#### 3.3.2 MCP 重载按钮

```vue
<SettingsSection title="MCP Server Management">
  <SettingsRow>
    <button @click="reloadMcpServers">Reload MCP Servers</button>
    <button @click="reloadMcpConfig">Reload MCP Config</button>
  </SettingsRow>
</SettingsSection>
```

**对应后端：** Epic 8 E8-S8

---

### 3.4 ChannelsSettings.vue — Channel 安全策略

**位置：** 现有 ChannelsSettings.vue 内所有 channel 配置区域

```vue
<!-- 在每个 Channel 配置卡片内 -->
<SettingsSection title="Security">
  <SettingsRow>
    <label>Allowlist</label>
    <textarea v-model="channel.allowlist" placeholder="One ID per line, empty = deny all" />
    <small>Leave empty to deny all (default deny)</small>
  </SettingsRow>

  <SettingsRow>
    <label>Auth Policy</label>
    <select v-model="channel.authPolicy">
      <option value="deny">Deny All</option>
      <option value="allowlist">Allowlist Only</option>
      <option value="permissive">Permissive</option>
    </select>
  </SettingsRow>
</SettingsSection>
```

**对应后端：** Epic 5 E5-S1

---

### 3.5 CronTaskManagementView.vue — Cron 鉴权提示

**位置：** 现有 CronTaskManagementView.vue 内新增提示区域

```vue
<!-- 在 Cron 任务列表上方 -->
<Alert v-if="!cronConfig.hasToken" type="warning">
  Cron HTTP API requires a gateway token. Configure in General Settings.
</Alert>

<!-- 在任务详情显示 owner -->
<SettingsRow>
  <label>Owner</label>
  <span>{{ task.owner || 'System' }}</span>
</SettingsRow>
```

**对应后端：** Epic 2 E2-C3

---

### 3.6 ChatView.vue — Token 使用量显示（可选）

**位置：** 现有 ChatView.vue 内气泡下方或输入框上方

```vue
<!-- 在输入框上方新增最小化显示 -->
<div class="token-usage-bar" v-if="showTokenUsage">
  <span>Session: {{ sessionTokens.total }} tokens</span>
  <span v-if="lastTurnTokens">Last turn: {{ lastTurnTokens }}</span>
</div>
```

**对应后端：** Epic 6 E6-S3

**注意：** 此功能为可选；若实现成本较高，可推迟至 V1.3。

---

### 3.7 audit/ — 日志查询界面（最小化）

**位置：** 复用现有 `audit/` 区域

**方案 A（推荐）：扩展 audit 区域**

在现有 `audit/` 审计日志界面增加过滤选项：

```vue
<!-- 在 audit 视图内新增过滤 -->
<SettingsRow>
  <label>Event Type</label>
  <select v-model="logFilter.eventType">
    <option value="all">All</option>
    <option value="security">Security</option>
    <option value="channel">Channel</option>
    <option value="cron">Cron</option>
    <option value="skill">Skill</option>
    <option value="tool">Tool</option>
    <option value="mcp">MCP</option>
  </select>
</SettingsRow>

<SettingsRow>
  <label>Time Range</label>
  <select v-model="logFilter.timeRange">
    <option value="1h">Last Hour</option>
    <option value="24h">Last 24 Hours</option>
    <option value="7d">Last 7 Days</option>
  </select>
</SettingsRow>
```

**方案 B（若 audit 区域不适合）：在 GeneralSettings.vue 内新增「Logs」区块**

**对应后端：** Epic 9 E9-S6

---

## 4. 后端 API 需求

为支持上述 UI，后端需新增以下 API：

| API | 方法 | 用途 | 对应 UI |
|-----|------|------|---------|
| `/api/config/skill-security` | GET/PUT | Skill 安全配置 | SkillsSettings.vue |
| `/api/config/reload` | GET/PUT | 热重载配置 | GeneralSettings.vue |
| `/api/config/guardian` | GET/PUT | Guardian 模式 | GeneralSettings.vue |
| `/api/config/heartbeat` | GET/PUT | Heartbeat 配置 | GeneralSettings.vue |
| `/api/debug/bundle` | POST | 导出 debug bundle | GeneralSettings.vue |
| `/api/tools/sets` | GET/PUT | 工具集配置 | McpSettings.vue |
| `/api/mcp/reload` | POST | 重载 MCP servers | McpSettings.vue |
| `/api/channels/{id}/security` | GET/PUT | Channel 安全策略 | ChannelsSettings.vue |
| `/api/cron/config` | GET | Cron 鉴权状态 | CronTaskManagementView.vue |
| `/api/logs/query` | GET | 日志查询 | audit/ 或 GeneralSettings.vue |
| `/api/session/tokens` | GET | Session token 使用量 | ChatView.vue（可选） |

---

## 5. 实施顺序

### Phase 1: 安全相关（必须）

1. Guardian 配置 UI (E10-S6)
2. Channel 安全策略 UI (E5-S1)
3. Skill 安全配置 UI (E1-S1)
4. Cron 鉴权提示 (E2-C3)

### Phase 2: 配置相关（必须）

5. 热重载开关 UI (E2-S3)
6. 工具集配置 UI (E7-S4)
7. MCP 重载按钮 (E8-S8)

### Phase 3: 可观测性（必须）

8. Debug bundle 导出按钮 (E9-S1)
9. 日志查询 UI (E9-S6)

### Phase 4: 可选

10. Token 使用量显示 (E6-S3)
11. Heartbeat 配置 UI (E10-S11)

---

## 6. 验收标准

| ID | 验收项 | 验证方法 |
|----|--------|----------|
| GUI-01 | Guardian 模式可在 Settings 中切换 | 手动测试 |
| GUI-02 | Channel allowlist 可在 ChannelsSettings 中编辑 | 手动测试 |
| GUI-03 | Skill 扫描 threshold 可在 SkillsSettings 中配置 | 手动测试 |
| GUI-04 | 热重载开关可在 GeneralSettings 中切换 | 手动测试 |
| GUI-05 | 工具集可在 McpSettings 中启用/禁用 | 手动测试 |
| GUI-06 | MCP 重载按钮可触发服务器重载 | 手动测试 |
| GUI-07 | Debug bundle 导出按钮可下载诊断包 | 手动测试 |
| GUI-08 | 日志查询可按事件类型和时间过滤 | 手动测试 |
| GUI-09 | 所有新增 UI 元素使用现有组件样式 | 代码审查 |
| GUI-10 | 不新增 sidebar/tab 导航项 | 代码审查 |

---

## 7. 不做项

| 项 | 原因 |
|----|------|
| 独立审计页面 | 复用现有 audit/ 日志能力 |
| 独立 MCP 管理页面 | 复用现有 McpSettings.vue |
| Provider 监控页面 | 后端指标，无需 UI |
| Channel 统计页面 | 后端指标，无需 UI |
| 复杂可视化图表 | 最小化原则 |
| 实时日志流 | 后端功能，UI 仅查询 |
| 用户权限管理界面 | 单用户系统，无需 RBAC UI |
| Todo 管理页面 | Epic 11 延后 GUI |
| 后台任务管理页面 | Epic 12 延后 GUI |
| Workspace 切换 UI | Epic 13 延后 GUI |

---

## 8. 与主 PRD 的关系

| 本 PRD 章节 | 主 PRD Epic/Story | 说明 |
|------------|-------------------|------|
| 3.1 | Epic 1 E1-S1, E1-S3 | Skill 安全 |
| 3.2.1 | Epic 2 E2-S3, E2-S4 | Config 热重载 |
| 3.2.2 | Epic 10 E10-S6, E10-S7 | Guardian 模式 |
| 3.2.3 | Epic 10 E10-S11~S13 | Heartbeat |
| 3.2.4 | Epic 9 E9-S1 | Debug bundle |
| 3.3.1 | Epic 7 E7-S4 | 工具集配置 |
| 3.3.2 | Epic 8 E8-S8 | MCP 热重载 |
| 3.4 | Epic 5 E5-S1 | Channel 安全 |
| 3.5 | Epic 2 E2-C3 | Cron 鉴权 |
| 3.6 | Epic 6 E6-S3 | Token 账本（可选） |
| 3.7 | Epic 9 E9-S6 | 日志查询 |

---

## 9. 估算

| 阶段 | 内容 | 估算人天 |
|------|------|----------|
| Phase 1 | 安全相关 UI | 2–3 |
| Phase 2 | 配置相关 UI | 2–3 |
| Phase 3 | 可观测性 UI | 2–3 |
| Phase 4 | 可选 UI | 1–2 |
| API 实现 | 后端 API | 3–5 |
| 测试 | GUI smoke test | 1–2 |
| **合计** | | **11–18 人天** |

---

*本 PRD 为 Harness V1.2 主 PRD 的附属文档，所有范围约束与主 PRD 一致。*
