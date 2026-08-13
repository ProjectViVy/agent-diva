# agent-diva 演进路线建议

> 生成日期：2026-06-01
> 基于 7 个项目对比分析，为 agent-diva 制定分阶段演进路线

---

## 战略定位

agent-diva 的核心定位是 **Agent 大脑/编排层**——不追求极强的代码能力，而是专注于：
- 管理外部 agent（Claude Code、Codex 等）执行操作
- 协调多平台消息路由
- 提供可靠的会话和记忆基础设施
- 作为智能体的"中枢神经系统"

因此，演进路线优先强化 **运行时可靠性** 和 **编排能力**，而非重建已有能力。

---

## Phase 0：立即可做（零成本/极低成本）

> 目标：消除最明显的短板，不改变架构

### 0.1 工具执行超时包装

| 项目 | 说明 |
|------|------|
| **任务** | 为每个工具执行添加 tokio::time::timeout 包装 |
| **参考** | openfang 超时包装 / Claude Code timeout 参数 |
| **改动** | agent-diva-tools 的 Tool trait execute 方法添加 timeout |
| **收益** | 防止单个工具挂死导致整个 agent loop 阻塞 |
| **工作量** | ~2 小时 |

### 0.2 工具结果截断

| 项目 | 说明 |
|------|------|
| **任务** | 工具输出超过阈值（如 50K 字符）时自动截断并附加摘要 |
| **参考** | Claude Code tool_result_budget / openfang 上下文预算 |
| **改动** | agent-diva-agent 的工具执行结果处理 |
| **收益** | 防止单个大结果撑爆上下文窗口 |
| **工作量** | ~3 小时 |

### 0.3 基础迭代上限

| 项目 | 说明 |
|------|------|
| **任务** | 在 AgentLoop 中添加 max_iterations 配置，默认 50 |
| **参考** | openfang 默认50 / Hermes 90 / GenericAgent 80 |
| **改动** | agent-diva-agent 的主循环添加计数器 |
| **收益** | 防止无限工具调用循环 |
| **工作量** | ~1 小时 |

### 0.4 Provider 错误日志结构化

| 项目 | 说明 |
|------|------|
| **任务** | 将 provider 错误分类为 retryable/non-retryable，添加结构化日志 |
| **参考** | Hermes ClassifiedError 数据类 |
| **改动** | agent-diva-providers 的错误处理 |
| **收益** | 为后续错误分类管道打基础 |
| **工作量** | ~3 小时 |

### 0.5 CLAUDE.md 添加记忆写入公理

| 项目 | 说明 |
|------|------|
| **任务** | 在 CLAUDE.md 中添加 "No Execution, No Memory" 约束 |
| **参考** | GenericAgent memory_management_sop.md 公理 |
| **改动** | CLAUDE.md 文档 |
| **收益** | 防止未经验证的信息写入长期记忆 |
| **工作量** | ~10 分钟 |

---

## Phase 1：短期（1-4 周）

> 目标：补全关键运行时能力，建立安全基础

### 1.1 上下文压缩管线（P0）

| 项目 | 说明 |
|------|------|
| **任务** | 实现 3 层压缩管线 |
| **参考** | Claude Code 4层管线 / Hermes 5阶段压缩 |
| **设计** | |
| | L1: **snip** — 裁掉无关的旧对话中间部分（0 API 调用） |
| | L2: **tool_budget** — 大工具结果落盘，只保留摘要（0 API 调用） |
| | L3: **auto** — LLM 生成摘要，仅在 token 超阈值时触发（1 API 调用） |
| **改动** | agent-diva-agent 的 context builder |
| **收益** | 解决长对话上下文溢出问题，降低 token 成本 |
| **工作量** | 1-2 周 |

### 1.2 结构化错误分类管道（P0）

| 项目 | 说明 |
|------|------|
| **任务** | 实现错误分类 + 自动恢复策略 |
| **参考** | Hermes 8级错误分类 / Claude Code 16+种reason code |
| **设计** | |
| | ClassifiedError { reason, retryable, should_compress, should_fallback } |
| | 分类管道：HTTP状态码 → 错误码 → 消息模式 → SSL瞬态 → 传输错误 → 未知 |
| | 恢复策略：重试 → 退避 → 压缩 → fallback |
| **改动** | 新增 agent-diva-error crate 或在 agent-diva-agent 中实现 |
| **收益** | 显著提升 agent 在异常情况下的鲁棒性 |
| **工作量** | 1 周 |

### 1.3 子Agent安全模型（P0）

| 项目 | 说明 |
|------|------|
| **任务** | 为 SubagentManager 添加安全控制 |
| **参考** | Hermes DELEGATE_BLOCKED_TOOLS + MAX_DEPTH + 角色模型 / Codex 角色系统 |
| **设计** | |
| | 工具黑名单：delegate_task, memory, send_message 等禁止子Agent访问 |
| | 深度控制：MAX_DEPTH = 1-3 可配置 |
| | 角色模板：explorer(只读+搜索), worker(读写), reviewer(只读) |
| **改动** | agent-diva-agent 的 SubagentManager |
| **收益** | 防止子Agent越权操作，提升安全性 |
| **工作量** | 1 周 |

### 1.4 断路器机制（P0）

| 项目 | 说明 |
|------|------|
| **任务** | 在 provider 和 MCP 层引入断路器 |
| **参考** | Hermes 连续3次失败→60秒冷却 / openfang 熔断 |
| **设计** | CircuitBreaker { failures, threshold, cooldown, state(Open/Closed/HalfOpen) } |
| **改动** | agent-diva-providers + MCP 客户端 |
| **收益** | 防止重试风暴，快速失败 |
| **工作量** | 3-5 天 |

### 1.5 分层 System Prompt（P1）

| 项目 | 说明 |
|------|------|
| **任务** | 将 system prompt 分为 stable/context/volatile 三层 |
| **参考** | Hermes 3层缓存 / Claude Code 3层缓存 |
| **设计** | |
| | Stable: 身份、工具引导、环境提示（几乎不变） |
| | Context: 项目指令、CLAUDE.md（会话级变化） |
| | Volatile: 记忆快照、时间戳、session ID（每轮可能变化） |
| **改动** | agent-diva-agent 的 context builder |
| **收益** | 最大化 LLM provider 前缀缓存命中率，降低延迟和成本 |
| **工作量** | 1 周 |

### 1.6 钩子系统（P1）

| 项目 | 说明 |
|------|------|
| **任务** | 引入核心钩子事件 |
| **参考** | Claude Code 27种hook / Hermes HOOK.yaml / OpenHarness 10个事件 |
| **设计** | |
| | 核心事件：PreToolUse, PostToolUse, Stop, SessionStart, SessionEnd |
| | 钩子类型：command(shell命令), callback(Rust函数) |
| | 配置：.agent-diva/hooks/ 目录 |
| **改动** | 新增 agent-diva-hooks crate |
| **收益** | 可扩展的 agent 行为控制，安全审计入口 |
| **工作量** | 1-2 周 |

### 1.7 IterationBudget 迭代预算（P1）

| 项目 | 说明 |
|------|------|
| **任务** | 实现线程安全的迭代计数器，支持 consume/refund |
| **参考** | Hermes IterationBudget(90次) / GenericAgent 多层级限制 |
| **设计** | IterationBudget { total, consumed, remaining, consume(), refund() } |
| **改动** | agent-diva-agent 的主循环 |
| **收益** | 精细化控制迭代次数，支持 execute_code 等廉价操作退还 |
| **工作量** | 2-3 天 |

---

## Phase 2：中期（1-3 月）

> 目标：架构升级，建立生态基础

### 2.1 工具并发执行

| 项目 | 说明 |
|------|------|
| **任务** | 实现只读并发 + 写入串行 + 路径冲突检测 |
| **参考** | Hermes ThreadPool(8) + 路径冲突检测 / Claude Code 只读并发 |
| **设计** | |
| | 工具分类：只读(read/glob/grep) vs 写入(write/edit/bash) |
| | 并发决策：_should_parallelize() 检查路径冲突 |
| | 线程池：max_workers = min(工具数, 8) |
| **改动** | agent-diva-tools 的执行引擎 |
| **收益** | 多工具场景延迟降低 50%+ |
| **工作量** | 2-3 周 |

### 2.2 记忆系统升级

| 项目 | 说明 |
|------|------|
| **任务** | 从扁平 MEMORY.md 升级为分层记忆 |
| **参考** | GenericAgent L1-L4 / openfang 5存储 / memtle BM25 |
| **设计** | |
| | L1: 索引层（≤30行，存在性编码，注入 system prompt） |
| | L2: 事实层（环境性事实：路径、配置、凭证） |
| | L3: 记录层（SOP + 工具脚本 + 操作模式） |
| | L4: 归档层（压缩的旧会话摘要） |
| | 检索：BM25 倒排索引（参考 memtle） |
| **改动** | 新增 agent-diva-memory crate |
| **收益** | 记忆检索效率提升，上下文注入更精准 |
| **工作量** | 3-4 周 |

### 2.3 评估框架建设

| 项目 | 说明 |
|------|------|
| **任务** | 建立三层测试体系 |
| **参考** | OpenHarness 三层(单元/集成/E2E) + 五层断言 |
| **设计** | |
| | 单元测试：Rust #[tokio::test]，Mock 外部服务 |
| | 集成测试：组件间交互，真实文件系统 |
| | E2E 测试：真实 API 调用，声明式场景(Scenario) |
| | 断言体系：工具轨迹 + 最终文本 + 文件状态 + 执行计数 + 内容检查 |
| **改动** | tests/ 目录 + scripts/e2e_smoke.py |
| **收益** | 建立质量基线，持续保证正确性 |
| **工作量** | 2-3 周 |

### 2.4 MCP 增强

| 项目 | 说明 |
|------|------|
| **任务** | 增强 MCP 安全和可靠性 |
| **参考** | Hermes 断路器+环境过滤+凭证脱敏+动态刷新 |
| **设计** | |
| | 断路器：连续3次失败→断路 |
| | 环境变量过滤：_build_safe_env() 仅传递安全变量 |
| | 凭证脱敏：错误消息中的 API key 替换为 [REDACTED] |
| | 动态刷新：响应 tools/list_changed 通知 |
| **改动** | agent-diva-tools 的 MCP 客户端 |
| **收益** | MCP 生态安全扩展 |
| **工作量** | 2 周 |

### 2.5 子Agent健康监控

| 项目 | 说明 |
|------|------|
| **任务** | 引入心跳 + 超时诊断 |
| **参考** | Hermes 心跳线程+超时诊断转储 / Codex wait_agent |
| **设计** | |
| | 心跳：每30秒触碰活动时间戳 |
| | 超时诊断：0次API调用后超时→转储配置和栈 |
| | 状态查询：subagent_status() 返回运行状态 |
| **改动** | agent-diva-agent 的 SubagentManager |
| **收益** | 子Agent可观测性，快速定位问题 |
| **工作量** | 1-2 周 |

### 2.6 工具Token优化

| 项目 | 说明 |
|------|------|
| **任务** | 引入工具描述缓存 + Progressive Disclosure |
| **参考** | GenericAgent "still active"缓存 / Hermes BM25 Progressive Disclosure |
| **设计** | |
| | 首次发送完整 schema，后续仅 "Tools: still active" 短提示 |
| | 工具数量超阈值时，非核心工具替换为 tool_search 桥接 |
| **改动** | agent-diva-providers 的 context builder |
| **收益** | 长对话中减少 30%+ 工具描述 token |
| **工作量** | 1-2 周 |

### 2.7 凭证安全

| 项目 | 说明 |
|------|------|
| **任务** | 错误消息凭证脱敏 + 记忆写入安全扫描 |
| **参考** | Hermes _sanitize_error + 威胁模式扫描 / memtle 错误消毒 |
| **设计** | |
| | 错误脱敏：正则替换 GitHub PAT、OpenAI key 等为 [REDACTED] |
| | 记忆扫描：写入前检查注入/外泄模式 |
| **改动** | agent-diva-agent 的错误处理 + 记忆系统 |
| **收益** | 防止凭证泄露，防止记忆注入攻击 |
| **工作量** | 1 周 |

---

## Phase 3：长期（3-6 月）

> 目标：生态建设，差异化竞争

### 3.1 Kanban 看板系统

| 项目 | 说明 |
|------|------|
| **任务** | 引入结构化多Agent任务编排 |
| **参考** | Hermes SQLite 看板 + 依赖门控 + Worker所有权守卫 |
| **设计** | |
| | SQLite 持久化任务看板 |
| | 任务状态：ready → assigned → running → completed/blocked |
| | 依赖门控：父任务完成→子任务自动晋升 |
| | Worker所有权：每个Worker只能操作自己的任务 |
| **改动** | 新增 agent-diva-kanban crate |
| **收益** | 支持复杂多Agent协作场景 |
| **工作量** | 3-4 周 |

### 3.2 Agent角色系统

| 项目 | 说明 |
|------|------|
| **任务** | 定义子Agent角色模板 |
| **参考** | Codex explorer/worker + 自定义角色 / Hermes orchestrator/leaf |
| **设计** | |
| | explorer: 只读工具 + 低推理强度 + 快速探索 |
| | worker: 读写工具 + 标准推理 + 执行任务 |
| | reviewer: 只读工具 + 高推理 + 代码审查 |
| | 自定义：用户可定义角色配置 |
| **改动** | agent-diva-agent 的 SubagentManager |
| **收益** | 子Agent配置标准化，提升编排效率 |
| **工作量** | 2-3 周 |

### 3.3 对抗性验证协议

| 项目 | 说明 |
|------|------|
| **任务** | 任务完成后强制 spawn 独立验证子Agent |
| **参考** | GenericAgent verify_sop（证伪目标，不给执行历史） |
| **设计** | |
| | 验证者目标：证明交付物不工作（而非确认工作） |
| | 不接收执行历史（避免上下文污染） |
| | 输出 VERDICT: PASS/FAIL/PARTIAL |
| | FAIL 触发修复循环，最多2次重试后升级给用户 |
| **改动** | agent-diva-agent 的任务完成回调 |
| **收益** | 显著提升任务交付质量 |
| **工作量** | 2-3 周 |

### 3.4 文件干预机制

| 项目 | 说明 |
|------|------|
| **任务** | 运行中的Agent每轮检查特殊文件 |
| **参考** | GenericAgent _keyinfo/_intervene/_stop |
| **设计** | |
| | _keyinfo: 注入工作记忆 |
| | _intervene: 纠正方向 |
| | _stop: 终止信号 |
| **改动** | agent-diva-agent 的 agent loop turn_end |
| **收益** | 无需WebSocket即可实时干预Agent行为 |
| **工作量** | 1 周 |

### 3.5 知识图谱（可选）

| 项目 | 说明 |
|------|------|
| **任务** | 引入轻量知识图谱存储 |
| **参考** | openfang 实体/关系/图遍历 / memtle 时间边界三元组 |
| **设计** | |
| | SQLite 存储：entities + triples 表 |
| | 时间边界：valid_from / valid_to |
| | 图遍历：BFS/DFS 查询 |
| **改动** | agent-diva-memory crate 扩展 |
| **收益** | 结构化知识存储，支持复杂查询 |
| **工作量** | 3-4 周 |

### 3.6 技能生态系统

| 项目 | 说明 |
|------|------|
| **任务** | 建立技能远程安装和安全扫描 |
| **参考** | Hermes Skills Hub(10+源) + 安全深度防御 |
| **设计** | |
| | 源适配器：GitHub、本地目录、远程索引 |
| | 安全扫描：路径规范化 + SSRF防护 + 隔离区扫描 |
| | 渐进披露：Tier 1(元数据) → Tier 2-3(完整内容) |
| **改动** | agent-diva-agent 的 skill loader 扩展 |
| **收益** | 技能生态扩展 |
| **工作量** | 4-6 周 |

---

## 里程碑时间线

```
Week 1-2:   Phase 0 完成 (超时/截断/迭代上限/错误日志/CLAUDE.md)
Week 3-4:   Phase 1.1 上下文压缩管线
Week 5-6:   Phase 1.2 错误分类管道 + Phase 1.4 断路器
Week 7-8:   Phase 1.3 子Agent安全模型 + Phase 1.7 IterationBudget
Week 9-10:  Phase 1.5 分层System Prompt + Phase 1.6 钩子系统
Week 11-14: Phase 2.1 工具并发 + Phase 2.3 评估框架
Week 15-18: Phase 2.2 记忆系统升级 + Phase 2.4 MCP增强
Week 19-22: Phase 2.5-2.7 健康监控/Token优化/凭证安全
Week 23-30: Phase 3.1-3.2 Kanban + 角色系统
Week 31-36: Phase 3.3-3.6 验证/干预/知识图谱/技能生态
```

---

## 风险与缓解

| 风险 | 影响 | 缓解策略 |
|------|------|----------|
| Phase 1 范围过大 | 延期 | 优先做 1.1(压缩) + 1.2(错误) + 1.4(断路器)，其余推到 Phase 2 |
| 记忆系统升级破坏兼容 | 用户数据丢失 | 保留 JSONL 作为 fallback，渐进迁移 |
| 评估框架投入大但回报慢 | 资源浪费 | 先建 5 个核心 E2E 场景，不追求全覆盖 |
| Kanban 系统复杂度高 | 过度设计 | 先实现最小可用版本(单看板+基础状态机)，再迭代 |

---

## 与竞品的差异化策略

agent-diva 不需要复制 Hermes 的全部功能（那是 22+ 通道的重量级平台），也不需要复制 Claude Code 的工具链（那是 Anthropic 的产品）。差异化在于：

1. **编排优先** — agent-diva 是大脑，Claude Code/Codex 是手脚
2. **Rust 原生** — 性能和类型安全是 Python 项目无法比拟的
3. **模块化** — 用户可以只使用需要的 crate，不需要全量部署
4. **消息总线** — 异步解耦架构天然支持多通道多Agent
5. **Windows 服务** — 唯一支持原生 Windows 服务部署的 Agent 框架

演进路线应强化这些差异化，而非追逐功能数量。
