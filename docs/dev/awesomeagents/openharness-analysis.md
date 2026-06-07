# OpenHarness 深度调研分析

> 调研日期：2026-06-01
> 项目位置：`.workspace/OpenHarness/`
> 调研视角：agent-diva 架构调研，关注评估/测试/QA 设计

---

## 一、项目概览

OpenHarness（包名 `openharness-ai`，版本 0.1.9）是 Claude Code 的开源 Python 移植版，由香港大学数据科学实验室（HKUDS/novix-science）维护。它不仅仅是一个 agent 框架，更是一个**完整的 agent 基础设施**，包含：

- 44 个内置工具（文件 I/O、Shell、搜索、Web、MCP、任务管理等）
- 10 个聊天平台集成（Feishu、Slack、Telegram、Discord、DingTalk、Email、Matrix、MoChat、QQ、WhatsApp）
- 多 agent 协调系统（Swarm）
- 持久化记忆系统（Memory）
- 技能/插件/Hook 系统
- Docker 沙箱执行环境

**关键定位**：OpenHarness 不是一个"评估框架"，而是一个**被评估的对象**——它自身就是 agent 运行时，配套的评估体系用来验证这个运行时的正确性。

---

## 二、评估架构

### 2.1 整体架构

OpenHarness 的评估体系分为**三个层次**：

| 层次 | 工具 | 特点 |
|------|------|------|
| **单元测试** | `tests/` (119 个 pytest 文件) | Mock API，测试单个组件 |
| **集成测试** | `scripts/test_harness_features.py` | 真实 API 调用，测试功能组合 |
| **端到端评估** | `scripts/e2e_smoke.py` + `harness-eval` skill | 全栈真实调用，验证完整 agent 行为 |

**核心设计理念**（来自 `.claude/skills/harness-eval/SKILL.md`）：

1. **在陌生项目上测试** —— 永远不在 OpenHarness 自身上测试（agent 会修改自己的代码）
2. **使用真实 API 调用** —— 不用 mock
3. **多轮对话** —— 始终测试 2+ 轮，验证上下文保持
4. **组合特性测试** —— hooks + skills + agent loop 一起测试，而非隔离测试
5. **验证工具执行** —— 检查工具调用列表和输出文件，而非模型文本声明

### 2.2 评估任务的类型

评估任务来自三个来源：

**来源 1：声明式场景对象**（`scripts/e2e_smoke.py`）

```python
@dataclass(frozen=True)
class Scenario:
    name: str                           # 场景名，如 "file_io", "skill_flow"
    prompt: str                         # 发送给模型的精确指令
    expected_final: str                 # 模型必须回显的标记字符串（如 "FINAL_OK"）
    required_tools: tuple[str, ...]     # 必须出现在工具轨迹中的工具
    validate: ScenarioValidate          # 自定义验证函数
    setup: ScenarioSetup | None         # 可选的工作空间 fixture 设置
    ask_user_answer: str | None         # 模拟用户回答
```

验证函数签名：
```python
ScenarioValidate = Callable[[Path, str, list[str], int, int], tuple[bool, str]]
# (cwd, final_text, tool_names, started_count, completed_count) -> (pass, detail)
```

**来源 2：命令式测试函数**（`scripts/test_harness_features.py`、`scripts/local_system_scenarios.py`）

每个测试是 `async def`，返回 `tuple[bool, str]`。有些使用真实 API 调用（通过 `_run_oh()` 子进程），有些直接实例化组件进行隔离测试。

**来源 3：Pytest fixtures**（`scripts/test_docker_sandbox_e2e.py`）

标准 pytest，类级测试组织，模块级 fixture 用于 Docker 镜像构建。

### 2.3 评估结果的数据结构

事件流收集结构（`collect()` 函数）：

```python
def collect(events):
    r = {"text": "", "tools": [], "turns": 0, "in_tok": 0, "out_tok": 0}
    for ev in events:
        if isinstance(ev, AssistantTextDelta):
            r["text"] += ev.text
        elif isinstance(ev, ToolExecutionStarted):
            r["tools"].append(ev.tool_name)
        elif isinstance(ev, AssistantTurnComplete):
            r["turns"] += 1
            r["in_tok"] += ev.usage.input_tokens
            r["out_tok"] += ev.usage.output_tokens
    return r
```

验证结果格式：`tuple[bool, str]` —— (通过/失败, 详细信息)。

### 2.4 多轮对话评估

**完全支持**。核心原则要求"始终测试 2+ 轮"。具体实现：

- `engine.submit_message()` 支持连续调用，引擎维护内部消息历史
- `context_flow` 场景测试模型是否记住 CLAUDE.md 规则和持久化记忆
- `session_save_resume` 场景测试跨会话的上下文保持
- 长期场景（`architecture_multiturn`）测试 3+ 轮的上下文积累

### 2.5 评估的自动化程度

- **CI 自动化**：`.github/workflows/ci.yml` 运行 114 个单元测试 + Ruff linting + TypeScript 类型检查
- **Autopilot 自动化**：三个 GitHub Actions workflow（`autopilot-pages.yml`、`autopilot-run-next.yml`、`autopilot-scan.yml`）实现自动扫描和运行
- **E2E 半自动化**：E2E 测试需要真实 API key，由 `harness-eval` skill 驱动，可手动或 CI 触发
- **Docker 沙箱自动化**：自动跳过 Docker 不可用的环境

---

## 三、Agent Loop 评测视角

### 3.1 衡量 Agent Loop 质量的维度

OpenHarness 通过 `feature-matrix.md` 定义了明确的评测维度：

| 维度 | 测试内容 | 关键断言 |
|------|---------|---------|
| **正确率** | 事实回忆、工具链执行、文件操作 | 第 3 轮回忆第 1 轮的事实 |
| **效率** | 并行工具调用、自动压缩 | 单轮 3+ 工具调用；5+ 任务无上下文溢出 |
| **鲁棒性** | 错误恢复、Hook 阻断后适应 | 工具失败后使用替代工具 |
| **上下文保持** | 多轮记忆、会话恢复 | 跨轮次信息不丢失 |
| **成本追踪** | Token 累积 | `in_tokens` 严格递增 |

### 3.2 标准化评测集

OpenHarness 有**结构化的评测集**，但不是传统意义上的 benchmark：

**E2E 场景集**（`scripts/e2e_smoke.py`）：
- `file_io` —— 文件写入/读取/验证
- `search_edit` —— glob → grep → read → edit 工具链
- `notebook_edit` —— Jupyter notebook 编辑
- `skill_flow` —— 技能加载和使用
- `hook_adapt` —— Hook 阻断后模型适应
- `context_flow` —— 上下文注入和记忆
- `session_resume` —— 会话保存和恢复
- `parallel_tools` —— 并行工具执行
- `error_recovery` —— 错误恢复
- `auto_compact` —— 自动压缩

**长周期场景**（推荐）：
- `architecture_multiturn` —— 3 轮架构分析
- `hook_block_and_recover` —— Hook 阻断和恢复
- `sandbox_multiturn` —— 沙箱多轮执行

### 3.3 "好的" Agent 行为定义

OpenHarness 通过**五层断言体系**定义"好的"行为：

1. **工具轨迹断言** —— 必须使用指定工具
2. **最终文本标记断言** —— 必须包含特定标记字符串（如 `FINAL_OK`）
3. **文件系统状态断言** —— 实际文件内容必须正确
4. **工具执行计数断言** —— 启动和完成的工具调用数量必须达标
5. **内容特定断言** —— 深度内容检查，验证上下文正确性

### 3.4 工具使用的评测指标

工具评测的核心指标：

- **工具选择正确性** —— 模型是否选择了正确的工具
- **工具链完整性** —— 多工具协作是否完整执行
- **工具调用参数正确性** —— 参数是否符合预期
- **工具执行成功率** —— 工具是否成功完成
- **错误恢复能力** —— 工具失败后是否能适应

---

## 四、工具链评测

### 4.1 工具测试方法

OpenHarness 的工具测试采用**分层策略**：

**层 1：单元测试（Mock）**

```python
# tests/test_tools/test_bash_tool.py
class _FakeProcess:
    async def read(self, *a): return b""
    async def wait(self): return 0
    def terminate(self): pass
    def kill(self): pass

# 使用 monkeypatch 替换 create_shell_subprocess
monkeypatch.setattr("openharness.tools.bash_tool.create_shell_subprocess", lambda **kw: fake_proc)
```

测试覆盖：交互式命令预检、超时处理、部分输出收集、stdin DEVNULL 验证、未关闭 stdout 防挂起。

**层 2：集成测试（真实文件系统）**

```python
# tests/test_tools/test_core_tools.py
def test_write_read_edit_roundtrip(tmp_path):
    ctx = ToolExecutionContext(cwd=tmp_path)
    # 测试 write_file → read_file → edit_file 完整流程
```

测试覆盖：写-读-编辑往返、glob 模式匹配、grep 各种选项、LSP 操作、git worktree 生命周期、Cron CRUD。

**层 3：E2E 测试（真实 API）**

```python
# scripts/e2e_smoke.py - Scenario("file_io")
Scenario(
    name="file_io",
    prompt="Create a file called smoke.txt with content OPENHARNESS_E2E_OK, then read it back and reply FINAL_OK",
    expected_final="FINAL_OK",
    required_tools=("write_file", "read_file"),
    validate=lambda cwd, text, tools, s, c: (
        (cwd / "smoke.txt").read_text().strip() == "OPENHARNESS_E2E_OK"
        and "FINAL_OK" in text,
        "file content or final text mismatch"
    ),
)
```

### 4.2 工具执行环境模拟

OpenHarness **不模拟工具执行环境**，而是使用：

- **真实文件系统**（`tmp_path`）—— 单元/集成测试
- **真实 LLM API** —— E2E 测试
- **真实 Docker 容器** —— 沙箱测试
- **Fake 进程对象** —— 仅用于 BashTool/GrepTool 的单元测试

### 4.3 工具调用正确性的评判标准

评判标准分三层：

1. **结构正确性** —— 工具是否被调用、参数格式是否正确
2. **语义正确性** —— 工具执行结果是否符合预期
3. **副作用正确性** —— 文件系统、Docker 容器等实际状态是否正确

### 4.4 错误恢复能力的评测

错误恢复是 OpenHarness 评测的核心维度之一：

```python
# 错误恢复场景
Scenario(
    name="error_recovery",
    prompt="Try to read /nonexistent/path, then create recovery.txt with RECOVERED and reply FINAL_OK",
    required_tools=("read_file", "write_file"),
    validate=lambda cwd, text, tools, s, c: (
        "RECOVERED" in (cwd / "recovery.txt").read_text() if (cwd / "recovery.txt").exists() else False,
        "recovery file not created after error"
    ),
)
```

Hook 阻断测试验证模型在工具被阻止后能否适应：

```python
# Hook 阻断 → 模型适应
Scenario(
    name="hook_adapt",
    prompt="List files using bash, then read one file and reply FINAL_OK",
    required_tools=("glob", "read_file"),  # bash 被 hook 阻止，模型必须用 glob 替代
    validate=lambda cwd, text, tools, s, c: (
        "bash" not in tools and "glob" in tools,
        "model did not adapt after bash was blocked"
    ),
)
```

---

## 五、A2A（多 Agent）评测

### 5.1 子 Agent 协作评测

OpenHarness 的多 agent 系统（Swarm）有完整的测试覆盖：

**Swarm 测试目录**（`tests/test_swarm/`）：
- `test_in_process.py` —— InProcessBackend 生命周期
- `test_registry.py` —— 后端注册和检测
- `test_mailbox.py` —— 文件邮箱消息传递
- `test_permission_sync.py` —— 权限同步
- `test_team_lifecycle.py` —— 团队 CRUD
- `test_types.py` —— 类型定义
- `test_spawn_utils.py` —— 子进程构建
- `test_subprocess_backend.py` —— 子进程后端
- `test_worktree.py` —— Git worktree 隔离
- `test_lockfile.py` —— 文件锁

**Coordinator 模式测试**（`tests/test_coordinator/`）：
- `test_coordinator_mode.py` —— 协调器检测和编排
- `test_agent_definitions.py` —— Agent 定义系统
- `test_registry.py` —— Agent 注册

### 5.2 多 Agent 场景的评估方法

**场景 1：并发队友**

```python
async def test_concurrent_teammates():
    await asyncio.gather(
        asyncio.wait_for(run_one("worker-a", "Count .py files"), timeout=30),
        asyncio.wait_for(run_one("worker-b", "Find main class"), timeout=30),
    )
    # 断言：两个都完成，总时间 < 2x 单个时间
```

**场景 2：协调器 + 通知**

```python
# 测试 Coordinator 发出 agent 工具调用，处理 XML 通知，综合 worker 结果
class CoordinatorLoopApiClient:
    # 第一次调用：发出 agent 工具使用
    # 第二次调用：返回文本
```

**场景 3：权限同步**

```python
# 测试 request → pending → resolve 流程
# 断言：pending 计数在 resolve 后变为 0
```

### 5.3 任务分解和委派的评测

任务分解通过 `AgentTool` 测试：

```python
# AgentTool 实现
class AgentTool(BaseTool):
    name = "agent"
    # 验证模式：local_agent, remote_agent, in_process_teammate
    # 查找 AgentDefinition 获取系统提示、模型、权限
    # 使用 SubprocessBackend 或 InProcessBackend 执行
    # 注册到团队注册表
    # 设置 SUBAGENT_STOP hook
```

Coordinator 系统提示定义了明确的委派指南：
- 并发指导：只读任务并行，写任务串行
- Worker 提示合成指南：永远不要委派理解，始终包含具体文件路径
- Continue vs Spawn 决策表

---

## 六、记忆与技能评测

### 6.1 记忆系统评测

记忆测试（`tests/test_memory/`）覆盖：

| 测试项 | 验证内容 |
|--------|---------|
| 路径稳定性 | `get_project_memory_dir` 返回预期路径 |
| 相关性搜索 | `find_relevant_memories` 返回正确匹配并按相关性排序 |
| 添加/去重 | 相同内容添加两次只产生一个文件 |
| 软删除 | `remove_memory_entry` 设置 `disabled=True`，文件保留 |
| 迁移 | `migrate_memory` 回填 schema_version，创建备份，幂等 |
| 使用追踪 | `mark_memory_used` 递增 `use_count` |
| 前端解析 | YAML 前端解析、降级处理、CJK 查询 |
| 运行时记忆 | 记忆提取、会话记忆、团队记忆守卫、Agent 记忆 |

**相关性评分公式**（`memory/search.py`）：

```python
score = (
    meta_hits * 2.0           # 标题 + 描述匹配，权重 2x
    + body_hits               # 正文预览匹配，权重 1x
    + header.importance * 0.4 # 显式重要性字段
    + min(use_count, 5) * 0.1 # 使用频率（上限 5）
    + _recency_boost(header)  # 0.3（≤14天）/ 0.1（≤30天）/ 0
)
```

### 6.2 技能系统评测

技能测试覆盖：
- 技能加载（4 层：bundled → user → project → plugins）
- YAML 前端解析
- 技能元数据注入系统提示
- `SkillTool` 运行时调用
- 模型主动使用技能

---

## 七、Hook 系统评测

### 7.1 Hook 事件和类型

10 个生命周期事件：

```python
class HookEvent(str, Enum):
    SESSION_START = "session_start"
    SESSION_END = "session_end"
    PRE_COMPACT = "pre_compact"
    POST_COMPACT = "post_compact"
    PRE_TOOL_USE = "pre_tool_use"
    POST_TOOL_USE = "post_tool_use"
    USER_PROMPT_SUBMIT = "user_prompt_submit"
    NOTIFICATION = "notification"
    STOP = "stop"
    SUBAGENT_STOP = "subagent_stop"
```

4 种 Hook 类型：Command、Prompt、HTTP、Agent。

### 7.2 Hook 测试覆盖

| 测试项 | 验证内容 |
|--------|---------|
| Command Hook | `printf 'booted'` 输出正确，不被阻止 |
| Prompt Hook 阻断 | `{"ok": false, "reason": "blocked by policy"}` 正确传播 |
| Shell 转义 | `$(echo INJECTED)` 作为文字文本存活 |
| 优先级排序 | 高优先级先执行，同优先级保持注册顺序 |
| 负优先级 | 在默认（零）优先级 Hook 之后执行 |

---

## 八、与 agent-diva 的关联

### 8.1 OpenHarness 可以作为 agent-diva 的 QA/自评估层吗？

**部分可以，但需要适配。**

**可直接借鉴的**：
- 评估架构设计（三层：单元 → 集成 → E2E）
- 声明式场景对象模式
- 五层断言体系
- 真实 API 调用的 E2E 测试理念

**需要适配的**：
- OpenHarness 是 Python，agent-diva 是 Rust —— 测试框架不同
- OpenHarness 的评估是**外部驱动**的（harness-eval skill），不是内建的自评估
- OpenHarness 没有"agent 自我评估"机制 —— 它评估的是框架本身，不是 agent 行为质量

**不适合作为直接 QA 层的原因**：
- OpenHarness 评估的是"框架功能是否正确"，而非"agent 行为是否智能"
- 没有主观质量评估（如回答质量、用户满意度）
- 没有性能基准测试（延迟、吞吐量）
- 没有安全评估（prompt injection、权限逃逸）

### 8.2 agent-diva 中缺失的评测维度

对比 OpenHarness 的评测体系，agent-diva 缺失以下维度：

| 维度 | OpenHarness 状态 | agent-diva 状态 |
|------|-----------------|----------------|
| **Agent Loop 正确性** | ✅ 完整覆盖 | ❌ 缺失 |
| **工具链测试** | ✅ 分层覆盖 | ⚠️ 部分（单元测试） |
| **多轮对话测试** | ✅ 核心要求 | ❌ 缺失 |
| **Hook/拦截测试** | ✅ 完整覆盖 | ❌ 缺失 |
| **记忆系统测试** | ✅ 完整覆盖 | ⚠️ 部分 |
| **多 Agent 协作测试** | ✅ 完整覆盖 | ❌ 缺失 |
| **会话恢复测试** | ✅ 完整覆盖 | ❌ 缺失 |
| **错误恢复测试** | ✅ 核心维度 | ❌ 缺失 |
| **真实 API E2E** | ✅ 核心理念 | ❌ 缺失 |
| **沙箱隔离测试** | ✅ Docker 支持 | ❌ 缺失 |
| **成本追踪测试** | ✅ Token 累积 | ❌ 缺失 |
| **权限系统测试** | ✅ 路径/命令级 | ⚠️ 部分 |

### 8.3 如何将评测集成到 agent-diva 的开发流程中

**阶段 1：建立基础测试框架**

```
agent-diva/
├── tests/
│   ├── unit/              # 现有的 Rust 单元测试
│   ├── integration/       # 新增：组件间集成测试
│   └── e2e/               # 新增：端到端评估
├── scripts/
│   ├── e2e_smoke.py       # 新增：声明式场景
│   └── eval_runner.py     # 新增：评估运行器
└── .claude/
    └── skills/
        └── diva-eval/     # 新增：评估 skill
```

**阶段 2：定义评测场景**

借鉴 OpenHarness 的 `Scenario` 模式，为 agent-diva 定义：

1. **基础功能场景** —— 消息路由、工具调用、上下文构建
2. **多轮对话场景** —— 记忆保持、上下文累积
3. **错误恢复场景** —— Provider 失败、工具超时、网络中断
4. **多平台场景** —— 不同 Channel 的消息处理
5. **安全场景** —— 权限检查、输入验证

**阶段 3：CI 集成**

```yaml
# .github/workflows/eval.yml
- name: Run unit tests
  run: cargo test --all
- name: Run integration tests
  run: cargo test --test integration
- name: Run E2E smoke (if API key available)
  if: env.ANTHROPIC_API_KEY != ''
  run: python scripts/e2e_smoke.py
```

### 8.4 是否适合作为 diva 自我改进的反馈回路

**适合，但需要扩展。**

OpenHarness 的评测体系可以作为**质量保证层**，但不适合作为**自我改进反馈回路**，因为：

1. **它是外部驱动的** —— 需要人工或 CI 触发，不是 agent 自主运行
2. **它是功能导向的** —— 测试"能不能做"，而非"做得好不好"
3. **它没有学习机制** —— 测试结果不会反馈到 agent 行为调整

**建议的自我改进架构**：

```
┌─────────────────────────────────────────────┐
│              Agent-Diva 自评估系统            │
├─────────────────────────────────────────────┤
│                                             │
│  ┌──────────┐    ┌──────────┐    ┌────────┐ │
│  │ 功能测试  │    │ 质量评估  │    │ 安全审计│ │
│  │ (借鉴OH) │    │ (新增)   │    │ (新增) │ │
│  └────┬─────┘    └────┬─────┘    └───┬────┘ │
│       │               │              │      │
│       └───────────────┼──────────────┘      │
│                       │                     │
│              ┌────────▼────────┐            │
│              │   评估报告生成   │            │
│              └────────┬────────┘            │
│                       │                     │
│              ┌────────▼────────┐            │
│              │  改进建议生成    │            │
│              │  (LLM 分析)     │            │
│              └────────┬────────┘            │
│                       │                     │
│              ┌────────▼────────┐            │
│              │  自动应用改进    │            │
│              │  (需人工审核)    │            │
│              └─────────────────┘            │
│                                             │
└─────────────────────────────────────────────┘
```

---

## 九、对 agent-diva 的启示

### 9.1 架构层面

1. **建立分层测试体系** —— 单元 → 集成 → E2E，每层有不同的 mock 程度
2. **引入声明式场景** —— 用 `Scenario` 数据结构定义测试，而非硬编码测试函数
3. **事件流驱动验证** —— 通过事件流（`StreamEvent`）收集 agent 行为，而非检查最终状态
4. **真实 API E2E** —— 在 CI 中可选地运行真实 API 测试，捕获 mock 无法发现的问题

### 9.2 评测维度

1. **多轮对话是核心** —— OpenHarness 的核心原则要求"始终测试 2+ 轮"
2. **错误恢复是必测项** —— 不仅测试 happy path，还要测试 agent 在错误后的行为
3. **组合特性测试** —— 不要隔离测试每个特性，要测试它们的交互
4. **副作用验证** —— 不要信任 agent 的文本声明，要检查实际状态

### 9.3 工具测试

1. **工具接口标准化** —— OpenHarness 的 `BaseTool` + `ToolResult` 模式值得借鉴
2. **错误不抛异常** —— 工具通过 `ToolResult(is_error=True)` 返回错误，而非抛异常
3. **超时和取消** —— 每个工具都必须处理超时和取消
4. **输出截断** —— 大输出自动截断，防止上下文溢出

### 9.4 多 Agent 测试

1. **后端抽象** —— `TeammateExecutor` Protocol 定义了统一的后端接口
2. **邮箱通信** —— 文件邮箱 + 消息队列的双通道通信模式
3. **权限同步** —— Leader-Worker 权限协调机制
4. **生命周期管理** —— 团队创建、成员管理、会话清理

### 9.5 记忆系统

1. **多因子相关性评分** —— 元数据权重 2x、正文 1x、重要性、使用频率、时间衰减
2. **签名去重** —— 内容签名避免重复存储
3. **软删除** —— 删除只是标记 `disabled`，文件保留
4. **使用追踪** —— 记录 `use_count`，用于相关性评分和过期检测

### 9.6 具体行动建议

| 优先级 | 行动项 | 预期收益 |
|--------|--------|---------|
| **P0** | 为 agent-diva-core 的 agent loop 建立单元测试 | 确保核心循环正确性 |
| **P0** | 定义 5 个基础 E2E 场景（消息路由、工具调用、多轮对话、错误恢复、会话恢复） | 建立质量基线 |
| **P1** | 引入事件流收集和验证机制 | 结构化的行为验证 |
| **P1** | 为工具系统建立分层测试（Mock → 真实文件系统 → 真实 API） | 工具可靠性 |
| **P2** | 建立 `diva-eval` skill，定义评估工作流 | 可重复的评估流程 |
| **P2** | 集成 CI 自动化评估 | 持续质量保证 |
| **P3** | 建立自评估反馈回路 | 自我改进能力 |

---

## 十、关键代码引用

| 组件 | 文件路径 | 关键行 |
|------|---------|--------|
| 评估 Skill 定义 | `.claude/skills/harness-eval/SKILL.md` | L1-194 |
| 特性测试矩阵 | `.claude/skills/harness-eval/references/feature-matrix.md` | L1-48 |
| 测试模板 | `.claude/skills/harness-eval/references/test-patterns.md` | L1-151 |
| E2E 场景定义 | `scripts/e2e_smoke.py` | L1-807 |
| 集成测试 | `scripts/test_harness_features.py` | L1-239 |
| 引擎测试 | `tests/test_engine/test_query_engine.py` | L1-1552 |
| 工具基类 | `src/openharness/tools/base.py` | BaseTool + ToolResult |
| Swarm 类型 | `src/openharness/swarm/types.py` | TeammateExecutor Protocol |
| 记忆相关性 | `src/openharness/memory/relevance.py` | 评分公式 |
| Hook 执行器 | `src/openharness/hooks/executor.py` | HookExecutor.execute() |
| Coordinator | `src/openharness/coordinator/coordinator_mode.py` | 系统提示和委派指南 |
| Agent 定义 | `src/openharness/coordinator/agent_definitions.py` | 7 个内置 agent |

---

## 十一、总结

OpenHarness 的评测体系是一个**成熟、实用的 agent 框架测试方案**，其核心价值在于：

1. **真实 API 调用** —— 不用 mock，捕获序列化和 API 兼容性 bug
2. **多轮对话测试** —— 验证上下文保持和记忆
3. **组合特性测试** —— 测试特性交互，而非隔离测试
4. **副作用验证** —— 检查实际状态，而非信任文本声明
5. **错误恢复测试** —— 验证 agent 在约束下的适应能力

对于 agent-diva，最有价值的借鉴是：**建立一个分层的、事件驱动的、以真实 API E2E 为核心的测试体系**，并将评测结果作为持续改进的反馈信号。
