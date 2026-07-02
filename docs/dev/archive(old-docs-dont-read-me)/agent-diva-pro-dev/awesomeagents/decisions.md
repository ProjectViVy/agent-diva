# agent-diva 能力增强决策记录

> 日期：2026-06-01
> 来源：基于 7 项目对比调研的 27 项未知缺陷分析，经用户逐项审核后的最终决策
> 调研文档：`docs/dev/awesomeagents/`

---

## 一、背景

基于对 `.workspace/` 下 7 个 agent 项目（GenericAgent、Hermes Agent、Claude Code、OpenHarness、Codex CLI、openfang、memtle）的深度对比调研，识别出 agent-diva 的 27 个未知缺陷和 24 个能力缺失项。

用户对每一项进行了逐项审核，依据以下原则：
- agent-diva 定位：智能体"大脑"，管理外部 agent 执行操作
- 不做执行器，shell 安全等归沙箱分支
- 不做重复轮子，已有调研或已有排期的不重复
- 需架构变更的必须先调研再决策

---

## 二、P0 — 立即执行（本周内）

不改架构，改动小，收益明确。

| 编号 | 缺陷 | 修复方向 | 参考项目 |
|------|------|---------|---------|
| **P0-1** | 子Agent安全三件套 | ① `DELEGATE_BLOCKED_TOOLS` 工具黑名单<br>② `max_depth` 递归深度硬限制<br>③ 子Agent 凭证最小化（选择性传递） | Hermes |
| **P0-2** | 无限循环无熔断 | ① 工具调用 SHA256 哈希去重<br>② 连续相同工具失败 N 次 → 熔断<br>③ `max_iterations` 硬上限 | openfang、Hermes |
| **P0-4** | 子Agent 超时无重试 | 仅超时重试逻辑，不做完整断路器模式 | — |
| **P0-5a** | 路径遍历攻击 | 文件操作前 `..` 遍历拒绝 + 工作区边界校验 | Hermes `validate_within_dir` |
| **P0-5b** | 记忆无写入安全检查 | 写入前正则扫描常见 injection 模式（"忽略所有指令"、"发送到外部"等） | Hermes 威胁模式扫描 |
| **日志脱敏** | API Key 在日志中泄露 | 日志层面正则过滤 + `[REDACTED]`，不做通用凭证脱敏 | — |

---

## 三、归其他分支（本目录不做）

| 编号 | 缺陷 | 归处 | 原因 | 审计结果 |
|------|------|------|------|---------|
| P0-3 Shell | Shell 命令注入安全 | 沙箱分支 | 非"大脑"范畴，属执行器安全 | ⚠️ 已部分实现（见第八节） |
| #5 | 流式响应中断无恢复 | 沙箱分支 | 同上 | ⚠️ 仅 keepalive+合成，无重试 |
| P1-6 分层记忆 | 扁平记忆检索效率低 | 已有另一个分支 | 已有独立演进路线 | 待核查 |

---

## 四、已有排期（不做重复）

| 编号 | 内容 | 状态 |
|------|------|------|
| P1-1 上下文压缩管线 | 已有调研，排期在当前链路之后 | — |

---

## 五、需进一步调研（排期顺序）

以下 5 项需要先产出调研方案，经审核后再决定是否实施及如何实施。

### 调研优先级排序

| 优先级 | 编号 | 内容 | 调研原因 |
|--------|------|------|---------|
| **🔴 最高** | P1-5 Prompt 缓存 | 3 层（stable/context/volatile）系统提示词缓存 | 每次 LLM 调用省 20-30% token，复利效应最大。可能提 P0 |
| **🟠 高** | P1-2 错误分类管道 | 8 级优先级错误分类 + 自动恢复策略 | 所有可靠性改进的地基，需审核方案，不可照搬 Hermes |
| **🟡 中** | P1-7 工具链优化 | 工具描述缓存 + 输出截断 + MCP 断路器 | 三个独立小优化，可逐个落地 |
| **🟡 中** | MCP 链路增强 | MCP 工具桥接脆弱、欠维护 | 当前可用但不优雅，承接工具链优化中的 MCP 断路器 |
| **🟢 低** | P1-8 子Agent质量保障 | 空响应检测 + 流式恢复 + 对抗性验证 | 依赖错误分类管道基础能力，对抗性验证依赖 P0-1 安全三件套 |

> 排序逻辑：每次 LLM 调用受益的优先（Prompt 缓存 > 错误分类），地基型的优先（错误分类 > 工具优化），独立的优先（MCP 增强承接 MCP 断路器），依赖多的垫后（子Agent 质量保障依赖前两项）。

### 各调研项要点

#### MCP 链路增强
- 现状评估：连接稳定性、重连机制、错误处理
- 参考 Hermes：自动重连 + 断路器 + 动态工具刷新
- 参考 openfang：call_with_retry + 熔断
- 产出：MCP 可靠性增强方案

#### Prompt 缓存
- 参考 Hermes：3 层分层（stable/context/volatile），字节稳定设计
- 参考 Claude Code：CLAUDE.md + rules + auto-memory
- 分析 agent-diva 当前 system prompt 组装流程
- 产出：分层缓存方案 + Token 节省预估

#### 错误分类管道
- 参考 Hermes：8 级优先级 + `ClassifiedError` 数据类
- 参考 openfang：幽灵动作检测 + 上下文溢出恢复
- 需考虑 agent-diva Rust 架构的适配
- 产出：分类方案 + 与现有错误处理的兼容分析

#### 工具链优化
- 工具描述缓存：参考 GenericAgent "Tools: still active"
- 工具输出截断：参考 Claude Code `tool_result_budget`
- MCP 断路器：参考 Hermes 3 次失败 → 60s 冷却

#### 子Agent 质量保障
- 空响应/幽灵动作检测：参考 openfang
- 流式中断恢复：参考 Claude Code `finish_reason=="length"`
- 对抗性验证：参考 GenericAgent `verify_sop.md`

---

## 六、明确不做

| 内容 | 原因 |
|------|------|
| RL 训练闭环 | 可选 Phase 4，非核心需求 |
| 蜂群模式 | 多个 Claude Code 本身就是蜂群 |
| agent-diva 自身具备极强代码能力 | 定位是大脑/编排者，非执行器 |
| 通用凭证脱敏（非日志） | 用户可能让 agent 配置密钥，厂商自带脱敏 |
| Shell 沙箱 | 归沙箱分支 |

---

## 七、相关文档

| 文档 | 路径 |
|------|------|
| GenericAgent 分析 | `docs/dev/awesomeagents/genericagent-analysis.md` |
| Hermes Agent 分析 | `docs/dev/awesomeagents/hermes-agent-analysis.md` |
| Claude Code 分析 | `docs/dev/awesomeagents/claude-code-analysis.md` |
| OpenHarness 分析 | `docs/dev/awesomeagents/openharness-analysis.md` |
| Codex 分析 | `docs/dev/awesomeagents/codex-analysis.md` |
| 其他项目分析 | `docs/dev/awesomeagents/other-projects-analysis.md` |
| 功能对比矩阵 | `docs/dev/awesomeagents/comparison-matrix.md` |
| 能力清单 | `docs/dev/awesomeagents/diva-capability-checklist.md` |
| 演进路线 | `docs/dev/awesomeagents/evolution-roadmap.md` |
| 未知缺陷分析 | `docs/dev/awesomeagents/unknown-deficits.md` |
| 沙箱能力清单（74项） | `docs/dev/awesomeagents/sandbox-audit-checklist.md` |
| 沙箱文件地图 | `docs/dev/awesomeagents/sandbox-files-map.md` |
| 沙箱审计 A（隔离+Shell） | `docs/dev/awesomeagents/sandbox-audit-a.md` |
| 沙箱审计 B（凭证+注入+熔断） | `docs/dev/awesomeagents/sandbox-audit-b.md` |
| 沙箱审计 C（子Agent+MCP+审计） | `docs/dev/awesomeagents/sandbox-audit-c.md` |

---

## 八、沙箱分支审计结果

> 审计日期：2026-06-02
> 目标：核查 `../agent-diva-sandbox/` 是否实现了 6 篇分析报告中提到的安全能力
> 方法：先提取 74 项安全能力清单，再逐项与沙箱代码交叉核查

### 8.1 沙箱分支架构

```
Agent Loop → ToolOrchestrator → Guardian(自动审批+熔断) → ExecPolicy(规则引擎) → SandboxManager → Platform Executor
                                                                                                   ├── Windows: RestrictedToken
                                                                                                   ├── Linux:   Landlock+Bwrap+Seccomp
                                                                                                   └── macOS:   Seatbelt(sandbox-exec)
```

共 15 个 Rust 核心文件，分层清晰，灵感源自 OpenAI Codex CLI。

### 8.2 审计汇总（29 项关键能力）

| 维度 | 报告 | ✅ | ⚠️ | ❌ |
|------|------|----|----|-----|
| A 沙箱隔离+Shell安全 | `sandbox-audit-a.md` | 5 | 3 | 1 |
| B 凭证安全+注入防护+熔断 | `sandbox-audit-b.md` | 2 | 4 | 3 |
| C 子Agent安全+MCP安全+审计 | `sandbox-audit-c.md` | 3 | 5 | 3 |
| **合计** | | **10** | **12** | **7** |

### 8.3 已完整实现（✅ 10 项）

- 三层 Shell 命令黑名单 + 5 阶段审批编排
- 命令参数过滤 + 工作区限制
- 双系统文件访问控制 + OS 级强制
- 8 层路径遍历防护纵深
- 平台沙箱：Linux Landlock+Bwrap+Seccomp / macOS Seatbelt
- 迭代预算控制（max_iterations=20）
- 审批缓存三级决策（Denied/ApprovedOnce/ApprovedForSession）
- 子Agent 递归深度控制
- MCP 工具短路保护
- 文件冲突检测（五层：版本号+SQLite+TOCTOU+断路器+限流）

### 8.4 高危缺失（❌ 7 项，P0 级）

| 缺陷 | 风险 | 修复难度 |
|------|------|---------|
| **MCP 环境变量全量透传** | 恶意 MCP 服务器可读取宿主机全部 API key | 低（白名单过滤即可） |
| **MCP 请求大小无限制** | 大请求可 OOM | 低（一行大小检查） |
| **Prompt 注入扫描** | MCP 工具描述/LM 输出可注入恶意指令 | 中（需正则+LLM 双重检测） |
| **威胁模式扫描（记忆写入）** | LLM 输出直写磁盘，持久化注入 | 中（参考 Hermes） |
| **子Agent 并发无限制** | 资源耗尽 | 低（max_concurrent 参数） |
| **Windows 沙箱存根** | is_available() 返回 false，无实质隔离 | 高（需 Windows API 适配） |
| **隔离区扫描** | 下载的插件/扩展无安全检查 | 中（四步流水线） |

### 8.5 部分实现（⚠️ 12 项，需补漏）

- 凭证日志脱敏：CLI config show 有脱敏，日志层无全局过滤
- 断路器：仅审批拒绝维度，无工具执行失败熔断
- 流式中断恢复：仅 250ms keepalive + 不完整流合成，无连接级重试
- 空响应/幽灵动作检测：仅空 final_content 兜底，无 stall 检测
- 子Agent 工具黑名单：仅 allow/deny 标记字段，未联动运行时
- 子Agent 超时：仅外层 tokio::timeout，无诊断转储
- 子Agent 凭证最小化：无选择性传递机制
- 健康检查：仅 CLI ping，无结构化端点
- 审计日志：仅 tracing span，无持久化
- 进程资源限制：仅超时，无 CPU/内存限制
- 网络出站控制：macOS/Linux 已实现，Windows 缺失
- 平台沙箱：Linux/macOS 完整，Windows 存根

### 8.6 与本目录的关系

以下 P0 项因沙箱分支已覆盖，本目录可降级或跳过：

| 本目录 P0 项 | 沙箱分支状态 | 决策 |
|-------------|------------|------|
| P0-3 Shell 命令注入 | ✅ 已有三层黑名单 | 本目录跳过 |
| 路径遍历防护 | ✅ 已有 8 层纵深 | 本目录 P0-5a 仍做（独立于沙箱的轻量防护） |
| 日志脱敏 | ⚠️ 仅 CLI 层 | 本目录补全局日志过滤 |

以下 P0 项沙箱分支完全未覆盖，本目录仍需做：

| 本目录 P0 项 | 沙箱分支状态 |
|-------------|------------|
| P0-1 子Agent安全三件套 | ⚠️ 部分（深度有，黑名单/凭证无） |
| P0-2 熔断器 | ⚠️ 仅审批维度 |
| P0-5b 记忆写入安全 | ❌ 完全缺失 |
| #5 流式中断恢复 | ⚠️ 仅 keepalive |

---

## 九、UI 设计决策

> 日期：2026-06-02
> 背景：6 篇参考项目分析均未涉及 UI 设计。agent-diva 作为"大脑"需要调度操作台，但遵循极简原则——一行字符能提示完成的绝不做复杂界面。

### 9.1 原则

- 只做功能性阻塞项——不做就卡住流程的东西
- CLI 优先，GUI 仅用于复杂配置（如沙箱规则编辑）
- 参考 agent-diva-pro 分支已有实现，避免重复造轮子

### 9.2 三个最小 UI 项

| 优先级 | 功能 | 形态 | 说明 |
|--------|------|------|------|
| **P0** | 沙箱审批 | CLI 一行交互 | `⚠ cmd: rm -rf ./x [y/n/session]?` — 后端审批管线沙箱分支已有 |
| **P0** | 沙箱配置 | GUI 独立设置页 | TOML 规则表单化编辑（Allow/Prompt/Forbidden 三元组），参考 pro 分支 |
| **P1** | 上下文健康 | CLI 状态行 | `[ctx 67% | 87K/130K]` 嵌入响应末尾，>70% 黄色 >85% 红色告警 |

### 9.3 明确不做

| 内容 | 原因 |
|------|------|
| Kanban 看板 | 等 plan 阶段结束后再考虑 |
| 成本统计（金额） | 中国 provider 生态混乱，仅记 token 数不计价 |
| 复杂仪表盘 | 违背极简原则 |
| 编排历史回溯 | 非功能性阻塞 |
| 记忆/Skill 可视化 | 非功能性阻塞 |

### 9.4 待核查

pro 分支 UI 实现审计完成 → `docs/dev/awesomeagents/pro-ui-audit.md`

**审计结论：**

| 功能 | 状态 | 关键发现 | 可复用 |
|------|------|---------|--------|
| 沙箱审批 | ⚠️ | 前有权限选择器 UI，后有 deny/allow 守卫，但中间断连 | `appDialog.ts`、权限选择器 UI |
| 沙箱配置 | ❌ | 无独立设置页，规则硬编码 | `ConfigEditor`、SettingsView 导航 |
| 上下文健康 | ⚠️ | 后端 TokenBudget 完善，但聊天界面零指示 | `TokenStatsPanel`、`TokenBudget` |

---

## 十、文档总览

```
docs/dev/awesomeagents/
├── decisions.md                      ← 本文档（最终决策记录）
│
├── 6 篇项目分析
│   ├── genericagent-analysis.md      (544行)
│   ├── hermes-agent-analysis.md      (1018行)
│   ├── claude-code-analysis.md       (707行)
│   ├── openharness-analysis.md       (627行)
│   ├── codex-analysis.md             (489行)
│   └── other-projects-analysis.md    (464行)
│
├── 4 篇汇总产出
│   ├── comparison-matrix.md          功能对比矩阵
│   ├── diva-capability-checklist.md  能力清单
│   ├── evolution-roadmap.md          演进路线
│   └── unknown-deficits.md           27 个未知缺陷
│
├── 沙箱审计
│   ├── sandbox-audit-checklist.md    74 项安全能力清单
│   ├── sandbox-files-map.md          沙箱文件结构
│   ├── sandbox-audit-a.md            沙箱隔离+Shell（5✅/3⚠️/1❌）
│   ├── sandbox-audit-b.md            凭证+注入+熔断（2✅/4⚠️/3❌）
│   └── sandbox-audit-c.md            子Agent+MCP+审计（3✅/5⚠️/3❌）
│
├── pro 分支审计
│   └── pro-ui-audit.md               审批/配置/上下文健康
│
├── sandbox-verification.md           沙箱初核（Shell+流式）
│
└── 已有文档（hermes-learning/、hermes-integration/）
```

### 当前状态总结

| 类别 | 数量 | 状态 |
|------|------|------|
| P0 执行项 | 6 | 决策已定，待开工 |
| 调研队列 | 5 | ①Prompt缓存 ②错误分类 ③工具链优化 ④MCP增强 ⑤子Agent质保 |
| 归沙箱分支 | 3 | 已审计：Shell ✅，流式恢复 ⚠️ |
| 已有排期 | 1 | 上下文压缩 |
| UI 项 | 3 | 审批 P0 / 配置 P0 / 上下文健康 P1 |
