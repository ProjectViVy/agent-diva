# Agent Diva User Guide

本指南面向命令行用户，目标是让你从零开始完成安装、初始化、配置、聊天、网关运行、Provider 管理、Channel 管理、定时任务管理和常见排障。

本文默认你在仓库根目录，CLI 可执行文件名为 `agent-diva`。在 Windows 开发环境中，如果还没有全局安装，可以用 `cargo run -p agent-diva-cli -- <args>` 临时代替。

## 1. 安装与运行方式

### 1.1 前置依赖

- Rust stable
- `cargo`
- 可选：`just`

### 1.2 本地构建

```bash
cargo build --all
```

### 1.3 直接运行 CLI

```bash
cargo run -p agent-diva-cli -- --help
```

### 1.4 安装到本机

```bash
cargo install --path agent-diva-cli
agent-diva --help
```

## 2. CLI 全局参数

所有主命令都支持这些全局参数：

```bash
agent-diva [OPTIONS] <COMMAND>
```

常用全局参数：

- `--config <FILE>`：指定某个 `config.json`
- `--config-dir <DIR>`：指定实例目录
- `--workspace <DIR>`：只覆盖当前命令使用的 workspace，不修改配置文件
- `--remote`：连接远端 `agent-diva-manager`
- `--api-url <URL>`：远端 API 地址，默认是 `http://localhost:3000/api`

示例：

```bash
agent-diva --config ~/.agent-diva/config.json status
agent-diva --config-dir ~/.agent-diva agent --message "hello"
agent-diva --remote --api-url http://127.0.0.1:3000/api status
```

## 3. 第一次使用

### 3.1 初始化配置

最简单的方式：

```bash
agent-diva onboard
```

这个命令会：

- 创建或刷新配置文件
- 创建 workspace
- 同步模板文件
- 提示下一步命令

### 3.2 非交互初始化

```bash
agent-diva onboard \
  --provider openai \
  --model openai/gpt-4o \
  --api-key sk-xxx \
  --workspace ~/.agent-diva/workspace
```

常用参数：

- `--provider <name>`
- `--model <model-id>`
- `--api-key <key>`
- `--api-base <url>`
- `--workspace <dir>`
- `--refresh`：保留已有值并补齐默认值
- `--force`：按默认配置重建再应用你传入的参数

## 4. 配置文件与实例路径

默认配置文件：

```text
~/.agent-diva/config.json
```

你可以查看当前实例实际使用的路径：

```bash
agent-diva config path
agent-diva config path --json
```

输出会包含：

- config 路径
- config 目录
- runtime 目录
- workspace
- cron store
- bridge 目录
- WhatsApp auth/media 目录

## 5. 配置管理命令

### 5.1 刷新配置模板

```bash
agent-diva config refresh
```

适用场景：

- 升级后补新字段
- 重新同步 workspace 模板
- 不想覆盖已有自定义值

### 5.2 校验配置

```bash
agent-diva config validate
agent-diva config validate --json
```

行为：

- 只做 schema/语义校验
- 无效配置返回退出码 `1`

### 5.3 诊断运行就绪状态

```bash
agent-diva config doctor
agent-diva config doctor --json
```

行为：

- 检查配置是否合法
- 检查默认模型能否解析出 provider
- 检查 workspace 是否存在
- 检查已启用 channel 是否缺少字段

退出码：

- `0`：有效且就绪
- `1`：配置无效
- `2`：配置有效，但存在 warning 或未就绪项

### 5.4 查看当前生效配置

```bash
agent-diva config show --format pretty
agent-diva config show --format json
```

说明：

- 输出中的 `api_key`、`token`、`secret`、`password` 会被脱敏

### 5.5 用 `config init` 走非交互初始化

```bash
agent-diva config init --provider deepseek --api-key sk-xxx
```

`config init` 复用了 `onboard` 的初始化逻辑，适合脚本化。

## 6. 查看整体状态

### 6.1 文本状态

```bash
agent-diva status
```

显示内容包括：

- 路径信息
- 默认模型 / 默认 provider
- provider 配置情况
- channel 配置情况
- doctor 健康状态
- cron job 数量
- MCP 配置数量

### 6.2 JSON 状态

```bash
agent-diva status --json
```

适合：

- GUI 消费
- 外部脚本采集
- 自动化检查

## 7. 直接发消息：`agent`

### 7.1 最简单的一次性调用

```bash
agent-diva agent --message "Hello, Agent Diva"
```

### 7.2 指定模型

```bash
agent-diva agent --model openai/gpt-4o --message "Summarize this repository"
```

### 7.3 指定 session

```bash
agent-diva agent --session cli:demo --message "remember my task"
agent-diva agent --session cli:demo --message "continue"
```

### 7.4 打开流式日志

```bash
agent-diva agent --message "analyze this" --logs
```

你会看到：

- reasoning delta
- tool start / finish
- 流式输出文本

### 7.5 禁用 markdown 风格输出

```bash
agent-diva agent --message "plain output" --no-markdown
```

### 7.6 结合显式实例路径

```bash
agent-diva \
  --config ~/.agent-diva/config.json \
  --workspace ~/my-workspace \
  agent \
  --session cli:task-001 \
  --message "plan today's work"
```

## 8. 轻量交互聊天：`chat`

`chat` 是轻量终端交互模式，比 `tui` 简单。

```bash
agent-diva chat
```

可选参数：

- `--model`
- `--session`
- `--markdown` / `--no-markdown`
- `--logs` / `--no-logs`

内置命令：

- `/quit`：退出
- `/clear`：清屏
- `/new`：新建 session
- `/stop`：停止当前 session 的运行

示例：

```bash
agent-diva chat --session cli:chat-demo --logs
agent-diva --remote chat --api-url http://127.0.0.1:3000/api
```

## 9. 终端 TUI：`tui`

```bash
agent-diva tui
agent-diva tui --model deepseek-chat
agent-diva --remote tui
```

适合：

- 全屏终端交互
- 查看 timeline
- 更强的交互体验

## 10. 启动网关：`gateway`

### 10.1 前台运行

```bash
agent-diva gateway run
```

网关会启动：

- agent loop
- manager API
- 已启用 channels
- cron service

通常本地 API 会暴露在：

```text
http://localhost:3000/api
```

### 10.2 使用特定实例启动

```bash
agent-diva --config ~/.agent-diva/config.json gateway run
```

适用场景：

- 多实例运行
- 单独调试某个环境

## 11. Provider 管理

### 11.1 查看可管理 provider 列表

```bash
agent-diva provider list
agent-diva provider list --json
```

显示内容包括：

- provider 名称
- registry 默认模型
- 是否已配置
- 当前是否 active

### 11.2 查看 provider 就绪状态

```bash
agent-diva provider status
agent-diva provider status --json
```

适合确认：

- 当前默认模型映射到哪个 provider
- 哪些 provider 缺 key
- 哪些 provider 缺 `api_base`

### 11.3 切换默认 provider

```bash
agent-diva provider set --provider deepseek --api-key sk-xxx
```

如果 registry 里该 provider 有默认模型，它会自动写入 `agents.defaults.model`。

### 11.4 显式指定模型

```bash
agent-diva provider set \
  --provider openai \
  --model openai/gpt-4o \
  --api-key sk-xxx
```

### 11.5 自定义 API Base

```bash
agent-diva provider set \
  --provider custom \
  --model my-model \
  --api-key sk-xxx \
  --api-base http://localhost:8000/v1
```

### 11.6 Provider 登录

```bash
agent-diva provider login openai
agent-diva provider login openai --json
```

当前状态：

- 这是稳定占位接口
- 当前会返回 `not_implemented`
- 未来可扩展为 OAuth / device flow

## 12. Channel 管理

### 12.1 查看 channel 状态

```bash
agent-diva channels status
agent-diva channels status --json
```

会展示：

- channel 是否启用
- 是否 ready
- 缺少哪些字段

### 12.2 Channel 登录

```bash
agent-diva channels login whatsapp
agent-diva channels login discord
```

注意：

- 不同 channel 的登录流程不同
- 某些 channel 只需要配置 token
- 某些 channel 需要额外 bridge / OAuth / 平台控制台配置

建议先运行：

```bash
agent-diva config doctor
agent-diva channels status
```

再启动 `gateway run`。

## 13. 定时任务：`cron`

### 13.1 添加按间隔执行的任务

```bash
agent-diva cron add \
  --name "heartbeat" \
  --message "check my inbox" \
  --every 600
```

### 13.2 添加 cron 表达式任务

```bash
agent-diva cron add \
  --name "weekday-standup" \
  --message "send standup reminder" \
  --cron-expr "0 9 * * 1-5" \
  --timezone "Asia/Shanghai"
```

### 13.3 添加一次性任务

```bash
agent-diva cron add \
  --name "one-shot" \
  --message "remind me to deploy" \
  --at "2026-03-12T20:00:00+08:00"
```

### 13.4 把结果投递到 channel

```bash
agent-diva cron add \
  --name "daily-report" \
  --message "generate daily report" \
  --cron-expr "0 18 * * *" \
  --deliver \
  --channel telegram \
  --to 123456789
```

### 13.5 列出任务

```bash
agent-diva cron list
agent-diva cron list --all
```

### 13.6 手动执行任务

```bash
agent-diva cron run <job_id>
agent-diva cron run <job_id> --force
```

### 13.7 启用 / 禁用任务

```bash
agent-diva cron enable <job_id> --enabled true
agent-diva cron enable <job_id> --enabled false
```

### 13.8 删除任务

```bash
agent-diva cron remove <job_id>
```

说明：

- 真正的定时执行依赖 `agent-diva gateway run`
- 只添加任务但不启动网关，任务不会自动触发

## 14. Windows Service

这是 Windows 平台专用命令组：

```bash
agent-diva service install
agent-diva service start
agent-diva service stop
agent-diva service restart
agent-diva service status
agent-diva service uninstall
```

适用场景：

- 把 gateway 作为后台服务运行
- 开机自启
- 用操作系统服务方式托管

注意：

- 这是 Windows companion service
- Linux/macOS 的 GUI service 管理走各自平台逻辑，不等同于这组 CLI

## 15. 远端模式

如果已经有一个运行中的远端 manager / gateway，可以用 `--remote`：

```bash
agent-diva --remote status
agent-diva --remote --api-url http://127.0.0.1:3000/api chat
agent-diva --remote --api-url http://127.0.0.1:3000/api agent --message "hello"
```

适合：

- GUI/CLI 分离部署
- 本机 CLI 控制远端 agent
- 自动化脚本调用

## 16. 多实例使用

推荐两种方式。

### 16.1 用不同 config 文件

```bash
agent-diva --config ~/.agent-diva/dev.json status
agent-diva --config ~/.agent-diva/prod.json gateway run
```

### 16.2 用不同 config 目录

```bash
agent-diva --config-dir ~/.agent-diva-dev status
agent-diva --config-dir ~/.agent-diva-prod gateway run
```

建议：

- 每个实例单独的 config 目录
- 每个实例单独的 workspace
- 不要让多个实例共享同一个 cron store

## 17. 环境变量

支持环境变量覆盖，常见场景：

```bash
AGENT_DIVA__AGENTS__DEFAULTS__MODEL=openai/gpt-4o
OPENAI_API_KEY=sk-xxx
ANTHROPIC_API_KEY=sk-ant-xxx
```

建议用途：

- CI/CD
- 临时切换 provider
- 不把敏感信息直接写入配置文件

## 18. 常见工作流

### 18.1 最短本地启动路径

```bash
agent-diva onboard
agent-diva config doctor
agent-diva agent --message "Hello"
```

### 18.2 本地守护型使用

```bash
agent-diva onboard
agent-diva config doctor
agent-diva gateway run
```

### 18.3 切换 provider 后验证

```bash
agent-diva provider set --provider deepseek --api-key sk-xxx
agent-diva provider status
agent-diva config doctor
agent-diva agent --message "test"
```

### 18.4 加一个定时任务

```bash
agent-diva cron add --name "nightly" --message "summarize today's work" --cron-expr "0 22 * * *"
agent-diva cron list
agent-diva gateway run
```

## 19. 排障

### 19.1 `config doctor` 返回退出码 2

表示：

- 配置合法
- 但还没完全 ready

常见原因：

- provider 缺 `api_key`
- `custom` provider 缺 `api_base`
- 启用的 channel 缺字段
- workspace 目录不存在

先看：

```bash
agent-diva config doctor --json
agent-diva provider status --json
agent-diva channels status --json
```

### 19.2 `No provider found for model`

表示模型 ID 无法映射到 registry 中的 provider。

解决方式：

- 改用 registry 支持的模型 ID
- 或显式 `provider set --provider ... --model ...`

### 19.3 `chat` / `agent` 没有连续上下文

检查是否复用了同一个 `--session`：

```bash
agent-diva agent --session cli:demo --message "remember this"
agent-diva agent --session cli:demo --message "what did I just say?"
```

### 19.4 cron 已添加但没有执行

检查：

- 是否启动了 `agent-diva gateway run`
- 任务是否被 disable
- 时间表达式和时区是否正确

### 19.5 Windows 下 `service` 命令不可用

检查：

- 是否在 Windows
- 是否有权限安装服务
- 当前是否使用正确实例目录

## 20. 建议的日常命令

```bash
agent-diva config doctor
agent-diva status
agent-diva provider status
agent-diva channels status
agent-diva agent --message "hello"
agent-diva chat
agent-diva gateway run
agent-diva cron list --all
```

## 21. 相关文档

- README: `README.md`
- 命令契约: `commands/commands.md`
- 开发文档: `docs/dev/development.md`
- 架构文档: `docs/dev/architecture.md`

