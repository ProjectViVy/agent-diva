# Summary

- 新增 `agent-diva-tooling` crate，下沉 `Tool` / `ToolRegistry` / `ToolError`，让工具接口层脱离 `agent-diva-tools` 的内建实现。
- 在 `agent-diva-agent` 中新增统一 `BuiltInToolsConfig` 与 `ToolAssembly`，主线 `AgentLoop` 和 `SubagentManager` 共用同一套工具装配逻辑。
- `ToolConfig` 新增 `builtin` 开关；`agent-diva-core` / `agent-diva-manager` / `agent-diva-cli` / `agent-diva-migration` 均已接通该配置。
- `.workspace/agent-diva-nano` 改为 path 依赖主仓 crate，并复用主线 `ToolAssembly` 包装层，不再自己维护一套完全独立的内建工具注册逻辑。
- `AgentLoop` 新增 `with_toolset(...)`，允许外部传入预构建的 registry；`nano` 标准模式现在走该路径，可按类别关闭 files/web/spawn 等工具。
