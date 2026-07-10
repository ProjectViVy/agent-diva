# agent-diva 计划模式流程调研报告

日期：2026-07-10
范围：`agent-diva/` 当前实现；`.workspace/oh-my-pi`、`.workspace/OpenHarness`、`.workspace/claude-code`、`.workspace/codex`；`.workspace/openakita` 的计划与 UI 相关实现。

## 1. 摘要

用户提出的目标流程是合理的，并且与几个参考项目中最稳定的共同模式一致：

```text
进入规划
  → 只读探索（查看目录、读取代码、必要的外部调研）
  → 生成完整计划
  → 用户审阅/修改计划
  → 用户明确批准
  → 进入编辑/执行模式
  → 按需生成或更新 TODO
  → 执行、验证、收尾
```

关键原则不是“计划模式里不能写任何东西”，而是要区分两类写入：

1. **规划产物写入**：计划文档、研究笔记、计划快照、可选 TODO；允许写入，但只能写入计划专用存储，不能写入项目源码或业务文件。
2. **执行产物写入**：源码、配置、脚本、数据库、外部系统和任意副作用；在用户批准前必须硬阻断。

因此，建议 agent-diva 将“计划阶段”和“文件修改权限”建模为独立但联动的状态：计划阶段默认是 `read-only exploration`，计划批准后才切换为 `edit/execute`。TODO 不是计划模式的必选副产物，而是对执行阶段有持续价值时才创建的可选工作清单。

## 2. 参考项目对比

| 项目 | 计划/探索边界 | TODO 做法 | 写入/批准边界 | 对 agent-diva 的启示 |
|---|---|---|---|---|
| oh-my-pi | 通过工具级别区分 `read`、`write`、`exec` | TODO 是会话状态，可分 phase，单个操作更新；失败时整次更新回滚 | 审批模式独立于工具声明；未知工具默认按 `exec` 安全处理 | 计划模式应使用工具白名单/能力策略，不应只依赖 prompt |
| OpenHarness | 模式切换会刷新 engine system prompt；测试明确验证 Plan mode 提示是否存在 | `TodoWriteTool` 写入工作区 `TODO.md`，有 upsert 和集成测试 | 文件编辑工具通过 approval callback 询问用户；拒绝则不写入 | 模式切换必须影响运行时配置和系统提示，并有测试覆盖 |
| claude-code | `EnterPlanMode`、`ExitPlanMode`、`TodoWrite` 是不同工具；退出计划模式是显式审批点 | TODO/Task 是独立跟踪层，UI 可单独展开/隐藏 | 计划审批与普通工具权限不是同一事件；用户可先反馈，再决定是否执行 | 不能把“计划存在”误当成“用户批准”；计划应可审阅后再放行 |
| codex | Plan 是 collaboration mode；sandbox/approval policy 是另一个维度 | 通过 plan delta / plan step 状态事件向 UI 推送 | 只读 sandbox 与 approval policy 可组合；官方讨论明确只读适合规划和解释 | 状态、沙箱、审批、UI 事件应分别建模，避免一个枚举承担全部语义 |
| openakita | 计划工具负责任务分解和进度；UI 以浮动进度条/任务视图呈现 | 任务树、步骤依赖、checkpoint/rollback 是执行可观测性的一部分 | 六层安全模型包含路径分区、确认门、命令过滤、快照和 OS 沙箱 | UI 应突出当前阶段、阻断原因、下一步操作，并把 TODO 当作进度视图而非强制表单 |

## 3. 参考项目证据

### 3.1 oh-my-pi：工具能力分级，而不是只靠模式提示

`docs/approval-mode.md` 定义了三种工具审批等级：`read`、`write`、`exec`。未声明审批等级的工具默认按 `exec` 处理。审批模式 `always-ask`、`write`、`yolo` 决定哪些等级自动放行，用户还可以按工具覆盖 `allow/deny/prompt`。

这套设计的价值是：即使模型误解了提示词，运行时仍能依据工具分类拒绝副作用。它也明确指出，子 agent 可以使用 headless 的自动批准，但父级 `task` 审批仍然是授权边界。

`docs/tools/todo.md` 则把 TODO 当成独立的会话状态模型：支持 phase、`pending/in_progress/completed/abandoned`，强制只有一个活动任务；失败操作不持久化部分结果，`view` 是只读操作。这个模型适合借鉴，但不应直接等同于项目级 TODO 文件。

### 3.2 OpenHarness：模式切换必须刷新运行时提示

`tests/test_ui/test_runtime_plan_mode.py` 验证了：构建运行时后，系统提示初始不包含 `Plan mode is enabled`；执行计划模式命令后，系统提示必须包含该标记。这说明 Plan mode 不是只在 UI 上显示的开关，而是会改变下一轮模型运行时上下文的状态。

`tests/test_tools/test_core_tools.py` 验证了文件编辑工具会调用 `edit_approval_prompt`，批准时返回成功并产生 diff 统计，拒绝时不会完成编辑；同一文件还验证了 `TodoWriteTool` 写入 `TODO.md`、重复写入的 upsert 行为以及集成流程。

可借鉴的组合是：

```text
mode transition → rebuild prompt/tool registry → tool-level approval → state persistence
```

### 3.3 claude-code：计划审批、澄清和 TODO 分离

`docs/design/tool-search-design-guide.md` 将 `EnterPlanMode`、`ExitPlanMode`、`VerifyPlanExecution` 与 `TodoWrite` 分开列为不同能力。`packages/builtin-tools/src/tools/AskUserQuestionTool/prompt.ts` 还特别规定：计划阶段可以用问题工具澄清需求，但不能用它询问“计划是否准备好了”；计划批准必须走 `ExitPlanMode`。

这直接支持用户提出的流程：

- 先探索和澄清；
- 形成可见的完整计划；
- 通过专门的批准事件进入执行；
- TODO 只是执行辅助，不是审批本身。

### 3.4 codex：Plan、sandbox、approval policy 三层分离

当前 `.workspace/codex` 的配置文档包含 `plan_mode_reasoning_effort`，并在 Rust 代码/测试中分别出现 `ModeKind::Plan`、`SandboxPolicy::ReadOnly`、`AskForApproval::OnRequest`。`app-server-test-client/src/lib.rs` 的测试场景明确把只读 sandbox 与按需审批组合起来。

此外，协议层包含 `PlanDeltaNotification`、`TurnPlanUpdatedNotification` 和计划步骤状态类型，说明计划不应只作为最后一段 Markdown 文本返回；它需要作为结构化事件流供 UI 增量更新。

OpenAI Codex 的公开讨论也明确说明 read-only 模式适合规划和解释，不适合代码生成；这支持“研究阶段和编辑阶段必须切换能力配置”的判断。[Codex 关于 read-only 模式的说明](https://github.com/openai/codex/discussions/7380)

### 3.5 openakita：把计划进度做成可持续观察的 UI

`README.md` 将 Plan Mode 描述为自动任务分解、逐步跟踪和失败回滚，并将 UI 表述为 floating progress bar。`src/openakita/tools/definitions/plan.py`、`src/openakita/tools/handlers/plan.py` 和 `tests/component/test_plan_handler.py` 形成了工具定义、执行处理器和组件测试的闭环。

`README_CN.md` 的安全模型还列出路径分区、操作确认门、命令拦截、文件快照、自保护和 OS 级沙箱。对 agent-diva UI 来说，最值得借鉴的不是视觉皮肤，而是把以下信息持续显示出来：

- 当前阶段：探索、计划审阅、等待批准、执行、验证；
- 当前计划是否完整；
- 当前是否允许修改文件；
- 需要用户做什么：审阅、批准、拒绝、补充信息；
- 当前 TODO/步骤的完成率和阻塞原因。

## 4. agent-diva 当前实现核对

### 4.1 已有的正确基础

当前代码已经具备若干重要基础：

- `agent-diva-core/src/planning/model.rs` 有 `Explore`、`Plan`、`AwaitingApproval`、`Execute`、`Verify`、`Completed`、`Failed`、`Partial` 阶段。
- `agent-diva-agent/src/agent_loop/loop_turn.rs` 对计划模式设置 `plan_guard_active`，并在运行时阻止不在计划白名单中的工具。
- 计划白名单包含 `read_file`、`list_dir`、`read_attachment`、`plan_create`、`plan_show`、`plan_transition`、`todo_show`、`todo_write`。
- 当计划进入 `AwaitingApproval` 时，代码会发出 `PlanReadyForApproval` 事件；同一轮不会继续向模型请求下一次工具调用。
- `agent-diva-agent/src/agent_loop/loop_runtime_control.rs` 的批准路径会将活动计划转为 `Execute`。
- `agent-diva-gui/src/App.vue` 已根据 `AwaitingApproval`、`Execute`、`Verify` 显示不同计划状态，并提供批准和撤销入口。

这些能力说明当前实现不是缺少状态机，而是需要重新收紧状态语义和产物边界。

### 4.2 主要问题

1. **请求模式与生命周期阶段混在一起**

   `is_plan_mode(&msg)` 的请求判断和活动计划的 `AwaitingApproval` 判断共同形成 `plan_guard_active`。这能防止一部分越权，但“用户本轮选择了 Plan”与“已有计划等待批准”是不同事实，应该分别存储。

2. **计划期允许 `todo_write`，但未定义它是否属于规划产物**

   当前白名单允许 `todo_write`，而且计划上下文要求模型“Create or update a plan and TodoList only”。这会让模型自然地在探索期间创建 TODO；如果这些 TODO 被当作执行任务或 UI 活动计划，就会产生用户所说的流程错位。

3. **完整计划的完成条件不够明确**

   当前有 `plan_transition` 和 `AwaitingApproval`，但完整计划至少应包含目标、范围、探索结论、涉及文件/模块、执行步骤、验证方式、风险和待确认问题。没有这些字段时，不应进入审批状态。

4. **UI 目前偏向“活动计划/TODO 条”而不是“阶段门禁”**

   `App.vue` 已经能显示计划运行时和批准按钮，但 UI 需要显式说明“当前禁止编辑”以及“批准后才进入编辑模式”。TODO 展示应该是可选区域，不应成为计划模式的默认主交互。

5. **全局仓库状态并不干净，不能把现有未跟踪内容归因于本调研**

   本次开始前 `agent-diva/` 已存在大量未跟踪项；本报告只新增一个研究文档，不应把这些既有内容误认为本次修改。

## 5. 建议的目标状态机

建议把计划状态拆成两个正交轴：

### 5.1 Plan lifecycle

```text
Exploring
  → Drafting
  → ReadyForReview
  → AwaitingApproval
  → Approved
  → Executing
  → Verifying
  → Completed | Failed | Partial
```

### 5.2 Capability mode

```text
ReadOnlyExplore
  → ReadOnlyPlanReview
  → WorkspaceEdit
```

约束：

- `Exploring` 和 `Drafting` 只能使用目录/文件读取、搜索、网络调研和计划存储工具。
- `ReadyForReview` / `AwaitingApproval` 禁止再次执行外部副作用；允许用户编辑计划文档或通过 UI 修改计划字段。
- 只有显式批准才能把 capability mode 切到 `WorkspaceEdit`。
- `todo_write` 默认不在探索期创建执行 TODO；如果需要记录待办，应写入计划内的“候选 TODO”区，且标记为 `optional`，批准后再物化为执行 TODO。
- `WorkspaceEdit` 下才允许写源码、配置、脚本和其他项目文件。
- 用户拒绝或撤销批准时，回到 `ReadOnlyPlanReview` 或终止计划，不得自动降级成可编辑模式。

## 6. TODO 设计建议

TODO 应是需求驱动的可选产物：

- 单次、简单、无依赖任务：可以没有 TODO，直接执行并验证。
- 多文件、多步骤、长时间或需要恢复的任务：生成 TODO。
- 调研本身：使用研究报告章节/计划步骤，不要把每一次探索动作都变成执行 TODO。
- 执行期新增工作：允许追加 TODO，但必须记录来源、原因和是否改变了已批准范围。
- 计划变更影响范围、风险、文件清单或验证方式时，应重新进入审阅/批准，而不是静默继续。

建议 TODO 至少包含：`id`、`title`、`phase`、`status`、`optional`、`depends_on`、`scope`、`evidence`、`created_from`。UI 默认显示步骤摘要，展开后显示 TODO；没有 TODO 时显示“本计划无需持久化 TODO”，而不是空白或虚假的进度卡。

## 7. UI 建议（参考 openakita，并结合现有 GUI）

建议在聊天输入区上方使用单一的“计划门禁卡”，而不是让计划卡和 TODO 卡同时争夺主视觉：

```text
┌ 计划：重构记忆模块                         [只读探索]
│ 阶段：等待审阅      发现：12 个文件 / 3 个风险
│ 计划完整度：已完成  ·  TODO：无（可选）
│ [查看完整计划] [编辑计划] [批准并进入编辑] [取消]
└──────────────────────────────────────────┘
```

交互要求：

- 只读阶段在顶部持续显示锁定标识和“不会修改项目文件”。
- 计划未完整时，批准按钮禁用，并显示缺少的字段。
- “查看完整计划”展开 Markdown/结构化计划；“编辑计划”只修改计划存储，不修改源码。
- 批准按钮文字应明确写出后果，例如“批准并进入编辑模式”，不要只写“继续”。
- 批准后卡片变成执行进度条；TODO 作为步骤的二级信息展开。
- 失败时保留计划和证据，显示“修订计划”或“重试验证”，不要直接清空。

## 8. 后续实现拆分建议

本报告不执行代码修改。若进入实现阶段，建议按以下原子变更拆分：

1. 定义独立的 capability mode 和 plan lifecycle 合约，补充非法迁移测试。
2. 把探索/审阅期工具白名单改为显式能力策略，补充“任何项目文件写入均失败”的测试。
3. 为计划增加完整性校验和 `ReadyForReview` 事件。
4. 将候选 TODO 与执行 TODO 分离，支持无 TODO 的计划。
5. 调整批准 API，使批准同时产生 capability transition 和审计事件。
6. 重做 GUI 计划门禁卡，补充只读、缺字段、批准、拒绝、撤销、无 TODO、失败重试的 UI 测试。
7. 最后再更新 TODOLIST/迭代日志，并运行完整 CI。

## 9. 结论

agent-diva 当前已经有可复用的计划生命周期和批准基础，但目标流程应从“模型被提示去计划”升级为“运行时强制的只读研究阶段 + 完整计划审阅门 + 明确的编辑模式切换”。最重要的改动方向是：

> 计划模式不是编辑模式的轻量变体；它是一个有独立能力边界、独立产物存储和独立批准事件的只读工作阶段。

TODO 应保留为可选的执行管理工具，而不是计划模式的强制输出。UI 应围绕“当前能不能改文件、为什么、下一步需要用户做什么”组织信息。

## 10. 证据索引

- `agent-diva-core/src/planning/model.rs`
- `agent-diva-agent/src/agent_loop/loop_turn.rs`
- `agent-diva-agent/src/agent_loop/loop_runtime_control.rs`
- `agent-diva-gui/src/App.vue`
- `.workspace/oh-my-pi/docs/approval-mode.md`
- `.workspace/oh-my-pi/docs/tools/todo.md`
- `.workspace/OpenHarness/tests/test_ui/test_runtime_plan_mode.py`
- `.workspace/OpenHarness/tests/test_tools/test_core_tools.py`
- `.workspace/claude-code/docs/design/tool-search-design-guide.md`
- `.workspace/claude-code/packages/builtin-tools/src/tools/AskUserQuestionTool/prompt.ts`
- `.workspace/codex/docs/config.md`
- `.workspace/codex/codex-rs/app-server-test-client/src/lib.rs`
- `.workspace/codex/codex-rs/analytics/src/reducer.rs`
- `.workspace/openakita/src/openakita/tools/definitions/plan.py`
- `.workspace/openakita/src/openakita/tools/handlers/plan.py`
- `.workspace/openakita/tests/component/test_plan_handler.py`
- `.workspace/openakita/README.md`
- `.workspace/openakita/README_CN.md`
