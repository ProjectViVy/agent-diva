# Agent-Diva-Sandbox 安全/沙箱相关文件清单

> 扫描范围：`../agent-diva-sandbox/` 项目根目录（排除 `target/` 构建产物）
> 扫描日期：2026-06-02
> 扫描方式：文件名关键词匹配 + 代码模块内容首行摘要

---

## 一、核心沙箱 crate：`agent-diva-sandbox/src/`

| 文件 | 说明 |
|------|------|
| `lib.rs` | 沙箱 crate 入口，声明所有子模块，定义沙箱整体功能边界（进程隔离、文件系统访问控制、审批缓存） |
| `policy.rs` | 沙箱策略枚举定义（`DangerFullAccess` / `ReadOnly` / `ExternalSandbox`），控制命令执行的权限级别 |
| `exec_policy.rs` | 基于规则的命令审批管理器（`ExecPolicyManager`），加载/评估 TOML 格式的命令规则，维护危险命令前缀黑名单（`BANNED_PREFIX_SUGGESTIONS`） |
| `rules.rs` | ExecPolicy 规则类型定义（`PrefixRule`），通过命令前缀匹配决定 Allow / Prompt / Forbidden |
| `decision.rs` | 决策枚举类型（`Decision`：Allow / Prompt / Forbidden），用于规则评估结果的聚合与排序 |
| `approval.rs` | 审批系统，缓存用户对命令的审批决定（`Denied` / `ApprovedOnce` / `ApprovedForSession`），提供共享审批存储 |
| `guardian.rs` | Guardian 自动审批系统，集成 `GuardianReviewer` 特征和熔断器（`GuardianRejectionCircuitBreaker`），实现安全的自动审批流 |
| `orchestrator.rs` | 工具执行编排器（`ToolOrchestrator`），协调审批 → 沙箱选择 → 执行 → 重试/升级的完整流程 |
| `manager.rs` | 沙箱管理器（`SandboxManager`），主入口，协调策略、审批和平台执行层 |
| `filesystem.rs` | 文件系统沙箱策略，定义读/写/无访问模式、受保护路径、`WritableRoot` 等文件系统级隔离规则 |
| `error.rs` | 沙箱错误类型（`SandboxError`），涵盖命令拒绝、权限不足、Token 创建失败、进程启动失败、超时等 |
| `platform/mod.rs` | 平台适配层抽象，定义 `SandboxType` 枚举（None / WindowsRestrictedToken / LinuxSeccomp / MacosSeatbelt） |
| `platform/windows.rs` | Windows 沙箱实现，使用 `CreateRestrictedToken` API（LUA_TOKEN + WRITE_RESTRICTED）实现进程隔离 |
| `platform/linux.rs` | Linux 沙箱实现，使用 Landlock LSM（文件系统访问控制）+ Seccomp-BPF（网络系统调用过滤）+ Bubblewrap（命名空间隔离） |
| `platform/macos.rs` | macOS 沙箱实现，使用 Seatbelt（sandbox-exec）+ SBPL 策略文件实现进程沙箱化 |

## 二、工具层安全相关：`agent-diva-tools/src/`

| 文件 | 说明 |
|------|------|
| `sanitize.rs` | 输入清洗/消毒模块，对工具调用参数进行安全过滤（如路径遍历防护） |
| `shell.rs` | Shell 工具实现，集成沙箱执行流程，是沙箱隔离的主要调用方 |
| `filesystem.rs` | 文件系统工具实现，包含路径访问控制逻辑 |

## 三、管理器层：`agent-diva-manager/src/`

| 文件 | 说明 |
|------|------|
| `manager.rs` | 管理器主逻辑，集成沙箱管理器进行安全的命令执行 |
| `state.rs` | 运行时状态管理，维护沙箱配置状态 |
| `handlers.rs` | HTTP 请求处理器，包含沙箱相关的 API 端点 |
| `server.rs` | HTTP 服务器，暴露沙箱控制接口 |
| `runtime/bootstrap.rs` | 运行时引导，初始化沙箱子系统 |

## 四、Agent 层集成：`agent-diva-agent/src/`

| 文件 | 说明 |
|------|------|
| `agent_loop.rs` | Agent 主循环，集成沙箱执行决策 |
| `subagent.rs` | 子代理管理，继承父代理的沙箱策略 |
| `agent_loop/loop_tools.rs` | 工具调用循环，将工具执行请求路由至沙箱层 |

## 五、GUI 前端：`agent-diva-gui/`

| 文件 | 说明 |
|------|------|
| `src/api/sandbox.ts` | 沙箱 API 客户端，与后端沙箱控制接口通信 |
| `src/components/settings/SandboxSettings.vue` | 沙箱设置面板，提供策略选择和规则配置的 UI |
| `src/components/SettingsView.vue` | 设置主视图，包含沙箱设置入口 |
| `src/components/settings/SettingsDashboard.vue` | 设置仪表盘，展示沙箱状态概览 |
| `src/locales/en.ts` | 英文本地化，包含沙箱相关翻译字符串 |
| `src/locales/zh.ts` | 中文本地化，包含沙箱相关翻译字符串 |

## 六、配置层：`agent-diva-core/src/`

| 文件 | 说明 |
|------|------|
| `config/schema.rs` | 配置 schema 定义，包含沙箱策略的序列化/反序列化结构 |

## 七、CLI：`agent-diva-cli/src/`

| 文件 | 说明 |
|------|------|
| `main.rs` | CLI 入口，解析沙箱相关命令行参数 |
| `chat_commands.rs` | 聊天命令处理，集成沙箱审批交互 |

## 八、脚本：`scripts/`

| 文件 | 说明 |
|------|------|
| `sandbox-smoke.ps1` | 沙箱冒烟测试脚本，验证沙箱基础功能是否正常 |

## 九、文档：`docs/`

| 文件 | 说明 |
|------|------|
| `docs/security.md` | 安全策略文档，描述整体安全模型和威胁模型 |
| `docs/sandbox.md` | 沙箱功能文档，说明沙箱架构、配置和使用方式 |
| `docs/config-reference.md` | 配置参考手册，包含沙箱相关配置项说明 |

## 十、审计日志：`docs/dev/sandbox-audit/`

| 文件 | 说明 |
|------|------|
| `agent-diva-sandbox-summary.md` | 沙箱审计总结 |
| `agent-diva-sandbox-test-report.md` | 沙箱测试报告 |
| `agent-diva-sandbox-code-review.md` | 沙箱代码审查 |
| `agent-diva-sandbox-migration-plan.md` | 沙箱迁移计划 |
| `agent-diva-sandbox-audit-plan.md` | 沙箱审计计划 |

---

## 架构总结

沙箱系统采用 **分层架构**：

```
Agent Loop / CLI
    ↓
ToolOrchestrator (orchestrator.rs)  ← 审批 → 重试/升级 编排
    ↓
Guardian (guardian.rs)              ← 自动审批 + 熔断保护
    ↓
ExecPolicyManager (exec_policy.rs)  ← 规则引擎 (TOML 规则)
    ↓
SandboxManager (manager.rs)         ← 策略 + 审批缓存 协调
    ↓
Platform Executor                   ← Windows RestrictedToken / Linux Landlock+Bwrap / macOS Seatbelt
```

核心设计灵感来源于 **OpenAI Codex CLI** 的沙箱架构。
