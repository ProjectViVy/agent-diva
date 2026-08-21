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

## 10. Oil Frontend 细化：先固定业务对象和数据所有权

### 10.1 核心对象不是路径字符串

GUI 管理的核心对象是 `WorkspaceContext`，而不是一个可随处修改的 `workspace: string`：

```ts
interface WorkspaceContext {
  root: string;                 // backend canonical absolute path
  displayName: string;          // basename，由 root 派生
  source: 'explicit' | 'configured' | 'process_cwd';
  agents: AgentsStatus;         // 同一次 status 响应中的快照
  runtimeGeneration: string;    // gateway 重建后的身份
}
```

字段所有权必须固定：

- Rust/Gateway 是 `root`、`source`、`agents` 和 `runtimeGeneration` 的权威来源；
- `App.vue`（或唯一的 `useWorkspaceContext`）持有当前运行时快照，并向 `NormalMode`、
  `SettingsView` 传递同一份对象；
- `WorkspaceSettings` 只持有尚未提交的 `candidatePath` 和候选预览，不能复制完整
  `WorkspaceContext`；
- `displayName`、状态文案和颜色都从同一快照派生，不在多个组件重复转换。

当前 `GeneralSettings.vue` 自己调用 `getConfigStatus()`，会与 App 启动时的状态形成第二个
来源。施工时应改为由上层注入已提交快照；workspace 页面和 Topbar 不得各自请求、各自缓存。

### 10.2 信息层级与应删除的内容

首屏只保留完成判断所需的信息：

1. 当前工作区名称和“正在使用”；
2. 一行可复制的完整路径；
3. workspace 来源；
4. AGENTS 状态；
5. 一个主操作“切换工作区”。

应从常驻界面删除：

- 内部 enum（`process_cwd` 等）和 `runtimeGeneration`；
- `config_dir`、cron store、bridge 等机器级路径；
- 同时出现在 Topbar、SettingsDashboard、GeneralSettings 的重复 workspace 路径；
- 只表达“存在”的装饰性徽章、重复的 `AGENTS.md` 文件图标和长段说明；
- 没有真实动作的“保存工作区”“应用 AGENTS”按钮。

完整 canonical path、digest、读取时间和字符预算只在详情或复制操作中出现；它们服务排障，
不应取代用户识别项目所需的名称。

## 11. 选择、预览与提交边界

### 11.1 这是选择流程，不是普通表单

目录选择由系统原生 picker 完成，GUI 不提供长期可编辑的路径文本框，也不显示“当前值 +
同一字段禁用 input”。用户点击“切换工作区”后才进入候选态：

```text
ready(current)
  → picker
  → candidate(path)
  → inspecting(candidate)
  → preview(candidate + agents)
  → confirming(candidate)
  → applying
  → ready(new current) | error(old current + candidate preserved)
```

`inspect_workspace(path)` 是只读预览，不写配置、不创建目录、不启动 Gateway；它检查目录有效性
并读取候选根下的 `AGENTS.md` 摘要。`apply_workspace(path)` 必须在 Rust 端再次校验候选路径，
防止 picker 返回后目录状态发生变化。

### 11.2 一个提交边界、一个主操作

候选预览页只提供：

- 主操作：`切换并重启 DIVA`；
- 次级操作：`取消`；
- 候选状态下的 `重新选择` 作为次级导航动作。

不要出现“确认 → 保存 → 重启”三个同义按钮。一次点击触发一个真实的 `apply_workspace`，
只有该命令成功且新的 status/session 已提交后，才把候选路径提升为当前对象。

确认文案只说明不可忽略的范围：相对路径、Shell、Session、Plan、Subagent 和 AGENTS.md；
不重复解释显然的按钮含义，也不在确认框增加独立“迁移数据”选项。

### 11.3 失败时保留上下文

`inspect_workspace` 或 `apply_workspace` 失败时：

- 当前 `WorkspaceContext` 保持旧值；
- `candidatePath`、候选 AGENTS 预览和用户当前页面位置保留；
- 就近显示可修正错误（路径不存在、不可读、Gateway 重建失败）；
- 提供 `重新选择` 或 `重试切换`，不要求用户重新打开设置或重新选择旧目录。

成功后更新原对象：先用 `apply_workspace` 返回的 committed status 替换快照，再刷新该 root
下的最近 session；不得通过复制一个“新 workspace 对象”让旧组件继续持有旧引用。

## 12. 状态与忙碌范围矩阵

### 12.1 Workspace 状态

`WorkspaceSettings` 的区域状态必须互斥：

| 状态 | 条件 | 内容 | 可用动作 |
|---|---|---|---|
| `loading` | 首次获取当前 root | 保留标题和布局骨架 | 无，等待当前状态 |
| `ready` | 当前 root 可用 | 当前路径、来源、AGENTS 状态 | 切换、复制、打开目录 |
| `refreshing` | 重扫 AGENTS | 保留旧快照，按钮局部显示刷新中 | 取消不支持时禁用重扫 |
| `candidate` | picker 已返回 | 候选路径和“尚未生效”标签 | 预览、重新选择、取消 |
| `inspecting` | 候选预览请求中 | 保留候选路径，AGENTS 卡片局部 loading | 不能提交未检查候选 |
| `confirming` | 预览完成，等待确认 | 影响范围与候选身份 | 取消、切换并重启 |
| `processing` | Gateway 正在重建 | 当前 root 不变，显示阶段文字 | 禁止关闭确认和重复提交 |
| `error` | 请求或重建失败 | 旧 root + 候选 + 可恢复错误 | 重试、重新选择 |

`missing`、`empty`、`unreadable`、`injected`、`truncated` 是 AGENTS 子状态，不得与页面
`loading`、`processing` 混为一个 spinner：

| AGENTS 子状态 | 说明 | 主动作 |
|---|---|---|
| `injected` | 根文件已读取并注入 | 查看摘要、重新扫描 |
| `truncated` | 已读取但超过预算 | 查看截断摘要、打开文件 |
| `missing` | 根目录没有文件 | 重新扫描 |
| `empty` | 文件存在但无有效内容 | 打开文件 |
| `unreadable` | 文件存在但读取失败 | 查看原因、重新扫描 |

### 12.2 忙碌范围

- AGENTS 重扫只锁定 AGENTS 状态卡和“重新扫描”按钮，不遮挡 Chat；
- 候选 inspect 只锁定候选预览区，当前 workspace 仍可读；
- apply workspace 是跨运行时操作：确认弹窗保持打开，主按钮进入 `processing`，遮罩只覆盖
  会发起新请求的 Chat、模型选择、审批和 workspace 操作；设置导航可保留但不能提交旧 root；
- 不用无限 spinner 表达 Gateway 重启，显示阶段：`停止运行时` → `保存工作区` →
  `重建 Gateway` → `恢复会话`；
- 只有新 status 和 session 恢复成功后关闭弹窗；失败时保留弹窗与候选信息。

## 13. 组件与数据流落点

建议按业务模块新增 `agent-diva-gui/src/features/workspace/`，而不是继续把路径状态堆进
`GeneralSettings.vue`：

```text
features/workspace/
  workspaceTypes.ts          # WorkspaceContext, AgentsStatus, switch states
  workspaceApi.ts            # get/inspect/apply/rescan/open commands
  useWorkspaceContext.ts     # 唯一查询、候选、切换和状态机
  WorkspaceChip.vue          # Topbar 只读入口
  WorkspacePopover.vue       # 触发器附属浮层，不拥有业务请求
  WorkspaceSettings.vue      # 设置页组合器
  WorkspaceSwitchDialog.vue  # 预览/确认/processing/error
  AgentsInstructionDrawer.vue# 只读摘要详情
```

职责边界：

- `useWorkspaceContext` 负责请求、候选、提交边界、过期响应保护和 session 恢复；
- `WorkspaceChip`、`WorkspaceSettings`、`AgentsInstructionDrawer` 只渲染 props 并 emit 用户意图；
- Tauri API 类型集中在 `workspaceApi.ts`，不在多个 Vue 组件重复声明响应结构；
- `SettingsView` 只做路由组合，`NormalMode` 只负责把 WorkspaceChip 放入 Topbar；
- `GeneralSettings` 保留通用运行状态，但移除自己的 workspace 请求和重复路径展示；
- 组件样式使用现有 tokens 和共享弹层/弹窗/抽屉，不用页面级 `!important`、固定截图宽高或
  临时 z-index。

后端建议返回一个权威的 `WorkspaceStatus` 快照。若为兼容现有 `StatusPathReport.workspace`
而暂时增加字段，应让旧字符串成为同一快照的序列化投影，不能让 GUI 同时维护两套可写状态。

## 14. 浮层、弹窗、抽屉和滚动

- `WorkspacePopover` 只承载当前身份、AGENTS 简短状态和导航动作；它是触发器附属浮层，
  由共享 overlay 负责定位、碰撞、点击外部关闭和视口安全边距；不在其中塞完整候选预览。
- `WorkspaceSwitchDialog` 是单一共享确认弹窗，使用 Header/Body/Footer：Header 显示
  “切换工作区”与候选 basename；Body 显示路径、AGENTS 预览、影响范围；Footer 固定
  `取消 / 切换并重启 DIVA`；Body 是唯一纵向滚动区。
- `AgentsInstructionDrawer` 适合只读详情，因为底层 Settings/当前 workspace 仍需保持可见；
  Drawer 内正文摘要滚动，关闭后回到原设置位置，不叠加第二个模态弹窗。
- Settings 页面由现有 `settings-body` 主滚动容器负责纵向滚动；Popover、Dialog、Drawer
  不把滚动传递给 body，也不新增无边界的 `overflow: auto`。
- 宽屏可将“当前 workspace”与“AGENTS 状态”并列；窄屏折叠为单列，主操作仍固定在
  Dialog Footer，长路径使用中间省略而不是撑破页面。

## 15. 后端接口与过期响应保护

前端不直接读取 workspace 文件，也不通过 `load_config/save_config` 拼接完整配置。建议接口：

```ts
getWorkspaceStatus(): Promise<WorkspaceStatus>
inspectWorkspace(path: string): Promise<WorkspacePreview>
applyWorkspace(path: string): Promise<WorkspaceSwitchResult>
rescanWorkspaceInstructions(): Promise<WorkspaceStatus>
openWorkspaceInFileManager(): Promise<void>
```

约束：

- `inspectWorkspace` 返回 `candidateId`/canonical root；`applyWorkspace` 只接受最新候选或
  重新 canonicalize，避免旧 picker 结果覆盖新选择；
- status、AGENTS 摘要和 root 必须来自同一响应快照；不允许先更新路径再异步补 AGENTS；
- 连续点击“重新扫描”只提交最后一次请求，过期响应不得覆盖当前状态；
- `applyWorkspace` 成功才更新 `WorkspaceContext`，前端不做乐观 root 更新；
- `WorkspaceSwitchResult` 返回 committed root、AGENTS status、runtime generation 和
  恢复的 session key，减少 GUI 再拼装第二套结果。

## 16. Oil Frontend 验收矩阵

### 组件测试

- `WorkspaceChip`：basename/path/source/AGENTS 五种状态和 processing 状态显示唯一且无重复；
- `WorkspaceSettings`：当前值只读，picker 返回后才出现候选态，取消不改变当前值；
- `WorkspaceSwitchDialog`：确认按钮只触发一次，processing 时不可关闭，失败保留候选与错误；
- `AgentsInstructionDrawer`：摘要、预算、digest 和路径缺失/截断状态正确映射。

### 数据流测试

- workspace status 只请求一次并由 Topbar/Settings 共享；
- inspect 的旧响应不能覆盖最新候选；
- apply 失败不替换旧 root；成功后只使用 committed result 更新 context 并刷新 session；
- AGENTS 重扫失败保留旧快照，不退化成 `missing` 或空页面。

### 真实 GUI smoke

1. 启动 GUI，确认 Topbar 与 Settings 显示同一 canonical root；
2. 选择临时目录 A，写入 `AGENTS.md`，确认候选预览显示 `injected`；
3. 切换到 A，确认 Gateway 重建、Chat 输入恢复、Session 来自 A；
4. 删除 A 的 `AGENTS.md` 并点重扫，确认变为 `missing`，旧内容不残留；
5. 在 streaming/Plan/approval 期间尝试切换，确认动作被阻止且当前 root 不变；
6. 模拟 apply 失败，确认旧 root、候选路径、页面位置和错误仍保留；
7. 用长路径和窄窗口检查不出现根页面横向滚动、双重纵向滚动或 Footer 被遮挡。
