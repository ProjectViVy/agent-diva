# agent-diva 功能清单：能力状态与差距分析

> 生成日期：2026-06-01
> 基于 7 个项目对比矩阵，定位 agent-diva 的能力现状

---

## 分类说明

- **已有且良好** — 不需要改变，保持现状
- **已有但不足** — 需要增强
- **完全缺失** — 需要新建
- **不需要** — 明确排除（标注原因）

优先级：P0 = 必须做，P1 = 应该做，P2 = 可以做，P3 = 以后再说

---

## 一、已有且良好 ✅

这些能力不需要改变，是 agent-diva 的核心竞争力。

| # | 能力 | 说明 | 优势来源 |
|---|------|------|----------|
| 1 | **消息总线架构** | 双队列 inbound/outbound，异步解耦 | 优于 GenericAgent 单线程 Queue，优于 Hermes 胖 Agent |
| 2 | **ChannelHandler trait** | 统一抽象，8+ 平台即插即用 | 与 Hermes PlatformRegistry 同级 |
| 3 | **Provider trait** | 统一 LLM 接口 + LiteLLM 兼容 | 优于 GenericAgent 手动 session 选择 |
| 4 | **Rust 类型安全** | thiserror + anyhow，编译期保证 | 所有 Python 项目无法比拟 |
| 5 | **模块化 workspace** | 多 crate 职责清晰 | 与 Codex CLI 同级 |
| 6 | **JSONL Session 持久化** | 可靠的会话存储 | 与 Claude Code 同级 |
| 7 | **CLI 统一入口** | onboard/gateway/agent/tui/cron | 完整的命令体系 |
| 8 | **Windows 服务** | agent-diva-service 原生服务 | 所有对比项目均无 |
| 9 | **文件附件系统** | 内容寻址(SHA256)自动去重 | 设计良好 |
| 10 | **MCP Server 能力** | 可作为 MCP 服务器被连接 | 与 Codex CLI 同级 |

---

## 二、已有但不足 ⚠️

这些能力存在但需要增强。按优先级排序。

### P0 — 必须增强

| # | 能力 | 现状 | 差距 | 参考项目 | 建议方向 |
|---|------|------|------|----------|----------|
| 1 | **上下文管理** | 基础组装，无压缩 | 所有对比项目都有压缩机制 | Claude Code 4层管线 / Hermes 5阶段压缩 | 引入分层压缩：L1 snip → L2 micro → L3 budget → L4 auto |
| 2 | **错误处理** | 简单 try/catch | 缺少结构化分类和自动恢复 | Hermes 8级错误分类 / Claude Code 16+种reason code | 引入 ClassifiedError 数据类 + 恢复策略管道 |
| 3 | **迭代控制** | 基础限制 | 缺少预算制、缺少退还机制 | Hermes IterationBudget(90次) / GenericAgent 多层级限制 | 引入 IterationBudget + consume/refund 模式 |
| 4 | **子Agent安全** | SubagentManager 轻量 spawn | 无工具黑名单、无深度控制、无角色模型 | Hermes DELEGATE_BLOCKED_TOOLS + MAX_DEPTH + 角色 | 引入工具黑名单 + 深度控制 + 角色模板 |

### P1 — 应该增强

| # | 能力 | 现状 | 差距 | 参考项目 | 建议方向 |
|---|------|------|------|----------|----------|
| 5 | **工具执行** | 纯串行执行 | 多工具场景延迟高 | Hermes ThreadPool(8) + 路径冲突检测 / Claude Code 只读并发 | 引入并发执行：只读并发 + 写入串行 + 路径冲突检测 |
| 6 | **MCP 集成** | 基础 MCP 支持 | 缺少断路器、缺少安全模型 | Hermes 断路器+环境过滤+凭证脱敏 / Claude Code 6种传输 | 增强 MCP：断路器 + 安全过滤 + 动态刷新 |
| 7 | **记忆系统** | JSONL + MEMORY.md 扁平结构 | 无分层、无检索、无压缩 | GenericAgent L1-L4 四层 / openfang 5存储统一API / memtle BM25 | 升级为分层记忆：索引层→事实层→归档层 |
| 8 | **System Prompt 缓存** | 无分层缓存策略 | 每轮重建，浪费 API 缓存 | Hermes 3层(stable/context/volatile) / Claude Code 3层缓存 | 引入分层 system prompt，最大化前缀缓存命中 |
| 9 | **技能系统** | Markdown Skills 基础加载 | 无热更新、无安全扫描、无渐进披露 | Hermes 渐进披露+安全扫描 / Claude Code 按需加载 | 增强：热更新 + 安全扫描 + 渐进披露 |

### P2 — 可以增强

| # | 能力 | 现状 | 差距 | 参考项目 | 建议方向 |
|---|------|------|------|----------|----------|
| 10 | **Session 管理** | JSONL 持久化 | 缺少 FTS5 搜索、缺少会话谱系 | Hermes SQLite+FTS5 / Codex SQLite+Rollout | 考虑迁移到 SQLite，获得搜索和更好的并发 |
| 11 | **Cron 系统** | cron 命令基础支持 | 缺少 at-most-once 语义、缺少链式引用 | Hermes no_agent+LLM 双模式 + context_from | 增强 Cron：作业链 + 多投递目标 |
| 12 | **Provider 错误消歧** | 无 | 402 错误可能被误判 | Hermes 区分支付错误和瞬时限速 | 在 provider 层引入错误消歧义 |
| 13 | **子Agent文件协调** | 无 | 子Agent修改文件后父Agent不知情 | Hermes 自动检测+追加重读通知 | 引入文件变更通知机制 |

---

## 三、完全缺失 ❌

这些能力需要新建。按优先级排序。

### P0 — 必须新建

| # | 能力 | 说明 | 参考项目 | 建议实现 |
|---|------|------|----------|----------|
| 1 | **上下文溢出恢复** | token 超限时自动截断/压缩 | openfang recover_from_overflow / Claude Code reactive compact | 在 context builder 中实现溢出检测→截断→重试管线 |
| 2 | **断路器机制** | 连续失败后阻止重试风暴 | Hermes 连续3次失败→60秒冷却 / openfang 熔断 | 在 provider 和 MCP 层引入断路器 |
| 3 | **工具执行超时** | 单个工具执行无超时限制 | Claude Code timeout / Codex 沙箱超时 / openfang 超时包装 | 每个工具执行添加超时包装 |
| 4 | **钩子系统** | 无法扩展 agent 行为 | Claude Code 27种hook / Hermes HOOK.yaml / OpenHarness 10个事件 | 引入 PreToolUse/PostToolUse/Stop 等核心钩子 |

### P1 — 应该新建

| # | 能力 | 说明 | 参考项目 | 建议实现 |
|---|------|------|----------|----------|
| 5 | **评估框架** | 无测试框架、无质量门禁 | OpenHarness 三层测试+五层断言 / Claude Code CI/CD | 建立 单元→集成→E2E 三层测试体系 |
| 6 | **子Agent健康监控** | 无心跳、无超时诊断 | Hermes 心跳线程+超时诊断转储 / Codex wait_agent | 引入心跳机制 + 超时诊断 |
| 7 | **工具Token优化** | 工具描述每轮重复发送 | GenericAgent "still active"缓存 / Hermes Progressive Disclosure(BM25) | 引入工具描述缓存 + 按需加载 |
| 8 | **凭证脱敏** | 错误消息可能泄露 API key | Hermes _sanitize_error / memtle 错误消毒 | 在错误处理管线中引入凭证过滤 |
| 9 | **Agent发现协议** | 子Agent无法被外部发现 | openfang a2a_discover + Agent Cards / Hermes delegate_task | 引入 Agent 注册和发现机制 |
| 10 | **工具结果截断** | 大结果直接发送给 LLM | Claude Code tool_result_budget / openfang 上下文预算 | 引入工具结果大小限制和截断 |

### P2 — 可以新建

| # | 能力 | 说明 | 参考项目 | 建议实现 |
|---|------|------|----------|----------|
| 11 | **Kanban 看板** | 结构化多Agent任务编排 | Hermes SQLite 看板 + 依赖门控 | 引入持久化任务看板 |
| 12 | **Agent角色系统** | 子Agent无角色区分 | Codex explorer/worker + 自定义角色 | 定义角色模板（explorer/worker/reviewer） |
| 13 | **Worktree 隔离** | 并行子Agent文件冲突 | Claude Code EnterWorktree / OpenHarness Git worktree | 引入 git worktree 隔离 |
| 14 | **知识图谱** | 无结构化知识存储 | openfang 实体/关系/图遍历 / memtle 时间边界三元组 | 引入轻量知识图谱 |
| 15 | **对抗性验证** | 无独立验证机制 | GenericAgent verify_sop（证伪目标） | 任务完成后强制 spawn 验证子Agent |
| 16 | **文件干预机制** | 无法实时注入上下文 | GenericAgent _keyinfo/_intervene/_stop | 在 agent loop 中检查特殊文件 |
| 17 | **记忆安全检查** | 记忆写入无安全扫描 | Hermes 威胁模式扫描 / OpenHarness 去重+软删除 | 引入记忆写入前的安全扫描 |
| 18 | **A2A 协议** | 无跨框架互操作 | openfang Google A2A(JSON-RPC 2.0) | 引入标准 A2A 协议 |

### P3 — 以后再说

| # | 能力 | 说明 | 参考项目 | 建议实现 |
|---|------|------|----------|----------|
| 19 | **Docker 沙箱** | 工具执行无容器隔离 | OpenHarness Docker 沙箱 / openfang WASM 沙箱 | 可选 Docker 沙箱执行 |
| 20 | **平台级沙箱** | 无操作系统级隔离 | Codex Seatbelt/Landlock/Windows | 未来考虑平台特定沙箱 |
| 21 | **技能远程安装** | 无技能市场 | Hermes Skills Hub(10+源) / Claude Code Marketplace | 建立技能生态系统 |
| 22 | **AAAK 压缩** | 无有损符号压缩 | memtle 30倍压缩比 | 适用于大量历史记忆场景 |
| 23 | **Dream 整合** | 无自动记忆整理 | Claude Dream 四层门控 | 会话结束后自动提取记忆 |
| 24 | **YoloClassifier** | 无自动审批分类 | Claude Code auto 模式 | 用 LLM 自动判断工具调用安全性 |

---

## 四、不需要 ✖️

明确排除的能力，附原因。

| # | 能力 | 排除原因 | 参考项目 |
|---|------|----------|----------|
| 1 | **RL 训练循环** | agent-diva 定位是编排框架，不是训练平台。训练应在外部完成 | 无项目实现 |
| 2 | **蜂群(Swarm)模式** | "多个 Claude Code 就是蜂群本身"——agent-diva 做大脑，不做蜂群 | OpenHarness Swarm |
| 3 | **极强代码能力** | agent-diva 重点是编排和协调，代码能力通过子Agent(Claude Code/Codex)获取 | Claude Code / Codex |
| 4 | **内嵌向量数据库** | 优先使用 BM25 倒排索引（无外部依赖），向量搜索作为可选后端 | openfang Qdrant |
| 5 | **40+ 通道适配器** | 质量优于数量，聚焦核心平台 | openfang 40通道 |
| 6 | **GUI 桌面应用** | agent-diva-gui 已存在（Tauri+Vue），不需要重新实现 | — |
| 7 | **图像生成工具** | 非核心能力，可通过 MCP 扩展 | Codex ImageGeneration |
| 8 | **浏览器自动化** | 非核心能力，可通过 MCP 扩展 | openfang 10个浏览器工具 |

---

## 五、能力成熟度总览

```
agent-diva 能力雷达图（满分 5 分）

Agent Loop      ████████░░  3/5  (基础完整，缺压缩/错误分类/预算)
工具链          ██████░░░░  2/5  (基础完整，缺并发/MCP增强/安全)
A2A             ████░░░░░░  2/5  (基础 spawn，缺安全/监控/编排)
记忆系统        ████░░░░░░  2/5  (基础持久化，缺分层/检索/压缩)
技能系统        ██████░░░░  2/5  (基础加载，缺热更新/安全/披露)
通道/多平台     ████████░░  4/5  (良好，缺钩子系统)
评估/QA         ██░░░░░░░░  1/5  (几乎空白)
学习/进化       ██░░░░░░░░  1/5  (几乎空白)
安全            ██░░░░░░░░  1/5  (几乎空白)
```

**总体评估**：agent-diva 的基础设施（消息总线、Channel、Provider、类型安全）是优秀的，但在运行时智能（压缩、错误恢复、安全）和生态（评估、记忆、技能）方面存在显著差距。Phase 1 的重点应放在 Agent Loop 增强和安全基础上。
