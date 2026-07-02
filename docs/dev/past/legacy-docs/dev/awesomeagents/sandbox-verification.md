# agent-diva-sandbox 安全能力验证报告

> 审计日期: 2026-06-02
> 审计目标: `../agent-diva-sandbox/` (v0.4.9, 13 个 crate)
> 审计范围: Shell 命令注入防护、流式响应中断恢复

---

## 核查项 1：Shell 命令注入防护

### 结论: ⚠️ 部分实现

沙箱项目构建了**多层防御体系**，覆盖面较广，但在若干关键维度存在缺口。

### 已实现的机制

#### 1.1 命令模式黑名单（正则 deny list）

**文件**: `agent-diva-tools/src/shell.rs:225-238`

`ExecTool::default_deny_patterns()` 定义了 8 条正则黑名单：

| 模式 | 拦截目标 |
|------|----------|
| `\brm\s+-[rf]{1,2}\b` | 递归强制删除 |
| `\bdel\s+/[fq]\b` | Windows 强制删除 |
| `\brmdir\s+/s\b` | Windows 递归删除目录 |
| `\b(format\|mkfs\|diskpart)\b` | 磁盘格式化 |
| `\bdd\s+if=` | 底层磁盘写入 |
| `>\s*/dev/sd` | 直接写块设备 |
| `\b(shutdown\|reboot\|poweroff)\b` | 系统关机/重启 |
| `:\(\)\s*\{.*\};\s*:` | Fork 炸弹 |

`guard_command()` 方法（第 242-291 行）在每次执行前检查命令是否命中黑名单，命中则直接阻断。

#### 1.2 Guardian 危险命令检测

**文件**: `agent-diva-sandbox/src/guardian.rs:284-326`

`DefaultGuardianReviewer::is_potentially_dangerous()` 检测以下类别：

- **权限提升**: `sudo`, `su`, `doas`, `run0`
- **文件删除**: `rm`（任何用法）
- **解释器内联执行**: `bash -c`, `python -c`, `node -e`, `perl -e`, `ruby -e` 等
- **包管理器安装/卸载**: `npm install`, `pip install`, `cargo install` 等

危险命令不会被自动批准，需要用户审批。

#### 1.3 禁止自动建议的命令前缀

**文件**: `agent-diva-sandbox/src/exec_policy.rs:30-82`

`BANNED_PREFIX_SUGGESTIONS` 列出了 30+ 个禁止作为 Allow 规则自动学习的命令前缀，覆盖 Python、Bash、Node、Perl、Ruby、PHP、Lua、sudo、PowerShell 等。`is_banned_prefix()` 函数确保沙箱升级重试时不会绕过此限制。

#### 1.4 操作系统级沙箱隔离

**Linux** (`agent-diva-sandbox/src/platform/linux.rs`):
- **Bubblewrap (bwrap)**: 文件系统命名空间隔离 + `--unshare-net` 网络隔离（第 192-226 行）
- **Landlock LSM**: 内核级文件系统访问控制，支持 ABI V1/V2/V3（第 569-907 行）
- **Seccomp-BPF**: 网络系统调用过滤，支持 `FullBlock`/`ProxyOnly`/`AllowPorts` 模式（第 910-1070 行）

**macOS** (`agent-diva-sandbox/src/platform/macos.rs`):
- **Seatbelt (sandbox-exec)**: 动态生成 `.sbpl` 策略文件，控制文件读写、进程执行、网络访问（第 370-421 行）
- 受保护路径（`.git`, `.diva`, `.env`, `*.pem`, `*.key`）始终写拒绝

**Windows** (`agent-diva-sandbox/src/platform/windows.rs`):
- `CreateRestrictedToken` API 存在代码框架（第 143 行），但 **`is_available()` 返回 `false`**，`RestrictedToken` 和 `Elevated` 级别返回 `PlatformError`（第 72-84 行）。**Windows 沙箱当前未生效。**

#### 1.5 文件系统访问控制

**文件**: `agent-diva-sandbox/src/filesystem.rs`

`FileSystemSandboxPolicy` 提供细粒度的文件系统访问控制：
- `Restricted` / `Unrestricted` / `ExternalSandbox` 三种模式
- `ReadDenyMatcher` 使用 glob 模式拒绝读取
- 默认保护路径: `.git`, `.diva`, `.agents`, `.env`, `*.pem`, `*.key`, `*.secret`

#### 1.6 审批与熔断机制

**文件**: `agent-diva-sandbox/src/approval.rs`, `agent-diva-sandbox/src/guardian.rs`

- `ApprovalStore` 缓存用户决策（Denied / ApprovedOnce / ApprovedForSession）
- `GuardianRejectionCircuitBreaker` 在 60 秒内拒绝 5 次后触发熔断，阻止自动批准

#### 1.7 路径遍历防护

**文件**: `agent-diva-core/src/security/path.rs`

`PathValidator` 检测：空字节、`../` 路径遍历、URL 编码遍历（`..%2f`）、波浪号展开、符号链接逃逸。

**文件**: `agent-diva-tools/src/shell.rs:263-288`

`guard_command()` 阻止路径遍历（`../`, `..\`）和工作目录外的绝对路径。

### 未实现 / 存在缺口的机制

#### ❌ 缺口 1: `/dev/tcp` 和 `curl` 数据外传未拦截

当前黑名单**未覆盖**以下常见数据外传通道：
- `/dev/tcp` (Bash 内置网络)
- `curl` / `wget` 外传
- `nc` / `netcat` 反向连接
- `python -c 'import socket...'` 网络外传

**风险**: 即使文件系统被沙箱隔离，网络通道仍可能被用于数据外传（除非 Linux seccomp `FullBlock` 模式已启用）。

#### ❌ 缺口 2: Windows 沙箱未生效

`WindowsSandboxExecutor::is_available()` 始终返回 `false`。在 Windows 平台上，命令执行实质上**无操作系统级隔离**，仅依赖正则黑名单和 Guardian 审批。

#### ⚠️ 缺口 3: 黑名单可被绕过

正则黑名单存在已知绕过方式：
- 变体写法: `r\m -rf`, `"rm" -rf`, `rm -r -f`（空格变体）
- 编码绕过: Base64 编码命令 + `eval`
- 路径重写: `/bin/rm -rf`, `/usr/bin/rm -rf`
- Guardian 的解释器检测也可被绕过: `python3` 不带 `-c` 但通过 stdin 传入代码

#### ⚠️ 缺口 4: 无命令参数深度解析

当前黑名单是纯文本正则匹配，不做 shell AST 解析。复杂的嵌套命令（如 `echo cmd | bash`, `bash <<< "rm -rf /"`）可能绕过检测。

---

## 核查项 2：流式响应中断恢复

### 结论: ❌ 未实现

### 当前实际实现

#### 2.1 SSE 流式解析（基础能力）

**文件**: `agent-diva-providers/src/litellm.rs:862-1074`

`chat_stream()` 方法实现了标准的 SSE 流式消费：
- 通过 `reqwest` 发送 `stream: true` 请求
- 使用 `response.chunk().await` 逐块读取
- `parse_sse_events()` 按 `\n\n` 分割 SSE 事件
- 累积 `content`、`reasoning_content`、`partial_calls`
- 处理 `[DONE]` 哨兵，发送 `Completed` 事件
- 通过 `tokio::sync::mpsc::unbounded_channel` 推送事件

#### 2.2 finish_reason 检测（部分实现）

**文件**: `agent-diva-providers/src/litellm.rs:960,1013-1014`

流式解析过程中会跟踪 `finish_reason`：
```rust
let mut finish_reason: Option<String> = None;
// ...
if let Some(reason) = &choice.finish_reason {
    finish_reason = Some(reason.clone());
}
```

**文件**: `agent-diva-agent/src/agent_loop/loop_turn.rs:343-354`

Agent loop 检测 `finish_reason == "error"` 并输出错误提示，但**不触发重试**。

#### 2.3 流错误处理（仅上报，不恢复）

**文件**: `agent-diva-providers/src/litellm.rs:974-978`

```rust
Err(err) => {
    tracing::error!("Stream error: {}", err);
    let _ = tx.send(Err(ProviderError::HttpError(err)));
    return;  // 直接退出，不重试
}
```

流错误（网络断开、超时等）被记录后直接传播给调用方，**不尝试重连或续写**。

### 未实现的机制

#### ❌ 流中断检测与重连

当前实现中，如果 SSE 连接在传输中途断开：
- `response.chunk().await` 返回 `Err` → 直接发送错误事件并退出
- 没有检测"流是否完整"（如收到了部分 TextDelta 但未收到 `[DONE]`）
- 没有重连机制

#### ❌ 从断点续写

没有记录"已收到多少 token"或"最后一个 SSE 事件 ID"的机制。无法向 LLM API 发送续写请求。

#### ❌ 降级为非流式重试

Agent loop 收到流错误后（`ProviderError::HttpError`），不会尝试降级为非流式 `chat()` 调用重新发送相同请求。

#### ❌ finish_reason 异常重试

当 `finish_reason` 为 `"length"`（输出被截断）时，当前代码不做任何处理。理想情况下应检测截断并尝试续写。

### 沙箱分支目前实际做了什么

流式响应处理**完全不在 `agent-diva-sandbox` crate 的职责范围内**。该 crate 仅处理 Shell 命令执行的沙箱隔离。流式响应逻辑位于：

| 职责 | Crate |
|------|-------|
| SSE 流解析 | `agent-diva-providers` |
| Agent 循环消费 | `agent-diva-agent` |
| 沙箱隔离 | `agent-diva-sandbox` |

沙箱分支没有对流式响应做任何特殊处理。

---

## 总结

| 核查项 | 状态 | 说明 |
|--------|------|------|
| Shell 命令注入防护 | ⚠️ 部分实现 | 多层防御体系较完整，但 Windows 沙箱未生效、`/dev/tcp`/`curl` 外传未拦截、正则黑名单可被绕过 |
| 流式响应中断恢复 | ❌ 未实现 | 仅有基础 SSE 解析和错误上报，无重连/续写/降级重试机制 |

### 改进建议

**Shell 命令注入防护**:
1. 在黑名单中增加 `/dev/tcp`、`curl`、`wget`、`nc`/`netcat` 模式
2. 启用或实现 Windows 受限令牌沙箱（当前代码框架已存在）
3. 考虑引入 shell 语法解析（如 `shell-words` crate 已在依赖中）替代纯正则匹配
4. 对网络相关命令（`curl`, `wget`, `nc`）在 seccomp/seatbelt 层做网络隔离

**流式响应中断恢复**:
1. 在 `chat_stream()` 中增加重连逻辑（最多 N 次，指数退避）
2. 检测 `finish_reason == "length"` 并触发续写请求
3. 在 agent loop 中实现流错误 → 非流式降级重试
4. 记录已累积的 content 长度，用于续写时的上下文拼接
