# Workspace 与 AGENTS.md：Diva 化适配研究包

> 日期：2026-08-22
> 状态：Research / Proposal，未授权生产实现
> 参考：本机 `.workspace/codex` 快照 `a9c111da5`

## 这次研究回答的两个问题

1. 不指定工作区时，Diva 是否应以默认目录工作；指定工作区后，执行路径是否随之切换。
2. 工作区存在 `AGENTS.md` 时是否注入；不存在时是否保持无额外处理。

## 结论先行

### 1. 工作区选择：可做，但要把“运行时家”和“项目工作区”拆开

Diva 当前已经有 CLI 全局 `--workspace`，并且它会影响 Session、Plan、文件工具、Shell、
Subagent 和 ContextBuilder；但没有指定时，默认值是 `~/.agent-diva/workspace`，不是进程
启动时的当前目录。Codex 的对应语义是 `--cd DIR`，不指定时使用当前目录，并把绝对化后的
`cwd` 贯穿到相对路径解析、Sandbox、会话和 AGENTS.md 发现。

建议 Diva 采用：

- **运行时家**：`~/.agent-diva`，保存配置、BML/Persona/Skill 等机器级权威；
- **项目工作区**：显式 `--workspace DIR`，否则使用启动进程的当前目录；
- **不调用进程级 `set_current_dir`**：把项目工作区作为每个 AgentLoop/Tool 的显式根，
  防止 Gateway、Channel、Cron 并发时互相改 CWD。

这不是简单把 `~/.agent-diva` 改名为工作区。当前 Diva 的机器级记忆和 Persona 边界要求
它们继续留在配置家；只有项目执行、会话、Plan、审计和项目规则跟随项目工作区。

### 2. AGENTS.md 注入：最小需求其实已经存在，但能力比 Codex 简化

当前 `ContextBuilder` 已读取 `<workspace>/AGENTS.md`，最多 4000 字符；文件不存在时不
追加 `Agent Rules` 段。这已经满足“工作区根有则注入、无则不处理”的最小合同。

与 Codex 的差距是：Diva 只看工作区根目录、没有 `.git` 项目根向下逐级合并、没有
`AGENTS.override.md`/fallback 文件名、没有统一的来源列表与字节预算配置，而且现有 Shell
工具还允许单次参数覆盖 `working_dir`，可能使执行目录和 AGENTS 来源不一致。

建议先做 **Diva 最小适配**：根目录 `AGENTS.md`、显式来源标记、字节/字符上限、按 session
缓存和显式刷新；暂不复制 Codex 的完整层级发现和 fallback 生态。

## 研究材料

- [当前 Diva 与 Codex 证据](./current-state-and-codex-evidence.md)
- [Diva 化方案与分阶段计划](./diva-adaptation-proposal.md)

## 研究 Gate

进入施工前只需确认三个产品选择：

1. “不指定工作区”的默认值是否改为进程当前目录，还是继续使用既有
   `~/.agent-diva/workspace`；
2. 选定工作区后，是否禁止 `exec.working_dir` 跳出该根目录；
3. 外部工作区启动时是否禁止自动创建 `PROFILE.md`、`TASK.md`、`masks/` 等模板。
