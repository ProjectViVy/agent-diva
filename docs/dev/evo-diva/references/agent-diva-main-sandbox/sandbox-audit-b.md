# Sandbox 审计报告 B：凭证安全 + 注入防护 + 熔断

> 审计日期：2026-06-02
> 审计范围：`agent-diva-sandbox/` 及关联核心模块
> 维度：凭证脱敏、注入防护、熔断机制、预算控制、审批缓存、隔离扫描、幽灵检测、流式恢复

---

## 1. 凭证脱敏（API Key 在错误/日志中过滤）

**⚠️ 部分实现**

| 子项 | 状态 | 说明 |
|------|------|------|
| CLI config show 脱敏 | ✅ | `redact_sensitive_value()` 递归替换含 `api_key`/`token`/`secret`/`password` 的字段为 `***REDACTED***` |
| 日志层全局脱敏 | ❌ | 无 `tracing` Layer 过滤，错误消息中若含 API Key 会原样写入日志 |
| ErrorContext 脱敏 | ❌ | `error_context.rs` 截断内容到 500 字符，但不脱敏密钥 |
| Provider HTTP 日志 | ❌ | `litellm.rs` 等请求/响应日志含 Authorization header，无过滤 |

**关键文件：**
- ✅ `agent-diva-cli/src/cli_runtime.rs:405-433` — `redact_sensitive_value()`
- ❌ `agent-diva-core/src/logging.rs` — 标准 tracing，无脱敏层
- ❌ `agent-diva-core/src/error_context.rs` — 截断但不脱敏

**建议：** 实现 `tracing::Layer` 拦截所有日志记录，正则匹配替换 `sk-*`、`Bearer *` 等模式为 `[REDACTED]`。

---

## 2. Prompt 注入扫描（MCP 工具描述/输出）

**❌ 未实现**

完全缺失。代码库中无任何 prompt injection 检测逻辑：

- 用户消息直接进入 `ContextBuilder` 拼接，无注入模式扫描
- 工具输出（含 MCP 工具返回的外部内容）直接喂入 LLM 上下文，无消毒
- 系统提示（soul prompt）与用户内容无隔离边界

**关键文件：**
- `agent-diva-agent/src/context.rs` — 拼接 soul/memory/skills/history，无过滤
- `agent-diva-tools/src/mcp.rs` — MCP 工具输出直传，无消毒

**建议：**
1. 入站消息扫描：检测 `ignore previous instructions`、`you are now`、`system:` 等注入模式
2. 工具输出消毒：外部内容标记为不可信区域（XML wrapper），限制其影响范围
3. MCP 工具描述审查：防止恶意工具描述覆盖系统指令

---

## 3. 威胁模式扫描（记忆写入安全检查）

**❌ 未实现**

记忆系统直接写入 LLM 输出，无任何安全审查：

- `MemoryManager::save_memory()` — 将 LLM 的 `memory_update` 参数直接写入 `MEMORY.md`
- `MemoryManager::append_history()` — 将 LLM 的 `history_entry` 参数直接写入 `HISTORY.md`
- 无大小限制、无内容扫描、无路径遍历检查

**关键文件：**
- `agent-diva-core/src/memory/manager.rs` — 直写磁盘，无过滤
- `agent-diva-agent/src/consolidation.rs` — LLM 输出 → 记忆，无安全检查

**风险：**
- 注入攻击：恶意内容写入记忆，在未来会话中被加载为系统上下文，形成持久化注入
- 数据渗出：记忆中嵌入 HTTP URL + 敏感数据，被后续 LLM 调用触发外发
- 路径遍历：记忆内容中嵌入 `../../` 路径（虽然当前写入固定路径，风险较低）

**建议：** 在写入前进行：① 大小上限检查 ② 注入模式扫描 ③ URL/路径审计

---

## 4. 断路器/熔断（工具调用连续失败）

**⚠️ 部分实现（仅审批拒绝熔断）**

`GuardianRejectionCircuitBreaker` 已实现，但仅追踪**用户拒绝决策**，不追踪工具执行失败：

| 子项 | 状态 | 说明 |
|------|------|------|
| 审批拒绝熔断 | ✅ | 滑动窗口 60s 内连续 5 次拒绝触发熔断，升级为 `RequireApproval` |
| 工具执行失败熔断 | ❌ | 工具返回错误/超时/崩溃时无计数，不会触发熔断 |
| 重复相同工具调用检测 | ❌ | 无 SHA256 去重（参见 `decisions.md` P0-2 方案，未落地） |

**关键文件：**
- ✅ `agent-diva-sandbox/src/guardian.rs:402-513` — `GuardianRejectionCircuitBreaker`
- ✅ `agent-diva-sandbox/src/guardian.rs:539-658` — `GuardianManager` 集成
- ❌ `agent-diva-agent/src/agent_loop/loop_turn.rs` — 无工具失败计数

**已实现细节：**
- 时间窗口：`rejection_window_secs`（默认 60s）
- 触发阈值：`max_consecutive_rejections`（默认 5）
- 审批重置计数器
- 测试覆盖：`test_circuit_breaker_basic`、`test_circuit_breaker_approval_reset`、`test_circuit_breaker_apply_to_decision`

**建议：** 在 `ToolRegistry::execute()` 层增加失败计数器，连续 N 次失败（含超时）触发工具级熔断。

---

## 5. 迭代预算控制

**✅ 已实现**

硬上限机制完善：

- `max_iterations` 默认 20，可通过 `agents.defaults.max_tool_iterations` 配置
- 主循环：`while iteration < self.max_iterations`（`loop_turn.rs:120`）
- 每次迭代递增计数器
- 会话取消检查嵌入迭代开始和工具调用之间
- 预算耗尽时返回已累积内容或默认消息
- 配置验证：`max_tool_iterations` 必须 > 0（`validate.rs:18`）
- 子 agent 独立预算：`subagent.rs` 使用 `max_iterations = 15`

**关键文件：**
- `agent-diva-agent/src/agent_loop.rs:100,149-184` — 定义与初始化
- `agent-diva-agent/src/agent_loop/loop_turn.rs:120-128` — 循环控制
- `agent-diva-core/src/config/schema.rs:89,103` — 配置定义（默认 20）
- `agent-diva-core/src/config/validate.rs:18-19` — 验证

**评价：** 实现完整，有配置、有验证、有测试。与 Hermes（默认 90）相比保守但安全。

---

## 6. 审批缓存/粘性授权（用户授权持久化）

**✅ 已实现**

三层审批决策 + 内存缓存：

| 决策类型 | 语义 | 生命周期 |
|----------|------|----------|
| `Denied` | 拒绝 | 会话内 |
| `ApprovedOnce` | 单次批准 | 消费后失效 |
| `ApprovedForSession` | 会话级批准 | 进程内存 |

**实现细节：**
- `ApprovalStore` = `HashMap<String, ReviewDecision>`，以 `CommandApprovalKey`（命令 + cwd）为键
- `SharedApprovalStore` = `Arc<Mutex<ApprovalStore>>`，跨组件共享
- `SandboxManager::check_approval_requirement()` 先查缓存再评估策略
- `ToolOrchestrator` 处理 `cached_decision()` 和 `consume_approved_once()`
- `GuardianManager` 独立维护 `approval_cache`（已知一致性问题，见 code review）

**关键文件：**
- `agent-diva-sandbox/src/approval.rs` — 核心实现（326 行）
- `agent-diva-sandbox/src/manager.rs:219-552` — SandboxManager 集成
- `agent-diva-sandbox/src/orchestrator.rs:411,559-655` — Orchestrator 使用

**已知问题：**
- 仅内存存储，进程重启后丢失（对 `ApprovedForSession` 语义合理）
- GuardianManager 与 SandboxManager 有两份独立缓存，存在潜在不一致
- `ApprovedOnce` 消费与执行之间非原子操作

---

## 7. 隔离区扫描（下载→隔离→扫描→安装）

**❌ 未实现**

完全缺失。无下载后安全扫描流程：

- `WebFetchTool` 直接返回抓取内容，无病毒/恶意内容扫描
- Skill 安装（`agent-diva-manager/src/skill_service.rs`）无内容审查
- 文件系统沙箱（`filesystem.rs`）仅做路径访问控制，不做内容威胁扫描

**关键文件：**
- `agent-diva-tools/src/web.rs` — WebFetch 直传，无消毒
- `agent-diva-manager/src/skill_service.rs` — Skill 安装无扫描

**建议：** 实现隔离区模式：下载 → 写入临时隔离目录 → 内容扫描（大小/类型/模式匹配）→ 通过后移入正式目录。

---

## 8. 空响应/幽灵动作检测

**⚠️ 部分实现（仅空内容兜底）**

| 子项 | 状态 | 说明 |
|------|------|------|
| 空 final_content 兜底 | ✅ | 返回 "I've completed processing but have no response to give." |
| 空消息跳过保存 | ✅ | 无内容无工具调用的助手消息不写入历史 |
| 幽灵动作检测 | ❌ | 工具调用成功但无实际副作用的情况未检测 |
| 重复空响应 stall 检测 | ❌ | LLM 连续返回空内容时无告警或终止 |
| 空参数工具调用检测 | ❌ | LLM 发出空参数的工具调用可静默成功 |

**关键文件：**
- ✅ `agent-diva-agent/src/agent_loop/loop_turn.rs:361-363` — 空内容兜底
- ✅ `agent-diva-agent/src/agent_loop/loop_turn.rs:597-603` — 空消息跳过
- ✅ `agent-diva-agent/src/consolidation.rs:149` — consolidation 缺失工具调用时警告

**建议：** 增加 stall 检测器：连续 N 次迭代无实质输出 → 强制终止并报告。

---

## 9. 流式响应中断恢复

**⚠️ 部分实现（仅 keepalive 超时）**

| 子项 | 状态 | 说明 |
|------|------|------|
| 流内 keepalive 超时 | ✅ | 250ms 超时后 `continue`，允许流恢复 |
| 流自然结束处理 | ✅ | `None` 时 `break` |
| 不完整流合成响应 | ✅ | 无 `Completed` 事件时从已累积内容构造 `LLMResponse` |
| TCP/连接断开重试 | ❌ | 连接断开时 `stream_event?` 传播错误，直接中止 |
| 瞬态网络故障重试 | ❌ | 无 LLM 调用重试机制 |
| 部分内容检查点 | ❌ | 无断点续传 |

**关键文件：**
- `agent-diva-agent/src/agent_loop/loop_turn.rs:143-219` — 流式处理循环

**建议：** 在流式循环外层增加重试包装：检测连接错误 → 等待退避 → 重试 LLM 调用（携带已累积内容提示）。

---

## 总结

| # | 维度 | 状态 | 核心文件 |
|---|------|------|----------|
| 1 | 凭证脱敏 | ⚠️ CLI 有，日志层无 | `cli_runtime.rs:405-433` / `logging.rs` |
| 2 | Prompt 注入扫描 | ❌ 完全缺失 | — |
| 3 | 威胁模式扫描（记忆写入） | ❌ 完全缺失 | `memory/manager.rs` / `consolidation.rs` |
| 4 | 断路器/熔断 | ⚠️ 仅审批拒绝熔断 | `guardian.rs:402-513` |
| 5 | 迭代预算控制 | ✅ 完整 | `agent_loop.rs` / `loop_turn.rs:120` |
| 6 | 审批缓存/粘性授权 | ✅ 完整 | `approval.rs` / `orchestrator.rs` |
| 7 | 隔离区扫描 | ❌ 完全缺失 | — |
| 8 | 空响应/幽灵动作检测 | ⚠️ 仅空内容兜底 | `loop_turn.rs:361-363` |
| 9 | 流式响应中断恢复 | ⚠️ 仅 keepalive | `loop_turn.rs:143-219` |

**统计：** ✅ 2/9 · ⚠️ 4/9 · ❌ 3/9

**优先修复建议（按风险排序）：**
1. **P0 — 凭证日志脱敏**：实现 tracing Layer，防止 API Key 泄露到日志文件
2. **P0 — Prompt 注入扫描**：入站消息 + 工具输出消毒，防止上下文污染
3. **P1 — 记忆写入安全检查**：大小限制 + 注入模式扫描，防止持久化注入
4. **P1 — 工具执行失败熔断**：在工具注册表层增加失败计数器
5. **P2 — 幽灵动作/stall 检测**：连续空输出检测与强制终止
6. **P2 — 流式重试机制**：连接断开时的指数退避重试
7. **P3 — 隔离区扫描**：下载内容安全审查管道
