# Agent Diva 沙箱配置参考

本文档详细说明 `agent-diva-sandbox` 的配置选项和配置方法。

## 配置位置

沙箱配置位于 `~/.agent-diva/config.json` 的 `sandbox` 部分：

```json
{
  "sandbox": {
    "mode": "workspace-write",
    "windows_level": "restricted-token",
    "network_access": false,
    "approval_policy": "on-failure",
    "writable_roots": [],
    "protected_paths": [],
    "timeout": 60
  }
}
```

## 配置选项详解

### mode (沙箱模式)

控制沙箱的整体行为级别。

| 值 | 说明 | 安全级别 |
|----|------|----------|
| `danger-full-access` | 无任何限制，直接执行 | 危险 |
| `read-only` | 完全只读，禁止所有写入操作 | 高 |
| `workspace-write` | 可写工作目录，其他只读（默认） | 中 |

**推荐配置**：

- 生产环境：`workspace-write`
- 开发调试：`read-only` 或 `workspace-write`
- 受信任环境：`danger-full-access`（需谨慎）

### windows_level (Windows 沙箱级别)

仅在 Windows 平台生效，控制 Windows 特定的隔离机制。

| 值 | 说明 |
|----|------|
| `disabled` | 禁用 Windows 特定隔离 |
| `restricted-token` | 使用 CreateRestrictedToken API |
| `elevated` | 提升权限模式（未实现） |

**默认值**：`disabled`

**注意**：`restricted-token` 模式使用 LUA_TOKEN 和 WRITE_RESTRICTED 标志创建受限令牌。

### network_access (网络访问)

控制沙箱内进程的网络访问权限。

| 值 | 说明 |
|----|------|
| `true` | 允许网络访问 |
| `false` | 禁止网络访问（默认） |

**平台行为**：

- **Windows**: 影响进程网络能力
- **Linux**: 使用 `--unshare-net` (bubblewrap) 或 Seccomp 网络过滤
- **macOS**: Seatbelt 网络策略

### approval_policy (审批策略)

控制何时需要用户审批才能执行命令。

| 值 | 说明 | 触发时机 |
|----|------|----------|
| `never` | 从不请求审批 | 直接执行 |
| `on-failure` | 沙箱执行失败后请求（默认） | 失败时 |
| `on-request` | LLM 决定何时请求 | 按需 |
| `unless-trusted` | 仅对未信任命令请求 | 检查信任度 |

**行为详解**：

- `never`: 
  - 直接执行所有命令
  - 仍会检查 ExecPolicy Forbidden 规则
  - Guardian 熔断器仍生效

- `on-failure`:
  - 先尝试沙箱执行
  - 失败后检查升级条件
  - 符合条件则请求审批重试

- `on-request`:
  - 等待 LLM 明确请求审批
  - 每个命令需单独请求

- `unless-trusted`:
  - 检查命令是否匹配 Allow 规则
  - 未匹配则请求审批

### writable_roots (可写根目录)

指定额外的可写目录列表。

```json
{
  "writable_roots": [
    "/home/user/projects",
    "/tmp/build"
  ]
}
```

**特性**：

- 每个目录自动添加保护子路径
- 支持绝对路径和相对路径
- WorkspaceWrite 模式默认包含 cwd

**保护子路径**（自动添加）：

- `.git` - 版本控制元数据
- `.diva` - Agent Diva 配置
- `.agents` - Agent 配置目录
- `.env` - 环境变量文件

### protected_paths (保护路径)

指定禁止写入的 glob 模式列表。

```json
{
  "protected_paths": [
    ".git",
    ".diva",
    "*.pem",
    "*.key",
    "*.secret",
    "credentials.json"
  ]
}
```

**默认值**：

```
[".git", ".diva", ".agents", ".env", "*.pem", "*.key", "*.secret"]
```

### timeout (超时时间)

命令执行的最大等待时间（秒）。

| 值 | 说明 |
|----|------|
| `60` | 默认值，60 秒 |
| 自定义 | 任意正整数 |

**注意**：超时后返回 `SandboxError::Timeout`。

## 环境变量覆盖

### AGENT_DIVA_SANDBOX_DISABLED

完全禁用沙箱，相当于 `mode: danger-full-access`。

```bash
# 禁用沙箱
export AGENT_DIVA_SANDBOX_DISABLED=1

# 或
export AGENT_DIVA_SANDBOX_DISABLED=true
```

**优先级**：环境变量 > config.json > 默认值

## ExecPolicy 规则文件

### 文件位置

默认位置：`.diva/exec.rules`

### TOML 格式

```toml
# .diva/exec.rules

# Allow 规则 - 自动允许执行
[[prefix_rules]]
pattern = ["git", "status"]
decision = "Allow"

[[prefix_rules]]
pattern = ["cargo", "build"]
decision = "Allow"

# Prompt 规则 - 需要用户审批
[[prefix_rules]]
pattern = ["git", "checkout"]
decision = "Prompt"

[[prefix_rules]]
pattern = ["npm", "install"]
decision = "Prompt"

# Forbidden 规则 - 禁止执行
[[prefix_rules]]
pattern = ["rm", "-rf"]
decision = "Forbidden"

[[prefix_rules]]
pattern = ["sudo"]
decision = "Forbidden"
```

### 规则评估

规则按顺序评估，匹配第一个：

```
command: ["git", "status", "--short"]
    │
    ├─ 检查 ["git", "status"] → 匹配 Allow → 允许
    │
    └ 检查 ["git"] → 未匹配（已返回）
```

### 决策类型

| 类型 | 行为 |
|------|------|
| `Allow` | 直接执行，无需审批 |
| `Prompt` | 需要用户审批 |
| `Forbidden` | 禁止执行 |

## Guardian 配置

Guardian 自动审批系统有独立的配置：

### 配置选项

```rust
GuardianConfig {
    max_consecutive_rejections: 5,     // 熔断器阈值
    rejection_window_secs: 60,         // 熔断器时间窗口
    auto_approve_known_safe: true,     // 自动审批已知安全命令
    auto_approve_read_only: false,     // 自动审批只读命令
    enable_auto_learning: true,        // 从审批中学习规则
    min_execution_time_for_approval_ms: 100, // 最小执行时间
}
```

### 预设配置

| 预设 | 说明 |
|------|------|
| `default()` | 平衡配置 |
| `strict()` | 严格配置，无自动审批 |
| `liberal()` | 宽松配置，更多自动审批 |

### 熔断器

当 `max_consecutive_rejections` 个拒绝在 `rejection_window_secs` 秒内发生时，熔断器触发：

- 阻止所有 AutoApprove 决策
- 所有 Defer 变为 RequireApproval
- 成功审批后自动重置

## API 配置

通过 `agent-diva-manager` HTTP API 配置：

### GET /api/sandbox

获取当前沙箱配置：

```json
{
  "mode": "workspace-write",
  "network_access": false,
  "approval_policy": "on-failure",
  "writable_roots": [...],
  "protected_paths": [...]
}
```

### PUT /api/sandbox

更新沙箱配置：

```json
{
  "mode": "read-only",
  "network_access": true
}
```

### DELETE /api/sandbox/approvals

清除审批缓存：

```json
{
  "cleared": 15
}
```

## GUI 配置

通过 `agent-diva-gui` 的 `SandboxSettings.vue` 配置：

### 策略模式选择

- 卡片式选择界面
- 三种模式直观展示
- 安全级别可视化

### 高级配置

- 可写目录列表编辑
- 保护路径 glob 配置
- 审批策略下拉选择
- 网络访问开关

## 配置迁移

从旧版 `restrict_to_workspace` 配置迁移：

| 旧配置 | 新配置 |
|--------|--------|
| `restrict_to_workspace: true` | `sandbox.mode: "workspace-write"` |
| `restrict_to_workspace: false` | `sandbox.mode: "danger-full-access"` |

迁移脚本参考：`agent-diva-migration`

## 配置示例

### 安全配置（生产环境）

```json
{
  "sandbox": {
    "mode": "read-only",
    "network_access": false,
    "approval_policy": "unless-trusted",
    "protected_paths": [
      ".git", ".diva", "*.pem", "*.key",
      "credentials.json", ".env"
    ],
    "timeout": 120
  }
}
```

### 开发配置

```json
{
  "sandbox": {
    "mode": "workspace-write",
    "network_access": true,
    "approval_policy": "on-failure",
    "writable_roots": ["./build", "./dist"],
    "timeout": 60
  }
}
```

### CI/CD 配置

```json
{
  "sandbox": {
    "mode": "danger-full-access",
    "approval_policy": "never",
    "timeout": 300
  }
}
```

**环境变量方式**：

```bash
export AGENT_DIVA_SANDBOX_DISABLED=1
```

## 故障排查

### 配置加载失败

检查 JSON 格式有效性：

```bash
cat ~/.agent-diva/config.json | python -m json.tool
```

### 规则文件解析失败

检查 TOML 格式：

```bash
cat .diva/exec.rules | python -c "import tomllib; tomllib.load(open('exec.rules', 'rb'))"
```

### 沙箱不生效

1. 检查环境变量：`echo $AGENT_DIVA_SANDBOX_DISABLED`
2. 检查配置模式：确保非 `danger-full-access`
3. 检查平台支持：Linux 需要 kernel 5.13+ 和 bwrap

### Linux bwrap 不可用

安装 bubblewrap：

```bash
# Debian/Ubuntu
apt install bubblewrap

# Fedora
dnf install bubblewrap

# Arch
pacman -S bubblewrap
```

### macOS sandbox-exec 不可用

确保 macOS 版本 >= 10.7，sandbox-exec 默认安装。

### WSL1 沙箱警告

WSL1 不支持 bubblewrap，建议升级到 WSL2：

```bash
wsl --set-version <distro> 2
```