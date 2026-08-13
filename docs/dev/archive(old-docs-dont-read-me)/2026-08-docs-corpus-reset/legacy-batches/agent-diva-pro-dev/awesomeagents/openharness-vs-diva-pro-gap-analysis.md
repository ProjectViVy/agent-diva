# OpenHarness vs agent-diva-pro：Agent Harness 能力差距分析

> 调研日期：2026-06-10
> 分析视角：Agent Harness（智能体运行时基础设施）能力对标
> 目标：识别 agent-diva-pro 相比 OpenHarness 欠缺的 harness 工程能力，为面具系统及其他架构演进提供输入

---

## 一、总览对照表

| 能力维度 | OpenHarness | agent-diva-pro | 差距 |
|---------|-------------|----------------|------|
| Agent Loop | 流式 + 并行工具执行 + 成本追踪 | 流式 + 推理模式 + 运行时控制 | **≈ 平** |
| Tool System | 43+ 内置工具，BaseTool 继承体系 | 10+ 内置工具，Trait 体系 + MCP 动态工具 | 🟡 工具数量少 |
| Memory | .md 文件 + 相关性评分 + 迁移 + 清理 | MEMORY.md + 日记 + 整合 + 预取 | **≈ 平** |
| 多 Agent / Swarm | **团队注册表 + 信箱 + 工作树 + 锁 + 权限同步 + 进程内/子进程后端** | SubagentManager（fire-and-forget，无通信） | **🔴 大差距** |
| Sandbox | Docker + srt + 路径验证 | Windows/Linux/macOS + 审批 + Guardian | **≈ 平** |
| Plugin/Hook | **完整插件生态 + Hook 热重载 + 4种 Hook 类型** | 仅文件/规划 Hook，无通用插件 | **🔴 大差距** |
| MCP | stdio + HTTP，自动重连 | rust-mcp-sdk，stdio + SSE | **≈ 平** |
| Skills | 多源加载 + 兼容 Anthropic 格式 | workspace/builtin + always-on + upload | 🟡 微差 |
| Task/后台任务 | **BackgroundTaskManager（shell + agent 子任务）** | 仅有 Plan/Todo 系统，无后台 shell 任务 | **🔴 大差距** |
| Cron | JSON 存储 + 时区 + 工具 CRUD | 三种调度类型 + HTTP API | **≈ 平** |
| Coordinator | **协调器模式 + Agent 定义 + 团队编排** | 仅 PlanOrchestrator，无多 Agent 协调 | **🔴 大差距** |
| Permission | **多级模式 + 敏感路径硬编码保护 + Hook 拦截** | SecurityPolicy + PathValidator + 沙箱审批 | 🟡 缺敏感路径硬保护 |
| Personalization | **对话事实自动提取（正则）** | Soul 系统 + VRM 宠物 + 语音 | 🟡 缺自动提取 |
| Auto-Dream | **自动记忆整合（10min 扫描）** | 整合机制存在（阈值触发） | 🟡 触发机制不同 |
| Channels | 11+ 适配器 + 事件总线 | 13 适配器 | **≈ 平** |
| Providers | 多 Provider 注册表 | 13 Provider + 动态切换 + 模型目录 | **≈ 平** |
| UI | **双 TUI（React/Ink + Textual）** | Tauri + Vue3 GUI | 🟡 缺 TUI |
| LSP | **语言服务器协议集成** | 无 | **🟡 中差距** |
| Voice | STT 流 + 语音模式 + 关键词检测 | TTS + ASR（多 Provider） | **≈ 平** |
| 自我进化 | **OHMO（自主分支/代码/PR）** | Soul 自修改 + 整合，无代码自改 | **🟡 中差距** |
| Autopilot | **自动 Agent 运行 + Dashboard** | 无 | **🟡 中差距** |
| Auth | **多 Provider 认证 + Copilot Auth** | 无统一 Auth 框架 | **🟡 中差距** |
| Network Guard | **网络访问验证** | 无独立网络守卫 | 🟡 小差距 |
| Bridge/Work-Secret | **外部工具连接桥** | 无 | 🟡 小差距 |
| Keybindings/Themes | **可配置键绑定 + 主题系统** | 无（GUI 有主题，CLI 无） | 🟡 小差距 |

---

## 二、关键差距详细分析

### 🔴 差距 1：Multi-Agent Swarm（最大差距）— **【2026-06-11 决策：暂缓，不作为参考路径】**

**OpenHarness 有什么：**

- `swarm/registry.py` — 团队注册表，命名 Agent、角色定义、成员管理
- `swarm/mailbox.py` — Agent 间消息传递（信箱机制）
- `swarm/worktree.py` — Git 工作树隔离，多 Agent 并行改代码不冲突
- `swarm/lockfile.py` — 共享资源协调锁
- `swarm/permission_sync.py` — 跨 Agent 权限状态同步
- `swarm/team_lifecycle.py` — 团队创建/管理/销毁全生命周期
- 双后端：subprocess（独立进程）+ in-process（同进程轻量）

**agent-diva 有什么：**

- `SubagentManager` — spawn 子 Agent，fire-and-forget，结果回传到 origin channel
- 子 Agent 之间**无通信**，**无共享状态**，**无协调机制**

**缺什么（技术上）：**

1. **Agent 间消息总线**（信箱/队列）
2. **团队注册与生命周期管理**
3. **Git 工作树隔离**（并行代码协作的核心）
4. **资源锁/协调原语**
5. **协调器模式**（coordinator 委派给 specialist）
6. **进程内轻量子 Agent 后端**（当前只有独立 tokio task，但缺乏正式的 in-process backend 抽象）

**【决策】**：

经产品负责人确认（2026-06-11），**agent-diva 不走 OpenHarness Swarm 路径**。理由：

1. **协作效率**：Swarm 的"头脑特工队"式平等协商，消息往返成本高、易竞争。Supervisor + 任务总线更高效。
2. **冲突解决成本**：Git worktree 隔离需要为每个 agent 维护独立 checkout，资源开销大。agent-diva 走 mask 系统的 supervisor 模式，成本更低。
3. **替代证据**：Hermes 的多 agent 协作走的是 SQLite-backed Kanban（任务总线），不是 mailbox-style swarm。
4. **mask Epic 3 已在路上**：当前 pro 的 mask 系统 Epic 3 (`f172895` parallel sub-agent orchestration) 正在用 supervisor 模式解决同样的问题。

**保留的可借鉴点（仅测试方法论，非架构）**：

- OpenHarness 的**测试体系**（事件驱动、E2E 真实 API、副作用验证）仍可参考，但这与 swarm 实现无关。

**参考文档**：

- `docs/dev/awesomeagents/swarm-branch-integration-feasibility.md` — 旧 `feature-swarm-humanlike` 分支接入可行性
- `docs/dev/awesomeagents/harness-landscape-2026.md` — 2026 年其他 harness 方案调研
- `docs/dev/awesomeagents/openharness-analysis.md` 第 5.4 节 — 决策记录原文

---

### 🔴 差距 2：Plugin/Hook 系统

**OpenHarness 有什么：**

- 插件从三个位置发现：用户目录、项目目录、内置
- 插件 manifest（JSON/YAML）定义 skills、commands、hooks、agent definitions
- 插件安装器（git repo / 本地路径）
- Hook 事件：`PreToolUse`、`PostToolUse`、`PreAgentTurn`、`PostAgentTurn`
- 4 种 Hook 类型：Command（shell）、Agent（子 Agent）、HTTP、Prompt（注入）
- **热重载** — 监控文件变化自动重载

**agent-diva-pro 有什么：**

- 文件操作 Hook（`agent-diva-files`）
- 规划生命周期 Hook（`agent-diva-agent`）
- 无通用插件注册表

**缺什么：**

1. **通用 Plugin Loader**（manifest 驱动、多来源发现）
2. **工具生命周期 Hook**（PreToolUse / PostToolUse 拦截）
3. **Agent 轮次 Hook**（PreAgentTurn / PostAgentTurn）
4. **Hook 热重载**
5. **Hook 多类型执行器**（Command / Agent / HTTP / Prompt）

---

### 🔴 差距 3：Background Task Manager

**OpenHarness 有什么：**

- `tasks/manager.py` — 后台任务管理器
- 支持 shell 任务和 agent 任务两种类型
- 创建、列表、获取输出、停止、更新
- 子进程 stdin/stdout 管道
- 任务状态机：pending → running → completed / failed / stopped

**agent-diva-pro 有什么：**

- Plan/Todo 系统（规划管理，不是后台任务）
- 子 Agent 后台运行（但没有统一的任务管理接口）

**缺什么：**

1. **统一的后台任务管理器**（shell + agent 任务）
2. **任务输出流式读取**
3. **任务生命周期管理**（创建/停止/更新/查询）
4. **任务状态持久化**

---

### 🔴 差距 4：Coordinator 编排模式

**OpenHarness 有什么：**

- `coordinator/coordinator_mode.py` — 检测协调器运行模式
- `coordinator/agent_definitions.py` — Agent 定义（frontmatter 解析：名称、颜色、effort、隔离模式、内存范围、权限模式）
- 协调器注入上下文到系统提示
- 团队注册表 + 跨团队消息

**agent-diva-pro 有什么：**

- `PlanOrchestrator` — 规划状态机（Explore → Plan → Execute → Verify）
- 无多 Agent 协调

**缺什么：**

1. **Coordinator 运行模式检测**
2. **Agent 定义规范**（frontmatter 驱动的 Agent 配置）
3. **协调器系统提示注入**
4. **跨 Agent 任务委派协议**

---

### 🟡 中等差距

| 能力 | OpenHarness | agent-diva-pro 缺什么 |
|------|-------------|----------------------|
| **LSP 集成** | `services/lsp/` — 语言服务器协议，代码智能 | 完全没有，可提供补全、跳转、诊断 |
| **Autopilot** | 自动 Agent 运行 + Dashboard | 无自动运行模式 |
| **Auth 框架** | 多 Provider 认证管理 + Copilot Auth | 无统一认证层 |
| **Personalization 自动提取** | 正则从对话中提取 SSH 主机、IP、路径、conda 环境等 | Soul 系统是手动/LLM 驱动的，无自动正则提取 |
| **工具搜索** | `tool_search_tool` — 运行时搜索可用工具 | 工具列表固定，无搜索能力 |
| **Notebook 编辑** | `notebook_edit_tool` — Jupyter notebook 操作 | 无 |
| **LSP 工具** | `lsp_tool` — 代码智能工具 | 无 |
| **Sleep 工具** | `sleep_tool` — Agent 主动等待 | 无 |
| **Remote Trigger** | `remote_trigger_tool` — 远程触发 | 无 |
| **Ask User Question** | `ask_user_question_tool` — 结构化提问 | 有 clarify 等效但可能不够灵活 |

---

### 🟡 小差距

| 能力 | 说明 |
|------|------|
| **TUI** | OpenHarness 有 React/Ink + Textual 双 TUI；diva 只有 CLI 交互 |
| **网络守卫** | OpenHarness 有独立 `network_guard.py`；diva 依赖沙箱网络策略 |
| **Bridge/Work-Secret** | OpenHarness 有外部工具连接桥；diva 无 |
| **Vim 模式** | OpenHarness 有 Vim 键绑定；diva 无 |
| **Output Styles** | OpenHarness 可配置输出格式；diva 无 |
| **敏感路径硬保护** | OpenHarness 硬编码拒绝 SSH key、云凭证等路径；diva 靠配置 |

---

## 三、agent-diva-pro 的独有优势

值得注意的是，diva 在某些方面**领先于** OpenHarness：

| 能力 | agent-diva-pro 独有 |
|------|---------------------|
| **3D VRM 宠物 + 桌面覆盖** | 无竞品，完整的 Three.js VRM 渲染 + 动画 + 外观定制 |
| **语音交互** | 多 Provider TTS/ASR（OpenAI, SiliconFlow, MiniMax, 浏览器） |
| **Tauri 桌面应用** | 原生桌面体验，系统托盘，嵌入式 Gateway |
| **Windows 服务** | `agent-diva-service` — Windows 原生服务化部署 |
| **Soul 治理** | 滚动窗口追踪 Soul 变化频率 + 边界确认 |
| **Rust 性能** | 全 Rust 异步 tokio，内存安全，无 GC 停顿 |
| **13 Channel 适配器** | 比 OpenHarness 的 11+ 更多，含 IRC、Mattermost、Nextcloud Talk |

---

## 四、优先级建议

> **【2026-06-11 更新】** 原 P0 项 "Multi-Agent Swarm" 经决策后**暂缓**（见差距 1 决策记录）。mask 系统 Epic 3 已在 supervisor 路径上推进。

如果要逐步补齐剩余差距，按影响力排序：

| 优先级 | 能力 | 理由 |
|--------|------|------|
| ~~P0~~ ~~Multi-Agent Swarm~~ | ⏸️ **已暂缓** | 决策：不走 OpenHarness swarm 路径。mask Epic 3 走 supervisor 模式替代。 |
| **P0** | Plugin/Hook 系统 | 生态系统的基础。没有通用插件，第三方无法扩展。PreToolUse/PostToolUse 是安全和可观测性的关键。 |
| **P1** | Background Task Manager | 用户期望 Agent 能管理后台任务（跑测试、编译、长时间脚本），不仅仅是子 Agent。 |
| **P1** | Coordinator 模式 | 复杂任务需要协调器委派，不是让主 Agent 一个人做所有事。 |
| **P2** | LSP 集成 | 代码智能体验的质变。 |
| **P2** | Auth 框架 | 多 Provider 场景下的凭证管理痛点。 |
| **P3** | 工具搜索 / Notebook / Sleep / Remote Trigger | 增强型工具，锦上添花。 |
| **P3** | TUI（ratatui） | Rust 生态有 ratatui，做起来自然。 |

---

## 五、与面具系统的关系

面具系统（Mask System）的设计目标之一就是补齐 harness 工程能力。原方案中的工具限制、子 agent 能力收窄、模型切换等机制，正是对 OpenHarness 所具备的 harness 能力的回应：

| OpenHarness 能力 | 面具系统对应设计 |
|-----------------|----------------|
| 团队注册表 + Agent 定义 | MaskRegistry + MaskConfig（frontmatter 驱动） |
| 权限同步 | `child ⊆ parent` 工具限制继承 |
| 协调器模式 | Batch spawn + SubAgentResult 结构化返回 |
| Hook 拦截 | 工具限制运行时强制执行 |
| 多后端（subprocess/in-process） | 子 agent 模型解析链 + 轻量/完整模式 |

面具系统是 agent-diva-pro 走向完整 harness 的第一步，但后续还需要 Swarm、Plugin、Task Manager 等基础设施才能真正对标 OpenHarness。

---

## 六、参考文档

| 文档 | 路径 |
|------|------|
| OpenHarness 深度调研 | `docs/dev/awesomeagents/openharness-analysis.md` |
| 7 项目对比决策记录 | `docs/dev/awesomeagents/decisions.md` |
| 能力清单 | `docs/dev/awesomeagents/diva-capability-checklist.md` |
| 对比矩阵 | `docs/dev/awesomeagents/comparison-matrix.md` |
| 未知缺陷分析 | `docs/dev/awesomeagents/unknown-deficits.md` |
| 演进路线 | `docs/dev/awesomeagents/evolution-roadmap.md` |
| 沙箱审计清单 | `docs/dev/awesomeagents/sandbox-audit-checklist.md` |

---

> 生成日期：2026-06-11
> 来源：session 20260610_215748_b8a3b8 中 OpenHarness vs agent-diva-pro 差距分析的整理归档
