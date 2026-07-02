# Harness Engineering 三方对比

调研方法: hermes 走深读(核心 8 文件 + 关键防御层 4 文件),alife 走补读(ModuleSystem / ChatBot / ChatActivity / DeveloperService),diva 走快查(security + cron + heartbeat + sandbox + tooling 目录)。本报告给 diva-pro Harness v2 设计负责人用。

---

## 1. Tool 系统
| 维度 | diva | alife | hermes |
|---|---|---|---|
| 注册方式 | `ToolRegistry` + `register(Arc<dyn Tool>)` | `[Module]`+`[XmlFunction]` attribute 反射 | `registry.register()` 导入时自注册,AST 扫描 |
| Schema | `Tool::parameters()` 返回 JSON Schema | XML 自闭合标签,XmlHandlerTable | OpenAI Function Calling 格式 |
| Validation | `validate_params(&Value) -> Vec<String>` | XmlStruct 类型转换 | schema_sanitizer + dynamic_schema_overrides |
| Error | 统一 `ToolError`,warn + ERROR_HINT | `Poke` 消息回灌 LLM | 三类 file_mutation / 不可见 / threat |
| 描述结构 | name/description/parameters/execute/validate | name/description/parameters(注解式) | name/toolset/schema/handler/check_fn/max_result_size_chars |

> 强项: hermes 的 `check_fn + TTL 缓存` + `dynamic_schema_overrides` 是工业级; alife 的"函数即方法"反射最简洁。

## 2. Module / Lifecycle
| 维度 | diva | alife | hermes |
|---|---|---|---|
| 统一基类 | `InteractiveModule` (来自 alife 借鉴) | `InteractiveModule<T>` 三段清晰 | ❌ 无基类,tools/*.py 各管各的 |
| Awake | `AwakeAsync(AwakeContext)` | `AwakeAsync(AwakeContext)` | 模块导入即完成 |
| Start | `StartAsync(Kernel, ChatActivity)` | `StartAsync(...)` 注入 ChatBot | N/A |
| Destroy | `DestroyAsync()` | `DestroyAsync()` 取消 ITimeIterative | N/A |
| 插件热重载 | `ModuleSystem.ReloadModules` | ✅ 核心卖点,Roslyn 热编译 | ❌ 无插件机制 |

> 强项: alife 三段式最规范;diva 已借鉴但还差热重载;hermes 完全无 Module 概念(用 tools 替代)。

## 3. DI / 模块装配
| 维度 | diva | alife | hermes |
|---|---|---|---|
| 容器 | 手动 registry(无容器) | **Autofac 容器**(主卖点之一) | ❌ 无容器,全局 import |
| 注入方式 | 显式构造参数 | 构造函数注入 + `.OnActivated` 钩子注入 Configuration | N/A(全局单例) |
| 服务可见性 | 通过 `ModuleSystem` 拉取 | `containerBuilder.Populate()` + 注册 moduleTypes | N/A |
| 作用域 | 进程内单例 | `SingleInstance` per ChatActivity | 进程单例 + contextvars |

> 强项: alife 的 Autofac + OnActivated 注入 config 是教科书式;diva v2 急需引入 IoC 容器。

## 4. Trigger 系统
| 维度 | diva | alife | hermes |
|---|---|---|---|
| Cron 调度 | `CronSchedule::At/Every/Cron`(3 模式) | ❌ 无原生 cron,ITimeIterative 内部循环 | 完整 croniter + 60s tick + 双池(parallel/sequential) |
| Heartbeat | 30 min 默认,`HeartbeatDecision{skip/run/tasks}` LLM 决策 | `ITimeIterative` 任意间隔 | kanban 专属 `kanban_heartbeat` |
| 退避 | ❌ 未见 | ❌ | `agent/retry_utils.jittered_backoff` |
| 频率限制 | `ActionTracker` 滑动窗口(per hour) | ❌ | `cron/jobs.py` job-level cooldown |
| 触发链 | cron → ChatActivity.Activate | DeveloperService 可手动 RestartActivity | gateway tick 触发 + `delegate_task` |

> 强项: hermes 的"双池 + 持久线程池 + 60s tick"是最工业的;diva 已有骨架但缺退避和频率。

## 5. 异步队列
| 维度 | diva | alife | hermes |
|---|---|---|---|
| 主聊天互斥 | Laputa 提案 / 迁移队列 | `ChatBot.RequestChatAsync` Semaphore(0/1) | gateway session 互斥 |
| Poke 队列 | ❌ 无 Poke 概念 | `ConcurrentQueue` + 11 条循环,30s 自动 flush | 多种 Aria/kanban queue |
| 后台任务 | Laputa proposal lifecycle | `Task.Run` 散落 | persistent ThreadPoolExecutor,顺序池 |
| 取消机制 | ❌ 不显式 | `CancellationToken` 注入(StartAsync) | `chatBreakSource` + Interrupt 工具 |
| 多 Agent 协调 | Laputa memory provider | 角色级独立 | kanban 任务板 + mirror 跨 gateway |

> 强项: alife 的 ChatBot Semaphore 互斥最干净;hermes 的"双池分离"最工程化。

## 6. Tool 硬约束
| 维度 | diva | alife | hermes |
|---|---|---|---|
| 审批决策 | `ReviewDecision{Denied,ApprovedOnce,ApprovedForSession}` | ❌ 无审批,直接执行 | tools/approval.py 1751 行(三层审批 + permanent allowlist) |
| 危险命令检测 | shell 层 + approval 缓存 | ❌ 零 | DANGEROUS_PATTERNS 30+ 模式 + SSH 路径 + system 路径 + macOS /private 镜像 + 凭据文件 |
| 沙箱 | `agent-diva-sandbox`(exec_policy/rules/guardian/orchestrator) | ❌ 无沙箱,直接 SemanticKernel invoke | tirith_security + 多种 backend (local/Docker/Modal/SSH) |
| YOLO 模式 | ❌ 无 | ❌ 无 | `HERMES_YOLO_MODE` 在 import 时冻结防注入提权 |
| 永久 allowlist | Session 级缓存 | ❌ 无 | config.yaml 持久化 + 失效逻辑 |
| 敏感路径 | path.rs 5 层(L1-L5) | ❌ | file_tools `_check_sensitive_path` + terminal 镜像保护 |

> 强项: hermes 完全碾压(1751 行 approval.py),diva 已有骨架但危险模式覆盖薄。

## 7. Schema validation
| 维度 | diva | alife | hermes |
|---|---|---|---|
| 入参校验 | `Tool::validate_params` 返回 Vec<String> | XmlStreamParser + 类型转换 | schema_sanitizer.py + JSON Schema 完整校验 |
| 工具定义动态化 | ❌ 全静态 | 编译时反射 | `dynamic_schema_overrides` 零参 callable 实时合并 |
| toolset 分组 | ❌ 简单数组 | ❌ 无 | 11+ 个 toolset(全栈/研究/浏览器/语音/看家/视频),支持 includes 组合 |
| check_fn 门控 | ❌ 无 | ❌ 无 | `check_fn_cached` 30s TTL 缓存外部依赖探测 |

> 强项: hermes 的 toolset 组合 + check_fn 缓存 + dynamic overrides 三件套工业级;diva 急需引入。

## 8. Token / 资源预算
| 维度 | diva | alife | hermes |
|---|---|---|---|
| 思考 token | `ThinkingTokenLimits` per provider | ❌ 无显式 | `iteration_budget.IterationBudget` |
| 单工具结果大小 | ❌ 无(看 tools/base.rs 未见) | ❌ 无 | budget_config.py: 3 层(per-tool PINNED / per-turn 200K / preview 1500) |
| Action budget | `SecurityError::ActionBudgetExhausted` | ❌ 无 | `PINNED_THRESHOLDS = {read_file: inf}` 防无限循环 |
| 计费 | 仅在 reasoning.rs | `TokenUsed` event hook | `agent/usage_pricing.normalize_usage` |
| 持久化 | ❌ | ❌ | tool_result_storage.py 滚动持久化 |

> 强项: hermes 的三层 budget + PINNED 防循环是唯一工业实现;diva 急需补。

## 9. Content filter
| 维度 | diva | alife | hermes |
|---|---|---|---|
| PII redact | ❌ 未见 | ❌ 未见 | `redact_sensitive_text` + gateway/session.py `_hash_sender_id/chat_id`(12 字符 SHA256) |
| 凭据文件保护 | path.rs 中有 hint | ❌ 无 | `_CREDENTIAL_FILES` (.netrc/.pgpass/.npmrc/.pypirc) 写入拦截 |
| 文本过滤 | ❌ | `ChatTextFilter(message)` hook 在 InteractiveModule.Chat | `agent.redact.redact_sensitive_text` 全链路 |
| 日志脱敏 | tracing 默认 | FileLogger | `tracing` 字段级 + 凭据 redact |

> 强项: hermes 是唯一有 PII 哈希 + 凭据 redact 的;diva 完全空白。

## 10. Prompt injection 防御
| 维度 | diva | alife | hermes |
|---|---|---|---|
| 威胁模式库 | ❌ 无 | ❌ 无 | threat_patterns.py: 50+ 模式,3 档 scope(all/context/strict) |
| 不可见 unicode | ❌ | ❌ | 17 个 INVISIBLE_CHARS 检测(零宽、双向覆盖) |
| 注入防御层级 | 无 | 无 | multi-word bypass`(?:\\w+\\s+)*` 抗填充词 |
| Tool result 隔离 | ❌ | ❌ 无 | `tool_dispatch_helpers._multimodal_text_summary` 文本摘要 |
| Cron prompt 扫描 | ❌ | ❌ | cronjob_tools.py 双重扫描(strict + 装配后兜底) |
| Skill install 拦截 | Laputa proposal 流程间接 | ReloadModules 流程 | `skills_guard.py` strict scope |
| 上下文文件扫描 | ❌ | ❌ | `prompt_builder` context scope |
| instruction hierarchy | ❌ | ❌ | turn_id 绑 contextvars,tool_call_id 关联审计 |

> 强项: hermes 是唯一有完整防御栈的;diva 完全没有 prompt injection 防御。

## 11. Audit / Logging
| 维度 | diva | alife | hermes |
|---|---|---|---|
| Trajectory | Laputa changelog | ❌ | `agent.trajectory.save_trajectory_to_file` |
| 关联 ID | ❌ 未见 | ❌ | turn_id + tool_call_id 全链路 |
| 结构化日志 | tracing | FileLogger + ILogger | loguru/标准 logging + structured |
| 事件总线 | `core/bus/` | ChatBot event(PokeSend/ChatSend/TokenUsed) | `gateway/session.py` event bus |
| Plugin hooks | Laputa | `event Func<...>` 多播 | `invoke_hook` 列表 + 16+ builtin hooks |
| 操作重放 | Laputa migration | ❌ | trajectory file |

> 强项: alife 的 ChatBot event 多播最干净;hermes 的 turn_id 关联最工程化。

## 12. Authority / Governance
| 维度 | diva | alife | hermes |
|---|---|---|---|
| 决策粒度 | 3 档(Denied/Once/Session) | ❌ 无显式 | per-call 审批 + permanent allowlist + per-platform config |
| 提案流程 | Laputa proposal | DeveloperService(自我升级) | pre_approval_request / post_approval_response hooks |
| 模块权限声明 | character config(Modules 数组) | character config(Modules) | config.yaml 永久 allowlist |
| 谁能改什么 | 通过 character.StorageKey 隔离 | Module 文件夹 + ReloadModules | config.yaml + 角色 owner |
| 自我升级 | ❌ | ✅ DeveloperService 主动写代码 | 需用户授权 |

> 强项: alife 的"AI 主动写代码自我升级"是独一份(但风险也大);hermes 的 hook + 永久 allowlist 治理最规范。

## 13. 多通道管理
| 维度 | diva | alife | hermes |
|---|---|---|---|
| 通道数 | 13+ (dingtalk/discord/email/feishu/irc/matrix/mattermost/qq/slack/telegram/whatsapp/nextcloud/neuro_link) | 1(OneBot/QChat 协议,Browser 桌面) | 17+ (telegram/discord/slack/whatsapp/signal/matrix/mattermost/dingtalk/feishu/wecom/sms/email/webhook/bluebubbles/qqbot/yuanbao) |
| 协议抽象 | Channel trait | OneBot 协议 | Platform adapter pattern |
| 跨 gateway 协同 | Laputa memory provider | 角色/Activity 独立 | `gateway/mirror.py` 镜像 |
| Session 隔离 | 进程级 | 角色级 | contextvars + session_env |
| CLI/GUI 出口 | agent-diva-cli + agent-diva-gui (Tauri) | Alife.Client (Blazor) + DeskPet | hermes-cli + hermes-desktop |
| 移动端 | ❌ | Alife.DeskPet 桌面宠物 | ❌ |

> 强项: 三家通道数都够用;hermes 的 mirror 跨 gateway 协同是独一份。

---

## 综合判断

**哪个最强**:
- **hermes 在**: Tool 硬约束(approval.py 1751 行)、Schema validation(toolset 组合 + check_fn TTL)、Token 预算(3 层 budget)、Content filter(PII 哈希 + 凭据 redact)、**Prompt injection 防御(50+ 模式 + 不可见 unicode + scope 分级)**
- **alife 在**: Module/Lifecycle(标准三段)、DI(Autofac + OnActivated)、异步队列(Semaphore 互斥最干净)、Authority(AI 主动自我升级,虽然风险大)
- **diva 在**: Tool Registry Rust trait 实现(类型安全)、多通道(13 个,见 alife 之外第二多)、Laputa 提案流程

**diva 缺的**(按紧迫度排序):
1. **Prompt injection 防御层** — 完全空白,`agent-diva-core/src/security/` 0 个相关文件
2. **三层 token 预算** — 只有 reasoning 层的 thinking budget,缺 per-tool/per-turn/preview
3. **PII redact** — tracing 默认无 redact,`agent-diva-core/src/session/search.rs` 有 sanitize 但仅 query
4. **永久 allowlist** — approval.rs 只有 session 级,缺 config.yaml 持久层
5. **check_fn 缓存** — `ToolRegistry` 静态,无外部依赖探测缓存
6. **toolset 组合** — `agent-diva-tools/src/` 没有组合机制,只有平铺
7. **危险命令模式** — `sandbox/approval.rs` 决策枚举有,但 patterns 库未见
8. **event 多播** — alife 的 `Func<...>` 多播是 Rust 也能做的漂亮模式
9. **Hot-reload 模块** — 借鉴 alife Roslyn 热编译
10. **跨 channel 镜像** — hermes 的 mirror 思路

**直接借鉴价值**:
- 从 hermes 借鉴:
  1. `threat_patterns.py` 的 3 档 scope 设计 → 直接对应 Rust 三 trait: `ThreatScanner::all/context/strict`
  2. `budget_config.py` 的三层 budget + PINNED_THRESHOLDS → Rust `BudgetPolicy` 不可变 dataclass
  3. `tools/approval.py` 的 contextvars 绑 turn_id/tool_call_id → 已有 contextvars crate 替代
  4. `Poke(11条循环队列 + 30s flush)` → diva 可加为 ChatActivity 的杂项消息推送
  5. `check_fn TTL 30s 缓存` → diva 的 Tool trait 加 `availability()` 默认方法
- 从 alife 借鉴:
  1. `ModuleSystem` 的 Roslyn 热编译 → 可选特性,diva 用 syn + rustc 不可行,但 `dlopen` + `cdylib` 思路可借鉴
  2. `OnActivated` 钩子注入 config → 在 diva 用 `Arc<dyn Module + Configurable>` 自动 wire
  3. `ITimeIterative` 显式接口 → diva 心跳已类似,可统一
  4. `ChatTextFilter` hook 在模块边界 → diva security 层加 `MessageFilter` trait
  5. `DeveloperService` 自我升级 → diva Laputa 可加更明确的人工确认闸

**警惕项**(不要照搬):
- **alife 自我升级**(DeveloperService): 风险极大,Laputa 提案流程比这稳得多;不要让 diva Harness 给 agent 写自身 crate 的代码
- **alife 单 ChatBot 互斥 Semaphore**: 在多 session 并发场景会成瓶颈;hermes 的 per-session cache + LRU 更好
- **hermes 的 YOLO freeze**: 实现巧妙但 export env var 模式 Rust 难复现,diva 可用 `OnceCell<YoloMode>` + `AtomicBool`
- **hermes 的 1751 行 approval.py**: Python 单文件巨型,diva 必须分模块: `dangerous_patterns.rs` / `approval_state.rs` / `approval_flow.rs` / `persistent_allowlist.rs`
- **alife 的 Autofac 全量依赖图**: 启动慢且复杂,diva v2 只需 `Module::Awake(Arc<Hub>)` 简单 hub 模式即可
- **hermes 的 toolset 11+ 类别**: 对小型 harness 过度,diva v2 先做 4-5 个核心 toolset(web/file/shell/cron/communication)即可

---

## 给 diva Harness v2 设计的 5 条建议

1. **建独立 crate `agent-diva-defense`** 装 threat_patterns + approve + sanitize + audit。复制 hermes 的 3 档 scope 模式,先覆盖"经典注入 + 凭据/SSH/AGENTS.md/CLAUDE.md 修改 + 不可见 unicode"四类,scope 至少 all/strict 两档。**位置**: 新建 crate,而不是塞进 `agent-diva-core/src/security/`(后者只剩路径校验,关注点应分离)

2. **`Tool` trait 加 `availability()` + `max_result_size` + `dangerous_patterns` 三个方法**。借鉴 hermes 的 `check_fn` 缓存 + `budget_config.py` PINNED_THRESHOLDS。**变更点**:
   - `agent-diva-tooling/src/base.rs` 改 `pub trait Tool`
   - 新增 `agent-diva-tooling/src/policy.rs` 装三层 BudgetPolicy(default 100K, per-turn 200K, preview 1500)
   - `agent-diva-tooling/src/registry.rs` 加 `register_with_policy()`

3. **approval.rs 拆成 4 模块 + 加 config.yaml 持久层**:
   - `dangerous_patterns.rs`(模式库,可序列化)
   - `approval_state.rs`(in-memory 状态)
   - `approval_flow.rs`(审批流程:CLI 交互 vs Gateway 异步)
   - `persistent_allowlist.rs`(新增,借鉴 hermes config.yaml 持久化)
   - 决策枚举保留 `Denied/Once/Session` 三档,新增 `ApprovedPermanent`

4. **加 `agent-diva-defense::Poke` 消息杂项队列**: 借鉴 alife ChatBot.Poke 的 11 条循环 + 30s flush 设计,做 diva 自己的"系统杂项消息推送"。**新增文件** `agent-diva-defense/src/poke_queue.rs`,作为 `bus/` 的伴生。**注意**: 不要让 Poke 通道成为 prompt injection 注入点 — flush 时也走 threat scanner 的 strict 档

5. **加 `EventBus` 多播 + 借鉴 alife `Func<...>` 多播模式**: 现有 `core/bus/` 偏 pub-sub,补 alife 风格的"单向 + 过滤"事件。最小集:`PreToolCall/PostToolCall/PreMessageOut/PostMessageOut/OnRateLimit/OnApprovalRequired`。让 Laputa changelog 自动订阅 `PreToolCall` 即可。**新增文件** `agent-diva-core/src/bus/multicast.rs`。**关联**: 让 hook 系统支持 `diva hooks list` CLI 自省

---

FINI
