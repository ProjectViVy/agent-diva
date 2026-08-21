# 当前状态与 Codex 证据

## 1. Codex 的工作区合同

参考快照：`.workspace/codex`，HEAD `a9c111da544c976d591343db5493a7da283b72e5`。

- `codex-rs/utils/cli/src/shared_options.rs` 提供 `--cd DIR`（短参数 `-C`），描述为把
  指定目录作为 agent 的 working root。
- `codex-rs/core/src/config/mod.rs` 的 `ConfigBuilder::build()` 在没有 `cwd` override
  时调用 `AbsolutePathBuf::current_dir()`；有 override 时相对当前目录解析并转为绝对路径。
- `Config::cwd` 被定义为“本次 session 的当前工作目录”，业务层所有相对路径都相对它解析。
- TUI 恢复/分叉会比较当前 CWD 与历史 session CWD；不同则让用户选择“当前目录”或“历史
  session 目录”，而不是静默混用（`codex-rs/tui/src/cwd_prompt.rs`）。
- App Server 的 `thread/start` 和 `turn/start` 也把 `cwd` 作为显式字段，省略时使用服务
  进程 CWD；线程记录 CWD，列表可按 CWD 过滤。

因此 Codex 的关键不是执行前调用一次全局 `chdir`，而是把绝对 CWD 作为线程/turn 的运行
上下文，传给工具、Sandbox、会话和提示词装配。

## 2. Codex 的 AGENTS.md 发现与注入

实现位于 `codex-rs/core/src/agents_md.rs`：

1. 从当前 CWD 向上寻找配置的 `project_root_markers`；默认 marker 是 `.git`。
2. 从项目根到当前 CWD（含两端）逐级寻找候选文件，每层最多选一个；优先
   `AGENTS.override.md`，再选 `AGENTS.md`，然后才看配置的 fallback 文件名。
3. 按根到叶的顺序拼接内容；没有任何文件时返回 `None`。
4. 默认总预算为 `32 * 1024` 字节，可由 `project_doc_max_bytes` 设为 0 完全关闭。
5. `user_instructions()` 把项目文档与已有用户指令以明确 separator 合并；同时可暴露
   `instruction_sources()`，让 UI/审计知道实际注入了哪些文件。

Codex 测试覆盖了：缺文件返回 None、超限截断、根/子目录合并、override 优先、目录而非
文件时忽略、fallback、来源顺序以及配置 marker。

## 3. Diva 当前工作区选择

### 已有能力

- `agent-diva-cli/src/main.rs` 已注册全局 `--workspace`，说明为只覆盖当前命令、不改配置。
- `agent-diva-cli/src/cli_runtime.rs::CliRuntime::effective_workspace()` 的优先级是：
  CLI override；否则读取 `config.agents.defaults.workspace` 并展开 `~`。
- `AgentDefaults::default()` 目前把 workspace 设为 `~/.agent-diva/workspace`。
- `ConfigLoader::new()` 的配置家是用户主目录下的 `~/.agent-diva`。

### 选择后实际跟随的路径

CLI 与 Gateway 将同一个 `workspace` 传入：

- `AgentLoop` / `ContextBuilder`；
- `SessionManager` 与 `PlanningConfig::open_workspace`；
- `ToolAssembly` 的文件安全根、Shell 默认 working directory、Subagent；
- token ledger、audit、AutoDream/Notebook 等项目运行面；
- Manager `AppState.workspace_root`。

所以“指定工作区后执行路径改变”不是从零开始，核心传播链已经有了。缺口在于默认值、
路径绝对化/校验和所有入口的一致合同。

### 当前重要风险

- `effective_workspace()` 没有把路径规范化为稳定的绝对路径；`--workspace .`、相对路径和
  symlink 可能产生多个 workspace identity。
- `ensure_workspace_templates()` 会在启动时向选定目录创建 `PROFILE.md`、`TASK.md`、
  `skills/` 和 `masks/`；若默认改成当前目录，普通项目会被自动写入模板。
- `ExecTool` 接受单次 `working_dir` 参数，优先级高于构造时的 workspace；若不启用严格限制，
  一次 Shell 调用可在选定工作区外运行。
- Manager/Gateway 的 `AppState.workspace_root` 在启动时固定；GUI 当前从配置计算 workspace，
  没有独立的项目工作区选择流程。

## 4. Diva 当前 AGENTS.md 行为

实现位于 `agent-diva-agent/src/context.rs`：

- `ContextBuilder` 持有 `workspace`；生产构造使用 `with_skill_home(workspace, config_dir, …)`，
  因而 AGENTS 来源是项目 workspace，不是机器级 config_dir。
- `build_agent_rules_and_skills_section()` 调用 `append_agent_rules()`；后者只读取
  `workspace.join("AGENTS.md")`。
- `read_trimmed_markdown()` 的预算是 `WORKSPACE_MD_MAX_CHARS = 4000`。
- 读取成功才追加 `## Agent Rules`；缺少、空文件或读取失败时不追加该段。
- 稳定前缀按 session 缓存；`invalidate_agent_rules(session_key)` 才会在已有 session
  的下一次装配重新读取。
- 已有单测覆盖存在、变更后缓存保持、显式 invalidate 后重新读取，以及没有文件时默认
  prompt 不因为 AGENTS 失败而中断。

这意味着优化 2 的最小版本已经落地，真正需要决定的是“是否提升发现范围和可观测性”，而
不是重新发明 Prompt 注入。
