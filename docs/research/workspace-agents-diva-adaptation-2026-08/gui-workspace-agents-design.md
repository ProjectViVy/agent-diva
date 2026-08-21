# GUI 适配设计：工作区选择与 AGENTS.md 状态

> 日期：2026-08-22
> 范围：Tauri/Vue 桌面端交互与运行时契约，不包含本轮生产实现

## 1. 设计目标

GUI 需要让用户明确回答两个问题：

1. **DIVA 现在在哪个项目目录里工作？**
2. **这个项目有没有被注入的 `AGENTS.md`？它来自哪里、是否被截断？**

工作区不是普通偏好设置，而是当前会话的运行边界。选择工作区会改变相对文件路径、Shell 默认目录、Session/Plan 存储、Subagent 和项目规则来源，因此不能做成静默保存的文本框。

AGENTS.md 是项目上下文，不是权限设置。GUI 可以展示来源和状态，但不能提供“让 AGENTS.md 获得更多权限”的开关，也不应把正文直接塞进普通设置表单。

## 2. 现有 GUI 入口与缺口

当前结构：

- `agent-diva-gui/src/components/NormalMode.vue`：主壳、侧栏、Topbar、Chat/Settings 切换；
- `agent-diva-gui/src/components/SettingsView.vue`：设置子页面路由；
- `agent-diva-gui/src/components/settings/SettingsDashboard.vue`：设置卡片入口；
- `agent-diva-gui/src/components/settings/GeneralSettings.vue`：运行时状态和解析路径展示；
- `agent-diva-gui/src/api/desktop.ts`：已有 `get_config_status()`，但 `StatusPathReport` 只有单个 workspace 字符串；
- `agent-diva-gui/src-tauri/src/commands.rs`：已有 `load_config/save_config/get_config_status`，尚无目录选择或切换 workspace 命令。

因此，本功能不应新增一套平行设置系统。应在现有设置树中增加“工作区”页面，并在聊天 Topbar 放一个只读的当前工作区状态入口，形成“快速查看 → 深入管理”的路径。

## 3. 推荐信息架构

### 3.1 Topbar：当前工作区胶囊

在 `NormalMode.vue` Topbar 左侧身份区之后、模型选择器之前放置 `WorkspaceChip`：

```text
┌ DIVA · 在线 ┐   [⌂ agent-diva  ▾]              [模型] [Mask]
```

显示策略：

- 首选显示目录 basename，例如 `agent-diva`；
- hover/展开显示完整 canonical path；
- 路径过长时只截断中间，不截断 basename；
- 左侧状态点：绿色“当前工作区”、琥珀色“切换待重启”、红色“路径不可用”；
- 若存在根 `AGENTS.md`，在胶囊尾部显示小型文档标记；只表达“存在”，不把正文或内容摘要放在 Topbar。

点击后打开 `WorkspacePopover`，内容顺序固定：

1. 当前路径和来源（显式选择 / 配置 / 当前目录）；
2. `AGENTS.md` 状态（已注入 / 未发现 / 被截断 / 不可读）；
3. “打开工作区设置”；
4. “切换工作区”。

Popover 不直接列出最近路径，避免把历史项目路径暴露在常驻 UI；最近工作区可以在设置页以用户主动打开的列表呈现，并且只存 canonical path 的显示名和最近使用时间。

### 3.2 设置页：独立的“工作区”页面

在 `SettingsView` 的 `SettingsSubview` 增加 `workspace`，在 `SettingsDashboard` 增加首张高优先级卡片，放在 Providers 之前：

```text
工作区
项目执行目录、项目规则与会话数据的边界
```

页面布局：

```text
工作区
项目执行边界与项目指令

当前工作区                                  [切换]
┌──────────────────────────────────────────┐
│ agent-diva                               │
│ C:\Users\...\agent-diva                  │
│ 来源：当前目录 / 已保存配置 / 显式选择       │
│ [在文件管理器中打开]                       │
└──────────────────────────────────────────┘

项目指令（AGENTS.md）                       [重新扫描]
┌──────────────────────────────────────────┐
│ ● 已注入  ·  根目录 AGENTS.md              │
│ 来源：C:\Users\...\agent-diva\AGENTS.md  │
│ 1,842 / 4,000 字符                         │
│ [查看摘要]                                  │
└──────────────────────────────────────────┘

工作区行为
☑ 将新会话固定在当前工作区（默认）
☐ 切换前询问未完成会话（推荐保留）

说明：工作区选择会影响文件、Shell、Session、Plan、Subagent 和 AGENTS.md。
机器级配置、BML、Persona 不会移动到项目目录。
```

这里的“工作区行为”第一期不做可选开关，作为只读说明更安全；只有确认产品合同后才将其变成真正的偏好。GUI 不应暴露“允许跨工作区 Shell”之类的危险选项。

### 3.3 AGENTS 详情抽屉

“查看摘要”打开右侧抽屉，而不是全屏编辑器：

- 标题：`项目指令 · AGENTS.md`；
- 显示状态、来源绝对路径、读取时间、字符预算、digest 前 12 位；
- 正文默认折叠，只显示前 12 行或安全摘要；
- 明确警示：项目指令不能授予工具权限、覆盖安全策略或写入 BML/Persona；
- 提供“在文件管理器中打开”，不提供 GUI 内编辑按钮；
- 文件不存在、为空或不可读时，显示对应原因和“重新扫描”，不显示错误堆栈。

这样既能诊断“为什么 DIVA 没有遵循项目规则”，又不会把项目文档伪装成系统设置或让用户误以为 GUI 正在编辑它。

## 4. 工作区切换交互

### 4.1 选择流程

1. 用户点击 Topbar `WorkspaceChip` → `切换工作区`，或设置页 `切换`；
2. 调用 Tauri 原生目录选择器；
3. 选择后先做本地验证：路径存在、是目录、可读，规范化为 canonical absolute path；
4. 预览确认弹窗展示影响范围和新路径：

   ```text
   切换到此工作区？
   C:\Projects\demo

   新会话、文件工具、Shell、Plan、Subagent 和 AGENTS.md 将使用此目录。
   当前正在运行的会话不会迁移；未完成操作会先停止。

   [取消] [切换并重启 DIVA]
   ```

5. 若当前正在流式回答、执行 Plan 或存在待审批操作，默认阻止切换并说明原因；用户需先停止/完成这些操作；
6. 用户确认后，GUI 进入“切换中”全屏阻塞态，停止后台 stream 和 embedded gateway；
7. 写入配置并重建 `GatewayRuntimeConfig/AppState`，重启 gateway；
8. 成功后清空当前 GUI chat view 的瞬时消息，按新 workspace 恢复该工作区最近会话；
9. 失败则保留旧 workspace 运行，恢复按钮和可复制错误摘要，不让 GUI 留在半切换状态。

### 4.2 不采用热切换

不在运行中的 `AppState.workspace_root` 上直接替换路径。Manager、Cron、Channel、Plan 和审批流可能持有旧 root；热切换会产生一半旧路径、一半新路径的会话。GUI 表面上是一个“切换”动作，运行时语义必须是“停止 → 保存 → 重建 → 恢复”。

### 4.3 未完成会话处理

第一期使用保守策略：

- 旧 workspace 的会话保留在旧目录；
- 新 workspace 不自动复制或迁移 Session/Plan；
- 切换完成后只加载新 workspace 的最近 GUI session；
- 若用户想回到旧项目，使用 workspace history 重新选择旧目录。

不要在切换确认中提供“迁移所有数据”复选框；迁移是独立、可审计的后续能力。

## 5. AGENTS 状态契约

GUI 需要从 `get_config_status` 或专用 `get_workspace_status` 获得结构化数据，而不是自行读取文件：

```ts
interface WorkspaceStatus {
  root: string;
  source: 'explicit' | 'configured' | 'process_cwd';
  agents: {
    status: 'injected' | 'missing' | 'empty' | 'unreadable' | 'truncated';
    path?: string;
    chars?: number;
    limit: number;
    digest_prefix?: string;
    loaded_at?: string;
  };
  gateway_restart_required: boolean;
}
```

推荐状态映射：

| 状态 | GUI 文案 | 颜色 | 操作 |
|---|---|---|---|
| `injected` | 已注入 | 绿色 | 查看摘要、重新扫描 |
| `truncated` | 已注入，已截断 | 琥珀色 | 查看摘要、打开文件 |
| `missing` | 未发现 AGENTS.md | 中性灰 | 重新扫描 |
| `empty` | 文件为空 | 中性灰 | 打开文件 |
| `unreadable` | 无法读取 | 红色 | 查看原因、重新扫描 |

正文不进入普通 status 日志；只返回路径、预算、digest 和状态，避免泄露项目提示词。

## 6. GUI → Tauri/运行时接口

建议增加窄接口，而不是让前端改完整 JSON：

```text
get_workspace_status() -> WorkspaceStatus
choose_workspace_directory() -> { path: string }
apply_workspace(path: string) -> WorkspaceSwitchResult
rescan_workspace_instructions() -> WorkspaceStatus
open_workspace_in_file_manager() -> void
```

`apply_workspace` 的返回值至少包含：`old_root`、`new_root`、`restart_performed`、`agents_status`、`recovered_session_key`。配置保存仍由 Rust 负责校验和原子写入；前端不能直接拼接 `agents.defaults.workspace` 并期待运行时自动改变。

如果暂时不引入专用命令，可以先扩展 `get_config_status` 的 `config` payload，但“选择目录”和“重建 gateway”仍应由单一 Tauri command 负责，避免前端编排多个停止/启动调用。

## 7. 响应式与可访问性

- Topbar 胶囊在窄窗口只显示 basename，完整路径放入 `title` 和可复制详情；
- 设置页卡片在 760px 以下单列；AGENTS 抽屉变成底部 sheet；
- 目录选择、确认、切换中、失败都提供键盘焦点顺序和 `aria-live` 状态；
- 颜色不是唯一状态信号，必须同时有文字和图标；
- 切换中禁用聊天输入、模型选择、审批按钮，防止用户在 gateway 重建时发起请求；
- 工作区路径和 AGENTS 摘要支持一键复制，但默认不写入 GUI 日志。

## 8. 施工顺序与 GUI 验收

### Phase G0：状态模型

- 先补 `WorkspaceStatus`/`WorkspaceSwitchResult` 类型和状态映射测试；
- GeneralSettings 先展示新状态，使用 mock payload 覆盖五种 AGENTS 状态；
- 不改变 workspace 语义。

### Phase G1：设置页管理

- 增加 `WorkspaceSettings.vue`、SettingsDashboard 卡片和 i18n；
- 增加 WorkspacePopover/AGENTS 抽屉；
- 完成路径截断、复制、重扫和不可读状态。

### Phase G2：原生选择与重启

- 引入 Tauri 原生目录选择能力；
- 由单一 command 执行校验、原子保存、停止/重建 gateway；
- 加入切换中阻塞态和失败回滚；
- 处理旧配置迁移提示，不静默把旧 `~/.agent-diva/workspace` 当成未配置。

### Phase G3：真实路径回归

- GUI smoke：选择临时目录 → 确认 → gateway 重启 → Topbar/Settings 显示新 root；
- 在新 root 放置 `AGENTS.md`，确认显示“已注入”；删除后重扫，确认显示“未发现”；
- 流式回答、Plan 审批、Shell 执行期间尝试切换，确认动作被阻止且无半切换；
- 重启 GUI，确认 workspace 与 AGENTS 状态恢复一致。

## 9. 明确不做的 GUI 设计

- 不在聊天输入框旁放一个会影响所有执行路径的隐形 cwd 文本框；
- 不让 GUI 内编辑 `AGENTS.md`，避免把项目规则变成 DIVA 管理配置；
- 不把 `~/.agent-diva` 的 BML/Persona 路径显示成项目 workspace；
- 不提供“AGENTS 授权”“跳过安全边界”“允许任意目录”开关；
- 不在切换 workspace 时自动复制 Session、Plan、Persona 或 Memory；
- 不在外部 workspace 首次打开时静默创建项目模板，初始化必须是明确动作。
