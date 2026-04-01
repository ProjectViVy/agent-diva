---
project_name: agent-diva-tools
date: 2026-03-30
module: agent-diva-tools
status: complete
parent_workspace: agent-diva
---

# agent-diva-tools

## 模块职责

提供 agent-diva 的**内置工具集**与 **`ToolRegistry`**：将 LLM 可调用的 function 以统一 `Tool` trait 暴露（名称、描述、JSON Schema 参数、`execute`），并支持与 OpenAI 风格 `to_schema` 对齐。涵盖文件读写列举、Shell 执行、子进程、定时任务、站内消息、Web 搜索/抓取、MCP 客户端封装及杂项（如 `wtf` 品牌输出）。

## 依赖与边界

- **上游**：`agent-diva-core`（如 `ErrorContext`、`MCPServerConfig`）。
- **关键库**：`reqwest`、`tokio`/`async-trait`、`rust-mcp-sdk`（stdio/SSE/streamable-http 客户端）、`which`、`regex`、`chrono`。
- **边界**：不负责编排多步 Agent 逻辑；仅单次工具调用与注册表查询。校验以 `validate_params` 为主（必填字段），非完整 JSON Schema 验证。

## 关键类型/入口

- `Tool` / `ToolError`（`base.rs`）：异步 `execute`、参数 schema、`to_schema`。
- `ToolRegistry`：`register` / `get` / `get_definitions` / `execute`（含参数校验失败时的 `ErrorContext` 日志）。
- 具体工具：`ReadFileTool`、`WriteFileTool`、`EditFileTool`、`ListDirTool`、`ExecTool`、`SpawnTool`、`CronTool`、`MessageTool`、`WebFetchTool`、`WebSearchTool`；MCP：`load_mcp_tools`、`McpSdkTool`、`probe_mcp_server` 等（`mcp_sdk.rs`）。
- `sanitize`：`sanitize_for_json`、结果与文件内容长度上限常量（防超大 JSON 与 API 400）。

## 实现约定

- **Shell（`ExecTool`）安全**：默认 **60s** `tokio::time::timeout`；内置 **危险命令正则拒绝**（`rm -rf`、`dd`、`shutdown`、fork bomb 等）；可选 **allow 列表**；`restrict_to_workspace` 时拦截 `../` 与工作区外绝对路径。扩展时优先收紧默认策略，而非放宽。
- **文件（`filesystem`）安全**：路径经 `canonicalize`，若构造时传入 `allowed_dir` 则**必须落在允许目录内**；大文件截断 + `sanitize_for_json`。部署时应对生产环境**始终设置 `allowed_dir`**，避免任意读写到系统路径。
- **Web（`web.rs`）**：URL 仅允许 `http`/`https`；`WebSearchTool` 客户端默认 **10s** 超时；注意 SSRF 风险——若上游可传 URL，应在更高层结合内网/IP 黑名单策略（本 crate 仅 scheme 校验）。
- **MCP**：stdio/SSE/HTTP 传输；工具结果经 `sanitize_json_strings`；注意子进程与远程 MCP 的**凭证与出站网络**由配置决定，勿在日志中打印密钥。

## 测试与检查

- `dev-dependencies`：`tokio-test`、`tempfile`（文件类测试）。
- `cargo test -p agent-diva-tools`、`cargo clippy -p agent-diva-tools`。

## 切勿遗漏

- 工具返回字符串前大结果应走 `sanitize_for_json` / 截断，避免控制字符与 ANSI 破坏上游 JSON。
- `ExecTool` 的正则防护**不能替代**操作系统级沙箱；高敏感环境应配合容器、专用用户与 `allowed_dir`。
- 注册新工具后需在业务侧 `register`；MCP 动态加载工具名称与 core 配置中的服务器列表保持一致。
