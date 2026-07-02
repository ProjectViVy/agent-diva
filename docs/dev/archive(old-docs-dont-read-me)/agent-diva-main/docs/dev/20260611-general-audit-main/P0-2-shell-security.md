# P0-2: Shell 工具默认无安全限制

## 问题描述

`agent-diva-tools/src/shell.rs` 的 `ExecTool::new` 默认将 `restrict_to_workspace` 设为 `false`：

```rust
pub fn new() -> Self {
    Self {
        timeout_secs: 60,
        working_dir: None,
        deny_patterns: Self::default_deny_patterns(),
        allow_patterns: Vec::new(),
        restrict_to_workspace: false,
    }
}
```

同一文件中只有当 `restrict_to_workspace` 为 `true` 时，才会检查路径穿越和绝对路径是否逃逸当前工作目录。默认关闭意味着 Shell 工具只依赖黑名单正则拦截少数危险命令。黑名单无法覆盖 PowerShell/cmd/bash 的组合语法、别名、变量展开、间接调用和平台差异。

`agent-diva-agent/src/agent_loop.rs` 的 `ToolConfig::default` 也将 `restrict_to_workspace` 设为 `false`，并在构建 `SubagentManager` 和 `ToolAssembly` 时把该值继续向下传递。也就是说，默认 AgentLoop 与默认 ExecTool 在未显式配置时都会 fail open。

```rust
impl Default for ToolConfig {
    fn default() -> Self {
        Self {
            exec_timeout: 60,
            restrict_to_workspace: false,
            // ...
        }
    }
}
```

## 影响评估

- 安全影响：LLM 获得 Shell 工具后可默认在用户权限范围内访问工作区外文件、执行系统命令、调用网络工具或读取敏感路径。
- 防护绕过：默认 denylist 只能识别固定危险模式，不能可靠阻止 `powershell -EncodedCommand`、脚本文件间接执行、路径拼接、环境变量展开等变体。
- 配置风险：安全依赖调用方主动开启 `restrict_to_workspace`，但默认路径没有安全基线，容易在 CLI、GUI、子代理或测试入口遗漏。
- 审计风险：不同构造路径的默认策略不一致时，后续新增入口可能再次引入不受限 Shell。

## 解决方案

将 Shell 默认策略改为 fail closed：默认限制在 workspace 内，并要求显式配置才能放宽。建议同步修改工具默认值和 AgentLoop 默认值。

示例修改方向：

```rust
impl ExecTool {
    pub fn new() -> Self {
        Self {
            timeout_secs: 60,
            working_dir: None,
            deny_patterns: Self::default_deny_patterns(),
            allow_patterns: Vec::new(),
            restrict_to_workspace: true,
        }
    }
}

impl Default for ToolConfig {
    fn default() -> Self {
        Self {
            exec_timeout: 60,
            restrict_to_workspace: true,
            // ...
        }
    }
}
```

同时建议补充配置层语义：

```rust
pub struct ToolConfig {
    /// Default true. Set false only for trusted local automation.
    pub restrict_to_workspace: bool,
}
```

更完整的修复应包括：

- 默认启用 workspace 限制。
- 把允许放宽的配置字段命名为显式风险选项，例如 `allow_workspace_escape`。
- 对绝对路径、重定向、脚本文件参数、PowerShell encoded command 增加专项测试。
- 在 CLI/GUI 配置界面中明确显示 Shell 当前安全模式。

## 验证方法

执行：

```powershell
cargo test -p agent-diva-tools shell
cargo test -p agent-diva-agent tool_config
just fmt-check
just check
```

预期结果：

- `ExecTool::default()` 和 `ToolConfig::default()` 的 `restrict_to_workspace` 均为 `true`。
- 工作区外绝对路径命令被拒绝。
- 包含 `../` 或 `..\\` 的路径穿越命令被拒绝。
- 显式配置关闭限制时，旧兼容路径仍可按预期运行，并有测试覆盖。

## 优先级

P0
