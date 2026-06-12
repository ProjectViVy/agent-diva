# agent-diva-sandbox Crate 架构审查报告

> 审查日期：2026-06-01
> 审查范围：policy.rs, orchestrator.rs, manager.rs, approval.rs, decision.rs, guardian.rs, lib.rs, error.rs, filesystem.rs, platform/mod.rs（以及引用到的 exec_policy.rs, rules.rs）
> 审查维度：模块职责、数据流、状态机、耦合度、错误处理、公共API

---

## 一、各模块详细审查

---

### 1. policy.rs — Sandbox策略定义

**职责描述**：
定义沙箱执行策略的核心类型，包括 SandboxPolicy（执行限制级别）、SandboxMode（配置友好的模式枚举）、ReadOnlyAccess（文件系统只读级别）、NetworkAccess（网络访问）、AskForApproval（审批策略）、WindowsSandboxLevel（Windows 沙箱级别）。

**关键类型/函数**：
- `SandboxPolicy` enum：DangerFullAccess / ReadOnly / ExternalSandbox / WorkspaceWrite
- `SandboxMode` enum：配置层抽象，提供 `to_policy(workspace)` 转换
- `AskForApproval` enum：Never / OnFailure / OnRequest / UnlessTrusted，附带 `should_ask_before_first_attempt()` 和 `allows_sandbox_failure_retry()` 方法
- `ReadOnlyAccess`, `NetworkAccess`, `WindowsSandboxLevel`

**与其他模块的关系**：
- 零内部依赖（仅依赖 serde）
- 被 manager.rs、orchestrator.rs、guardian.rs、filesystem.rs（测试）引用
- SandboxPolicy 是数据流的起点

**发现问题**：
1. `SandboxMode` 与 `SandboxPolicy` 存在概念重叠。`SandboxMode` 是配置层的简化枚举（3个变体），`SandboxPolicy` 是运行时的完整枚举（4个变体）。两者之间的映射在 `to_policy()` 中实现，但 ExternalSandbox 变体在 SandboxMode 中没有对应项，导致 SandboxMode 无法表达 ExternalSandbox 场景。
2. `SandboxMode::to_policy()` 硬编码了默认值（ReadOnlyAccess::FullDisk, network_access: false），调用者无法自定义这些参数，灵活性不足。
3. `AskForApproval` 的 Default 是 `OnFailure`，这是一个合理的默认但需要在文档中明确说明。

**职责评分**：清晰 ★★★★☆（SandboxMode/SandboxPolicy 二重性略有混淆）

---

### 2. orchestrator.rs — 执行编排器

**职责描述**：
实现命令执行的完整编排流程：Guardian自动审批 → 策略审批 → 首次尝试（沙箱/直接） → 沙箱拒绝处理 → 重试/升级。还定义了 Approvable 和 Sandboxable trait。

**关键类型/函数**：
- `ToolOrchestrator`：核心编排器，持有 SandboxManager、可选的 ExecPolicyManager、可选的 GuardianManager
- `SandboxAttempt`：表示一次沙箱执行尝试的上下文
- `OrchestratorRunResult`：编排执行的结果
- `SandboxOverride`：控制是否绕过沙箱
- `SandboxPermissions`：权限级别
- `Approvable` / `Sandboxable` traits：供外部工具实现的抽象接口
- `run()`：主入口方法，实现了5阶段编排流程
- `retry_after_sandbox_failure()`：失败后的重试升级逻辑

**与其他模块的关系**：
- **重度依赖**（导入7个内部模块）：approval, error, exec_policy, filesystem, guardian, manager, policy
- 通过 SandboxManager 间接访问 ApprovalStore（`sandbox_manager.approval_store()`）
- 通过 GuardianManager 进行自动审批
- 通过 ExecPolicyManager 进行规则匹配

**发现问题**：
1. **超高耦合（God Module 倾向）**：orchestrator.rs 是目前耦合度最高的模块，依赖几乎所有其他模块。它承担了审批检查、沙箱选择、执行调度、重试升级、Guardian集成等过多职责。
2. **审批逻辑碎片化**：`check_approval()` 在 orchestrator 中实现，但同时 manager.rs 也有 `check_approval_requirement()`。两者逻辑相似但有微妙差异，容易导致行为不一致。
3. **直接访问内部状态**：`cached_decision()` 和 `consume_approved_once()` 方法直接通过 `self.sandbox_manager.approval_store().lock().unwrap()` 访问 ApprovalStore，破坏了 SandboxManager 的封装性。
4. **Guardian 集成不完整**：Phase 0 的 Guardian 自动审批中，`create_rule` 分支的代码被注释掉（"Would create Allow rule for..."），表明自动学习功能尚未实现。
5. **SandboxAttempt 的生命周期**：`SandboxAttempt<'a>` 持有对 policy 和 fs_policy 的引用，但在异步上下文中使用引用需要考虑生命周期安全。目前在 `run()` 方法中通过 `self.sandbox_manager.policy()` 获取引用，这在同一次调用中是安全的，但如果 SandboxAttempt 被传递到其他异步任务中可能会出问题。
6. **重试逻辑的重复**：`retry_after_sandbox_failure()` 和 `resolve_retry_permission()` 与 `resolve_initial_override()` 有大量相似的审批缓存检查逻辑，应抽取为共享方法。

**职责评分**：职责过重 ★★☆☆☆

---

### 3. manager.rs — 沙箱管理器

**职责描述**：
作为沙箱执行的统一入口，协调策略（SandboxPolicy）、文件系统策略（FileSystemSandboxPolicy）、审批缓存（ApprovalStore）和平台执行器（Platform Executor）。负责构建沙箱配置、执行命令、处理审批需求。

**关键类型/函数**：
- `SandboxManager`：核心管理器，持有 policy, fs_policy, windows_level, approval_policy, approval_store
- `SandboxConfig`：管理器初始化配置，含 `from_core_config()` 用于从 agent-diva-core 转换
- `SandboxCommand` / `SandboxExecRequest`：命令表示和执行请求
- `execute_sandboxed()`：主执行入口，处理审批检查和沙箱执行
- `execute_direct()`：直接执行（绕过沙箱）
- `execute_unsandboxed()`：公开的绕过沙箱执行接口
- `execute_with_platform()`：平台特定沙箱执行
- `check_approval_requirement()`：审批需求检查
- `can_read_path()` / `can_write_path()`：路径权限检查

**与其他模块的关系**：
- 依赖：approval, error, filesystem, platform, policy
- 条件编译导入：windows::WindowsSandboxExecutor, linux::LinuxSandboxExecutor, macos::MacOsSandboxExecutor
- 被 orchestrator.rs 通过 Arc<SandboxManager> 持有和调用
- 对外提供 `approval_store()` 公开方法（返回 SharedApprovalStore），这导致了封装泄漏

**发现问题**：
1. **审批检查重复**：`check_approval_requirement()` 与 orchestrator.rs 的 `check_approval()` 功能重叠。Manager 的版本使用 `ExecApprovalRequirement`（来自 approval.rs），而 Orchestrator 的版本使用 `ApprovalRequirement`（来自 exec_policy.rs）。两套类型系统并行存在，增加理解负担。
2. **platform 分支代码膨胀**：`execute_with_platform()` 方法中三个平台分支（Windows/Linux/macOS）代码几乎完全相同（创建 executor → 检查可用性 → 执行），应抽取为宏或泛型。
3. **build_fs_policy() 逻辑简化**：为每个 writable_root 仅创建一个 Write 权限的 entry，没有考虑 path 的层级关系和子路径保护。WritableRoot 的 `read_only_subpaths`（如 .git, .diva）未在 entry 构建中使用。
4. **`execute_direct()` 的跨平台差异**：Windows 使用 PowerShell，Unix 使用 sh -c，这种差异应该文档化，且可能存在行为差异（如引号处理、转义规则不同）。
5. **成功/失败的输出处理不一致**：`execute_direct()` 中对非零退出码返回 Ok(format!(...)) 而不是 Err，这模糊了成功和失败的语义，上层调用者需要解析输出字符串来判断是否真正成功。

**职责评分**：职责适中但存在边界模糊 ★★★☆☆

---

### 4. approval.rs — 审批系统

**职责描述**：
定义审批决策类型和审批缓存存储。提供 ReviewDecision（用户决策）、CommandApprovalKey（缓存键）、ExecApprovalRequirement（审批需求判定）、ApprovalStore（审批缓存）。

**关键类型/函数**：
- `ReviewDecision` enum：Denied / ApprovedOnce / ApprovedForSession
- `CommandApprovalKey`：以 {command, cwd} 为键，支持 JSON 序列化缓存
- `ExecApprovalRequirement`：Skip / NeedsApproval / Forbidden
- `ApprovalStore`：基于 HashMap 的审批缓存，支持 session 审批和 deny 检查
- `SharedApprovalStore`：`Arc<Mutex<ApprovalStore>>`

**与其他模块的关系**：
- 零内部依赖（仅依赖 serde, std）
- 被 manager.rs、orchestrator.rs、guardian.rs 使用
- 是审批数据流的中心类型

**发现问题**：
1. **缓存键使用 JSON 序列化**：`to_cache_key()` 使用 `serde_json::to_string()` 将整个结构序列化为 JSON 字符串作为 HashMap 键。这带来了不必要的序列化开销。对于简单的 {command, cwd} 对，直接拼接字符串（如 `format!("{}|{}", command, cwd.display())`）会更高效。
2. **与 exec_policy::ApprovalRequirement 的重复**：`ExecApprovalRequirement` 和 `exec_policy::ApprovalRequirement` 是两个几乎相同的枚举。`ExecApprovalRequirement` 在 approval.rs 中定义，包含 Skip/NeedsApproval/Forbidden；`ApprovalRequirement` 在 exec_policy.rs 中定义（需要确认是否有类似变体）。这造成了类型系统的冗余。
3. **session_approved_commands() 和 denied_commands() 的反序列化开销**：这些方法对每个缓存键执行 `serde_json::from_str()`，效率较低。如果这些方法被频繁调用，应考虑改变缓存结构（直接使用 CommandApprovalKey 作为键而不是字符串）。
4. **ApprovalStore 缺乏持久化**：当前审批缓存仅在内存中，重启后丢失。对于 ApprovedForSession 这种语义来说是合理的，但如果未来需要跨会话保持 deny 决策，需要持久化支持。

**职责评分**：清晰、单一 ★★★★☆

---

### 5. decision.rs — 规则决策类型

**职责描述**：
定义 ExecPolicy 规则评估的决策类型系统。包括 Decision 枚举（Allow/Prompt/Forbidden）、RuleMatch（单条规则匹配结果）、Evaluation（综合评估结果）。Decision 通过 Ord 实现支持多规则聚合（最严格者胜）。

**关键类型/函数**：
- `Decision` enum：Allow < Prompt < Forbidden（通过 derive Ord 实现优先级）
- `Decision::aggregate()`：取最严格决策
- `RuleMatch`：单条规则匹配信息（decision, is_exact_match, matched_pattern）
- `Evaluation`：综合评估（decision, matches, has_matches）
- 实现 FromStr、Display、Serialize/Deserialize

**与其他模块的关系**：
- 零内部依赖（仅依赖 serde）
- 被 rules.rs、exec_policy.rs 使用
- 是 ExecPolicy 系统的核心类型

**发现问题**：
1. **Decision 的 Default 是 Prompt**：这意味着未匹配任何规则时默认要求审批。这是安全的默认，但需要在文档中显式标注，因为有些场景可能期望默认 Allow。
2. **Evaluation 缺乏置信度信息**：当 `has_matches` 为 false 时，返回的决策是 Prompt（默认），但调用者无法区分"没有规则匹配所以默认 Prompt"和"有规则匹配且结果是 Prompt"。
3. **Decision 与 approval.rs 的 ReviewDecision / ExecApprovalRequirement 之间映射关系未显式化**：三个不同的枚举都涉及"允许/审批/禁止"的概念，但它们之间的转换逻辑分散在各处（如 guardian.rs 中手动匹配）。

**职责评分**：清晰、单一 ★★★★★

---

### 6. guardian.rs — Guardian 自动审批系统

**职责描述**：
提供命令自动审批能力，包括 GuardianReviewer trait（可扩展的审查器）、DefaultGuardianReviewer（基于规则和启发式的默认实现）、GuardianRejectionCircuitBreaker（熔断器）、GuardianManager（统一管理审查器和熔断器）。

**关键类型/函数**：
- `GuardianConfig`：Guardian 行为配置（熔断阈值、自动学习开关等）
- `GuardianDecision`：AutoApprove / RequireApproval / Denied / Defer
- `GuardianReviewer` trait：可扩展的审查器接口
- `DefaultGuardianReviewer`：包含 `is_known_safe()`, `appears_read_only()`, `is_potentially_dangerous()` 三个启发式方法
- `GuardianRejectionCircuitBreaker`：时间窗口 + 计数熔断器
- `GuardianManager`：协调审查器和熔断器，持有独立的 approval_cache

**与其他模块的关系**：
- 依赖：approval（CommandApprovalKey, ReviewDecision）、exec_policy（ApprovalRequirement, ExecPolicyManager）、policy（AskForApproval）
- 被 orchestrator.rs 通过 Option<Arc<GuardianManager>> 集成
- 使用 parking_lot::Mutex 而非 std::sync::Mutex（不一致）

**发现问题**：
1. **独立审批缓存**：GuardianManager 持有自己的 `approval_cache: Arc<Mutex<HashMap<String, ReviewDecision>>>`，与 SandboxManager 的 ApprovalStore 完全独立。这导致两个审批缓存的潜在不一致——Guardian 记录的 auto-approve 不会同步到 SandboxManager 的 ApprovalStore 中。虽然在 orchestrator.rs 中手动做了同步（GuardianDecision::AutoApprove 时调用 `guardian.record_approval()`），但这种分散的同步容易遗漏。
2. **锁机制不一致**：guardian.rs 使用 `parking_lot::Mutex`，而 approval.rs 使用 `std::sync::Mutex`。这会导致两个问题：(a) 依赖不统一，(b) 如果代码中混合使用两种锁可能造成混淆。
3. **circuit_breaker 的两个 Mutex 设计有竞态条件**：GuardianRejectionCircuitBreaker 使用两个独立的 Mutex（rejections 和 triggered），检查触发条件（`record_rejection()`）和读取触发状态（`is_triggered()`）之间存在时间窗口，可能导致竞态。
4. **GuarlianReviewer trait 的 review() 方法签名冗余**：同时接受 `ApprovalRequirement` 和 `GuardianConfig`，但 `GuardianConfig` 在 GuardianManager 调用时是 self.config，而 trait 要求调用者每次传入 config。这破坏了信息的单向流动。
5. **默认审查器的启发式硬编码**：`appears_read_only()` 和 `is_potentially_dangerous()` 中的命令列表硬编码，不可配置。无法根据不同场景扩展。
6. **GuardianRejectionCircuitBreaker 实现了 GuardianReviewer trait**：熔断器本身也是一个审查器，这种设计将两种不同关注点（安全防护 vs 自动决策）混合在一起。熔断器应该是一个独立的横切关注点。

**职责评分**：功能丰富但内聚性不足 ★★★☆☆

---

### 7. lib.rs — Crate 根模块

**职责描述**：
声明所有子模块，重导出公共 API，定义环境变量 `AGENT_DIVA_SANDBOX_DISABLED` 和 `is_sandbox_disabled()` 函数。

**关键类型/函数**：
- 模块声明（10个子模块）
- 公共 API 重导出（60+ 类型/函数）
- `is_sandbox_disabled()`：环境变量检查

**发现问题**：
1. **重导出过多**：从 10 个模块重导出了约 60+ 项，包括内部类型如 `BANNED_PREFIX_SUGGESTIONS`、`ExecPolicyError` 等。这模糊了公共 API 的边界。应该区分"稳定的公共 API"和"内部可用但不推荐外部使用的 API"。
2. **SandboxConfig 未重导出**：`SandboxConfig` 在 manager.rs 中定义，是配置 SandboxManager 的关键类型，但未在 lib.rs 中重导出。用户需要使用 `agent_diva_sandbox::manager::SandboxConfig` 全路径或额外 import。
3. **`is_sandbox_disabled()` 是自由函数而非与 SandboxManager 集成**：这个函数在 lib.rs 根级别定义，并在 manager.rs 的 new() 中调用。这创建了从 manager.rs 到 lib.rs 的隐式依赖。

**职责评分**：作为入口点合理，但 API 导出策略需优化 ★★★☆☆

---

### 8. error.rs — 错误类型

**职责描述**：
定义 SandboxError 枚举和 SandboxResult 类型别名。使用 thiserror derive。

**关键类型/函数**：
- `SandboxError`：12 个变体（Denied, ApprovalRequired, PermissionDenied, TokenCreation, SpawnFailed, Timeout, InvalidCommand, PlatformNotSupported, PlatformError, Disabled, Internal）
- `SandboxResult<T>`：`Result<T, SandboxError>`

**与其他模块的关系**：
- 零内部依赖
- 被 manager.rs、orchestrator.rs 广泛使用
- 是整个 crate 的统一错误类型

**发现问题**：
1. **TokenCreation 变体被 cfg(windows) 限制**：这意味着非 Windows 平台编译时该变体不可见，但 PlatformNotSupported/PlatformError 对所有平台可见。这是合理的条件编译，但需注意跨平台 match 时需要处理 `#[cfg(not(windows))]` 分支。
2. **`execute_direct()` 对非零退出码返回 Ok 而非 Err**：这导致错误类型中没有"命令执行失败"的变体。ExitCode 失败被包装为 Ok 字符串，丢失了结构化错误信息。
3. **`SpawnFailed(String)` 和 `Internal(String)` 的字符串参数缺乏结构化信息**：无法程序化地判断具体失败原因，只能解析错误消息。
4. **Missing variant for file system violations**：虽然有 `PermissionDenied`，但没有专门表示"路径不在白名单中"或"路径被黑名单阻止"的变体。这些情况目前可能被映射到 Denied 或 Internal。

**职责评分**：清晰、单一但一些变体粒度不够 ★★★☆☆

---

### 9. filesystem.rs — 文件系统沙箱策略

**职责描述**：
定义文件系统访问控制类型，包括 FileSystemSandboxPolicy（策略容器）、FileSystemSandboxEntry（路径+访问模式）、FileSystemPath（路径匹配）、WritableRoot（可写根目录+保护子路径）、ReadDenyMatcher（glob 模式拒绝匹配器）。

**关键类型/函数**：
- `FileSystemSandboxPolicy`：Restricted / Unrestricted / ExternalSandbox 三种类型
- `FileSystemSandboxEntry`：路径与访问模式的绑定
- `FileSystemPath`：Path / GlobPattern / Special 三种路径表示
- `WritableRoot`：可写根目录 + `read_only_subpaths` 保护（.git, .diva, .agents）
- `ReadDenyMatcher`：基于 GlobSet 的拒绝匹配器
- `default_protected_paths()`：默认保护路径列表
- `default_read_only_subpaths_for_writable_root()`：默认保护子路径

**与其他模块的关系**：
- 依赖 globset（外部）、serde
- 零内部模块依赖（测试中使用了 policy.rs 的类型）
- 被 manager.rs 用于构建文件系统策略
- 被 orchestrator.rs 用于 SandboxAttempt 中的 fs_policy 引用

**发现问题**：
1. **FileSystemPath::matches() 的路径匹配逻辑过于简单**：`Path` 变体使用 `starts_with`，这意味着 `/workspace/a` 会匹配 `/workspace/abc`（误匹配）。应使用更精确的路径前缀匹配（检查边界是否为 `/`）。
2. **Special 路径未实际解析**：`FileSystemPath::Special` 的 `matches()` 始终返回 false，注释说"需要上下文解析"。这些 Special 路径（如 CurrentWorkingDirectory, Tmpdir）在匹配前没有被解析为具体路径，导致使用 Special 路径的 entry 永远不会匹配。
3. **WritableRoot 的 read_only_subpaths 使用了 starts_with 但也存在同样的问题**，并且只检查绝对路径。
4. **ReadDenyMatcher 创建失败时静默降级**：`new()` 中如果 Globs::new() 失败或 GlobSetBuilder::build() 失败，会静默跳过或使用空 GlobSet。这导致配置错误（如无效的 glob 模式）被无声忽略，应记录警告。
5. **default_read_only_subpaths_for_writable_root 中的 protect_missing_dot_diva 参数**：当该参数为 true 时，即使 .diva 目录不存在也会被加入保护列表。这在逻辑上是合理的，但调用者需要明确了解这个语义。

**职责评分**：清晰但路径匹配需修复 ★★★☆☆

---

### 10. platform/mod.rs — 平台抽象

**职责描述**：
定义平台特定沙箱的公共接口。包括 SandboxType 枚举和 current_platform_sandbox_type() 函数。条件编译导入 windows.rs、linux.rs、macos.rs。

**关键类型/函数**：
- `SandboxType` enum：None / WindowsRestrictedToken / LinuxSeccomp / MacosSeatbelt（条件编译）
- 手动实现 Default（因 cfg 变体）
- `current_platform_sandbox_type()`：返回当前平台的沙箱类型

**与其他模块的关系**：
- 零内部依赖
- 被 manager.rs 用于获取平台沙箱类型和导入平台特定执行器
- 子模块（windows.rs, linux.rs, macos.rs）各自独立实现平台沙箱

**发现问题**：
1. **缺乏统一的 PlatformExecutor trait**：三个平台模块（windows.rs, linux.rs, macos.rs）各自有自己的 Executor 类型（WindowsSandboxExecutor, LinuxSandboxExecutor, MacOsSandboxExecutor），但没有统一的 trait 约束。Manager 通过条件编译直接使用具体类型，无法在运行时切换。
2. **SandboxType 的 cfg 变体导致模式匹配不完整**：在非 Windows 平台上，SandboxType 没有 WindowsRestrictedToken 变体。如果代码中有 `match sandbox_type { ... }`，在不同平台上需要不同的 match 臂。这造成了跨平台编译警告或错误。
3. **Manual Default 的实现使用多重 cfg**：代码有 4 个 cfg 分支，最后一个是 `not(any(...))` 的兜底。这种方式容易在新增平台时遗漏。

**职责评分**：作为平台抽象层过于简化 ★★★☆☆

---

## 二、数据流分析

### 声明的数据流

```
SandboxPolicy → ToolOrchestrator → SandboxManager → Platform Executor
```

### 实际数据流

```
用户请求 (command, cwd)
    │
    ▼
ToolOrchestrator.run()
    ├─ Phase 0: GuardianManager.review()            ── 自动审批决策
    │   └─ DefaultGuardianReviewer.review()
    │       ├─ ExecPolicyManager (规则匹配)
    │       └─ CircuitBreaker (熔断检查)
    │
    ├─ Phase 1: check_approval()                     ── 策略审批检查
    │   └─ ExecPolicyManager.get_approval_requirement()
    │       └─ Decision 聚合 (rules.rs → decision.rs)
    │
    ├─ Phase 2: execute_attempt()                    ── 首次尝试
    │   ├─ should_bypass_sandbox()?
    │   │   ├─ Yes → SandboxManager.execute_unsandboxed()
    │   │   │         └─ execute_direct() → powershell/sh -c
    │   │   └─ No  → SandboxManager.execute_sandboxed()
    │   │             ├─ check_approval_requirement() (再次审批)
    │   │             └─ execute_with_platform()
    │   │                 └─ WindowsSandboxExecutor / LinuxSandboxExecutor / MacOsSandboxExecutor
    │   │
    │   └─ 成功? → 返回 OrchestratorRunResult
    │
    └─ Phase 3: retry_after_sandbox_failure()        ── 失败重试/升级
        ├─ resolve_retry_permission()
        │   ├─ allows_sandbox_failure_retry()?
        │   ├─ is_banned_prefix()?
        │   └─ cached_decision()?
        └─ RetryDirect → execute_attempt(bypass=true)
```

### 数据流问题

1. **审批检查出现三次**：
   - orchestrator.rs `check_approval()` → 使用 ExecPolicyManager
   - manager.rs `check_approval_requirement()` → 使用 ApprovalStore + AskForApproval
   - guardian.rs `review()` → 使用 DefaultGuardianReviewer
   
   这三层审批检查是独立的，彼此不共享决策结果，可能导致不一致。

2. **ApprovalStore 绕过封装**：
   Orchestrator 通过 `sandbox_manager.approval_store().lock().unwrap()` 直接操作 ApprovalStore，破坏了 Manager 的单一入口原则。审批缓存应该只通过 Manager 的公开方法修改。

3. **Guardian 审批缓存与 Manager 审批缓存分离**：
   GuardianManager 和 SandboxManager 各自维护独立的审批缓存，虽然在 orchestrator 中做了手动同步（Guardian auto-approve 时同时记录到两边），但这种设计天生脆弱。

---

## 三、状态机分析

### 审批流状态机

```
                    ┌─────────────┐
                    │  新命令请求   │
                    └──────┬──────┘
                           │
                    ┌──────▼──────┐
              ┌─────│   Guardian   │──────┐
              │     │  审查 (Phase0)│      │
              │     └──────┬──────┘      │
              │            │             │
         AutoApprove    Require/     Denied
              │         Defer           │
              │            │             │
    ┌─────────▼──┐  ┌─────▼──────┐  ┌───▼───┐
    │ 直接执行    │  │ 策略审批    │  │ 拒绝   │
    │ (跳过审批)  │  │ (Phase 1)  │  │ (终止) │
    └────────────┘  └─────┬──────┘  └───────┘
                          │
              ┌───────────┼──────────┐
              │           │          │
         Never/      OnRequest/  Forbidden
         OnFailure   UnlessTrusted    │
              │           │          │
    ┌─────────▼──┐  ┌────▼─────┐  ┌──▼───┐
    │ 直接执行    │  │需要审批   │  │拒绝   │
    │ (沙箱)     │  │(Approval  │  │(终止) │
    └────────────┘  │Required)  │  └──────┘
                    └─────┬─────┘
                          │
                    ┌─────▼──────┐
                    │ 审批缓存检查 │
                    └──┬──┬──┬───┘
                       │  │  │
              Approved  │  │  Denied
              Session/  │  │
              Once      │  │
                │    None  │
                │      │   │
    ┌───────────▼┐  ┌──▼───▼─┐
    │ 绕过沙箱执行│  │审批错误  │
    │ (Phase 2)  │  │(需用户)  │
    └────────────┘  └─────────┘
```

### 状态机问题

1. **Guardian AutoApprove 跳过了审批流**：当 Guardian 返回 AutoApprove 时，执行直接发生（带沙箱），不经过 Policy Approval 阶段。这绕过了 ExecPolicyManager 的规则检查。如果 Guardian 误判，命令会逃逸规则审查。

2. **审批状态转换不完整**：`ReviewDecision::ApprovedOnce` 在使用后被消费（consume_approved_once），但消费操作和实际执行之间不是原子操作。如果在消费后执行前发生 panic/error，审批状态将丢失，用户需要重新审批。

3. **重试/升级循环缺乏上限**：`retry_after_sandbox_failure()` 只在 `should_offer_escalation()` 判断后调用一次，没有重试次数限制或退避机制。如果升级后的直接执行也失败，错误会直接向上传播，没有再降级回沙箱或其他恢复策略。

---

## 四、模块耦合度分析

### 依赖关系图

```
lib.rs (根)
├── policy.rs         ★ 零内部依赖
├── error.rs          ★ 零内部依赖
├── filesystem.rs     ★ 零内部依赖
├── decision.rs       ★ 零内部依赖
├── approval.rs       ★ 零内部依赖
├── platform/mod.rs   ★ 零内部依赖
├── rules.rs           → decision.rs
├── exec_policy.rs     → decision.rs, policy.rs, rules.rs
├── manager.rs         → approval, error, filesystem, platform, policy
├── guardian.rs        → approval, exec_policy, policy
└── orchestrator.rs    → approval, error, exec_policy, filesystem, guardian, manager, policy
                         (7/10 内部依赖 - 最高)
```

### 依赖统计

| 模块 | 被依赖次数（入度）| 依赖次数（出度）| 评注 |
|------|------------------|----------------|------|
| policy.rs | 4 (manager, orchestrator, guardian, exec_policy) | 0 | 核心基础类型 |
| approval.rs | 3 (manager, orchestrator, guardian) | 0 | 核心基础类型 |
| decision.rs | 2 (rules, exec_policy) | 0 | ExecPolicy 基础类型 |
| error.rs | 2 (manager, orchestrator) | 0 | 全局错误类型 |
| filesystem.rs | 2 (manager, orchestrator) | 0 | 文件系统策略 |
| platform/mod.rs | 1 (manager) | 0 | 平台抽象 |
| rules.rs | 1 (exec_policy) | 1 (decision) | ExecPolicy 规则定义 |
| exec_policy.rs | 2 (orchestrator, guardian) | 3 | 规则引擎 |
| manager.rs | 1 (orchestrator) | 5 | 核心管理器 |
| guardian.rs | 1 (orchestrator) | 3 | 自动审批 |
| orchestrator.rs | 0 | 7 | **编排器（最高耦合）** |

### 循环依赖

当前代码中 **未发现静态循环依赖**。依赖图是单向无环的。

但存在 **运行时循环风险**：
- ToolOrchestrator → SandboxManager → ApprovalStore ← ToolOrchestrator（通过 approval_store() 公开方法访问）
- 这不是编译期循环依赖，但逻辑上形成了 Orchestrator 和 Manager 的相互渗透

### 耦合问题总结

1. **Orchestrator 是超级耦合器**：依赖 7 个内部模块，承担了过多的编排职责。如果 Orchestrator 需要变更，影响面极大。

2. **Manager 和 Orchestrator 边界模糊**：两者都有审批检查、执行调度、路径检查等功能。理念上 Orchestrator 应该只做编排（调度），Manager 做执行（管理），但当前实现中 Orchestrator 做了很多 Manager 应该做的事（如审批缓存直接操作）。

3. **Guardian 和 Manager 的审批缓存分离**：两个审批缓存增加了整体系统的状态空间，容易出现不一致。

---

## 五、错误处理一致性分析

### 错误类型使用情况

| 错误变体 | 使用位置 | 评注 |
|---------|---------|------|
| Denied | orchestrator, manager | 策略拒绝 |
| ApprovalRequired | orchestrator, manager | 需要审批（Manager 中作为临时方案返回错误） |
| PermissionDenied | filesystem? | 文件系统权限拒绝（定义但使用有限） |
| TokenCreation | windows.rs | Windows 特化 |
| SpawnFailed | manager (execute_direct) | 进程启动失败 |
| Timeout | manager (execute_direct) | 超时 |
| InvalidCommand | orchestrator (shell_words split) | 命令解析失败 |
| PlatformNotSupported | manager, platform | 平台不支持 |
| PlatformError | manager | 平台特定错误 |
| Disabled | 未明确使用 | 沙箱禁用（定义为变体但主要检查在外部） |
| Internal | 未广泛使用 | 内部错误 |

### 一致性问题

1. **错误语义不一致**：非零退出码被包装为 `Ok(String)` 而非 `Err(SandboxError)`。在 `execute_direct()` 中，命令失败返回的是 Ok，上层调用者无法区分"成功输出"和"失败输出"。

2. **ApprovalRequired 作为错误**：`SandboxError::ApprovalRequired` 既是错误又是流程控制信号。在 Manager 中它表示"需要审批"（这是一个正常的业务流程状态，不一定是错误），但在 Orchestrator 中它被当作错误来触发 escalation 流程。这种双重语义容易导致误解。

3. **平台错误传播不一致**：`execute_with_platform()` 中，Windows 沙箱不可用时返回 `PlatformError("Windows sandbox is unavailable...")`，但这实际上应该属于"平台不支持"而不是"平台错误"。

4. **错误消息是面向开发者还是用户**：有些错误消息如 "Command denied by sandbox policy: {reason}" 是面向用户的，而 "Failed to spawn sandboxed process: {0}" 中包含的技术细节可能不适合直接展示给最终用户。

5. **缺乏错误链**：SandboxError 没有实现 `source()` 方法来链接底层错误（如 IO 错误），导致调试时只能看到最表层的原因。

---

## 六、公共 API 设计分析

### 当前 API 重导出清单

从 lib.rs 重导出约 60+ 项，包括：
- 配置类型：SandboxMode, SandboxPolicy, AskForApproval, GuardianConfig 等
- 管理器：SandboxManager, ToolOrchestrator, GuardianManager
- 审批：ApprovalStore, ReviewDecision, ExecApprovalRequirement, CommandApprovalKey
- 决策：Decision, Evaluation, RuleMatch
- 文件系统：FileSystemSandboxPolicy, FileSystemAccessMode, WritableRoot
- 规则：Policy, PrefixRule, RulesFile
- Trait：Approvable, Sandboxable, GuardianReviewer
- 内部实现细节：BANNED_PREFIX_SUGGESTIONS, ExecPolicyError, ExecPolicyAmendment, is_banned_prefix

### 问题

1. **公共 API 边界不清晰**：内部实现细节（如 BANNED_PREFIX_SUGGESTIONS、ExecPolicyAmendment）不应暴露为公共 API。建议使用 `pub(crate)` 限制可见性。

2. **SandboxConfig 未重导出**：作为创建 SandboxManager 的主要配置类型，SandboxConfig 应该被重导出。当前用户需要 `use agent_diva_sandbox::manager::SandboxConfig`。

3. **功能门（Feature Gate）缺失**：Phase 1 和 Phase 2 的功能混合在一起重导出，没有通过 cargo feature 区分。用户无法选择只使用核心沙箱而不引入 ExecPolicy/Guardian 等更高级的功能。

4. **类型命名不一致**：
   - `ExecApprovalRequirement` (approval.rs) vs `ApprovalRequirement` (exec_policy.rs)
   - `SandboxCommand` vs `SandboxExecRequest`（两个都在 manager.rs 中，功能重叠）
   - `FileSystemSandboxPolicy` (filesystem.rs) vs `SandboxPolicy` (policy.rs) — 两个"Policy"类型在不同模块中

5. **Approvable / Sandboxable trait 设计**：这两个 trait 仅在 orchestrator.rs 中定义，但没有被 crate 内部使用（只有测试）。它们是面向外部使用者的扩展点。但当前它们的默认实现与 SandboxPolicy/AskForApproval 紧密耦合（如 `wants_no_sandbox_approval()` 直接依赖 AskForApproval 的具体变体），限制了外部实现者的灵活性。

6. **异步接口不统一**：`ToolOrchestrator::run()` 是 async，`SandboxManager::execute_sandboxed()` 也是 async，但 `GuardianManager::review()` 是同步的。在异步上下文中调用同步 review 可能阻塞 executor。

---

## 七、整体架构评分

### 各维度评分

| 维度 | 评分 | 说明 |
|------|------|------|
| 模块职责清晰度 | ★★★☆☆ (3/5) | policy/error/decision/approval 职责清晰；orchestrator 职责过重；manager 边界模糊 |
| 数据流合理性 | ★★★☆☆ (3/5) | 基本流向正确，但审批检查存在三层重复，缓存访问破坏封装 |
| 状态机正确性 | ★★★☆☆ (3/5) | 审批流设计完整但存在逃逸路径；重试/升级缺乏上限；原子性问题 |
| 耦合度控制 | ★★★☆☆ (3/5) | 无循环依赖，但 orchestrator 耦合度极高（7/10）；Guardian 缓存独立 |
| 错误处理一致性 | ★★☆☆☆ (2/5) | 错误语义不一致（非零退出码=Ok）；ApprovalRequired 的双重语义；缺乏错误链 |
| 公共 API 设计 | ★★☆☆☆ (2/5) | 重导出过多；SandboxConfig 未导出；功能门缺失；类型命名不一致 |

**整体评分：2.8 / 5.0**

### 架构亮点

1. **依赖方向正确**：所有依赖都是从高层（orchestrator）指向低层（policy/error/decision），没有循环依赖。
2. **核心类型设计良好**：SandboxPolicy、Decision、ReviewDecision 等枚举设计合理，使用 derive 宏简化实现。
3. **平台抽象存在**：通过条件编译支持 Windows/Linux/macOS，各平台实现独立。
4. **熔断器模式**：GuardianRejectionCircuitBreaker 是一个良好的安全防护模式。
5. **丰富的测试**：大部分模块都有完整的单元测试覆盖。

---

## 八、关键风险和整改建议

### 关键风险（按严重程度排序）

#### 🔴 高风险

1. **Guardian 绕过规则审查**
   - Guardian AutoApprove 路径直接执行，跳过了 ExecPolicyManager 的规则检查
   - 如果 Guardian 的启发式误判（如 `appears_read_only()` 将危险命令误判为安全），可能导致未授权执行
   - **建议**：在 orchestrator.rs 的 Guardian Phase 0 中，对 Guardian auto-approve 的结果仍应用 ExecPolicy 检查；或至少对 Forbidden 规则进行二次确认

2. **双重审批缓存导致状态不一致**
   - GuardianManager 和 SandboxManager 各自维护独立的审批缓存
   - 当前通过 orchestrator 手动同步，但这是脆弱的耦合
   - **建议**：统一审批缓存，只保留 SandboxManager 的 ApprovalStore，GuardianManager 通过引用访问同一个 ApprovalStore

3. **非零退出码被当作成功**
   - `execute_direct()` 对任何进程退出都返回 Ok，丢失错误语义
   - 上层无法区分"命令成功"和"命令失败但有输出"
   - **建议**：增加 `SandboxError::CommandFailed { exit_code, stdout, stderr }` 变体，对非零退出码返回 Err

#### 🟡 中风险

4. **Orchestrator 超高耦合**
   - orchestrator.rs 依赖 7/10 个内部模块，变更影响面极大
   - **建议**：将审批流程抽象为独立的 `ApprovalFlow` 类型，将重试逻辑抽取为 `RetryStrategy`；Orchestrator 只做编排（委托调度）

5. **FileSystemPath 路径匹配缺陷**
   - 使用 `starts_with` 导致误匹配（`/workspace/a` 匹配 `/workspace/abc`）
   - Special 路径永远不匹配（未解析）
   - **建议**：修复路径前缀匹配逻辑（增加路径分隔符边界检查），在匹配前解析 Special 路径

6. **锁机制不统一**
   - guardian.rs 使用 parking_lot::Mutex，其他模块使用 std::sync::Mutex
   - **建议**：统一使用 parking_lot（性能更好）或全部使用 std

#### 🟢 低风险

7. **SandboxMode/SandboxPolicy 二重性**
   - 两个枚举有重叠但又不完全对应（ExternalSandbox 缺失）
   - **建议**：要么合并为一个枚举（通过 serde tag 区分来源），要么在文档中明确说明两者的使用场景

8. **公共 API 膨胀**
   - 内部实现细节（BANNED_PREFIX_SUGGESTIONS 等）暴露为公共 API
   - SandboxConfig 未重导出
   - **建议**：使用 pub(crate) 限制内部类型，添加 cargo feature 门控 Phase 1/2 功能

9. **ApprovalRequired 作为错误的语义模糊**
   - 既是错误又是流程控制信号
   - **建议**：将 ApprovalRequired 从 SandboxError 中移出，改为 `SandboxResult` 的第三个状态（如 `Result<Output, Error, ApprovalNeeded>` 或使用自定义枚举）

### 整改优先级

```
P0 (立即):  #3 非零退出码处理, #2 统一审批缓存
P1 (本迭代): #1 Guardian 绕过规则审查, #5 路径匹配修复
P2 (下迭代): #4 Orchestrator 解耦, #6 锁统一
P3 (后续):   #7 SandboxPolicy 合并, #8 API 清理, #9 错误语义优化
```

---

## 九、附录：模块依赖矩阵

```
               pol err fil dec app pla rul epo man gua orc
policy.rs       ·   ·   ·   ·   ·   ·   ·   ·   ·   ·   ·
error.rs        ·   ·   ·   ·   ·   ·   ·   ·   ·   ·   ·
filesystem.rs   ·   ·   ·   ·   ·   ·   ·   ·   ·   ·   ·
decision.rs     ·   ·   ·   ·   ·   ·   ·   ·   ·   ·   ·
approval.rs     ·   ·   ·   ·   ·   ·   ·   ·   ·   ·   ·
platform/mod.rs ·   ·   ·   ·   ·   ·   ·   ·   ·   ·   ·
rules.rs        ·   ·   ·   ·   ·   ·   ·   X   ·   ·   ·
exec_policy.rs  X   ·   ·   ·   ·   ·   ·   X   ·   ·   ·
manager.rs      X   X   X   ·   X   X   ·   ·   ·   ·   ·
guardian.rs     X   ·   ·   ·   X   ·   ·   X   ·   ·   ·
orchestrator.rs X   X   X   ·   X   ·   ·   X   X   X   ·

图例: X = 导入关系, · = 无直接依赖
列 = 被依赖方, 行 = 依赖方

缩写说明:
  pol = policy, err = error, fil = filesystem, dec = decision
  app = approval, pla = platform, rul = rules, epo = exec_policy
  man = manager, gua = guardian, orc = orchestrator
```

---

*报告生成：Hermes Agent (deepseek-v4-pro)*
*Crate 版本：Phase 2 MVP*
