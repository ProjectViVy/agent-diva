---
title: "Hermes Harness 能力研究 — diva Harness v2 借鉴源"
date: 2026-06-20
status: research baseline (大湿对话整理, 2026-06-19-20 实测)
owner: 大湿
applies_to: agent-diva-pro
supersedes: null
related:
  - docs/research/harness-engineering-three-way-comparison.md          # 13 维度三方对比 (220 行)
  - docs/research/harness-engineering-three-way-detailed-checklist.md   # 21 维度详细 checklist (324 行)
  - DECISION-v2-next-phase.md                                          # Harness Engineering 决策 (411 行)
source_of_truth:
  - docs/dev/past/legacy-docs/dev/awesomeagents/hermes-agent-analysis.md               # 1018 行 hermes 运行时深度分析
  - docs/dev/past/legacy-docs/dev/hermes-learning/01-hermes-capabilities.md           # 784 行 hermes 学习能力
  - docs/research/hermes-cronjob-tool.md                                              # 135 行 cronjob tool 调研
  - docs/dev/archive(old-docs-dont-read-me)/multimodal/hermes-agent-vision-analysis.md  # 522 行 vision 视角
research_method: hermes 走深读 (核心 8 文件 + 关键防御层 4 文件)
research_date: 2026-06-19 至 2026-06-20
research_perspective: 把 hermes 当主角 (不跟 diva 对位), 看 hermes 自身有哪些 harness 能力, 作为 diva Harness v2 借鉴源
revision_notes:
  - v0.1 (2026-06-20) — 初版: 17 类 harness 能力盘点 + 与 diva 对位 + DECISION-v2 §5 对应清单
superseded_by: null
---

# Hermes Harness 能力研究 — diva Harness v2 借鉴源

> **TL;DR**: Hermes 是个 **生产级 AI agent 框架**, 24 大类能力中 diva 完成度 **41%** vs hermes **91%**. diva harness 的核心缺口在"防失控 / 稳 prompt cache / 保 context 不爆 / 保审批一致 / 保会话隔离"5 个维度的生产级防护层. DECISION-v2 已经拍了 8 项 P0 (含 #5.6 prompt injection + #X-4 hot reload), 剩下是 P1 任务.

---

## 0. 一句话哲学

> **Hermes 把 agent 当生产系统对待** — 防失控 / 稳 prompt cache / 保 context 不爆 / 保审批一致 / 保会话隔离. 核心目标不是"让 LLM 更聪明", 而是"让 LLM 在不可预测的运行时环境下稳定运行".

---

## 1. 核心 Runtime（3 类）

### 1.1 Agent Loop 主循环（`conversation_loop.py:351-378`）

**5 阶段**:

```
1. 初始化     → 重试计数器(10+ 种) + IterationBudget(90) + todo/nudge 恢复 + system prompt 缓存恢复
2. 预检压缩   → 粗估 token, 超阈值则最多 3 轮压缩, 重置重试计数
3. 插件钩子   → pre_llm_call 注入上下文 + memory prefetch
4. 主循环     → 中断检查 → 预算消耗 → /steer 排空 → 消息准备 → API + 重试 → 错误分类 → 工具调用
5. 收尾       → post_llm_call → 提取 reasoning → token 统计 → memory sync → 后台审查(异步)
```

> **后台审查** (`_spawn_background_review`) 是核心精巧设计 — 内存审查 + 技能审查在响应交付后**异步**运行, 不阻塞主对话.

### 1.2 System Prompt 3 层缓存（`system_prompt.py:61-345`）

| 层 | 内容 | 关键设计 |
|----|------|---------|
| **Stable** | SOUL.md / 工具引导 / 任务完成引导 / 模型族引导 / 环境/平台提示 | 几乎不变 |
| **Context** | system_message / AGENTS.md | 会话级 |
| **Volatile** | MEMORY.md 快照 / USER.md / 外置 memory / **仅日期不含分钟**的时间戳行 | 每轮可能变 |

> **精巧点**: 时间戳行只含日期 → 字节稳定 → Anthropic 前缀缓存命中率最大化. snapshot 在加载时**冻结**, 会话期间永不变.

### 1.3 IterationBudget 防 LLM 死循环（`agent/iteration_budget.py:17-62`）

| 角色 | 默认预算 |
|------|---------|
| 父 agent | `max_iterations = 90` |
| 子 agent | `delegation.max_iterations = 50` |
| `execute_code` 调用 | **`refund()` 退还**（廉价 RPC） |
| 父子计数 | **独立**（总迭代可超父预算） |

> 跟 diva `ActionTracker` (per hour 滑动窗口) 比, **per-turn 硬上限**才是真防死循环. **diva 缺**.

---

## 2. 上下文治理（3 类）

### 2.1 上下文压缩 5 阶段（`agent/context_compressor.py`）

```
1. 工具输出修剪      → 去重 → 信息性摘要替换 → 截断大型 tool_call 参数
2. 边界确定          → 头部保护 (system prompt + 前 N 条) + 尾部保护 (反向累加 token 预算)
3. LLM 摘要生成      → 结构化模板 (Active Task / Goal / Completed / Active State)
4. 确定性回退        → LLM 失败时本地提取关键信息
5. 组装与清理        → 清孤立 tool_call/tool_result 对 + 去历史图片 + 反抖动保护
```

> **反抖动**: 连续 2 次压缩节省 <10% 停止（防反复触发）.

### 2.2 工具治理（`tools/registry.py`）

#### 2.2.1 ToolEntry 元数据（11 字段）

`name` / `toolset` / `schema` / `handler` / **`check_fn`** (30s TTL 缓存) / `requires_env` / `is_async` / `max_result_size_chars` / **`dynamic_schema_overrides`** (零参 callable 实时合并)

#### 2.2.2 注册机制

- **自注册**（模块级 `registry.register(...)`）
- **AST 自动发现**（仅导入含 register 的模块, 排除 `__init__.py`/`registry.py`/`mcp_tool.py`）

#### 2.2.3 Toolset 分组（11+ + 组合）

```
叶子:    web / terminal / file / vision / ...
组合:    debugging = [terminal, process] + includes [web, file]
平台:    hermes-discord = 核心 + discord 工具
安全子集: _HERMES_WEBHOOK_SAFE_TOOLS (仅 web_search/web_extract/vision_analyze/clarify)
```

特殊别名 `"all"` / `"*"` 递归解析所有 toolset, `resolve_toolset()` 带环检测.

#### 2.2.4 Tool Search 渐进披露（防 context 爆炸）

MCP/plugin 工具 schema token 超 **10%** 上下文阈值 → 自动替换为 3 个桥接工具:
- `tool_search` — BM25 关键词搜索延迟工具目录
- `tool_describe` — 加载单个工具完整 schema
- `tool_call` — 按名称调用

> 核心工具永不被延迟. diva 完全无此机制.

#### 2.2.5 工具错误净化（防注入上下文）

**`_sanitize_tool_error`** (`model_tools.py:576-599`):
- 剥离异常中的 XML 角色标签 / CDATA / markdown 代码围栏
- 截断至 2000 字符
- **防注入到模型上下文的错误消息触发角色混淆**

### 2.3 多模态 / Vision 双模式路由（`agent/image_routing.py`）

**`decide_image_input_mode()`**:
- `auto` / `native` / `text` 三种模式
- 决策依据: `config.yaml` + 模型 `supports_vision` 能力 + `auxiliary.vision` 显式配置

**Native 模式**: 图片 base64 data URL → OpenAI content 数组 → provider 适配器翻译为各自原生格式（Anthropic image block / Codex input_image / Bedrock Converse）

**响应式大小处理**（不预先按 provider 限制调整）:
```
先发原图 → provider 拒绝 (如 Anthropic 5MB → 400) → Pillow 缩小重试
最多 4 轮, JPEG 质量 85 → 70 → 50
硬上限 20MB / 嵌入目标 4MB / 重试目标 5MB / 下载上限 50MB
```

---

## 3. 防失控（6 类）

### 3.1 8 级错误分类 + 10 步恢复链（`agent/error_classifier.py:438-720`）

| 优先级 | 检查 | 例 |
|--------|------|-----|
| 1 | Provider 特定模式 | 内容策略 / thinking 签名 / OAuth 1M beta |
| 2 | HTTP 状态码 + 消息细化 | **402 可能是伪装限速**（"Usage limit, try again in 5 minutes"） |
| 3 | 错误码分类 | `error_code` 字段匹配 |
| 4 | 消息模式匹配 | 无状态码时 |
| 5 | SSL/TLS 瞬态 | → timeout |
| 6 | 服务器断开 + 大会话 | → context_overflow |
| 7 | 传输错误启发式 | 网络层 |
| 8 | 未知 | → 可重试 |

**22 种错误类型**（`FailoverReason` 枚举）

**10 步恢复链**:
```
Nous 费用刷新 → 凭证池轮换 → 图片缩小 → 多模态工具内容降级 →
OAuth 1M beta 禁用 → 各 provider 认证刷新 → thinking 签名恢复 →
加密内容重放禁用 → 上下文压缩 → fallback
```

### 3.2 Plugin 钩子系统

```
pre_llm_call / post_llm_call / transform_llm_output
pre_tool_call / post_tool_call / transform_tool_result
```

每个都有 try/finally + 异常隔离. **16+ builtin hooks** 通过 `invoke_hook` 列表管理.

### 3.3 MCP 集成安全（`tools/mcp_tool.py:1096-1860`）

**5 层安全模型**:
1. 环境变量过滤（`_build_safe_env()` 仅传递安全基线, 防宿主 API key 泄露）
2. 凭证脱敏（`_sanitize_error()` GitHub PAT / OpenAI key → `[REDACTED]`）
3. Prompt 注入扫描（扫描 MCP 工具描述中的注入模式）
4. **OSV 恶意软件检查**（stdio 模式启动子进程前查 OSV 数据库）
5. URL 校验（拒绝非 http(s) 协议）

**Resilience**:
- 动态工具刷新（`notifications/tools/list_changed` 差异更新）
- **断路器**（连续 3 次失败 → 60s 冷却）
- 自动重连（指数退避最多 5 次）+ keepalive（每 180s `list_tools`）
- `_rpc_lock` 串行化客户端 RPC

### 3.4 Cron / 后台任务（`cron/scheduler.py`）

**统一 `cronjob` tool**（替代 alife EWait/EWake）:
- **3 态 schedule 字符串**: `"30m"` 延迟 / `"2026-02-03T14:00"` 绝对 / `"every 30m"` 或 `"0 9 * * *"` 周期
- **`deliver` 字段**多 channel 推送: `local` / `telegram` / `discord` / `feishu` / `slack` / `dingtalk` / `webchat`

**调度器**（`cron/scheduler.py:1857-2035`）:
- 每 60s tick, 文件锁（**同 tick 不并发**）
- 双执行模式: `no_agent`（脚本即作业）/ LLM（构建 AIAgent 注入 prompt）
- **at-most-once 语义**: 执行前先 `advance_next_run()`
- 错过作业: 超 grace 窗口的循环作业快速前进, **不补执行**

### 3.5 记忆与上下文持久化（`tools/memory_tool.py`）

#### 3.5.1 内置 — 冻结快照模式

- **`_system_prompt_snapshot`** 加载时冻结, 会话期间**永不变**（保前缀缓存稳定）
- `memory_entries` / `user_entries` 实时状态, 工具调用立即修改 + 持久化
- **字符限制**: MEMORY.md **2200 字符** / USER.md **1375 字符**（字符计数非 token, 模型无关）
- 跨平台文件锁（Unix `fcntl.flock` / Windows `msvcrt.locking`）
- 原子写入（先临时文件 + `os.replace`, 防截断竞态）
- 写入前威胁扫描（`threat_patterns` strict 范围）
- 外部漂移检测（`MEMORY.md` 被外部改动时创建 `.bak.<ts>` 备份 + 拒绝写入）

#### 3.5.2 外置 Provider（最多 1 个）

Honcho / OpenViking / Mem0 / Hindsight / Holographic / RetainDB / ByteRover

**5 生命周期钩子**:
- `on_turn_start` / `on_session_end` / `on_session_switch`（`/resume` / `/branch` / `/reset`）
- `on_pre_compress` / `on_memory_write`（镜像内置到外置）/ `on_delegation`

### 3.6 会话分支 — 3 类 child session

**`hermes_state.py` SQLite FTS5 WAL**:

| 类型 | SQL 标记 | 用途 |
|------|---------|------|
| `branched_from` | `_BRANCH_CHILD_SQL` | 探索分支, 可列出但**不级联删** |
| `compression` | `_COMPRESSION_CHILD_SQL` | 压缩衍生, `end_reason='compression'` |
| `ephemeral` | `_ephemeral_child_sql` | **级联删除** subagent run |

**多用户隔离**: sender_id + chat_id 双 key（**12 字符 SHA256 哈希**）

---

## 4. 防护层（3 类）

### 4.1 Prompt 注入防御（`tools/threat_patterns.py`）

- **50+ 模式**
- **3 档 scope**（`all` / `context` / `strict`）— strict 用于 cron prompt / skill install
- **17 个 INVISIBLE_CHARS 检测**（零宽 / 双向覆盖）
- **Multi-word bypass 正则** `(?:\\w+\\s+)*` 抗填充词
- Tool result 隔离
- Cron prompt **双重扫描**（装配前 strict + 装配后兜底）
- Skill install 拦截（strict scope）

### 4.2 PII / 隐私（`agent.redact`）

- `redact_sensitive_text` 全链路 redact
- Gateway/session 12 字符 SHA256 哈希 sender_id / chat_id
- 凭据文件保护（`.netrc` / `.pgpass` / `.npmrc` / `.pypirc` 拦截写入）

### 4.3 Token 预算（`tools/budget_config.py`）

- **per-tool PINNED_THRESHOLDS**（`{read_file: inf}` 防无限循环）
- per-turn 200K
- preview 1500

---

## 5. 治理层（2 类）

### 5.1 批准系统（`tools/approval.py` 1751 行）

- `Denied` / `ApprovedOnce` / `ApprovedForSession` / **`ApprovedPermanent`**（4 档）
- config.yaml 持久化 permanent allowlist + 失效信号
- `contextvars.ContextVar` 绑 turn_id + per-session thread-safe
- **YOLO freeze**: `_YOLO_MODE_FROZEN = is_truthy_value(os.getenv("HERMES_YOLO_MODE"))` **import 时冻结**防注入提权

### 5.2 配置管理（`hermes_cli/config.py`）

| 能力 | 实现 |
|------|------|
| **Hot reload** | `(config_path, mtime_ns, size)` 三元组缓存 + 失效机制 |
| **多 profile** | `HERMES_PROFILE` env + `profiles/<name>/` 目录 |
| **环境变量覆盖** | `HERMES_HOME` / `HERMES_PROFILE` / `HERMES_CONFIG` / `HERMES_ENV` |
| **Corrupt backup** | 解析失败时自动备份 corrupt config + warn once |
| **Schema 文档** | 字段级 docstring + CLI dump |
| **Migration** | `hermes_constants.py` + `_delegate_from_json` 兼容字段 |

---

## 6. 可观测性（1 类）

### 6.1 审计与日志

| 能力 | 实现 |
|------|------|
| **Trajectory 保存** | `agent/trajectory.save_trajectory_to_file` |
| **关联 ID 全链路** | turn_id + tool_call_id + chat_id 全链路追踪 |
| **结构化日志** | loguru / 标准 logging + structured |
| **16+ builtin hooks** | `invoke_hook` 列表 |
| **插件 hook 列表 CLI** | `diva hooks list` 自省 |
| **运行时审查**（后台） | `_spawn_background_review`（内存审查 + 技能审查） |

---

## 7. 部署形态（1 类）

### 7.1 跨入口共享 session

```
hermes_cli (CLI)  ↔  acp_adapter  ↔  gateway (IM 平台)
        ↓                ↓               ↓
        └────────────────┴───────────────┘
                         ↓
              同一个 AIAgent + session DB
```

- CLI / TUI / GUI / ACP adapter / gateway 入口都共享同一 AIAgent 实例
- 多入口 Approval 一致（`tests/gateway/test_telegram_approval_buttons.py` 等 4 平台都有）

---

## 8. Hermes vs Diva Harness 能力对照（20 项）

| Hermes harness 能力 | diva 状态 | 缺口 |
|---------------------|----------|------|
| 1. Agent Loop 5 阶段 + 后台审查 | ⚠️ diva 有 agent loop 但无后台审查 | 后台异步审查 |
| 2. **3 层 system prompt 缓存** | ❌ diva 无（每次重建） | 缓存字节稳定 + 冻结 snapshot |
| 3. **IterationBudget**（per-turn 硬上限） | ❌ diva 只有 ActionTracker（per hour） | per-turn 上限 + execute_code refund |
| 4. 上下文压缩 5 阶段 + 反抖动 | ⚠️ diva 有 compaction 但 1 阶段 | 5 阶段 + 反抖动 + 确定性回退 |
| 5. **8 级错误分类 + 402 消歧** | ❌ diva 无此分层 | 错误恢复链 + 22 类型枚举 |
| 6. 工具治理（check_fn TTL + Toolset + Tool Search 渐进披露） | ❌ diva 单层 ToolRegistry | check_fn 30s TTL + toolset 组合 + 延迟披露 |
| 7. **Plugin 钩子系统**（pre/post LLM + tool） | ⚠️ diva MessageBus 是粗粒度 | 7 个细粒度钩子 + 异常隔离 |
| 8. MCP 5 层安全 + 断路器 | ⚠️ diva MCP 未实接 | OSV 检查 + 动态刷新 + 60s 断路 |
| 9. Cron `cronjob` 统一 tool + deliver | ⚠️ diva cron 已有但无统一 tool | 统一 tool + 多 channel 推送 |
| 10. 记忆冻结快照 + 字符限制 + 漂移检测 | ⚠️ diva `HybridMemoryProvider` 部分对位 | 冻结 snapshot + 字符限制 + drift 备份 |
| 11. **Vision 双模式路由 + 响应式大小** | ❌ diva 无 vision 治理 | native/text 路由 + 响应式 Pillow 缩小 |
| 12. **3 类 child session（branch/compression/ephemeral）** | ❌ diva 单一 session | 3 类 + 级联删策略 |
| 13. **配置 hot reload + 多 profile + corrupt backup** | ❌ diva 全无（DECISION-v2 §X-4 待做） | mtime 缓存 + 多 profile + corrupt 备份 |
| 14. **Prompt 注入 50+ 模式 + 3 档 scope + 17 不可见 Unicode** | ❌ diva 完全空白（DECISION-v2 §5.6 P0 待做） | 防御全栈 |
| 15. PII redact + 12-char SHA256 sender 哈希 | ⚠️ diva session/search 有 sanitize 但仅 query | 全链路 redact + sender 哈希 |
| 16. **Token 3 层 budget + PINNED 防循环** | ⚠️ diva 单层（context_budget.rs） | per-tool PINNED + per-turn + preview |
| 17. **Approval 1751 行 + permanent allowlist + YOLO freeze** | ⚠️ diva approval 只有枚举，行为缺 | 4 档 + 持久化 + YOLO 冻结 |
| 18. 审计（trajectory + turn_id 全链路关联） | ⚠️ diva Laputa changelog 部分对位 | trajectory 文件 + turn_id 关联 |
| 19. **跨入口共享 session**（CLI/TUI/GUI/ACP/gateway） | ⚠️ diva CLI/GUI/gateway 各自独立 | acp_adapter + 共享 AIAgent |
| 20. **Tool 错误净化**（防注入上下文） | ❌ diva 无 | XML/CDATA/MD 围栏剥离 + 2000 字符截断 |

**完成度对照**: hermes **91%** / diva **41%** / alife **36%**

---

## 9. 跟 DECISION-v2 / Harness Engineering 对位

### 9.1 DECISION-v2 §5 P0 已拍板覆盖

| DECISION-v2 §5 P0 项 | Hermes 对应能力 | 本研究来源 |
|----------------------|----------------|-----------|
| #5.1 Module 生命周期 / Autofac DI 等价 | heremes 没 Module 概念, 用 tools 替代 | §2.2 工具治理 |
| #5.2 纯 Tool 硬约束 | check_fn TTL + Tool Search + ACP 审批 | §2.2.1, §2.2.4 |
| #5.3 Schema validation cross-layer 测试 | dynamic_schema_overrides + schema_sanitizer | §2.2.1 |
| #5.4 PII / 涉密 filter | redact_sensitive_text + 12-char SHA256 | §4.2 |
| #5.5 行为审计 + GUI 独立审计 | trajectory + turn_id 关联 + 结构化日志 | §6.1 |
| #5.6 Prompt Injection 防御三层 | 50+ 模式 + 3 档 scope + 17 不可见 Unicode | §4.1 |
| #5.7 3 态用户存在 | (不在 hermes 范畴) | — |
| #5.8 Poke 8 事件链全拆 | hook 系统 + bus 事件 | §3.2 |

### 9.2 DECISION-v2 §5 P1 待拍板

| P1 项 | Hermes 对应能力 |
|-------|----------------|
| 纯 Tool 硬约束扩展 | check_fn TTL + Toolset 组合 + Tool Search |
| 威胁模式库 | threat_patterns.py 50+ 模式 + 3 档 scope |
| Token 三层 budget | budget_config.py 3 层 + PINNED_THRESHOLDS |
| Config hot reload | `(config_path, mtime_ns, size)` 三元组缓存 |
| Toolset 分组 + check_fn | 11+ toolset + check_fn_cached 30s TTL |
| LLM token-level streaming | provider 有 stream + agent loop 消费 |
| Per-turn parallel tool calls | `_MAX_TOOL_WORKERS = 8` + `_should_parallelize_tool_batch` |
| Skill bundle + 多级 scope | skill_bundles.py + 4 级 scope (system/user/optional-skills/per-profile) |
| 多 profile 隔离 | HERMES_PROFILE + profiles/<name>/ |
| Branch/compression/ephemeral child session | _BRANCH_CHILD_SQL + _COMPRESSION_CHILD_SQL + _ephemeral_child_sql |

### 9.3 本研究未覆盖（产品级能力, 不在 harness 范围）

剔除: RL 训练闭环 (GRPO) / Skills Hub 10 源 / ImageGenProvider ABC / ACP adapter 外部 IDE 接入 / 14+ 多通道 IM / A2A delegate_task / Kanban 看板. 这些是产品形态, 跟 harness runtime 防护无关.

---

## 10. 一句话哲学（再强调）

> **Hermes 的 harness 核心价值不是"让 LLM 更聪明", 而是"让 LLM 在不可预测的运行时环境下稳定运行"**. 17 类能力都围绕 5 个目标: **防失控 / 稳 prompt cache / 保 context 不爆 / 保审批一致 / 保会话隔离**. diva 当前 runtime 已有, 但生产级防护层几乎空白. DECISION-v2 已拍 8 P0, 25 任务清单待开干.

---

## 附录 A: 关键文件索引

### Hermes 核心源码（已深读, per `awesomeagents/hermes-agent-analysis.md`）

| 文件 | 关键内容 | 行号 |
|------|---------|------|
| `run_agent.py` | AIAgent 门面 | 294 |
| `conversation_loop.py` | 主循环 5 阶段 | 351-378 |
| `system_prompt.py` | 3 层 system prompt 缓存 | 61-345 |
| `iteration_budget.py` | 父 90 + 子 50 + refund | 17-62 |
| `context_compressor.py` | 5 阶段压缩 + 反抖动 | 522+ |
| `error_classifier.py` | 8 级错误分类 + 402 消歧 | 438-720 |
| `memory_tool.py` | 冻结快照 + 字符限制 + 漂移检测 | 113-600 |
| `memory_manager.py` | 外置 provider 编排 | 全文 |
| `memory_provider.py` | MemoryProvider ABC | 全文 |
| `image_routing.py` | Vision 双模式路由 | 287-317 |
| `image_gen_provider.py` | ImageGenProvider ABC | 51-143 |
| `vision_tools.py` | Vision 工具 + 响应式大小 | 716-997, 344-447 |
| `mcp_tool.py` | MCP 5 层安全 + 断路器 | 1096-1860 |
| `delegate_tool.py` | 子 agent 派生 | 870-1174 |
| `kanban_tools.py` | 看板协调 | 49-91, 132-161 |
| `cron/scheduler.py` | Cron 调度器 | 1857-2035 |
| `cron/cronjob_tools.py` | cronjob 统一 tool | 465 |
| `cron/jobs.py` | 三态 schedule 解析 | 185, 206 |
| `hermes_state.py` | SQLite FTS5 WAL session | 全文 |
| `hermes_cli/config.py` | YAML 配置 + hot reload + profile | 全文 |
| `tools/approval.py` | 4 档审批 + YOLO freeze | 1751 行全文 |
| `tools/threat_patterns.py` | 50+ 注入模式 + 3 档 scope | 全文 |
| `tools/budget_config.py` | 3 层 token budget + PINNED | 全文 |
| `tools/skill_utils.py` | Skill 发现 + 平台条件 | 128-169, 241-324 |
| `tools/registry.py` | 工具注册 + AST 发现 | 57-74, 234-305 |
| `tools/tool_search.py` | BM25 延迟工具搜索 | 347-418 |
| `agent/skill_commands.py` | Skill 触发命令 | 263-326 |
| `agent/anthropic_adapter.py` | Anthropic image block 翻译 | 1510-1523 |

### diva 对照文件

| 维度 | diva 文件 |
|------|-----------|
| Agent Loop | `agent-diva-agent/src/agent_loop/loop_turn.rs` |
| 上下文压缩 | `agent-diva-agent/src/compaction/{mod,exec,prompt,quality}.rs` |
| Tool 注册 | `agent-diva-tools/src/` |
| Memory | `agent-diva-core/src/memory/{manager,storage,provider,hybrid}.rs` |
| Sandbox / Approval | `agent-diva-sandbox/src/{exec_policy,rules,guardian,orchestrator}.rs` (5318 LOC) |
| Session | `agent-diva-core/src/session/{manager,store,search}.rs` |
| Heartbeat | `agent-diva-core/src/heartbeat/service.rs` (766 LOC) |
| Cron | `agent-diva-core/src/cron/` (1275 LOC) |
| Bus | `agent-diva-core/src/bus/queue.rs` |
| Context budget | `agent-diva-agent/src/context_budget.rs` |
| GUI | `agent-diva-gui/src/` |
| Laputa | `agent-diva-laputa/src/memory_provider.rs` |

---

## 附录 B: 复盘触发器

- [ ] DECISION-v2 §5 拍板的 8 项 P0 落地后, 把本研究的 20 项能力对照表更新 ✅/⚠️/❌ 状态
- [ ] DECISION-v2 §5.1 Module 生命周期实接后, 把 Hermes §2.2 工具治理的 check_fn / Toolset / Tool Search 借鉴到 diva
- [ ] DECISION-v2 §5.5 行为审计实接后, 借鉴 Hermes §6.1 trajectory + turn_id 关联模式
- [ ] DECISION-v2 §X-4 hot reload 实接后, 借鉴 Hermes §5.2 `(config_path, mtime_ns, size)` 三元组缓存 + corrupt backup
- [ ] DECISION-v2 §5.6 prompt injection 实接后, 借鉴 Hermes §4.1 50+ 模式 + 3 档 scope + 17 不可见 Unicode
- [ ] harness-engineering-three-way-detailed-checklist.md 25 任务清单开干时, 引用本研究 §8 对照表定位每项借鉴方向
- [ ] Hermes 上游新版本发布时, 重新跑一遍本研究方法 (核心 8 文件 + 关键防御层 4 文件), 更新本研究 + DECISION-v2

---

> **报告结束**. 如需进一步细化某个能力 (例如把 Hermes §2.2.4 Tool Search 渐进披露的具体 BM25 算法写到 diva 设计草稿), 或生成 Rust 实现 POC, 请指示.
