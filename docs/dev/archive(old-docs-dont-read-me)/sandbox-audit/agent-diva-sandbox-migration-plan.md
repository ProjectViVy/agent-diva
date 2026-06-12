# Agent-Diva Sandbox 集成审查与迁移分析

> 审查日期: 2026-06-01  
> 源分支: `agent-diva-with-sandbox`  
> 目标分支: `main`  
> 合并基础: `50f58c2` (sandbox 分支 HEAD 即基于 main 最新)  

---

## 一、改动规模总览

| 类别 | 文件数 | 新增行 | 说明 |
|------|--------|--------|------|
| 新 crate | 15 (.rs) + 1 (.toml) | ~4500 | `agent-diva-sandbox` 完整沙箱引擎 |
| core 配置 | 1 | +134 | SandboxConfig 等 4 个新类型 |
| tools/shell | 1 | +341 | ExecTool 三层执行架构改造 |
| agent 循环 | 3 | +87 | ToolConfig 扩展, subagent 穿线 |
| manager | 7 | +310 | 沙箱 API + 审批缓存管理 |
| GUI (Vue/Tauri) | 7 | +230 | 沙箱设置页面 + i18n |
| CLI | 2 | +2 | ToolConfig 初始化适配 |
| 根 Cargo | 2 | +10 | workspace 成员 + globset |
| filesystem | 1 | +2 | 未使用变量修复 |
| **合计** | **~40** | **~5300** | |

未跟踪新文件: `agent-diva-sandbox/` (整个 crate), `SandboxSettings.vue`, `sandbox.ts`, `docs/sandbox.md`, `docs/security.md`, `scripts/sandbox-smoke.ps1`

---

## 二、逐模块 Diff 摘要

### 2.1 agent-diva-sandbox/ (全新 crate, 15 个 Rust 源文件)

```
agent-diva-sandbox/
├── Cargo.toml          # 依赖: tokio, serde, tracing, globset, shell-words, toml, parking_lot
│                       # 平台: windows crate, landlock(linux), seccompiler(linux)
├── src/
│   ├── lib.rs          # crate 入口, re-export 全部公开 API
│   ├── manager.rs      # SandboxManager — 核心执行调度器 (624行)
│   ├── orchestrator.rs # ToolOrchestrator — 审批→沙箱→重试编排引擎 (888行)
│   ├── approval.rs     # ApprovalStore, CommandApprovalKey, ReviewDecision (326行)
│   ├── policy.rs       # SandboxPolicy, SandboxMode, AskForApproval 等 (196行)
│   ├── filesystem.rs   # FileSystemSandboxPolicy, 全局/路径访问控制 (445行)
│   ├── error.rs        # SandboxError 枚举 (11 种错误) + SandboxResult
│   ├── exec_policy.rs  # ExecPolicyManager 规则引擎 + 黑名单前缀
│   ├── guardian.rs     # GuardianManager 自动审批系统
│   ├── decision.rs     # Decision/Evaluation 决策类型
│   ├── rules.rs        # Policy/PrefixRule/RulesFile 规则定义
│   └── platform/
│       ├── mod.rs      # SandboxType 枚举
│       ├── windows.rs  # Windows Restricted Token 沙箱
│       ├── linux.rs    # Linux Landlock/Seccomp 沙箱
│       └── macos.rs    # macOS 沙箱 (占位)
```

**公开 API 清单**:
- `SandboxManager` — 沙箱管理器 (创建、执行、审批检查)
- `ToolOrchestrator` — 工具编排器 (审批→沙箱→重试完整流)
- `ApprovalStore` / `SharedApprovalStore` — 审批缓存
- `SandboxPolicy` / `SandboxMode` / `AskForApproval` — 策略枚举
- `SandboxExecRequest` — 执行请求
- `SandboxError` — 错误类型

**迁移判定**: ✅ 可直接迁入 (全新代码，main 分支无冲突)

---

### 2.2 agent-diva-core/src/config/schema.rs (+134 行, 文件末尾追加)

新增 4 个类型，全部追加在文件末尾 `ExecToolConfig` 之后:

```rust
// 新增枚举
SandboxMode        { DangerFullAccess, ReadOnly, WorkspaceWrite }
AskForApproval     { Never, OnFailure, OnRequest, UnlessTrusted }
WindowsSandboxLevel { Disabled, RestrictedToken, Elevated }

// 新增结构体
SandboxConfig {
    mode, network_access, approval_policy, windows_level,
    writable_roots, protected_paths, deny_patterns, timeout
}
```

`ToolsConfig` 新增字段: `pub sandbox: SandboxConfig`

便捷方法: `danger_full_access()`, `workspace_write()`, `is_disabled()`

`is_disabled()` 逻辑:
1. `mode == DangerFullAccess` → disabled
2. 环境变量 `AGENT_DIVA_SANDBOX_DISABLED=1|true` → disabled

**迁移判定**: ✅ 可直接迁入 (纯追加, 无冲突风险)

---

### 2.3 agent-diva-tools/src/shell.rs (+341 行, 重点审查)

这是改动最大的单个文件，是迁移的核心难点。

**主线版本**: 438 行, 简单两层执行  
**沙箱版本**: 773 行, 复杂三层执行 + 审批 + 重试

#### 改动点

| 改动 | 行数 | 说明 |
|------|------|------|
| 新增 imports | +10 | SandboxConfig, SandboxManager, ToolOrchestrator, ApprovalStore, Arc |
| 结构体字段 | +3 | sandbox_manager, tool_orchestrator, approval_policy |
| 新构造函数 | +93 | with_sandbox, with_shared_sandbox, with_orchestrator, with_orchestrator_and_policy |
| setter 方法 | +18 | set_sandbox_manager, set_orchestrator, set_approval_policy |
| 查询方法 | +18 | is_sandbox_enabled, has_sandbox_manager, is_orchestrator_enabled |
| execute() 改造 | +30 | 三层优先级: Orchestrator → SandboxManager → Direct |
| execute_with_orchestrator() | +33 | 新方法, 编排器执行入口 |
| 新测试 | +125 | 14 个新测试函数 |

#### 执行优先级架构

```
execute(params)
  ├─ guard_command()           ← 第一层: 模式匹配 (原有, 未变)
  ├─ [1] ToolOrchestrator      ← 新增: 审批→沙箱→重试全流
  ├─ [2] SandboxManager        ← 新增: 纯沙箱执行 (无编排)
  └─ [3] Direct execution      ← 原有: 直接执行 (fallback)
```

#### 重要设计决策

1. **沙箱失败不回退到直接执行** — 测试 `test_exec_does_not_fallback_to_direct_execution_after_sandbox_failure` 明确验证了这一点
2. **SandboxConfig → SandboxConfig 转换**: 通过 `from_core_config()` 桥接 core 与 sandbox crate 的配置类型
3. **restrict_to_workspace 自动跟随**: `with_shared_sandbox` 中当 sandbox 启用时自动设为 true

**迁移判定**: ⚠️ 需适配迁入 (冲突风险中等)

**冲突点分析**:
- 主线 `ExecTool::new()` / `with_config()` 签名与沙箱版一致 (只是新增了字段默认值)
- 主线 `execute()` 方法结构简单; 沙箱版插入三层优先级分支
- 主线没有 `sandbox_manager` / `tool_orchestrator` / `approval_policy` 字段
- `agent-diva-tools/Cargo.toml` 需新增 `agent-diva-sandbox` 依赖

---

### 2.4 agent-diva-agent/src/agent_loop.rs (+43 行)

#### 改动点

| 位置 | 改动 | 说明 |
|------|------|------|
| imports | +2 | SandboxConfig, SharedApprovalStore |
| ToolConfig 结构体 | +2 字段 | sandbox, approval_store |
| ToolConfig::default() | +2 | 初始化新字段为 danger_full_access + 空审批缓存 |
| ToolConfig | +1 方法 | workspace_only_fs() 兼容辅助 |
| AgentLoop::new() | 参数变更 | 传递 SandboxConfig + approval_store 给 SubagentManager |
| setup_tools() | 参数变更 | 传递 tool_config.sandbox + approval_store |
| ExecTool 创建 | **关键变更** | `ExecTool::with_config(...)` → `ExecTool::with_shared_sandbox(...)` |
| 测试 | +6 | 添加 `.await.unwrap()` 修复测试 |

#### 冲突点

1. **ToolConfig 结构体字段差异**: 主线可能有 `soul_context` 等字段扩展
2. **SubagentManager::new() 签名变更**: 从 `restrict_to_workspace: bool` → `sandbox_config: SandboxConfig, approval_store: SharedApprovalStore`
3. **测试修复**: 主线测试调用 `AgentLoop::new()` 缺少 `.await` (潜在编译错误, 沙箱分支修复了)

**迁移判定**: ⚠️ 需适配迁入 (冲突风险中等)

---

### 2.5 agent-diva-agent/src/agent_loop/loop_tools.rs (+7 行)

简单变更:
- `tool_config.restrict_to_workspace` → `tool_config.workspace_only_fs()` (1 处)
- `ExecTool::with_config(...)` → `ExecTool::with_shared_sandbox(...)` (1 处)

**迁移判定**: ✅ 可直接迁入 (低风险)

---

### 2.6 agent-diva-agent/src/subagent.rs (+37 行)

#### 改动点

| 位置 | 改动 |
|------|------|
| imports | +2 (SandboxConfig, SharedApprovalStore) |
| SubagentManager 结构体 | `restrict_to_workspace: bool` → `sandbox_config: SandboxConfig, approval_store: SharedApprovalStore` |
| SubagentManager::new() | 参数变更 |
| exec_subagent_task() | 参数适配 |
| build_subagent_tools() | `ExecTool::new()` → `ExecTool::with_shared_sandbox(...)`, 并恢复 exec_timeout 参数使用 |

**迁移判定**: ⚠️ 需适配迁入 (参数签名变更)

---

### 2.7 agent-diva-manager/ (7 个文件, +310 行)

| 文件 | 行数 | 改动概要 |
|------|------|----------|
| Cargo.toml | +1 | 新增 `agent-diva-sandbox` 依赖 |
| state.rs | +58 | 新增 SandboxCommand, SandboxConfigResponse, SandboxConfigUpdate |
| handlers.rs | +104 | 3 个新 handler: get/update/clear |
| server.rs | +7 | 3 个新路由: GET/PUT /api/sandbox, DELETE /api/sandbox/cache |
| manager.rs | +116 | handle_sandbox_command + 3 个 handler impl + 测试 |
| runtime.rs | +9 | approval_store 穿线: GatewayBootstrap, build_agent_loop |
| bootstrap.rs | +4 | approval_store 创建 + 传递 |
| task_runtime.rs | +2 | approval_store 解构 + 传递给 Manager::new |
| file_service.rs | +2 | 字段名修正: file_name → filename |

#### 设计要点

1. **Manager 新增字段**: `approval_store: SharedApprovalStore`
2. **审批缓存生命周期**: 在 `bootstrap.rs` 创建, 贯穿整个运行时
3. **sandbox 配置持久化**: 通过 ConfigLoader 保存到 YAML
4. **restrict_to_workspace 自动同步**: `handle_update_sandbox` 中设置 `config.tools.restrict_to_workspace = !config.tools.sandbox.is_disabled()`

**迁移判定**: ⚠️ 需适配迁入 (多文件穿线变更)

**潜在冲突**:
- `manager.rs` 中 `Manager::new()` 签名变更 (新增 approval_store 参数)
- `runtime.rs` 中 `build_agent_loop()` 签名变更
- `state.rs` 中 `ManagerCommand` 枚举新增 `Sandbox` 变体

---

### 2.8 agent-diva-gui/ (Vue/Tauri, +230 行)

| 文件 | 改动 |
|------|------|
| SettingsView.vue | 新增 SandboxSettings 子视图 |
| SettingsDashboard.vue | 新增 Shield 图标 + 沙箱入口卡 |
| commands.rs | 3 个 Tauri 命令 |
| app_state.rs | 3 个 HTTP 调用方法 |
| lib.rs | 注册 3 个 Tauri 命令 |
| en.ts / zh.ts | 沙箱设置完整 i18n (~50 条) |
| (未跟踪) SandboxSettings.vue | 完整沙箱设置页面 |
| (未跟踪) sandbox.ts | 前端 API 封装 |

**迁移判定**: 🔄 建议延期 (GUI 变更可独立 PR)

---

### 2.9 agent-diva-cli/ (+2 行)

`chat_commands.rs` + `main.rs`: ToolConfig 初始化添加 `sandbox: config.tools.sandbox.clone()`

**迁移判定**: ✅ 可直接迁入 (极低风险)

---

### 2.10 根 Cargo.toml (+6 行)

- workspace members: 添加 `"agent-diva-sandbox"`
- workspace dependencies: 添加 `globset = "0.4"`, `agent-diva-sandbox = { path = "agent-diva-sandbox" }`

**迁移判定**: ✅ 可直接迁入

---

### 2.11 agent-diva-tools/src/filesystem.rs (+2 行)

修复未使用变量警告: `temp_dir` → `_temp_dir`

**迁移判定**: ✅ 可直接迁入

---

## 三、迁移判定矩阵

| 模块 | 判定 | 理由 | 风险 |
|------|------|------|------|
| agent-diva-sandbox/ (全新) | ✅ 直接迁 | 新 crate, main 无冲突 | 低 |
| core/config/schema.rs | ✅ 直接迁 | 文件末尾纯追加 | 低 |
| Cargo.toml (根) | ✅ 直接迁 | workspace 成员追加 | 低 |
| tools/filesystem.rs | ✅ 直接迁 | 1 行变量重命名 | 极低 |
| agent-loop/loop_tools.rs | ✅ 直接迁 | 2 处调用替换 | 低 |
| agent-diva-cli/ | ✅ 直接迁 | 2 处字段初始化 | 极低 |
| tools/shell.rs | ⚠️ 适配迁 | 300+ 行结构变更 | 中 |
| agent-loop/agent_loop.rs | ⚠️ 适配迁 | ToolConfig 结构变更 | 中 |
| agent/subagent.rs | ⚠️ 适配迁 | 参数签名变更 | 中 |
| manager/ (7 文件) | ⚠️ 适配迁 | 多文件穿线 | 中 |
| agent-diva-gui/ | 🔄 延期 | 独立功能, 可后续 PR | 低 |

---

## 四、第一 PR 切片建议

### PR #1: 沙箱引擎 + 配置层 (推荐首批)

**范围**: 纯新增 + 追加型变更, 零行为影响

```
agent-diva-sandbox/                    # 整个新 crate (16 文件)
agent-diva-core/Cargo.toml             # 如需要跨 crate 类型引用则添加 dep
agent-diva-core/src/config/schema.rs   # 文件末尾追加 4 个类型
Cargo.toml                             # workspace members + globset dep
Cargo.lock                             # 自动生成
```

**验证标准**:
- `cargo build -p agent-diva-sandbox` 通过
- `cargo test -p agent-diva-sandbox` 通过
- `cargo build` 全项目编译通过 (sandbox crate 尚未被引用, 无行为影响)
- 配置序列化/反序列化测试通过

**预估工作量**: 0.5 天 (无冲突合并)

---

### PR #2: Shell 工具 + Agent 循环改造 (核心 PR)

**范围**: 主执行路径改造

```
agent-diva-tools/Cargo.toml            # 添加 agent-diva-sandbox dep
agent-diva-tools/src/shell.rs          # 完整沙箱版 shell.rs
agent-diva-tools/src/filesystem.rs     # 变量重命名修复
agent-diva-agent/Cargo.toml            # 添加 agent-diva-sandbox dep
agent-diva-agent/src/agent_loop.rs     # ToolConfig 扩展 + 执行路径变更
agent-diva-agent/src/agent_loop/loop_tools.rs  # 调用适配
agent-diva-agent/src/subagent.rs       # 参数签名变更
agent-diva-cli/src/chat_commands.rs    # ToolConfig 初始化
agent-diva-cli/src/main.rs             # ToolConfig 初始化
```

**关键注意事项**:
1. ToolConfig 新增 `sandbox` + `approval_store` 字段, 需检查与主线其他字段的兼容性
2. SubagentManager::new() 签名变更 — 所有调用点需更新
3. ExecTool 创建从 `with_config` → `with_shared_sandbox` — 确保默认行为不变
4. `workspace_only_fs()` 兼容方法 — 保留 `restrict_to_workspace` 兼容性

**验证标准**:
- `cargo build` 全项目编译通过
- `cargo test -p agent-diva-tools` 通过 (含 14 个新测试)
- `cargo test -p agent-diva-agent` 通过
- 默认配置下 (danger_full_access) 行为与主线完全一致
- 手动测试: 基本 echo/ls 命令执行

**预估工作量**: 1-2 天 (需解决合并冲突)

---

### PR #3: Manager 运行时集成

**范围**: Manager API + 审批缓存穿线

```
agent-diva-manager/Cargo.toml          # 添加 agent-diva-sandbox dep
agent-diva-manager/src/state.rs        # 新类型 + ManagerCommand 扩展
agent-diva-manager/src/handlers.rs     # 3 个新 handler
agent-diva-manager/src/server.rs       # 3 个新路由
agent-diva-manager/src/manager.rs      # 沙箱命令处理
agent-diva-manager/src/runtime.rs      # approval_store 穿线
agent-diva-manager/src/runtime/bootstrap.rs   # approval_store 创建
agent-diva-manager/src/runtime/task_runtime.rs # approval_store 传递
agent-diva-manager/src/file_service.rs # 字段名修正
```

**验证标准**:
- `cargo build -p agent-diva-manager` 通过
- 单元测试通过 (含新 sandbox_config_round_trip 测试)
- curl 测试: GET/PUT /api/sandbox, DELETE /api/sandbox/cache

**预估工作量**: 1 天

---

### PR #4: GUI 设置页面 (延期可选)

**范围**: Vue/Tauri 前端

```
agent-diva-gui/src/api/sandbox.ts              # 前端 API (新文件)
agent-diva-gui/src/components/settings/SandboxSettings.vue  # 设置页面 (新文件)
agent-diva-gui/src/components/SettingsView.vue  # 路由注册
agent-diva-gui/src/components/settings/SettingsDashboard.vue # 入口卡
agent-diva-gui/src/locales/en.ts / zh.ts        # i18n
agent-diva-gui/src-tauri/src/commands.rs        # Tauri 命令
agent-diva-gui/src-tauri/src/app_state.rs       # HTTP 客户端
agent-diva-gui/src-tauri/src/lib.rs             # 命令注册
```

**预估工作量**: 0.5 天

---

## 五、与当前主线冲突点详细分析

### 5.1 ToolConfig 结构体冲突 (agent_loop.rs)

```
主线 ToolConfig 字段:               沙箱分支 ToolConfig 字段:
├── network                         ├── network
├── exec_timeout                    ├── exec_timeout
├── restrict_to_workspace           ├── sandbox           ← NEW
├── mcp_servers                     ├── approval_store    ← NEW
├── cron_service                    ├── restrict_to_workspace  (保留但降级为兼容)
├── soul_context                    ├── mcp_servers
├── notify_on_soul_change           ├── cron_service
├── soul_governance                 ├── soul_context
                                    ├── notify_on_soul_change
                                    ├── soul_governance
```

**解决方案**: 在现有字段之间插入 `sandbox` 和 `approval_store`, 保留 `restrict_to_workspace` 兼容

### 5.2 SubagentManager::new() 签名冲突

```
主线: new(provider, workspace, bus, model, network_config, exec_timeout, restrict_to_workspace: bool)
沙箱: new(provider, workspace, bus, model, network_config, exec_timeout, sandbox_config: SandboxConfig, approval_store: SharedApprovalStore)
```

**解决方案**: 合并参数, 所有调用点更新

### 5.3 Manager::new() 签名冲突

```
主线: new(api_rx, bus, provider, loader, provider_name, model, ..., cron_service, file_manager)
沙箱: new(api_rx, bus, provider, loader, provider_name, model, ..., cron_service, file_manager, approval_store)
```

**解决方案**: 末尾追加 `approval_store` 参数, 所有调用点更新

### 5.4 Cargo.toml 依赖冲突

**风险**: `agent-diva-sandbox` crate 依赖 `globset = "0.4"` (workspace dep)。需确认全局 `Cargo.toml` 无版本冲突。

---

## 六、迁移风险清单

### 高风险

| # | 风险 | 影响范围 | 缓解措施 |
|---|------|----------|----------|
| 1 | **默认行为变更**: `ExecTool::with_shared_sandbox` 默认 `SandboxConfig::danger_full_access()` 应保持与主线 `with_config` 完全相同的行为 | shell 执行 | PR #2 中强制默认 danger_full_access, 等价于当前主线行为 |
| 2 | **审批流死锁**: `OnRequest` 模式下, 命令首次执行总是返回 `ApprovalRequired` 错误, 无 GUI 时用户无法审批 | CLI 模式 | CLI 模式默认使用 `Never` 或 `OnFailure`, 或 CLI 中添加命令行审批交互 |
| 3 | **平台支持**: Windows Restricted Token 沙箱在所有 Windows 版本上可用性未知; Linux Landlock 需要内核 5.13+ | 沙箱实际执行 | 所有平台沙箱实现中 `is_available()` 必须正确降级; 不可用时 fallback 到 error (不 fallback 到直接执行) |

### 中风险

| # | 风险 | 影响范围 | 缓解措施 |
|---|------|----------|----------|
| 4 | **依赖版本冲突**: `globset = "0.4"` 可能与 workspace 其他 crate 的 glob/globset 版本冲突 | 编译 | PR #1 中验证全项目编译 |
| 5 | **ConfigLoader 兼容**: `SandboxConfig` 新增字段在旧配置文件中反序列化为默认值, 需确认 `#[serde(default)]` 正确 | 配置 | 已有 `#[serde(default)]`, 测试旧配置文件加载 |
| 6 | **审批缓存内存泄漏**: `ApprovalStore` 无 TTL/容量上限; 长时间运行可能无限增长 | 运行时内存 | 短期可接受 (每个 Key ~100B); 后续添加 LRU |
| 7 | **测试并行冲突**: 沙箱测试使用全局环境变量 `AGENT_DIVA_SANDBOX_DISABLED`, 可能与并行测试冲突 | CI | 隔离测试或使用 `#[serial]` |

### 低风险

| # | 风险 | 影响范围 | 缓解措施 |
|---|------|----------|----------|
| 8 | **shell.rs 代码膨胀**: 从 438 行 → 773 行 (+77%), 单文件过大 | 可维护性 | 后续 PR 中将 orchestrator 集成逻辑提取到单独模块 |
| 9 | **重复类型**: `SandboxMode`/`AskForApproval`/`WindowsSandboxLevel` 在 `agent-diva-core` 和 `agent-diva-sandbox` 中重复定义, 通过 `from_core_config()` 桥接 | 类型系统 | 短期可行; 后续统一到 core 或 sandbox |
| 10 | **docs 文件未跟踪**: `docs/sandbox.md`, `docs/security.md` 是未跟踪新文件, 需在适当 PR 中提交 | 文档 | 纳入 PR #1 或单独文档 PR |

---

## 七、迁移执行计划

```
Week 1:
  Day 1-2: PR #1 沙箱引擎 + 配置层
    - 创建 feature 分支
    - 复制 agent-diva-sandbox/ 整个 crate
    - 追加 core/config/schema.rs 新类型
    - 更新根 Cargo.toml
    - cargo build && cargo test

  Day 3-5: PR #2 Shell 工具 + Agent 循环
    - 合并 PR #1 到 feature 分支
    - 移植 shell.rs 完整版本
    - 适配 agent_loop.rs ToolConfig 结构
    - 适配 subagent.rs 参数签名
    - 更新 cli 初始化
    - cargo build && cargo test --workspace
    - 手动测试基本命令执行

Week 2:
  Day 1-2: PR #3 Manager 运行时集成
    - 合并 PR #2 到 feature 分支
    - 移植 manager/ 所有变更
    - approval_store 穿线
    - cargo test -p agent-diva-manager

  Day 3-4: 集成测试 + 边界验证
    - 启动完整 agent-diva 服务
    - 测试 end-to-end 命令执行
    - 测试 sandbox 配置 CRUD
    - 测试审批缓存流程
    - 文档更新

  Day 5: PR #4 GUI (可选)
    - 移植 Vue/Tauri 变更
    - i18n 适配
```

---

## 八、关键约束与决策

1. **不 fallback 原则**: 沙箱失败后**绝不**静默降级为直接执行。这是安全红线。
2. **向后兼容**: `danger_full_access` 模式行为等价于主线无沙箱行为。
3. **环境变量紧急开关**: `AGENT_DIVA_SANDBOX_DISABLED=1` 可完全禁用沙箱。
4. **审批缓存不持久化**: `ApprovalStore` 仅在内存中, 进程重启后清空。
5. **默认审批策略**: `OnFailure` — 沙箱成功后不打扰用户, 失败时才请求绕行审批。
