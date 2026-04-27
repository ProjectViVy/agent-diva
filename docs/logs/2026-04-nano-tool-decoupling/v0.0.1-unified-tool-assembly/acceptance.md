# Acceptance

1. 打开 `agent-diva-core/src/config/schema.rs`，确认 `tools.builtin.*` 提供了 filesystem/shell/web_search/web_fetch/spawn/cron/mcp/attachment 开关。
2. 打开 `agent-diva-agent/src/tool_assembly.rs`，确认主线存在统一 `ToolAssembly`，并按开关装配主 agent 与 subagent 工具。
3. 打开 `agent-diva-agent/src/agent_loop.rs`，确认存在 `AgentLoop::with_toolset(...)`，且 `with_tools(...)` 不再手写注册默认工具。
4. 打开 `.workspace/agent-diva-nano/src/agent.rs`，确认标准模式通过 `AgentLoopToolSet` 将预构建 registry 注入主线 `AgentLoop`。
5. 打开 `.workspace/agent-diva-nano/Cargo.toml`，确认其对主仓 crate 使用 path 依赖，且通过独立 `[workspace]` 保持不加入根 workspace。
6. 运行 `cargo check --manifest-path .workspace/agent-diva-nano/Cargo.toml`，确认 `nano` 可以在本地独立编译。
