# P2-9: Tool trait 双定义

## 问题描述

当前 pro 分支同时存在两个几乎相同的 Tool 抽象：

`agent-diva-tools/src/base.rs`：

```rust
#[async_trait]
pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn parameters(&self) -> Value;
    async fn execute(&self, args: Value) -> Result<String>;
    fn validate_params(&self, params: &Value) -> Vec<String> { ... }
    fn to_schema(&self) -> Value { ... }
}

pub enum ToolError { ... }
pub type Result<T> = std::result::Result<T, ToolError>;
```

`agent-diva-tooling/src/base.rs`：

```rust
#[async_trait]
pub trait Tool: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    fn parameters(&self) -> Value;
    async fn execute(&self, args: Value) -> Result<String>;
    fn validate_params(&self, params: &Value) -> Vec<String> { ... }
    fn to_schema(&self) -> Value { ... }
}

pub enum ToolError { ... }
pub type Result<T> = std::result::Result<T, ToolError>;
```

两者 API 表面几乎一致，但类型身份完全不同：`Arc<dyn agent_diva_tools::Tool>` 不能作为 `Arc<dyn agent_diva_tooling::Tool>` 使用，两个 `ToolError` 也不能直接互换。

引用关系显示主执行链已经偏向 `agent-diva-tooling`：

```text
agent-diva-agent/src/agent_loop.rs: use agent_diva_tooling::{Tool, ToolError, ToolRegistry};
agent-diva-agent/src/tool_assembly.rs: use agent_diva_tooling::{Tool, ToolError, ToolRegistry};
agent-diva-agent/src/subagent.rs: use agent_diva_tooling::ToolRegistry;
agent-diva-tools/src/filesystem.rs: use agent_diva_tooling::{Result, Tool};
agent-diva-tools/src/shell.rs: use agent_diva_tooling::{Tool, ToolError};
```

但 planning 工具仍实现旧的 `agent_diva_tools::Tool`：

```text
agent-diva-agent/src/planning/tools.rs:
impl agent_diva_tools::Tool for PlanApproveTool
impl agent_diva_tools::Tool for PlanTransitionTool
impl agent_diva_tools::Tool for PlanShowTool
```

同时 `agent-diva-tools/src/lib.rs` 仍导出自己的 `base::{Tool, ToolError}` 和 `registry::ToolRegistry`，而 `agent-diva-tooling/src/lib.rs` 也导出 `Tool`、`ToolError`、`ToolRegistry`。这会形成双注册表、双 trait object、双错误类型的维护面。

## 影响评估

可维护性影响为 P2。新增工具时开发者可能不知道应实现哪个 `Tool`，导致工具能编译但无法注册到当前 agent loop 使用的 registry。

功能风险中等。planning 工具使用 `agent_diva_tools::Tool`，而 agent loop/tool assembly 使用 `agent_diva_tooling::ToolRegistry`，两者之间需要额外适配，否则会出现工具没有出现在最终 schema 或无法执行的问题。

架构影响中等。`agent-diva-tools` 本应承载内置工具实现，`agent-diva-tooling` 本应承载共享抽象；双定义削弱了 crate 边界，后续修改 trait 方法或错误枚举时容易漏改。

## 解决方案

建议统一以 `agent-diva-tooling` 作为唯一 Tool 抽象来源，`agent-diva-tools` 只保留工具实现并重新导出 tooling 类型以兼容旧 import。

第一步，在 `agent-diva-tools/src/lib.rs` 中改为兼容性 re-export：

```rust
pub use agent_diva_tooling::{Result, Tool, ToolError, ToolRegistry};
```

随后删除或停止公开使用 `agent-diva-tools/src/base.rs` 和 `agent-diva-tools/src/registry.rs`，避免新代码继续引用旧定义。如果暂时不能删除，可以在模块上标记 deprecated：

```rust
#[deprecated(note = "use agent_diva_tooling::Tool instead")]
pub mod base;
```

第二步，迁移 planning 工具：

```rust
use agent_diva_tooling::{Tool, ToolError};

fn core_err(e: agent_diva_core::Error) -> ToolError {
    ToolError::ExecutionFailed(e.to_string())
}

#[async_trait::async_trait]
impl Tool for PlanApproveTool {
    ...
}
```

第三步，清理依赖和引用：

1. 搜索 `agent_diva_tools::Tool` 和 `agent_diva_tools::ToolError`，全部迁移到 `agent_diva_tooling` 或使用 `agent_diva_tools` 的兼容 re-export。
2. 确认 `ToolRegistry` 只有一个实际实现。
3. 更新 README 示例，避免继续展示 `agent_diva_tools::ToolRegistry` 作为权威来源。
4. 增加一个编译期集成测试，验证内置工具、MCP 工具、planning 工具都能注册进同一个 `agent_diva_tooling::ToolRegistry`。

## 验证方法

推荐命令：

```powershell
rg "agent_diva_tools::Tool|agent_diva_tools::ToolError|agent_diva_tools::ToolRegistry" -n
cargo check -p agent-diva-agent
cargo check -p agent-diva-tools
cargo test -p agent-diva-tooling
cargo test -p agent-diva-agent planning
```

预期结果：

1. 源码中不再直接实现 `agent_diva_tools::Tool`。
2. 所有工具类型最终都可作为 `Arc<dyn agent_diva_tooling::Tool>` 注册。
3. `agent-diva-tools` 对外可继续兼容导出 `Tool`，但实际类型来自 `agent-diva-tooling`。
4. planning 工具出现在 agent loop 最终 tool schema 中，并可正常执行。

## 优先级

P2
