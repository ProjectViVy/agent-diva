1|# 安全/沙箱/隔离/权限控制能力清单
2|
3|> 提取日期：2026-06-02
4|> 来源：全部 6 篇分析报告（openharness / codex / openfang / memtle / hermes / claude-code / genericagent）
5|
6|---
7|
8|## 安全能力汇总表
9|
10|| 能力名称 | 来源项目 | 简要说明 |
11||---------|---------|---------|
12|| Docker 沙箱执行环境 | OpenHarness | 通过 Docker 容器隔离工具执行，自动跳过不可用环境 |
13|| 路径/命令级权限系统 | OpenHarness | 支持路径和命令粒度的权限控制与测试验证 |
14|| Shell 转义防护 | OpenHarness | 防止命令注入，`$(echo INJECTED)` 等恶意输入作为纯文本存活 |
15|| 工具执行超时处理 | OpenHarness | 每个工具必须处理超时和取消，防止无限挂起 |
16|| stdin DEVNULL 验证 | OpenHarness | 交互式命令 stdin 重定向到 /dev/null，防止阻塞等待输入 |
17|| Hook 阻断机制 | OpenHarness | 通过 Hook 系统（pre_tool_use）阻止危险工具执行，模型需自适应 |
18|| 平台级沙箱 | Codex CLI | macOS Seatbelt、Linux Landlock+bubblewrap、Windows 受限令牌三平台原生沙箱 |
19|| SandboxManager 统一接口 | Codex CLI | 跨平台沙箱抽象层，统一不同操作系统的沙箱 API |
20|| SandboxPolicy 分层策略 | Codex CLI | ReadOnly / WorkspaceWrite / DangerFullAccess 三级全局安全策略 |
21|| AskForApproval 审批策略 | Codex CLI | Never / OnRequest / UnlessTrusted / OnFailure / Granular 五种审批模式 |
22|| FileSystemSandboxPolicy | Codex CLI | 细粒度文件系统读/写/拒绝策略，精确控制文件访问范围 |
23|| NetworkSandboxPolicy | Codex CLI | 网络访问控制策略，限制 Agent 的网络请求能力 |
24|| PermissionProfile 组合权限 | Codex CLI | 将多个权限策略组合为可复用的配置文件，由用户授予 |
25|| ExecApprovalRequest 审批流程 | Codex CLI | 工具执行前发送审批请求到 UI，用户可批准/拒绝/会话级授权 |
26|| 粘性授权持久化 | Codex CLI | 用户授予的权限可持久化，避免重复审批相同操作 |
27|| 子 Agent 线程限制 | Codex CLI | agent_max_threads（默认 6）控制最大并发子 Agent 数量 |
28|| 子 Agent 嵌套深度限制 | Codex CLI | agent_max_depth（默认 1）控制子 Agent 嵌套层数，防止无限递归 |
29|| 子 Agent 运行时间限制 | Codex CLI | agent_job_max_runtime_seconds 限制子 Agent 最大运行时长 |
30|| PermissionsInstructions 上下文注入 | Codex CLI | 将沙箱/审批模式信息注入模型上下文，让模型感知安全约束 |
31|| WASM 双计量沙箱 | openfang | 使用 WebAssembly 沙箱隔离执行，双重计量（CPU + 内存）防止资源滥用 |
32|| Merkle 哈希链审计 | openfang | 审计日志使用 Merkle 哈希链结构，保证日志不可篡改 |
33|| 信息流污染追踪 | openfang | 追踪数据来源和传播路径，标记不可信数据防止污染扩散 |
34|| Ed25519 签名 Agent 清单 | openfang | Agent 清单使用 Ed25519 数字签名，防止篡改和伪造 |
35|| SSRF 防护 | openfang | 阻止 Agent 发起指向内网/敏感地址的服务器端请求 |
36|| 密钥清零 | openfang | 内存中的密钥使用后立即清零，防止内存泄露 |
37|| OFP 互认证 | openfang | Agent 间通信使用 HMAC-SHA256 进行双向身份认证 |
38|| 能力门控 | openfang | 基于能力（capability）的访问控制，Agent 只能访问已授权的工具和资源 |
39|| 安全头注入 | openfang | HTTP 响应自动注入安全头（CSP、HSTS 等） |
40|| 健康端点脱敏 | openfang | 健康检查接口移除敏感系统信息，防止信息泄露 |
41|| 子进程沙箱 | openfang | 子进程在受限环境中执行，隔离于主进程 |
42|| 提示注入扫描 | openfang | 扫描用户输入中的提示注入攻击模式，拦截恶意指令 |
43|| 循环守卫 | openfang | SHA256 工具调用哈希检测无限循环，触发熔断机制 |
44|| 会话修复 | openfang | validate_and_repair 丢弃孤立消息、合并连续消息，防止状态异常 |
45|| 路径遍历防护 | openfang | 阻止 `../` 等路径遍历攻击，限制文件访问在工作区内 |
46|| GCRA 限流 | openfang | 使用 GCRA 算法进行请求限流，防止滥用和 DoS |
47|| Shell 元字符注入阻断 | openfang | 阻断 Shell 命令中的元字符注入，防止命令注入攻击 |
48|| 审批门（敏感工具审批） | openfang | 敏感工具执行前必须经过人工审批确认 |
49|| Agent 间调用深度限制 | openfang | Agent 间调用最大深度限制为 5，防止无限级联 |
50|| 工作区沙箱 | openfang | 文件操作限制在工作区目录内，禁止越界访问 |
51|| 1 MiB 请求帧硬上限 | memtle | MCP 请求帧大小限制为 1 MiB，防止 OOM 攻击 |
52|| 错误信息脱敏 | memtle | 内部路径和数据库细节永不泄露，仅转发 `"public": true` 的错误信息 |
53|| 输入验证（路径/空字节） | memtle | 拒绝路径遍历、空字节、非字母数字字符等恶意输入 |
54|| 控制字符剥离 | memtle | 剥离控制字符防止终端转义注入攻击 |
55|| 工具 Schema 白名单 | GenericAgent | 只有 `tools_schema.json` 中定义的工具才暴露给 LLM，防止模型调用未注册能力 |
56|| LLM 协议结构约束 | GenericAgent | 强制 `<thinking>`→`<summary>`→`<tool_use>` 输出结构，限制模型自由发挥 |
57|| 工具级操作防护 | GenericAgent | `file_patch` 要求唯一匹配才执行；`code_run` 仅允许白名单语言运行 |
58|| SOP 操作红线 | GenericAgent | 通过 Markdown SOP 文件定义操作边界，约束 Agent 行为范围 |
59|| 密钥加密存储 | GenericAgent | `memory/keychain.py` 使用 XOR 加密 + `SecretStr` 掩码保护敏感凭证 |
60|| 工具执行 Hook 可观测性 | GenericAgent | `plugins/hooks.py` 的 `tool_before/after` 事件 + Langfuse 追踪，实现工具调用审计 |
61|| 进程级代码执行隔离 | GenericAgent | `code_run` 通过 `subprocess.Popen` 子进程执行代码，与主进程隔离 |
62|| 代码执行超时控制 | GenericAgent | 超时监控线程 + `stop_signal` 列表，防止代码执行无限挂起 |
63|| Windows 无窗口执行 | GenericAgent | 使用 `CREATE_NO_WINDOW` 标志隐藏子进程窗口，避免 UI 干扰 |
64|| 多层级迭代上限 | GenericAgent | max_turns(40/80/100)、Plan Mode 硬停(120)、每7轮警告、每75轮检查点，防止无限循环 |
65|| 文件停止信号 | GenericAgent | `_stop` 文件触发外部终止控制，允许运行时干预正在执行的 Agent |
66|| 对抗性验证子 Agent | GenericAgent | 独立验证者目标是证伪而非确认，不接收执行历史避免上下文污染 |
67|| BBS API Key 访问控制 | GenericAgent | BBS 公告板使用 API Key 认证，支持多 board 隔离 |
68|| Goal Mode 双重限制 | GenericAgent | 时间预算(≥3h) + 轮次上限(50) 双控，防止自治任务失控 |
69|| MCP 环境变量过滤 | Hermes | `_build_safe_env()` 仅传递安全基线变量，防止子进程泄露宿主 API Key |
70|| 凭证自动脱敏 | Hermes | `_sanitize_error()` 在错误消息中替换 GitHub PAT、OpenAI Key 等为 `[REDACTED]` |
71|| Prompt 注入扫描 | Hermes | `_scan_mcp_description()` 扫描 MCP 工具描述中的注入模式（警告级） |
72|| OSV 恶意软件检查 | Hermes | stdio 模式下启动 MCP 子进程前检查包是否在 OSV 恶意软件数据库中 |
73|| MCP URL 协议校验 | Hermes | 拒绝非 http(s) 协议的 MCP 服务器 URL |
74|| MCP 工具注册覆写保护 | Hermes | 不同 toolset 之间的同名注册默认被拒绝，防止工具劫持 |
75|| 子 Agent 工具黑名单 | Hermes | `DELEGATE_BLOCKED_TOOLS` 禁止子 Agent 访问 `delegate_task`、`memory`、`send_message` 等 |
76|| 子 Agent 委托深度控制 | Hermes | `MAX_DEPTH=1`（默认），可配置最大 3 层，防止递归委托失控 |
77|| Kanban Worker 所有权守卫 | Hermes | `_enforce_worker_task_ownership()` 限制 Worker 只能操作自己的任务，防止越权 |
78|| 记忆威胁模式扫描 | Hermes | 写入前使用 `threat_patterns` 的 `strict` 范围扫描注入/外泄模式 |
79|| 记忆外部漂移检测 | Hermes | 检测到外部修改时创建 `.bak` 备份并拒绝写入，防止数据竞争 |
80|| 记忆跨平台文件锁 | Hermes | Unix 用 `fcntl.flock`，Windows 用 `msvcrt.locking`，保证并发安全 |
81|| 记忆原子写入 | Hermes | 先写临时文件再 `os.replace`，避免写入中途截断导致数据损坏 |
82|| 技能路径遍历防护 | Hermes | `has_traversal_component` + `validate_within_dir` 拒绝绝对路径、`..` 遍历、驱动器号 |
83|| 技能符号链接检测 | Hermes | 安装前遍历隔离区拒绝符号链接/连接点重定向 |
84|| 技能 SSRF 防护 | Hermes | `is_safe_url` + `check_website_access` + 重定向跟踪限制（5次） |
85|| 技能隔离区扫描 | Hermes | 下载→隔离→安全扫描→安装的四步安全流程 |
86|| 技能安装审计日志 | Hermes | 记录所有技能安装/卸载操作，支持事后追溯 |
87|| 技能缓存隔离 | Hermes | 缓存目录写入 `.ignore` 文件防止 ripgrep 搜索到未审查内容 |
88|| Cron Prompt 注入阻断 | Hermes | `CronPromptInjectionBlocked` 异常防止恶意 skill 内容注入 cron prompt |
89|| 媒体投递安全 | Hermes | 拒绝列表 + 可选严格模式 + 最近文件信任窗口，防止恶意文件投递 |
90|| 权限模式分层 | Claude Code | `default`/`plan`/`auto`/`bypassPermissions`/`bubble` 五种权限模式，按场景分级 |
91|| 权限行为四态 | Claude Code | `allow`/`deny`/`ask`/`passthrough` 四种权限行为，精细控制每个工具调用 |
92|| 8 层权限配置源 | Claude Code | userSettings、projectSettings、localSettings、flagSettings、policySettings、cliArg、command、session 按优先级合并 |
93|| 权限冒泡机制 | Claude Code | 子 Agent 的权限对话框冒泡到父终端，确保权限决策不被绕过 |
94|| YoloClassifier 自动判断 | Claude Code | auto 模式下工具调用+上下文发送给分类器 LLM 自动判断是否允许执行 |
95|| 子 Agent 上下文隔离 | Claude Code | 子 Agent 使用全新 `messages[]`，中间过程不污染主 Agent 上下文 |
96|| 子 Agent 递归禁止 | Claude Code | 子 Agent 无 `task` 工具，防止递归 spawn 新子 Agent |
97|| 子 Agent 权限继承 | Claude Code | 子 Agent 工具调用也走 PreToolUse hook，上下文隔离不代表权限隔离 |
98|| 只读/写入工具分区 | Claude Code | 只读工具(read/glob/grep)并发执行，写入工具(write/edit/bash)串行执行 |
99|| PreToolUse/PostToolUse Hook | Claude Code | 工具执行前后的 Hook 注入点，可拦截危险操作或收集审计信息 |
100|| Worktree 文件隔离 | Claude Code | 每个子 Agent 在独立 Git Worktree 中工作，避免并行文件冲突 |
101|| 工具白名单/黑名单 CLI | Claude Code | `--allowedTools`/`--disallowedTools` 命令行参数精确控制子 Agent 可用工具集 |
102|| 自定义 Agent 工具限制 | Claude Code | `.claude/agents/` 定义中可声明 `tools` 列表限制 Agent 能力范围 |
103|| Hook 阻断能力 | Claude Code | Hook 返回 `blockingError` 可阻断工具执行，返回 `permissionBehavior` 可覆盖权限决策 |
104|| Stop Hook 循环防护 | Claude Code | `stopHookActive` 机制防止 Stop hook 无限循环 |
105|| SOP 驱动的行为约束 | GenericAgent | 通过 Markdown SOP 文件定义操作红线和行为边界，约束 Agent 自治范围（区别于已有"SOP 操作红线"，侧重自治任务的行为约束） |
106|| 密钥 XOR 加密 + SecretStr 掩码 | GenericAgent | `memory/keychain.py` 使用 XOR 加密存储 + `SecretStr` 掩码防止日志泄露敏感凭证（区别于已有"密钥加密存储"，补充具体实现细节） |
107|| 代码执行超时监控线程 | GenericAgent | 超时监控线程 + `stop_signal` 列表，防止代码执行无限挂起（区别于已有"代码执行超时控制"，补充线程级实现） |
108|| 多层级迭代上限（自治控制） | GenericAgent | max_turns(40/80/100)、Plan Mode 硬停(120)、每7轮警告、每75轮检查点，多层防失控（侧重自治任务的迭代控制） |
109|| 文件停止信号 | GenericAgent | `_stop` 文件触发外部终止控制，允许运行时干预正在执行的 Agent |
110|| 对抗性验证子 Agent | GenericAgent | 独立验证者目标是证伪而非确认，不接收执行历史避免上下文污染 |
111|| BBS API Key 访问控制 | GenericAgent | BBS 公告板使用 API Key 认证，支持多 board 隔离，防止未授权 Agent 参与协作 |
112|| Goal Mode 双重限制 | GenericAgent | 时间预算(≥3h) + 轮次上限(50) 双控，禁止提前交付，防止自治任务失控 |
113|| 三层系统提示词缓存安全设计 | Hermes | stable/context/volatile 分层 + 字节稳定时间戳，防止缓存失效导致的安全上下文丢失 |
114|| 断路器模式 | Hermes | MCP 连续 3 次失败后断路器打开（60 秒冷却），防止级联故障和重试风暴 |
115|| MCP 工具并行安全标记 | Hermes | MCP 工具声明 `parallel_safe` 标记，不安全工具强制串行执行避免竞态条件 |
116|| 工具错误净化 | Hermes | `_sanitize_tool_error` 剥离异常中的 XML/CDATA/markdown 代码围栏，截断至 2000 字符，防止错误消息注入模型上下文 |
117|| Webhook 安全子集 | Hermes | `_HERMES_WEBHOOK_SAFE_TOOLS` 仅暴露 web_search、web_extract、vision_analyze、clarify 四个工具 |
118|| 全局暂停标志 | Hermes | `_spawn_paused` 全局标志可暂停所有子 Agent 生成，实现紧急制动 |
119|| 子 Agent 文件修改通知 | Hermes | 子 Agent 完成后检查是否修改了父 Agent 读取过的文件，在摘要中追加重读提醒 |
120|| 超时诊断转储 | Hermes | 子 Agent 0 次 API 调用后超时时，转储配置、prompt 大小、worker 线程栈到日志 |
121|| 27 种 Hook 事件类型 | Claude Code | 覆盖工具、会话、用户交互、子 Agent、压缩、团队、Worktree 全生命周期 |
122|| Hook 6 种执行方式 | Claude Code | command/prompt/agent/http/callback/function 六种 Hook 类型，支持多样化安全拦截 |
123|| HookResult 14 字段精细控制 | Claude Code | `blockingError`/`permissionBehavior`/`updatedInput` 等 14 个返回字段，实现精细化权限和行为控制 |
124|| 受限记忆提取 Agent | Claude Code | 记忆提取通过 forked agent 执行，`skipTranscript:true` + `maxTurns:5`，限制提取过程的权限和轮次 |
125|| Dream 四层门控 | Claude Code | 时间门(24h)、扫描节流、会话门(5 转录)、锁门四层机制，防止记忆整合滥用 |
126|| 输出格式结构化 | Claude Code | `--output-format json`/`stream-json` 结构化输出，便于父 Agent 解析验证，防止输出注入 |
127|| MCP 工具池合并命名空间 | Claude Code | `mcp__server__tool` 命名格式合并内置和 MCP 工具，防止不同 server 的工具名冲突和劫持 |
128|| MessageBus 文件收件箱隔离 | Claude Code | 每个 Agent 独立 `.jsonl` 邮箱 + `proper-lockfile` 防并发写冲突，实现消息级权限隔离 |
129|
130|---
131|
132|## 按安全维度分类
133|
134|### 沙箱与隔离
135|- Docker 沙箱执行环境（OpenHarness）
136|- 平台级沙箱：Seatbelt / Landlock+bubblewrap / Windows 受限令牌（Codex CLI）
137|- WASM 双计量沙箱（openfang）
138|- 子进程沙箱（openfang）
139|- 工作区沙箱（openfang）
140|- 进程级代码执行隔离（GenericAgent）
141|- 子 Agent 上下文隔离（Claude Code）
142|- Worktree 文件隔离（Claude Code）
143|- Webhook 安全子集（Hermes）
144|- 受限记忆提取 Agent（Claude Code）
145|- 工具并发串行分区（Claude Code）
146|- MCP 工具池合并命名空间（Claude Code）
147|
148|### 权限与访问控制
149|- SandboxPolicy 分层策略（Codex CLI）
150|- AskForApproval 审批策略（Codex CLI）
151|- FileSystemSandboxPolicy 细粒度文件权限（Codex CLI）
152|- NetworkSandboxPolicy 网络访问控制（Codex CLI）
153|- 能力门控（openfang）
154|- 路径/命令级权限系统（OpenHarness）
155|- 审批门（openfang）
156|- 工具 Schema 白名单（GenericAgent）
157|- 工具级操作防护（GenericAgent）
158|- 子 Agent 工具黑名单（Hermes）
159|- 子 Agent 委托深度控制（Hermes）
160|- Kanban Worker 所有权守卫（Hermes）
161|- 权限模式分层（Claude Code）
162|- 权限行为四态（Claude Code）
163|- 8 层权限配置源（Claude Code）
164|- 权限冒泡机制（Claude Code）
165|- YoloClassifier 自动判断（Claude Code）
166|- 子 Agent 递归禁止（Claude Code）
167|- 工具白名单/黑名单 CLI（Claude Code）
168|- 自定义 Agent 工具限制（Claude Code）
169|- SOP 驱动的行为约束（GenericAgent）
170|- 27 种 Hook 事件类型（Claude Code）
171|- Hook 6 种执行方式（Claude Code）
172|- HookResult 14 字段精细控制（Claude Code）
173|- MessageBus 文件收件箱隔离（Claude Code）
174|
175|### 审批流程
176|- ExecApprovalRequest 审批流程（Codex CLI）
177|- 粘性授权持久化（Codex CLI）
178|- Hook 阻断机制（OpenHarness）
179|- 子 Agent 权限继承（Claude Code）
180|- PreToolUse/PostToolUse Hook（Claude Code）
181|- Hook 阻断能力（Claude Code）
182|- 文件停止信号（GenericAgent）
183|- 对抗性验证子 Agent（GenericAgent）
184|- Dream 四层门控（Claude Code）
185|
186|### 注入防护
187|- Shell 转义防护（OpenHarness）
188|- Shell 元字符注入阻断（openfang）
189|- 提示注入扫描（openfang）
190|- 路径遍历防护（openfang）
191|- 输入验证：路径/空字节/非字母数字（memtle）
192|- 控制字符剥离（memtle）
193|- SSRF 防护（openfang）
194|- 工具错误净化（Hermes）
195|- 三层系统提示词缓存安全设计（Hermes）
196|- 输出格式结构化（Claude Code）
197|
198|### 资源限制
199|- 工具执行超时处理（OpenHarness）
200|- 子 Agent 线程/深度/运行时间限制（Codex CLI）
201|- Agent 间调用深度限制（openfang）
202|- 循环守卫（openfang）
203|- GCRA 限流（openfang）
204|- 1 MiB 请求帧硬上限（memtle）
205|- 代码执行超时监控线程（GenericAgent）
206|- 多层级迭代上限（GenericAgent）
207|- Goal Mode 双重限制（GenericAgent）
208|- 断路器模式（Hermes）
209|- MCP 工具并行安全标记（Hermes）
210|- 全局暂停标志（Hermes）
211|
212|### 审计与认证
213|- Merkle 哈希链审计（openfang）
214|- 信息流污染追踪（openfang）
215|- Ed25519 签名 Agent 清单（openfang）
216|- OFP 互认证 HMAC-SHA256（openfang）
217|- BBS API Key 访问控制（GenericAgent）
218|
219|### 信息泄露防护
220|- 密钥清零（openfang）
221|- 安全头注入（openfang）
222|- 健康端点脱敏（openfang）
223|- 错误信息脱敏（memtle）
224|- 密钥 XOR 加密 + SecretStr 掩码（GenericAgent）
225|- 超时诊断转储（Hermes）
226|- 子 Agent 文件修改通知（Hermes）
227|
228|### 状态完整性
229|- 会话修复（openfang）
230|- stdin DEVNULL 验证（OpenHarness）
231|- 文件停止信号（GenericAgent）
232|- 全局暂停标志（Hermes）
233|

| 工具黑名单（子Agent） | Hermes Agent | DELEGATE_BLOCKED_TOOLS 禁止子Agent调用 delegate_task/clarify/memory/send_message/execute_code |
| Worker 所有权守卫 | Hermes Agent | _enforce_worker_task_ownership() 防止 worker 越权操作其他任务 |
| MCP 环境变量过滤 | Hermes Agent | _build_safe_env() 仅传递安全基线变量，防止泄露宿主 API key |
| 凭证脱敏（错误消息） | Hermes Agent | _sanitize_error() 替换 GitHub PAT、OpenAI key 等为 [REDACTED] |
| Prompt 注入扫描（MCP） | Hermes Agent | _scan_mcp_description() 扫描 MCP 工具描述中的注入模式 |
| 威胁模式扫描（记忆） | Hermes Agent | 记忆写入前扫描注入/外泄威胁模式（threat_patterns strict 范围） |
| 路径遍历防护 | Hermes Agent | has_traversal_component + validate_within_dir 防止 .. 遍历 |
| 隔离区扫描（插件安装） | Hermes Agent | 下载 → 隔离 → 安全扫描 → 安装四步安全流程 |
| 8 级错误分类管道 | Hermes Agent | ClassifiedError 数据类，22 种错误类型，8 级优先级自动恢复 |
| 断路器（MCP/Provider） | Hermes Agent | 连续 3 次失败 → 60 秒冷却 → 半开探测 |
| 迭代预算控制 | Hermes Agent | IterationBudget consume/refund 模式，execute_code 退还预算 |
| 子Agent 深度控制 | Hermes Agent | MAX_DEPTH=1-3 硬限制，leaf/orchestrator 角色模型 |
| 文件状态协调 | Hermes Agent | 子Agent 修改文件后通知父Agent 重读，防止状态不一致 |
| 心跳/超时诊断 | Hermes Agent | _dump_subagent_timeout_diagnostic 转储配置+prompt大小+线程栈 |
| 工具并发冲突检测 | Hermes Agent | _should_parallelize 路径冲突检测 + _NEVER_PARALLEL_TOOLS 黑名单 |
| 3 层系统提示词缓存 | Hermes Agent | stable/context/volatile 分层，字节稳定设计最大化前缀缓存命中 |
| 冻结快照模式 | Hermes Agent | 系统提示稳定 vs 工具响应实时，保持前缀缓存稳定 |
| 权限模式系统 | Claude Code | default/acceptEdits/plan/auto/dontAsk/bypassPermissions 六种模式 |
| 工具白名单/黑名单 | Claude Code | --allowedTools/--disallowedTools，支持通配符和正则 |
| 工作区信任对话框 | Claude Code | 首次访问目录需确认信任，缓存信任状态 |
| 权限冒泡 | Claude Code | 子Agent 权限对话框冒泡到父终端，统一审批 |
| Hook 安全阻断 | Claude Code | PreToolUse hook 可阻断危险命令（exit 2 = 阻断） |
| Worktree 隔离 | Claude Code | EnterWorktree 为每个 Agent 创建隔离 Git 工作区 |
| 上下文窗口监控 | Claude Code | /context 命令可视化上下文使用率，70%+ 精度下降警告 |
| 4 层压缩管线 | Claude Code | snip/micro/budget/auto 自动防上下文溢出 |
| 文件干预机制 | GenericAgent | _keyinfo / _intervene / _stop 文件实现零连接实时注入 |
| SOP 程序性记忆固化 | GenericAgent | 常用操作模式固化为 SOP 文件，执行前 file_read 加载 |
| 对抗性验证协议 | GenericAgent | verify_sop：独立子Agent 证伪，不接收执行历史 |
| 空响应熔断 | GenericAgent | 连续空响应 3 次后退出，防止幻觉循环 |
| Ghost 动作检测 | GenericAgent | LLM 声称已执行通道动作但未调用工具→自动重新提示 |
| BBS 协作公告板 | GenericAgent | agent_bbs.py 多 Worker 异步任务领取，支持大规模并行探索 |
| Goal Mode 预算自治 | GenericAgent | 时间预算 + 轮次上限双控，禁止提前交付 |

