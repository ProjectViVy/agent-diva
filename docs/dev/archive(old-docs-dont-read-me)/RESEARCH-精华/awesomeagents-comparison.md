# AwesomeAgents 调研精华（压缩版）

> 原始：7 个项目 × 6 篇分析 + 4 篇汇总 = 24 篇文档，~5000 行
> 压缩原则：只保留结论和可行动作，去除过程性描述

---

## 1. 调研范围

| 项目 | 语言 | 代码量 | 关键特色 |
|------|------|--------|----------|
| GenericAgent | Python | ~3K | 自进化、L1-L4 记忆、verify_sop |
| Hermes Agent | Python | 大型 | 22+ 通道、8 级错误分类、3 层 prompt 缓存 |
| OpenHarness | Python | 大型 | Claude Code 开源移植、Docker 沙箱、3 层评估 |
| Claude Code | TypeScript | 大型 | 4 层压缩管线、27 种 hook、worktree 隔离 |
| Codex CLI | Rust | 中型 | Op/Event 驱动、平台级沙箱、角色系统 |
| openfang | Rust | ~137K | 16 层纵深防御、call_with_retry+熔断、WASM 沙箱 |
| agent-diva | Rust | 多 crate | 消息总线、ChannelHandler trait、Provider 抽象 |

---

## 2. agent-diva 的 7 个显著短板（按优先级）

| # | 短板 | 对比参照 | 修复方向 |
|---|------|----------|----------|
| 1 | **无上下文压缩** | Claude 4 层 / Hermes 5 阶段 | 3 层管线（snip→budget→auto） |
| 2 | **无错误分类** | Hermes 8 级 / Claude 16+ reason | ClassifiedError + 恢复策略 |
| 3 | **无工具并发** | Hermes ThreadPool(8) | 只读并发 + 写入串行 + 路径冲突 |
| 4 | **无评估体系** | OpenHarness 3 层 + 5 断言 | 单元→集成→E2E 渐进建设 |
| 5 | **扁平记忆** | GenericAgent L1-L4 / openfang 5 存储 | L1 索引层先落地 |
| 6 | **无安全沙箱** | Codex 平台级 / openfang 16 层 | 沙箱分支已有部分实现 |
| 7 | **无钩子系统** | Claude 27 种 / Hermes HOOK.yaml | PreToolUse/PostToolUse 等核心事件 |

---

## 3. agent-diva 的 5 个相对优势（应强化）

1. **消息总线架构** — 双队列异步解耦
2. **ChannelHandler trait** — 统一通道抽象，即插即用
3. **Provider trait** — 统一 LLM 抽象 + LiteLLM 兼容
4. **Rust 类型安全** — thiserror + 模块化设计
5. **Windows 原生服务** — 唯一支持 Windows 服务部署的 Agent 框架

---

## 4. 关键安全发现（沙箱审计摘要）

| 维度 | 通过 | 警告 | 失败 | 关键缺失 |
|------|------|------|------|----------|
| A 隔离+Shell | 5 | 3 | 1 | Windows RestrictedToken dead code |
| B 凭证+注入+熔断 | 2 | 4 | 3 | 日志层无全局脱敏 |
| C 子Agent+MCP+审计 | 3 | 5 | 3 | MCP env 全量透传、子Agent 并发无限制 |

**高危 P0 缺失（7 项）**：
1. MCP 环境变量全量透传
2. MCP 请求大小无限制
3. Prompt 注入扫描缺失
4. 威胁模式扫描（记忆写入）
5. 子Agent 并发无限制
6. Windows 沙箱存根
7. 隔离区扫描缺失

---

## 5. 演进路线速查

### Phase 0（零成本，本周）
- 工具执行超时包装
- 工具结果截断
- 基础迭代上限（max_iterations=50）
- Provider 错误日志结构化
- CLAUDE.md 记忆写入公理

### Phase 1（1-4 周）
- 上下文压缩管线
- 结构化错误分类管道
- 子Agent 安全模型（黑名单 + depth 控制）
- 断路器机制
- 分层 System Prompt
- 钩子系统
- IterationBudget

### Phase 2（1-3 月）
- 工具并发执行
- 记忆系统升级（L1-L4）
- 评估框架
- MCP 增强
- 子Agent 健康监控
- 工具 Token 优化
- 凭证安全

### Phase 3（3-6 月）
- Kanban 看板系统
- Agent 角色系统
- 对抗性验证协议
- 文件干预机制
- 知识图谱（可选）
- 技能生态系统

---

## 6. 原始文档索引

如需查阅原始全文：
- 6 篇项目分析：`agent-diva-main/docs/dev/archive/awesomeagents/*-analysis.md`
- 4 篇汇总：`comparison-matrix.md`、`diva-capability-checklist.md`、`evolution-roadmap.md`、`unknown-deficits.md`
- 沙箱审计：`sandbox-audit-{a,b,c}.md`、`sandbox-verification.md`
- 决策记录：`decisions.md`
