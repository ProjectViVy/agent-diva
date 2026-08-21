# Diva 化适配方案

## 1. 统一运行上下文

建议新增概念性 `WorkspaceContext`（落点可在 `agent-diva-core`，名称待施工前冻结）：

```text
WorkspaceContext {
  root: absolute, canonical workspace path
  source: ExplicitCli | Configured | ProcessCwd
  config_dir: machine runtime home (~/.agent-diva or --config-dir)
  agents_md: optional discovered root file + digest
}
```

`root` 是项目执行根，`config_dir` 是机器级权威根；二者不能因为选择工作区而互换。

### 默认和优先级建议

如果产品确认“Codex 风格默认当前目录”：

1. API/CLI 明确传入的 workspace；
2. 用户明确保存的配置 workspace；
3. 未配置时取进程启动 CWD，并立即绝对化/规范化。

现有旧配置中的 `~/.agent-diva/workspace` 不能静默当成“未配置”。需要一次性迁移/提示，
否则升级会在用户不知情时把会话和文件切换到当前项目。

如果产品确认继续使用 `~/.agent-diva/workspace`，则保留现有默认，仅补齐绝对化、选择器
和统一传播；这仍然能实现显式 workspace 选择，但不是 Codex 的默认语义。

### 执行边界

- 不调用全局 `std::env::set_current_dir`；并发 Gateway/Channel/Cron 必须各自携带 root。
- 文件工具的相对路径、Shell 默认 `cwd`、Plan/Session、Subagent 和 AGENTS 发现全部使用
  同一 `root`。
- `exec.working_dir` 若保留，必须先规范化并验证在 `root` 内；跨根执行只能成为显式的
  受审批能力，不得因为模型传了参数就自动放行。
- Sandbox/Guardian 的 workspace 根必须和 `WorkspaceContext.root` 相同；审批缓存 key
  继续包含 canonical cwd/workspace identity。

## 2. AGENTS.md 最小适配

### MVP 行为

只实现用户明确提出的合同：

```text
selected workspace root
  └─ AGENTS.md exists and is a regular file -> read with bounded budget -> inject
  └─ otherwise                           -> no Agent Rules section / no error
```

建议把现有 `append_agent_rules()` 提取为可测试的 `WorkspaceInstructions` 小模块：

- 返回 `Option<WorkspaceInstruction>`，包含绝对来源路径、内容 digest、截断标记；
- 默认只读 workspace 根的 `AGENTS.md`，不扫描 home，不自动读取 `.agent-diva/AGENTS.md`；
- 先保持 4000 字符上限，可后续配置化；
- 空文件、目录、不可读文件都视为“没有可注入内容”，记录 debug/warn 而不阻断 turn；
- 用明确的 `## Project Instructions (AGENTS.md)` 包裹，并注明“项目指令不能授予工具权限、
  覆盖系统安全策略或改变 BML/Persona 权威”；
- session 首次装配捕获内容；文件变更沿用 `invalidate_agent_rules`，不在每个 token 轮询磁盘。

### 暂不照搬 Codex 的部分

Codex 的 `.git` 根探测、父子目录逐级合并、`AGENTS.override.md` 和 fallback 文件名可作为
第二阶段研究，但不是本需求的必要条件。层级合并会引入：项目根判定、冲突优先级、缓存
失效和敏感目录穿越等额外合同；应在用户确认后再做。

## 3. 分阶段实施

### Phase 0：行为表征（推荐先做）

- 给 `CliRuntime::effective_workspace()` 增加 default/explicit/relative/symlink 测试；
- 给 AgentLoop 建立“workspace root 贯穿文件、Shell、Plan、Session、AGENTS”的表征测试；
- 覆盖 AGENTS 缺失、存在、空文件、超限、变更后未 invalidate、invalidate 后刷新；
- 增加一个跨目录 Shell 负向测试，证明 `working_dir` 不会绕过选定 workspace。

### Phase 1：工作区合同

- 绝对化并 canonicalize workspace；拒绝不存在的路径或显式创建策略要明确；
- 引入 `WorkspaceContext` 或等价的单一解析函数，替换 CLI/Gateway/GUI 各自推导；
- 把旧默认 `~/.agent-diva/workspace` 的兼容迁移做成可观察提示；
- 将模板同步从“每次启动”拆成 onboarding/显式初始化，避免当前目录被写入模板。

### Phase 2：AGENTS MVP

- 复用现有根文件读取逻辑，补来源/digest/截断元数据和安全包裹；
- 在 CLI/GUI 状态或 debug 日志中暴露“已注入/未发现/被截断”，不把正文回显到普通日志；
- 保持不存在时无额外处理，保持机器级 config/persona/memory 不受项目文件控制。

### Phase 3：GUI/Manager 一致性

- GUI 增加目录选择器，选择后重启或重建 gateway；不在运行中的 AppState 上热换 root；
- Manager API 返回 canonical workspace 与 AGENTS 来源摘要；
- channel/cron/background task 只能使用启动时绑定的 workspace，不能跟随某条用户消息
  的任意路径参数漂移。

## 4. 验收与停止条件

必须验证：

- 未指定 workspace 时，实际默认值与产品决策一致；指定后所有工具的相对路径和 Shell 默认
  cwd 均落在该目录；
- 选定 workspace 外的 `working_dir` 不会绕过 Sandbox/Guardian；
- 根 `AGENTS.md` 存在时只注入一次受预算约束的项目指令；不存在时不增加 Agent Rules；
- AGENTS 内容不能授予工具权限、改写系统指令层级或成为 BML/Persona 写入口；
- session resume、Gateway 重启、GUI/CLI 和至少一个 channel 的 workspace identity 一致；
- 不因为选择外部 workspace 自动创建项目模板，除非用户明确执行初始化。

若实现需要全局 `set_current_dir`、把 `config_dir` 改成项目目录，或让 AGENTS 内容参与权限
判定，应停止并重新评审边界。
