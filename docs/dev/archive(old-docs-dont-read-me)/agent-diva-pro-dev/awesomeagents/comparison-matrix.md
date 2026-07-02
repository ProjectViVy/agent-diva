# 功能对比矩阵：7 大 Agent 项目全景对比

> 生成日期：2026-06-01
> 基于 6 篇深度分析报告汇总

---

## 对比项目一览

| 项目 | 语言 | 定位 | 代码量 |
|------|------|------|--------|
| **GenericAgent** | Python | 自进化自治框架 | ~3K 行 |
| **Hermes Agent** | Python | 多平台 AI Agent 框架 | 大型 |
| **OpenHarness** | Python | Claude Code 开源移植版 | 大型 |
| **Claude Code** | TypeScript | Anthropic 官方 Agent CLI | 大型 |
| **Codex CLI** | Rust | OpenAI 官方 Agent CLI | 中型 |
| **openfang** | Rust | Agent 操作系统 | ~137K LOC |
| **agent-diva** | Rust | Agent 大脑/编排框架 | 多 crate workspace |

---

## 1. Agent Loop 维度

| 子项 | GenericAgent | Hermes | OpenHarness | Claude Code | Codex CLI | openfang | agent-diva |
|------|-------------|--------|-------------|-------------|-----------|----------|------------|
| **循环驱动方式** | Python Generator + Queue | while 循环 + 模块提取 | 异步引擎 | while(true) 异步生成器 | Op/Event 事件驱动 | 双模式(同步/流式) | 消息总线驱动 |
| **迭代控制** | ✅ 多层级(max_turns/Plan/Goal) | ✅ IterationBudget(90次) | ✅ 配置上限 | ✅ max-turns + max-budget | ✅ 可配置 | ✅ 默认50次 | ⚠️ 基础限制 |
| **上下文组装** | ✅ System Prompt + 工作记忆注入 | ✅ 3层缓存(stable/context/volatile) | ✅ CLAUDE.md 分层注入 | ✅ 3层缓存 + 4层压缩 | ✅ 上下文片段模块化 | ✅ 记忆段落注入 | ⚠️ 基础组装 |
| **错误恢复** | ✅ 重试+退避+故障转移 | ✅ 8级错误分类管道 | ✅ Hook阻断适应 | ✅ 16+种reason code | ✅ 自动压缩重试 | ✅ call_with_retry+熔断 | ❌ 简单try/catch |
| **会话管理** | ✅ _history.json 序列化 | ✅ SQLite SessionDB + FTS5 | ✅ 会话保存/恢复 | ✅ JSONL + --resume/--fork | ✅ SQLite + Rollout文件 | ✅ SQLite WAL + JSONL | ✅ JSONL 持久化 |
| **上下文压缩** | ✅ trim_messages + 标签压缩 | ✅ 5阶段压缩算法 | ✅ 自动压缩 | ✅ 4层管线(snip/micro/budget/auto) | ✅ 预采样+自动压缩 | ✅ 溢出恢复管线 | ❌ 无压缩机制 |
| **流式输出** | ✅ yield chunks | ✅ SSE streaming | ✅ 流式事件 | ✅ streaming API | ✅ SSE + WebSocket | ✅ StreamEvent channel | ✅ chat_stream |
| **中断支持** | ✅ stop_sig + 文件信号 | ✅ _interrupt_requested | ✅ 事件中断 | ✅ Interrupt Op | ✅ 用户中断 | ✅ 生命周期钩子 | ⚠️ 基础cancel |

---

## 2. 工具链维度

| 子项 | GenericAgent | Hermes | OpenHarness | Claude Code | Codex CLI | openfang | agent-diva |
|------|-------------|--------|-------------|-------------|-----------|----------|------------|
| **工具注册** | 反射式命名(do_*) | 自注册+AST发现 | BaseTool+注册 | 静态注册表 | ToolSpec+ToolRouter | 能力门控注册 | Tool trait |
| **工具数量** | 9 个 | 30+ 内置 | 44 内置 | 50+ 内置 | 11 内置 | 53+ 内置 | 可扩展 |
| **工具发现** | ❌ 静态schema | ✅ AST扫描 | ✅ 4层加载 | ✅ SearchExtraTools | ✅ tool_search动态 | ✅ 三级回退 | ❌ 手动注册 |
| **并发执行** | ❌ 纯串行 | ✅ ThreadPool(8) + 路径冲突检测 | ✅ 并行工具 | ✅ 只读并发/写入串行 | ✅ ToolCallRuntime | ⚠️ 有限 | ❌ 纯串行 |
| **MCP支持** | ❌ 自定义XML协议 | ✅ 完整(stdio/HTTP/SSE) | ✅ MCP集成 | ✅ 完整(6种传输) | ✅ 客户端+服务器 | ✅ 双向MCP | ⚠️ 基础MCP |
| **安全模型** | ⚠️ 6层轻量防御 | ✅ 深度防御(环境过滤/凭证脱敏/注入扫描/OSV检查) | ✅ 权限系统 | ✅ 多层权限+YoloClassifier | ✅ 分层沙箱(Seatbelt/Landlock/Windows) | ✅ 16层纵深防御 | ❌ 无沙箱 |
| **Token优化** | ✅ "still active"缓存 | ✅ Progressive Disclosure(BM25) | ⚠️ 基础 | ✅ 4层压缩管线 | ✅ 远程压缩 | ✅ 上下文预算截断 | ❌ 无优化 |
| **工具热更新** | ❌ 需重启 | ✅ 动态刷新 | ✅ 热加载 | ✅ 动态发现 | ✅ 运行时配置 | ⚠️ 有限 | ❌ 不支持 |
| **沙箱隔离** | ❌ 进程级无沙箱 | ❌ 无内置沙箱 | ✅ Docker沙箱 | ❌ 依赖用户审批 | ✅ 平台级沙箱 | ✅ WASM沙箱 | ❌ 无沙箱 |

---

## 3. A2A（Agent-to-Agent）维度

| 子项 | GenericAgent | Hermes | OpenHarness | Claude Code | Codex CLI | openfang | agent-diva |
|------|-------------|--------|-------------|-------------|-----------|----------|------------|
| **子Agent机制** | ✅ 完整进程spawn | ✅ delegate_task(安全模型) | ✅ AgentTool+后端抽象 | ✅ AgentTool(上下文隔离) | ✅ spawn_agent(原生工具) | ✅ agent_spawn | ✅ SubagentManager |
| **多Agent编排** | ✅ Goal Hive + BBS | ✅ Kanban看板(依赖门控) | ✅ Coordinator模式 | ✅ Teams(Lead+Teammate) | ✅ 角色系统(explorer/worker) | ✅ 三层架构 | ⚠️ 轻量delegate |
| **通信协议** | ✅ 文件系统/HTTP/BBS | ✅ delegate_task + Kanban | ✅ 邮箱+消息队列 | ✅ 文件收件箱(.jsonl) | ✅ Mailbox(mpsc通道) | ✅ 事件总线+任务队列 | ⚠️ CLI调用 |
| **任务分发** | ✅ Plan委托标签/Map并行 | ✅ Kanban_create+依赖晋升 | ✅ Coordinator委派指南 | ✅ Task系统(依赖图) | ✅ followup_task | ✅ task_post/claim/complete | ⚠️ delegate_task |
| **Agent发现** | ✅ BBS注册+文件发现 | ✅ delegate_task内置 | ✅ Agent定义注册 | ✅ 自定义agent定义 | ✅ AgentRegistry | ✅ agent_find+a2a_discover | ❌ 无发现协议 |
| **健康监控** | ✅ Supervisor轮询 | ✅ 心跳线程+超时诊断 | ✅ 生命周期测试 | ✅ TaskStop/TaskOutput | ✅ wait_agent+超时 | ✅ 健康端点 | ❌ 无监控 |
| **嵌套深度控制** | ❌ 无限制 | ✅ MAX_DEPTH=1-3 | ⚠️ 有限 | ❌ 无显式限制 | ✅ max_depth=1 | ✅ 最大5层 | ❌ 无限制 |
| **对抗性验证** | ✅ verify_sop(证伪目标) | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ |

---

## 4. 记忆系统维度

| 子项 | GenericAgent | Hermes | OpenHarness | Claude Code | Codex CLI | openfang | agent-diva |
|------|-------------|--------|-------------|-------------|-----------|----------|------------|
| **短期记忆** | ✅ 3层(对话/工作/LLM历史) | ✅ 冻结快照+实时双状态 | ✅ 会话历史 | ✅ 会话记忆 | ✅ ContextManager | ✅ 会话修复 | ✅ JSONL session |
| **长期记忆** | ✅ 4层(L1索引/L2事实/L3 SOP/L4归档) | ✅ MEMORY.md + 外部Provider | ✅ 记忆系统 | ✅ 4类(user/feedback/project/reference) | ✅ SQLite + 记忆摘要 | ✅ 5存储统一API | ⚠️ MEMORY.md + HISTORY.md |
| **分层架构** | ✅ L0-L3严格分层+公理 | ✅ 双存储(快照+实时) | ✅ 多因子相关性 | ✅ LLM驱动选择 | ✅ 上下文片段 | ✅ 结构化/语义/知识图谱/会话/用量 | ❌ 扁平结构 |
| **检索方式** | ✅ 场景关键词→记忆定位 | ✅ FTS5+威胁扫描 | ✅ 相关性评分(BM25+元数据) | ✅ LLM选择(非embedding) | ✅ 会话搜索 | ✅ 向量嵌入+LIKE+图遍历 | ❌ 全量注入 |
| **压缩策略** | ✅ ROI公式(保留/压缩/删除) | ✅ 5阶段压缩+反抖动 | ✅ 自动压缩 | ✅ 4层管线 | ✅ 远程压缩端点 | ✅ LLM摘要压缩 | ❌ 无压缩 |
| **跨会话保留** | ✅ _history.json + L4归档 | ✅ SessionDB(FTS5搜索) | ✅ 会话恢复 | ✅ --resume/--fork | ✅ resume/fork | ✅ 标签+规范会话 | ⚠️ JSONL文件 |
| **知识图谱** | ❌ | ❌ | ❌ | ❌ | ❌ | ✅ 实体/关系/图遍历 | ❌ |
| **记忆安全** | ✅ XOR加密密钥 | ✅ 威胁模式扫描+漂移检测 | ✅ 去重+软删除 | ✅ 提取受限权限 | ✅ 权限控制 | ✅ 污染追踪 | ❌ 无安全检查 |

---

## 5. 技能系统维度

| 子项 | GenericAgent | Hermes | OpenHarness | Claude Code | Codex CLI | openfang | agent-diva |
|------|-------------|--------|-------------|-------------|-----------|----------|------------|
| **格式规范** | SOP Markdown | SKILL.md(YAML frontmatter) | SKILL.md | Markdown Skills | .codex/skills/ Markdown | HAND.toml | Markdown Skills |
| **加载机制** | ✅ file_read按需加载 | ✅ 发现+平台匹配+渐进披露 | ✅ 4层(bundled/user/project/plugins) | ✅ 按需加载(两层注入) | ✅ 上下文片段注入 | ✅ 能力门控 | ✅ SkillLoader |
| **热更新** | ❌ 需重启 | ✅ mtime缓存 | ✅ 热加载 | ✅ 动态 | ✅ 运行时 | ⚠️ 有限 | ❌ 不支持 |
| **自动创建** | ✅ 解题结晶为SOP | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ |
| **安全扫描** | ❌ | ✅ 路径规范化+SSRF防护+隔离区扫描 | ✅ Shell转义 | ✅ 权限检查 | ✅ 审批流 | ✅ 安全头 | ❌ 无扫描 |
| **远程安装** | ✅ 远程技能搜索(105K+) | ✅ Skills Hub(10+源) | ❌ | ✅ Marketplace | ❌ | ❌ | ❌ |
| **技能束** | ❌ | ✅ Skill Bundles | ❌ | ❌ | ❌ | ✅ Hands(能力包) | ❌ |
| **模板变量** | ❌ | ✅ ${HERMES_SKILL_DIR}等 | ❌ | ❌ | ❌ | ✅ 需求检查 | ❌ |

---

## 6. 通道/多平台维度

| 子项 | GenericAgent | Hermes | OpenHarness | Claude Code | Codex CLI | openfang | agent-diva |
|------|-------------|--------|-------------|-------------|-----------|----------|------------|
| **平台支持数** | 7+(CLI/Telegram/WeChat/QQ/DingTalk/Conductor/文件) | 22+ | 10 | 4(CLI/Web/VSCode/JetBrains) | 3(CLI/App Server/TUI) | 40 | 8+(Telegram/Discord/Slack/WhatsApp/Feishu/DingTalk/Email/QQ) |
| **消息路由** | ✅ queue.Queue汇聚 | ✅ DeliveryRouter(多目标投递) | ✅ 消息路由 | ✅ 内置 | ✅ Op/Event | ✅ 限流+DM/群组策略 | ✅ MessageBus双队列 |
| **会话管理** | ✅ 按任务目录 | ✅ SessionStore(SQLite)+自动过期 | ✅ 会话管理 | ✅ SessionManager | ✅ SQLite | ✅ 标签+规范会话 | ✅ SessionManager |
| **适配器模式** | 每平台独立frontend | ✅ PlatformRegistry(插件优先) | ✅ 适配器模式 | ✅ 内置 | ✅ 内置 | ✅ 适配器模式 | ✅ ChannelHandler trait |
| **Cron系统** | ✅ scheduler.py | ✅ 两种模式(no_agent/LLM) | ✅ Cron工具 | ✅ CronCreate | ❌ | ✅ 调度 | ✅ cron命令 |
| **事件钩子** | ✅ plugins/hooks.py | ✅ HOOK.yaml+handler.py | ✅ 10个生命周期事件 | ✅ 27种hook事件 | ✅ 会话钩子 | ✅ 4个生命周期钩子 | ❌ 无钩子系统 |

---

## 7. 评估/QA 维度

| 子项 | GenericAgent | Hermes | OpenHarness | Claude Code | Codex CLI | openfang | agent-diva |
|------|-------------|--------|-------------|-------------|-----------|----------|------------|
| **自评估** | ✅ 子Agent审查评分 | ❌ | ✅ harness-eval skill | ❌ | ❌ | ❌ | ❌ |
| **测试框架** | ❌ | ⚠️ 基础 | ✅ 三层(单元/集成/E2E) | ✅ 完整测试套件 | ✅ Rust测试 | ✅ 2696+测试 | ⚠️ 基础单元测试 |
| **质量门禁** | ✅ Plan Mode验证门 | ❌ | ✅ 五层断言体系 | ✅ CI/CD | ✅ CI/CD | ✅ 零clippy warning | ❌ 无门禁 |
| **多轮对话测试** | ❌ | ❌ | ✅ 核心要求(2+轮) | ✅ | ✅ | ✅ | ❌ |
| **错误恢复测试** | ❌ | ❌ | ✅ 核心维度 | ✅ | ✅ | ✅ | ❌ |
| **E2E评估** | ❌ | ❌ | ✅ 真实API调用 | ✅ | ✅ | ✅ | ❌ |
| **成本追踪** | ❌ | ✅ Token统计+成本估算 | ✅ Token累积 | ✅ 预算控制 | ✅ | ✅ UsageStore | ❌ |

---

## 8. 学习/进化维度

| 子项 | GenericAgent | Hermes | OpenHarness | Claude Code | Codex CLI | openfang | agent-diva |
|------|-------------|--------|-------------|-------------|-----------|----------|------------|
| **RL训练** | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ |
| **Trajectory记录** | ✅ LLM日志+L4压缩+显著性挖掘 | ✅ 后台审查 | ✅ 事件流收集 | ✅ 会话转录 | ✅ Rollout文件 | ✅ 审计日志 | ⚠️ JSONL session |
| **自我改进** | ✅ 自治自改进+Goal Mode+Morphling | ✅ 后台审查(记忆+技能) | ❌ | ✅ Dream整合 | ❌ | ❌ | ❌ |
| **记忆蒸馏** | ✅ start_long_term_update | ✅ on_pre_compress | ❌ | ✅ 记忆提取(forked agent) | ✅ 记忆摘要API | ✅ ConsolidationEngine | ❌ |
| **技能固化** | ✅ 解题→SOP | ✅ 技能审查 | ❌ | ❌ | ❌ | ✅ Hands配置 | ❌ |
| **情绪/行为分析** | ✅ 显著性挖掘(情绪事件) | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ |

---

## 关键差异总结

### agent-diva 的相对优势

1. **消息总线架构** — 双队列异步解耦，优于 GenericAgent 的单线程 Queue
2. **Channel 抽象** — 统一 ChannelHandler trait，即插即用
3. **Provider 抽象** — 统一 Provider trait + LiteLLM 兼容
4. **类型安全** — Rust 类型系统 + thiserror 错误处理
5. **模块化设计** — 多 crate workspace，职责清晰

### agent-diva 的显著短板

1. **无上下文压缩** — 所有对比项目都有某种形式的压缩
2. **无错误分类** — 简单 try/catch，无结构化恢复
3. **无工具并发** — 纯串行执行
4. **无评估体系** — 缺少测试框架和质量门禁
5. **扁平记忆** — 无分层、无检索、无压缩
6. **无安全沙箱** — 无工具执行隔离
7. **无钩子系统** — 无法扩展 agent 行为
8. **无子Agent安全模型** — 无工具黑名单、无深度控制
