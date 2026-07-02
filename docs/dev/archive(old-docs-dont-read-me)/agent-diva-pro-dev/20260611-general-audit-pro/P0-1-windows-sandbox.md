# P0-1: Windows 沙箱未实现

## 问题描述

`agent-diva-sandbox/src/platform/windows.rs` 中 `WindowsSandboxExecutor::is_available()` 对所有 Windows 沙箱级别都返回 `false`：

```rust
pub fn is_available(&self) -> bool {
    match self.level {
        WindowsSandboxLevel::Disabled => false,
        WindowsSandboxLevel::RestrictedToken => false,
        WindowsSandboxLevel::Elevated => false,
    }
}
```

同一文件的 `execute()` 也没有实现真正的受限进程创建：`RestrictedToken` 分支直接返回 `"Windows restricted-token sandbox is disabled because real restricted-process creation is not implemented"`，`Elevated` 分支直接返回 `"Windows elevated sandbox is not implemented"`。虽然文件下方已有 `create_restricted_token()`，并调用了 `OpenProcessToken` 和 `CreateRestrictedToken`，但它目前没有被执行路径使用，也没有配套 `CreateProcessAsUserW`/Job Object/句柄清理/文件系统策略约束。

`agent-diva-sandbox/src/manager.rs` 的调用链进一步放大了这个问题。`SandboxManager::execute_sandboxed()` 在非 disabled 模式下最终进入 `execute_with_platform()`；Windows 分支会先构造 `WindowsSandboxExecutor`，然后检查 `executor.is_available()`。由于该方法恒为 `false`，所有配置为 `ReadOnly` 或 `WorkspaceWrite` 的 Windows 沙箱执行都会返回 `SandboxError::PlatformError("Windows sandbox is unavailable for the configured level")`，无法获得真实隔离。

`agent-diva-sandbox/src/orchestrator.rs` 的 `ToolOrchestrator::run()` 会在沙箱失败后进入 `retry_after_sandbox_failure()`。当审批策略允许失败后升级且已有会话批准时，执行会转入 `execute_unsandboxed()`，也就是直接调用宿主 PowerShell。`agent-diva-sandbox/src/guardian.rs` 的 Guardian 机制只负责自动审批、拒绝和熔断，它不能替代 OS 级沙箱；如果 Guardian 自动批准或用户已批准重试，最终仍可能在当前用户权限下运行命令。

## 影响评估

安全影响为 P0。Windows 是该项目的重点运行环境之一，默认 `WindowsSandboxLevel::RestrictedToken` 但实际不可用，会造成配置语义和运行时安全边界不一致。

稳定性影响较高。需要沙箱的命令在 Windows 上会失败并触发审批/升级分支，用户体验表现为“配置了沙箱但无法执行”，或者在批准后降级为无沙箱执行。

可维护性影响较高。当前代码已经声明了 `RestrictedToken`、`Elevated`、`create_restricted_token()` 和 Guardian/Orchestrator 流程，容易让后续开发误判为 Windows 沙箱已具备最小可用能力。

## 解决方案

第一阶段实现最小可用 `RestrictedToken`，并明确 `Elevated` 暂不支持。建议目标是：`Disabled` 返回不可用，`RestrictedToken` 在可创建 restricted token 且可启动子进程时返回可用，`Elevated` 继续返回不可用或改名为未来能力。

核心修改方向：

```rust
pub fn is_available(&self) -> bool {
    match self.level {
        WindowsSandboxLevel::Disabled => false,
        WindowsSandboxLevel::RestrictedToken => {
            unsafe {
                match self.create_restricted_token() {
                    Ok(token) => {
                        let _ = CloseHandle(token);
                        true
                    }
                    Err(_) => false,
                }
            }
        }
        WindowsSandboxLevel::Elevated => false,
    }
}
```

`RestrictedToken` 的 `execute()` 不应继续返回 `PlatformError`，而应：

1. 使用当前 `create_restricted_token()` 生成受限 token。
2. 使用 `CreateProcessAsUserW` 或等价 Windows API 启动 `powershell -NoProfile -NonInteractive -Command <command>`。
3. 将进程放入 Job Object，设置进程树清理、超时后终止、可选 CPU/内存限制。
4. 将 stdout/stderr 管道化，保持现有 `SandboxResult<String>` 行为。
5. 依据 `FileSystemSandboxPolicy` 至少阻断明显越界的工作目录和写入根；如果不能在 Windows ACL/AppContainer 层面实现完整文件系统策略，需要在文档和错误信息中明确限制。
6. 所有 `HANDLE` 使用 RAII 包装或显式 `CloseHandle`，避免泄漏。

建议同步调整 Orchestrator 策略：Windows 沙箱不可用时，不应自动把“平台未实现”当作普通沙箱拒绝来升级，除非用户明确确认“无沙箱执行”。可以将 `PlatformError` 区分为 `SandboxUnavailable`，在 UI/CLI 层显示高风险提示。

## 验证方法

推荐命令：

```powershell
cargo test -p agent-diva-sandbox
cargo test -p agent-diva-sandbox test_restricted_token_execution
cargo clippy -p agent-diva-sandbox -- -D warnings
```

Windows 手工烟测：

```powershell
cargo test -p agent-diva-sandbox test_on_failure_retry_requires_cached_approval
```

预期结果：

1. `WindowsSandboxExecutor::new(WindowsSandboxLevel::RestrictedToken).is_available()` 在支持 Windows API 的环境中返回 `true`。
2. `RestrictedToken` 执行 `echo hello` 成功返回输出。
3. 未批准的沙箱失败不会静默降级为当前用户权限执行。
4. 超时命令会被终止，子进程不会残留。

## 优先级

P0
