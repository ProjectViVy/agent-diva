# P3-12: clippy 不通过

## 问题描述

pro 分支存在至少两类 clippy 问题：`unused_mut` 和 `derivable_impls`。

`agent-diva-files/src/manager.rs` 的 `FileManager::clone_ref()` 中，`cloned` 绑定被声明为 `mut`：

```rust
let mut cloned = handle.clone();
cloned.ref_count.store(new_count, Ordering::SeqCst);
```

这里调用的是 `AtomicU32`/原子字段的 `store` 方法，不需要可变绑定；`Ok(cloned)` 也只是移动所有权。因此 `mut` 是多余的。

`agent-diva-core/src/config/schema.rs` 中多个枚举手写了 `Default`，但可以使用 `#[derive(Default)]` 和 `#[default]` 标记默认 variant：

```rust
impl Default for SandboxMode {
    fn default() -> Self {
        Self::WorkspaceWrite
    }
}

impl Default for AskForApproval {
    fn default() -> Self {
        Self::UnlessTrusted
    }
}

impl Default for WindowsSandboxLevel {
    fn default() -> Self {
        Self::RestrictedToken
    }
}

impl Default for AgentMode {
    fn default() -> Self {
        Self::Normal
    }
}
```

这些问题不会改变运行时行为，但在仓库要求 `cargo clippy --all -- -D warnings` 或 `just check` 的前提下，会阻塞 CI 和发布。

## 影响评估

稳定性影响低。问题本身主要是 lint，不直接导致业务错误。

交付影响中等。仓库约定 `just check` 使用 warnings denied，clippy 不通过会阻止 PR 合入、release 验证和后续审计基线复用。

可维护性影响低到中。手写可派生的默认实现增加样板代码，也让默认 variant 不如 `#[default]` 直观。

## 解决方案

`unused_mut` 直接移除 `mut`：

```rust
let cloned = handle.clone();
cloned.ref_count.store(new_count, Ordering::SeqCst);
```

`derivable_impls` 建议改为派生默认值，并在默认枚举项上加 `#[default]`：

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum SandboxMode {
    DangerFullAccess,
    ReadOnly,
    #[default]
    WorkspaceWrite,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum AskForApproval {
    Never,
    OnFailure,
    OnRequest,
    #[default]
    UnlessTrusted,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum WindowsSandboxLevel {
    Disabled,
    #[default]
    RestrictedToken,
    Elevated,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum AgentMode {
    #[default]
    Normal,
    Assist,
}
```

修改后删除对应的手写 `impl Default` 块。需要注意保持序列化字段名和现有默认行为不变。

## 验证方法

推荐命令：

```powershell
cargo clippy -p agent-diva-files -- -D warnings
cargo clippy -p agent-diva-core -- -D warnings
just fmt-check
just check
```

预期结果：

1. `agent-diva-files/src/manager.rs` 不再报告 `unused_mut`。
2. `agent-diva-core/src/config/schema.rs` 不再报告 `derivable_impls`。
3. `Config::default()`、`SandboxConfig::default()`、`AgentMode` 默认值保持原行为。
4. `just check` 不再被这两个 lint 阻塞。

## 优先级

P3
