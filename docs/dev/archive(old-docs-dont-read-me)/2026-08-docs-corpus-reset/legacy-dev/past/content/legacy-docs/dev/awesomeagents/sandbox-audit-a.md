# 沙箱安全审计报告 — 维度 A：沙箱隔离 + Shell 安全

**审计对象:** `agent-diva-sandbox/`（含 `agent-diva-core/src/security/`、`agent-diva-tools/src/`）
**审计日期:** 2026-06-02

---

## 核查结果总览

| # | 维度 | 状态 | 关键文件 |
|---|------|------|----------|
| 1 | 平台级沙箱隔离 | ⚠️ | `platform/windows.rs`, `platform/linux.rs`, `platform/macos.rs` |
| 2 | Shell 命令黑名单 | ✅ | `exec_policy.rs`, `shell.rs`, `guardian.rs` |
| 3 | Shell 命令审批门 | ✅ | `orchestrator.rs`, `approval.rs`, `exec_policy.rs` |
| 4 | 命令参数过滤/净化 | ✅ | `exec_policy.rs`, `shell.rs` |
| 5 | 网络出站控制 | ⚠️ | `policy.rs`, `platform/linux.rs`, `platform/macos.rs` |
| 6 | 进程资源限制 | ⚠️ | `manager.rs`, 各平台 executor |
| 7 | 文件系统访问控制 | ✅ | `filesystem.rs`, `core/security/policy.rs`, `tools/filesystem.rs` |
| 8 | 路径遍历防护 | ✅ | `core/security/path.rs`, `shell.rs` |
| 9 | 子进程环境变量过滤 | ❌ | `policy.rs`（仅 schema 存根） |

**评分：5 ✅ / 3 ⚠️ / 1 ❌**

---

## 1. 平台级沙箱隔离 — ⚠️

### Windows RestrictedToken — ❌ 未启用

- `platform/windows.rs:143` — `create_restricted_token()` 调用 `CreateRestrictedToken(DISABLE_MAX_PRIVILEGE | LUA_TOKEN | WRITE_RESTRICTED)`，但为死代码
- `is_available()` 返回 `false`，`execute()` 对 `RestrictedToken` 级别直接返回错误
- `execute_direct()` 无任何 token 限制运行

### Linux Landlock + Bwrap + Seccomp — ⚠️ 大部分实现

- **Bwrap:** `platform/linux.rs:192` — 完整的命名空间隔离（`--ro-bind`, `--bind`, `--tmpfs`, `--unshare-net`），含 WSL1 检测
- **Landlock:** `platform/linux.rs:575` — 使用 `landlock 0.4` crate 构建 `PathBeneath` 规则，ABI V3/V2/V1 自动检测
- **Seccomp-BPF:** `platform/linux.rs:950` — 使用 `seccompiler 0.4` 阻断 `connect`/`sendto`/`sendmsg`/`recvfrom`/`recvmsg` 系统调用
- **不足:** Landlock 和 Seccomp 模块已定义但未完全接入 `execute()` 路径

### macOS Seatbelt — ✅ 完整实现

- `platform/macos.rs:73` — 生成 `.sbpl` 策略（`deny network*`、`deny file-write*` 等）
- `platform/macos.rs:370` — 通过 `/usr/bin/sandbox-exec -f <policy>` 执行，临时文件自动清理

---

## 2. Shell 命令黑名单 — ✅

**三层独立防护：**

| 层 | 位置 | 机制 |
|----|------|------|
| L1 禁止前缀 | `exec_policy.rs:30-74` | 30+ 静态前缀（`sudo`, `bash -c`, `python3 -c`, `node -e` 等），`is_banned_prefix()` 检查 |
| L2 正则拒绝 | `shell.rs:225-239` | `rm -rf`, `del /f`, `format`, `mkfs`, `dd if=`, `shutdown`, fork bomb 等 |
| L3 Guardian | `guardian.rs:284-326` | 检测提权（`sudo`/`su`/`doas`）、`rm`、内联执行标志、包管理器 install/remove |

`BANNED_PREFIX_SUGGESTIONS` 不可被自动学习为 Allow 规则。

---

## 3. Shell 命令审批门 — ✅

- `orchestrator.rs:383` — 5 阶段流水线：Guardian 自动审批 → 策略审批 → 首次执行 → 沙箱拒绝处理 → 重试执行
- 4 种审批策略（`policy.rs:147-180`）：`Never` / `OnFailure` / `OnRequest` / `UnlessTrusted`
- `approval.rs` — `ApprovalStore` 缓存用户决策（`ApprovedOnce` / `ApprovedForSession` / `Denied`），按 `(command, cwd)` 键
- Guardian 熔断器（`guardian.rs:399`）— 连续 5 次拒绝后强制人工审批

---

## 4. 命令参数过滤/净化 — ✅

- **ExecPolicy 规则:** `exec_policy.rs:236` — TOML 配置的前缀规则映射 `Allow` / `Prompt` / `Forbidden`
- **工作区限制:** `shell.rs:264-288` — 阻断 `../`、`..\\`、绝对路径（Windows `C:\` 和 POSIX `/`），`canonicalize()` 后校验是否在 `cwd` 内
- **命令解析:** 多处使用 `shell_words::split()` 防止引号注入

---

## 5. 网络出站控制 — ⚠️

| 平台 | 状态 | 机制 |
|------|------|------|
| macOS | ✅ | Seatbelt `(deny network*)` / `(allow network*)` |
| Linux | ✅ | bwrap `--unshare-net` + Seccomp BPF 阻断 socket 系统调用 |
| Windows | ❌ | 无实现 |

- 策略层 `NetworkAccess` 枚举（`policy.rs:119`）默认 `Denied`
- Seccomp `ProxyOnly` 模式（检查 sockaddr 仅允许回环）标注为未完成

---

## 6. 进程资源限制 — ⚠️

- **超时:** ✅ 所有执行路径均使用 `tokio::time::timeout`，默认 60 秒，可配置
- **CPU 限制:** ❌ 无 `ulimit`/cgroup/`setrlimit`
- **内存限制:** ❌ 无实现
- `BwrapOptions` 无 CPU/内存限制字段

---

## 7. 文件系统访问控制 — ✅

**双系统防护：**

### 系统 1：沙箱文件系统策略（`sandbox/src/filesystem.rs`）

- `FileSystemSandboxPolicy` — `Restricted` / `Unrestricted` / `ExternalSandbox`
- `FileSystemSandboxEntry` — 路径映射到 `Read` / `Write` / `None`
- `WritableRoot` — 可写目录 + `read_only_subpaths`（自动保护 `.git`、`.agents`、`.diva`）
- 默认保护路径：`.git`, `.diva`, `.env`, `*.pem`, `*.key`, `*.secret`

### 系统 2：核心安全策略（`core/src/security/policy.rs`）

- 8 层路径校验流水线
- 滑动窗口速率限制（100 次/小时 Standard，20 次/小时 Paranoid）
- 文件大小限制（默认 10MB）
- 禁止扩展名（`.exe`, `.dll`, `.bat`, `.cmd`, `.sh`）
- 禁止路径（`/etc`, `/root`, `~/.ssh`, `~/.aws` 等）
- 符号链接限制（默认禁止）
- TOCTOU 安全的父目录校验

### 平台级强制

- Linux: bwrap bind mount 只读/读写
- macOS: Seatbelt 策略
- Windows: 仅逻辑检查，无 OS 级强制

---

## 8. 路径遍历防护 — ✅

`core/src/security/path.rs` 的 `PathValidator` 提供 8 层防御：

| 层 | 检查内容 |
|----|----------|
| 1 | 空字节检测 |
| 2 | `..` 父目录遍历 |
| 3 | URL 编码遍历（`..%2f`, `%2f..`, `..%5c`） |
| 4 | 波浪号展开（`~user`） |
| 5 | 绝对路径阻断（工作区模式） |
| 6 | 禁止前缀（`/etc`, `~/.ssh` 等） |
| 7 | 禁止扩展名 |
| 8 | `canonicalize()` 后路径包含检查 |

附加：`validate_no_symlink_escape()` 逐级检查父目录符号链接逃逸。Shell 工具额外在命令字符串中阻断 `../` 和绝对路径。

---

## 9. 子进程环境变量过滤 — ❌

- `SandboxCommand` 和 `SandboxExecRequest` 有 `env` 字段，但无过滤逻辑
- `policy.rs` 声明了 `exclude_tmpdir_env_var` 字段，但**从未在任何 executor 中读取或执行**
- `shell.rs:434` 的直接执行路径未调用 `cmd.env_clear()` — 子进程继承父进程完整环境变量
- 无 allowlist/denylist，无敏感变量（`API_KEY`、`HOME`、`PATH`）过滤

---

## 附加发现

| 项目 | 说明 |
|------|------|
| 沙箱 Kill Switch | `AGENT_DIVA_SANDBOX_DISABLED=1` 可完全禁用所有沙箱（`lib.rs:66`） |
| 输出净化 | `sanitize.rs` — 剥离 ANSI 转义和控制字符，截断限制 80K/60K |
| 速率限制 | `rate_limit.rs` — 滑动窗口，适用于所有文件系统工具操作 |

---

## 建议优先级

1. **P0 — 环境变量过滤:** 实现 env allowlist，子进程默认 `env_clear()` 后注入白名单变量
2. **P0 — Windows 沙箱激活:** 实现 `CreateRestrictedToken` 的实际进程创建，否则 Windows 用户完全无 OS 级保护
3. **P1 — CPU/内存限制:** Linux 使用 cgroup v2，macOS 使用 `setrlimit`，Windows 使用 Job Object
4. **P1 — Linux Landlock/Seccomp 接入:** 将已实现的 builder 模块接入 `execute()` 实际路径
5. **P2 — Windows 网络过滤:** 使用 Windows Filtering Platform (WFP) 或绑定回环代理
6. **P2 — Kill Switch 审计:** 考虑限制 `AGENT_DIVA_SANDBOX_DISABLED` 仅在开发环境生效
