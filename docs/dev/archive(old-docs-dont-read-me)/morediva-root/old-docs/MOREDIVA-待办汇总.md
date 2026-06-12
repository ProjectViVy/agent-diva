# morediva 待办汇总

更新日期：2026-06-05
工作目录：C:\Users\Administrator\Desktop\morediva
整理原则：优先收敛“仍然活跃、能直接转成行动项、且不是纯历史归档”的内容；archive / logs 仅在它们明确承载尚未迁入主清单的待办时才引用。

## 1. 顶层项目盘点

| 目录 | 类型 | 当前分支/状态 | 是否纳入本次汇总 | 备注 |
|---|---|---|---|---|
| agent-diva | Git 仓库 | `main` | 是 | 主线；有明确 `TODOLIST.md` |
| agent-diva-pro | Git 仓库 | `feature/context-compaction` | 是 | 当前正在推进 context compaction / thinking / pet UI 等 |
| agent-diva-selfinprove | Git 仓库 | `autoresearch/agent-diva-autodream-rhythm-session-history-evid-20260531` | 是 | 主要是 GenericAgent / AutoDream / 记忆体系研究沉淀 |
| agent-diva-sandbox | Git 仓库 | `agent-diva-with-sandbox` | 是 | 有独立迁移审查和大量未合入改动 |
| _bmad-output | 规划输出目录 | 非 Git | 是 | 有一套尚未完全落地的 UI stories / PRD / architecture |
| agent-diva-agent-new | Git 仓库 | `agent-diva-agent-new` | 参考 | 与 selfinprove 高度重叠，主要作为旁证，不单独列主待办 |
| agent-diva-channel-adapter-and-plugins | Git 仓库 | `agent-diva-plugings-and-adapter` | 参考 | 当前未发现比主线/研究文档更独立的新待办主题 |
| agent-diva-tui | Git 仓库 | `main` | 参考 | 当前未发现独立高优先级未尽事项 |
| agent-diva-vrm-memory-test | Git 仓库 | `vrm-memory-test` | 参考 | 与 VRM / memory 方向强相关，可作为后续专项分支 |
| legancy | 非 Git 旧资料 | 非 Git | 是（低优先） | 有大量历史 VRM/桌宠实施清单，适合抽取可复用项 |
| diva-olv-package | 非 Git | 非 Git | 暂不纳入 | 仅有少量信号，未见明确主线阻塞 |
| tui-test | 非 Git | 非 Git | 否 | 暂无待办信号 |
| .omc | 配置目录 | 非 Git | 否 | 非项目待办来源 |

## 2. 核心汇总：主线仍未完成事项

### 2.1 agent-diva（主线 main）
来源：`agent-diva/TODOLIST.md`

当前主线待办是全仓最可信的一份“项目级 backlog”。其中已完成项较多，但以下项目仍处于 Open 状态：

#### P1 核心基础设施
1. P1-1 Plan+TodoList implementation
   - 现状：6 篇设计文档已完成，但代码仍为 0。
   - 缺口：`PlanOrchestrator`、`PlanStateStore`、`PlanVerifier`、`PlanPhase`、`TodoList`、`TodoItem` 等均不存在。
   - 依赖：`D-3 SQLite vs file-backed JSON` 先定案。

2. P1-2 Phase B: Thin Observability Layer
   - 现状：tracing 基础已在，但 spec 合规度低。
   - 缺口：typed `TraceId/TraceEvent`、JSONL writer、structured event emission、debug bundle。

3. P1-3 Sandbox audit remediation
   - 现状：核心安全能力部分已有；平台级 sandbox 仍未闭环。
   - 缺口：RestrictedToken / Landlock / Seatbelt、env filtering、prompt injection scanning、MCP limits、subagent concurrency。

4. P1-4 Permission mode UI wired to backend
   - 现状：前端选择器存在，但未接后端。
   - 缺口：permission / approval / three-tier / sticky 相关后端与穿线实现。

5. P1-6 Error classification system
   - 现状：`ToolError` 仍是 flat variants。
   - 缺口：error category / code / retry classification / 跨 crate taxonomy。

#### 决策类阻塞项
6. D-1 Hermes learning integration go/no-go
   - 现状：已有 6 篇规划文档、13-18 周路线图，但零进展。

7. D-2 HA'S-PROJECT memory system replacement
   - 现状：记忆方向需要最终取舍；文档倾向 LAPUTA + MENTLE，HA'S-PROJECT 不建议继续。

8. D-3 SQLite vs file-backed JSON for plan storage
   - 现状：前置研究推荐 SQLite；需在 Plan 模式落地前最终拍板。

9. D-4 5-layer bypass prevention design review
   - 现状：spec 有，代码没有；依赖 Plan Mode。

10. D-5 NAG mechanism threshold validation
    - 现状：全代码库无 nag/reminder/inject；依赖 Plan Mode。

#### Housekeeping
11. H-1 `docs/dev/README.md` 断链
12. H-2 `awesomeagents/decisions.md` 有未提交修改
13. H-3 self-evolution UI 调研文档仍标记为 `future/pro`
14. H-4 `plan-todo-ui-scope-extract.md` 需校验与源文档一致性

结论：`agent-diva` 当前最该收敛的是“Plan 模式 / Permission / Sandbox / Observability”这组核心基础设施，而不是再开新方向。

### 2.2 agent-diva-pro（feature/context-compaction）
来源：
- `agent-diva-pro/TODOLIST.md`
- `docs/adr/0010-context-compaction.md`
- `docs/epics/context-compaction-epics.md`
- `docs/prds/prd-agent-diva-pro-2026-06-03/prd.md`
- `docs/research/thinking-mode-integration-report.md`
- `git status`

#### 当前明确未完成项
1. GUI multimodal 图片粘贴体验
   - 直接来自 `TODOLIST.md`
   - 目标：GUI composer 支持剪贴板图片粘贴，经现有 attachment path 上传并预览。

2. Context Compaction P0 实装还未收尾
   - ADR 已定义交付物：
     - `TokenEstimator`
     - `ContextBudgetMonitor`
     - `CompactSummary`
     - `ContextCompactor`
     - session store / manager 序列化适配
     - agent loop 集成
   - Epic 文档已拆到 7 个 Epic、每个 3-5 个 Story。
   - `git status` 显示已出现未提交实现：
     - 修改：`agent-diva-agent/src/agent_loop/loop_turn.rs`
     - 修改：`agent-diva-agent/src/context.rs`
     - 修改：`agent-diva-core/src/session/{manager,mod,store}.rs`
     - 新增：`agent-diva-agent/src/compaction/`
     - 新增：`agent-diva-agent/src/context_budget.rs`
     - 新增：`agent-diva-agent/src/token_estimate.rs`
     - 新增：`agent-diva-agent/tests/`
   - 判断：这是“已开工但未收尾/未验证/未提交”的高优先级事项。

3. Thinking mode 只完成 80%，还缺 GUI 与配置层
   - 研究报告明确列出 4 个 gap：
     - 每模型独立 reasoning 配置
     - GUI thinking block 折叠/展开渲染
     - 用户可切换的 thinking mode 开关
     - `ModelCapabilities.reasoning` 仍硬编码为 false
   - 这是一组清晰可实施的后续工作。

4. Pet 内嵌视图全屏交互优化 PRD 尚未落地
   - `docs/prds/prd-agent-diva-pro-2026-06-03/prd.md`
   - 范围包括：
     - 宠物页全屏沉浸模式
     - overlay 侧边栏
     - 5 秒自动消失
     - 浮动 mini status bar
   - 当前更像需求文档，未见已完成实现证据。

#### 分支现场状态（需先清一轮）
- 分支：`feature/context-compaction`
- 已修改 9 个文件，未跟踪 9 个路径。
- 除 context-compaction 代码外，还有：
  - `docs/research/agent-diva-thinking-injection-points.md`
  - `docs/research/cherry-studio-thinking-analysis.md`
  - `docs/adr/`, `docs/epics/`, `docs/prds/` 整批未提交
- 结论：此分支现在更像“调研 + 设计 + 半实现混在一起”，需要先按主题拆清：context compaction、thinking、pet UI 三条线。

### 2.3 agent-diva-selfinprove（AutoDream / GenericAgent 研究线）
来源：
- `docs/dev/genericagent/README.md`
- `context-compaction-research.md`
- `autodream-rhythm-distillation-design.md`
- `shared-memory-rendering-research.md`
- `candidate-evidence-journal-audit-design.md`
- `git status`

这一支主要不是代码 backlog，而是“研究已成熟、待进入实施”的架构 backlog。

#### A. Context Compaction 后续项
1. P1 provider-aware context window
2. P1 manual `/compact` 入口
3. P1 reactive compact safety net
4. P1 post-compact file restore（重注入最近访问文件提示）
5. P2 multiple compaction merging / stacked compaction 更智能处理

#### B. Shared MEMORY.md 分层渲染
1. MVP：在 `agent-diva-core/src/memory/manager.rs` 内做 section-aware rendering
   - parse sections
   - tier classify
   - relevance score
   - budget enforcement
   - prefetch archive recall
2. Open Questions 仍未拍板：
   - budget tuning
   - relevance 取 session context 还是当前消息
   - consolidation 如何保结构
   - AutoDream target section 机制
   - deprecated 生命周期
   - multi-agent memory sharing
   - user editing feedback

#### C. AutoDream Rhythm Distillation
1. P0a Manual Trigger + Structured Output
   - 候选类型、checkpoint、lock、audit、worker、manager endpoint、集成测试都还要落地
2. P0b Session-End Eligibility Signal
   - 非阻塞 eligibility check + AutoDreamEvent 事件
3. P1
   - time gate / session count gate
   - startup catch-up
   - restricted subagent runner
   - candidate inbox integration
   - journal draft integration
   - ReviewCard in chat
4. P2
   - daily/weekly/monthly rhythm
   - Mentle recall integration
   - 自动低风险 merge policy
   - GUI Journal tab
   - `JournalRefCard`
   - changelog / rollback
   - cross-channel aggregation
5. Open Questions
   - prompt 结构
   - evidence budget
   - Mentle tool exposure
   - 是否等待 Source Capsule
   - multi-channel aggregation
   - candidate review UX
   - retention policy
   - `sync_turn()` 与 AutoDream 长期关系

#### D. 现场未提交研究资产
`git status` 显示：
- 修改：`docs/dev/genericagent/README.md`
- 修改：`autodream-rhythm-distillation-design.md`
- 修改：`context-compaction-research.md`
- 未跟踪：
  - `autoresearch.sh`
  - `autoresearch_validate.py`
  - `candidate-evidence-journal-audit-design.md`
  - `shared-memory-rendering-research.md`
  - `docs/logs/2026-05-shared-memory-rendering/`

结论：selfinprove 现在最像“下一代记忆/节律系统的设计仓”，需要决定哪些设计升级为主线 epics，哪些继续留作研究。

## 3. 除用户点名外，仍有未尽事项的额外来源

### 3.1 agent-diva-sandbox
来源：
- `docs/dev/sandbox-audit/agent-diva-sandbox-summary.md`
- `docs/dev/sandbox-audit/agent-diva-sandbox-migration-plan.md`
- `git status`

这是一个非常明确的“未合入但接近可迁”的专题分支。

#### 最关键结论
- 综合结论：`MIGRATE WITH FIXES`
- 不能直接合主线，因为有 2 个 CRITICAL 安全问题。

#### 必修整改
P0（合入前必须修）
1. 修复 shell 命令重新拼接导致的注入路径
2. 修复 Windows RestrictedToken fallback，确保不静默降级
3. 修复 `test_exec_does_not_fallback` 测试

P1（第一批合入前）
4. macOS fail-open → fail-closed
5. Guardian 默认 `auto_approve_known_safe=false`
6. BANNED_PREFIX 路径别名匹配补齐

P2（第二批）
7. 扩展 protected paths
8. 修复路径 starts_with 匹配
9. 统一审批缓存
10. 非零退出码改为 Err

P3（后续优化）
11. Orchestrator 解耦
12. API 清理 + feature gate
13. 与主线 SecurityPolicy 统一

#### 迁移切片
- PR #1：sandbox 引擎 + 配置层
- PR #2：shell 工具 + agent 循环
- PR #3：manager 运行时集成
- PR #4：GUI 设置页（可选）

#### 分支现场
- `agent-diva-with-sandbox` 工作区改动非常大：34 modified + 11 untracked
- 包含新 crate `agent-diva-sandbox/`、GUI 设置页、manager 接口、文档与脚本
- 这是一个需要“专题收口”的分支，不能继续放散。

### 3.2 _bmad-output
来源：
- `planning-artifacts/prds/prd-my-project-2026-06-02/prd.md`
- `planning-artifacts/architecture.md`
- `implementation-artifacts/stories/*.md`

这不是代码仓，但它是一个明确的“待实现池”。

#### 主要未落地内容
- 决策卡片 `DecisionCard.vue`
- Todo 卡片 `TodoCard.vue`
- Approval Banner
- NotebookView
- SelfEvolutionSettings
- MemoryChangelog
- SandboxSettings
- Event stream production
- Settings route / i18n / DTO interface 等一整套 story

适合作为：
- `agent-diva-pro` GUI 自进化路线的任务来源池
- 但需要先和真实代码状态对账，避免重复做已完成项

### 3.3 legancy（旧资料，但仍可抽取任务）
来源：`legancy/live2d-vrm-intergrate-test/...`

有大量 VRM / 桌宠增强相关的完整验收与任务清单，虽然是旧资料，但仍然含可执行事项：
- VRM enhancement acceptance / deployment / project management
- desktop pet menu enhancement
- avatar rebuild plan
- AniPet 动态表达生成分析

建议只抽取“仍对当前 pro / vrm-memory-test 有价值”的条目，不要整包回灌主线。

## 4. 横向观察：哪些目录信号最密集

按 signal_files / unchecked_boxes 粗看：
1. `agent-diva-pro`：60 个带信号 md，347 个未勾选框
2. `agent-diva-selfinprove`：50 个带信号 md，327 个未勾选框
3. `agent-diva`：50 个带信号 md，185 个未勾选框
4. `agent-diva-agent-new`：49 个带信号 md，327 个未勾选框（但与 selfinprove 重叠高）
5. `agent-diva-vrm-memory-test`：40 个带信号 md，320 个未勾选框（偏专项分支）
6. `agent-diva-sandbox`：24 个带信号 md，196 个未勾选框
7. `_bmad-output`：17 个带信号 md，108 个未勾选框
8. `legancy`：12 个带信号 md，309 个未勾选框（历史资料居多）

解释：数量大不等于都该做。对当前主线最有价值的还是：`agent-diva`、`agent-diva-pro`、`agent-diva-selfinprove`、`agent-diva-sandbox`、`_bmad-output`。

## 5. 建议收敛成的执行队列

### 第一优先级：先止乱
1. 清 `agent-diva-pro` 分支现场
   - 把 context compaction / thinking / pet UI 文档与代码按主题分组
   - 明确哪些已经实现、哪些仅文档化、哪些未验证
2. 清 `agent-diva-selfinprove` 研究现场
   - 把 context compaction / shared memory / autodream / audit schema 四条线整理成“可转 epic 的实施包”
3. 决定 `agent-diva-sandbox` 是否进入主线修复迁移周期
   - 不然它会继续成为巨型悬空分支

### 第二优先级：主线必须补的基础设施
4. `agent-diva` 的 Plan+TodoList
5. Permission mode UI 接后端
6. Sandbox remediation / merge path
7. Observability Phase B
8. Error classification

### 第三优先级：产品体验面
9. `agent-diva-pro` multimodal GUI 图片粘贴
10. thinking mode GUI 渲染与切换
11. pet 全屏沉浸模式
12. `_bmad-output` 里的自进化 UI stories 对账落地

### 第四优先级：中长期架构
13. shared memory rendering MVP
14. AutoDream P0a / P0b
15. context compaction P1 安全网
16. cross-channel rhythm / journal / candidate inbox

## 6. 建议下一步动作（可直接拆任务）

A. 先做一次“文档去重 + 主题归档”
- 把 `agent-diva-pro`、`agent-diva-selfinprove`、`agent-diva-sandbox` 中本轮提到的待办整理到统一编号表
- 标明：主线 / 分支实验 / 历史参考

B. 再做一次“真实代码状态对账”
- 对 `_bmad-output`、pro 的 PRD/ADR/Epic、selfinprove 的研究设计，逐项核对是否已有实现或部分实现

C. 最后产出一版真正可执行的 backlog
- 推荐按 4 条主线建看板：
  1. main 基础设施
  2. pro 交互与 compaction
  3. selfinprove 研究转实施
  4. sandbox 专题迁移

## 7. 本次扫描用到的关键来源

- `C:\Users\Administrator\Desktop\morediva\agent-diva\TODOLIST.md`
- `C:\Users\Administrator\Desktop\morediva\agent-diva-pro\TODOLIST.md`
- `C:\Users\Administrator\Desktop\morediva\agent-diva-pro\docs\adr\0010-context-compaction.md`
- `C:\Users\Administrator\Desktop\morediva\agent-diva-pro\docs\epics\context-compaction-epics.md`
- `C:\Users\Administrator\Desktop\morediva\agent-diva-pro\docs\prds\prd-agent-diva-pro-2026-06-03\prd.md`
- `C:\Users\Administrator\Desktop\morediva\agent-diva-pro\docs\research\thinking-mode-integration-report.md`
- `C:\Users\Administrator\Desktop\morediva\agent-diva-selfinprove\docs\dev\genericagent\README.md`
- `C:\Users\Administrator\Desktop\morediva\agent-diva-selfinprove\docs\dev\genericagent\context-compaction-research.md`
- `C:\Users\Administrator\Desktop\morediva\agent-diva-selfinprove\docs\dev\genericagent\autodream-rhythm-distillation-design.md`
- `C:\Users\Administrator\Desktop\morediva\agent-diva-selfinprove\docs\dev\genericagent\shared-memory-rendering-research.md`
- `C:\Users\Administrator\Desktop\morediva\agent-diva-sandbox\docs\dev\sandbox-audit\agent-diva-sandbox-summary.md`
- `C:\Users\Administrator\Desktop\morediva\agent-diva-sandbox\docs\dev\sandbox-audit\agent-diva-sandbox-migration-plan.md`
- `C:\Users\Administrator\Desktop\morediva\_bmad-output\planning-artifacts\prds\prd-my-project-2026-06-02\prd.md`
- `C:\Users\Administrator\Desktop\morediva\_bmad-output\planning-artifacts\architecture.md`
- `C:\Users\Administrator\Desktop\morediva\_bmad-output\implementation-artifacts\stories\*.md`

## 8. 备注

1. 本文有意不把 `docs/logs/` 大量迭代记录全部转成待办，只提炼其中已经明确成为未完成事项的主题。
2. `archive(old-docs-dont-read-me)`、旧 roadmaps、旧 QA checklist 信号很多，但优先级低于当前活跃分支和项目级 TODOLIST。
3. 若要继续精简，下一步应做“去重版 backlog”，把重复出现在多个分支的 pet / memory / sandbox 主题合并成一张表。
