# Agent Diva Pro — UI/UX 审计报告

审计日期: 2026-06-02
审计范围: `../agent-diva-pro/` 项目 GUI (Tauri + Vue.js) 及后端 Rust 代码

---

## 1. 沙箱审批 UI

### 1.1 审批弹窗/对话框

| 状态 | 项目 | 说明 | 可复用度 |
|------|------|------|----------|
| ⚠️ | 通用对话框系统 | `appDialog.ts` + `AppDialogLayer.vue` 提供了 `appConfirm()` / `appAlert()` 两个通用弹窗。但仅用于会话删除确认(`chat.confirmDeleteSession`)和数据擦除确认，**未用于工具执行审批** | 高 — 框架现成，扩展为工具审批弹窗成本低 |
| ❌ | 工具执行审批弹窗 | 不存在。Shell/文件系统工具执行时无任何用户可见的确认弹窗 | — |

### 1.2 审批流程

| 状态 | 项目 | 文件路径 | 说明 | 可复用度 |
|------|------|----------|------|----------|
| ⚠️ | 权限模式选择器 | `ChatView.vue:637-664` | 下拉菜单提供 3 种模式: `cautious`(全部确认)、`smart`(低风险自动)、`trusted`(仅高风险确认)。有 i18n 支持(`en.ts:96-100`) | 高 — UI 组件完整 |
| ❌ | 权限模式 → 后端联动 | — | `permissionMode` 仅是前端 ref 变量，**未通过 invoke/IPC 传递给后端 agent loop**。后端 `ExecTool` 的 `guard_command()` 是硬编码的 deny/allow patterns，不响应前端权限模式 | — |
| ❌ | CLI 交互式审批 | — | CLI (`chat_commands.rs`, `main.rs`) 无 tool-execution 前的交互式确认。工具执行直接返回结果或 error 字符串 | — |

### 1.3 Yes/No/Session 三级决策

| 状态 | 项目 | 说明 | 可复用度 |
|------|------|------|----------|
| ❌ | Yes/No/Session 三级决策 | 完全不存在。无 "本次允许" / "本次拒绝" / "本会话始终允许" 的选项 | — |
| ❌ | 粘性授权/审批缓存 | 不存在。无任何 session-level 或 persistent 的授权缓存机制 | — |

### 1.4 沙箱安全机制（后端）

| 状态 | 项目 | 文件路径 | 说明 | 可复用度 |
|------|------|----------|------|----------|
| ✅ | Shell 命令 deny patterns | `shell.rs:85-98` | 8 条硬编码正则: `rm -rf`, `del /f`, `format`, `dd`, `shutdown`, fork bomb 等 | 中 — 可作为规则引擎基础 |
| ✅ | allow patterns 框架 | `shell.rs:116-119` | 支持 allowlist 模式(空=全部允许，非空=仅允许匹配项) | 高 |
| ✅ | 工作区路径限制 | `shell.rs:124-151`, `filesystem.rs:10-28` | `restrict_to_workspace` 开关，阻止路径遍历和工作区外访问 | 高 |
| ❌ | 可配置的规则编辑 | — | deny_patterns 通过 `default_deny_patterns()` 硬编码，**无法通过配置文件或 UI 修改** | — |

---

## 2. 沙箱配置页面

### 2.1 独立沙箱设置页

| 状态 | 项目 | 文件路径 | 说明 | 可复用度 |
|------|------|----------|------|----------|
| ❌ | 独立沙箱设置页 | — | SettingsView 的 9 个子页面为: General, MCP, Skills, Providers, Channels, Network, Language, About, Theme。**无 Sandbox/Security/Tool Policy 页面** | — |
| ✅ | 设置页导航框架 | `SettingsView.vue`, `SettingsDashboard.vue` | 标准化的 dashboard → subview 导航模式，新增页面只需添加 `SettingsSubview` 类型和对应组件 | 高 |

### 2.2 配置编辑方式

| 状态 | 项目 | 文件路径 | 说明 | 可复用度 |
|------|------|----------|------|----------|
| ✅ | 原始 JSON 编辑器 | `ConfigEditor.vue` | 支持 `config.json` 全文编辑，有 JSON 实时校验(valid/invalid 状态提示)、保存、重载功能 | 高 |
| ❌ | 表单化规则编辑 | — | `restrict_to_workspace` 是 config.json 中的布尔字段(`schema.rs:913`)，但只能通过原始 JSON 编辑器修改，**无表单化 UI** | — |
| ❌ | Allow/Prompt/Forbidden 三元组编辑 | — | 不存在三元组概念。后端只有 deny(硬编码) + allow_patterns(配置级，但 UI 不可见) + restrict_to_workspace(布尔)。无 "Prompt" 中间态 | — |

### 2.3 配置生效方式

| 状态 | 项目 | 文件路径 | 说明 | 可复用度 |
|------|------|----------|------|----------|
| ⚠️ | 配置保存后重启 | `ConsoleView.vue:101-117` | 保存 config.json 后调用 `saveRawConfig()` → `loadRawConfig()` 重新读取，但**不自动重启 gateway**。需手动 stop/start gateway | — |
| ⚠️ | Network/Web 设置即时生效 | `NetworkSettings.vue` | 网络搜索配置有独立保存按钮，保存后写入 config，但同样不触发运行时热加载 | — |

---

## 3. 上下文窗口健康

### 3.1 Token 使用量显示

| 状态 | 项目 | 文件路径 | 说明 | 可复用度 |
|------|------|----------|------|----------|
| ✅ | Token 统计面板 | `TokenStatsPanel.vue` | 完整的 token 统计面板，包含: 总 token 数、输入/输出 token、缓存 token、估算费用、按模型/端点/会话分组统计、时间线图表、导出 JSON | 高 |
| ✅ | 后端统计 API | `token_stats.rs`, `usage/` 模块 | 6 个 API 端点: total, summary, timeline, sessions, models, realtime。支持 1d/3d/1w/1m/6m/1y 时间范围 | 高 |
| ✅ | Tauri IPC 桥接 | `tokenStats.ts` | 7 个 `invoke()` 调用，类型完整的 TypeScript 接口定义 | 高 |
| ✅ | 实时内存统计 | `InMemoryStats` | 当前会话的实时 token 计数(request_count, total_tokens, cost) | 高 |

### 3.2 上下文使用率指示器

| 状态 | 项目 | 文件路径 | 说明 | 可复用度 |
|------|------|----------|------|----------|
| ⚠️ | 使用率进度条 | `TokenStatsPanel.vue:291-306` | 进度条存在，但 **budget 硬编码为 200K** (`getUsagePercentage()` L100-104)，不反映实际模型的 context window 大小 | 中 — UI 组件可用，需接入真实 budget |
| ❌ | 实时上下文压力指示 | — | 聊天界面(ChatView)中**无任何 context 使用率指示**。TokenStatsPanel 在 ConsoleView 中，不在聊天流旁边 | — |
| ⚠️ | TokenBudget 模块 | `budget.rs` | 后端有完整的 budget 系统: `warning_threshold`(默认 0.8)、`should_warn()`、`is_exceeded()`、`get_warning_message()`。支持 `+500k`/`+1m` 指令解析 | 高 |

### 3.3 阈值告警机制

| 状态 | 项目 | 文件路径 | 说明 | 可复用度 |
|------|------|----------|------|----------|
| ⚠️ | 后端告警消息生成 | `budget.rs:76-86` | `get_warning_message()` 返回 `[TOKEN BUDGET WARNING] 80% used` 或 `[TOKEN BUDGET EXCEEDED] Please wrap up immediately`。但仅作为字符串生成，**未确认是否注入到 agent prompt 中** | 中 |
| ❌ | GUI 告警通知 | — | 聊谈界面无 toast/banner/push 通知。`AppToastLayer.vue` 存在但未用于 token 告警 | — |
| ❌ | CLI 状态栏 | — | CLI 无状态栏。`chat_commands.rs` 中无 token 使用量的实时显示 | — |

### 3.4 显示位置总结

| 位置 | 状态 | 说明 |
|------|------|------|
| GUI Console → TokenStatsPanel | ✅ | 完整统计，但需要用户主动导航到 Console 页面 |
| GUI ChatView 侧边栏 | ❌ | 无。聊天时看不到 token 用量 |
| CLI 状态栏 | ❌ | 无状态栏 |
| CLI 聊天内联显示 | ❌ | 无 |

---

## 可复用资产汇总

### 高复用度（可直接复用或小幅改造）

| 资产 | 路径 | 用途 |
|------|------|------|
| `appDialog.ts` + `AppDialogLayer.vue` | `gui/src/utils/`, `gui/src/components/` | 扩展为工具审批弹窗 |
| 权限模式选择器 UI | `ChatView.vue:257-261, 637-664` | cautious/smart/trusted 三级模式下拉 |
| TokenStatsPanel 完整组件 | `gui/src/components/console/TokenStatsPanel.vue` | token 统计面板 |
| TokenBudget 后端模块 | `agent-diva-core/src/usage/budget.rs` | budget 计算、告警、指令解析 |
| ConfigEditor 原始编辑器 | `gui/src/components/console/ConfigEditor.vue` | JSON 编辑+校验 |
| SettingsView 导航框架 | `gui/src/components/SettingsView.vue` | 新增设置子页面 |
| deny/allow patterns 框架 | `agent-diva-tools/src/shell.rs` | 可配置化规则引擎基础 |
| restrict_to_workspace | `agent-diva-tools/src/shell.rs`, `filesystem.rs` | 路径限制机制 |

### 中复用度（需显著改造）

| 资产 | 路径 | 说明 |
|------|------|------|
| 使用率进度条 | `TokenStatsPanel.vue:291-306` | 需接入真实 model context window |
| 后端告警消息 | `budget.rs:76-86` | 需要注入到 agent prompt 并传递到 GUI |

---

## 关键差距（Gap Analysis）

1. **审批流断裂**: 前端有权限模式 UI，后端有安全守卫，但**中间没有连接**。前端 `permissionMode` 不传递给后端，后端工具执行无用户确认步骤。
2. **沙箱配置不可见**: `restrict_to_workspace`、`deny_patterns`、`allow_patterns` 等安全配置只能通过原始 JSON 编辑，无表单化 UI，且 deny_patterns 完全硬编码不可配置。
3. **上下文健康与聊天脱节**: Token 统计功能完整但仅在 Console 页面，聊天时用户无法感知上下文压力。后端 budget 告警未确认是否传递到聊天流。
4. **无三元组(Allow/Prompt/Forbidden)模型**: 当前只有 allow/deny 二元模型，缺少 "Prompt" 中间态（即需要用户确认的操作类别）。
