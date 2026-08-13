# Laputa 认知工作区 Clean-Break 总 EPIC 编排

- EPIC：`LAPUTA-COGNITIVE-WORKSPACE-RESET`
- 状态：`Epic Defined / R0+R1+R2 Packages Complete / Research Gate Pending / Architecture Design Blocked`
- 记录日期：2026-08-13
- 当前授权：仅冻结产品边界、编排研究与后续设计；**不授权目标架构定稿或代码实施**

## 一句话目标

将当前互相混杂的 Persona、Memory、STM、Evolution、Proposal 和 Approval 拆回四个
用户可理解、权威唯一的产品工作区：

| 工作区 | 唯一职责 | 不再承担 |
| --- | --- | --- |
| Persona | 四份核心人格 Markdown、WORLD、内容变更审查与完整历史 | 普通 Memory、STM、Skill、通用安全审批、JSON 编辑 |
| Memory | BML 长期记忆管理，以及独立的跨会话 STM 入口 | Persona、Evolution、治理提案、文件型长期记忆 |
| Evolution | Skill 的形成、审查、管理与复用；SOP 关系由研究决定 | Persona 沉淀、Memory 提案、旧 AutoDream 梦境流水线 |
| Chat Approval Center | 危险工具执行等真正需要人类授权的运行时审批 | Memory CRUD、Persona 首次初始化、STM 日常维护、Evolution 页面治理投影 |

本 EPIC 是今天全部讨论的总控记录。下列分项决策是其产品约束来源：

- [Evolution / GenericAgent 重置决策](../evolution-genericagent-reset-2026-08/decision-record.md)
- [Persona Markdown Clean Break 决策](../persona-markdown-clean-break-2026-08/decision-record.md)
- [STM 跨会话 Clean Break 决策](../stm-cross-session-clean-break-2026-08/decision-record.md)

若分项记录中存在早期的类型草图、目录建议、API 示意或实施顺序，它们只作为研究输入，
不构成已批准的新架构。产品边界以分项决策为准，阶段授权和开工顺序以本总 EPIC 为准。

## 当前阶段刻意不做什么

本记录不是新架构设计。当前不得提前决定或实现：

- STM 的物理存储、schema、容量算法、自动更新状态机或上下文装配顺序；
- Evolution 的目标领域模型、SOP 与 Skill 的最终关系、生成/验证/晋升状态机；
- Persona revision store、Diff 引擎、API DTO、并发控制和编辑器技术选型；
- clean break 的具体提交切片、删除顺序、数据备份工具和发布流程；
- 为维持旧链路而新增迁移、双读、双写、fallback 或兼容 runtime。

这些内容只有在对应研究包完成并通过 Research Gate 后，才能进入架构设计。架构设计再经
用户评审通过后，才允许建立保护性分支并开始破坏性实施。

## 已冻结的产品决策

以下是需求约束，不再以“待研究”为由重新打开；研究只负责找到正确实现：

1. **Memory CRUD 不走审批。** 增、删、改、查直接作用于 BML Memory 权威；误操作保护由
   精确目标、历史、软删除、撤销或恢复承担。Memory 不创建 Laputa Proposal，不进入
   Evolution 或 Governance Ledger。
2. **BML 是普通长期 Memory 的唯一生产权威。** `MemoryMd` / `memory_md` 文件型长期记忆
   链路完整非兼容删除，不自动导入旧数据。
3. **STM 是跨会话、自动管理、有界的活动上下文。** 它不是长期 Memory，也不是现有
   session-scoped `working_memory` checkpoint；用户入口只属于 Memory 工作区，日常维护
   和用户修正不走审批。
4. **Persona 不等于 Memory。** Identity、Relationship、Commitment、Preferences 四份人格
   与 WORLD 都采用 Markdown 正文；GUI 不再显示或要求用户编辑人格 JSON。
5. **Persona 工作区只有一个中央区域。** 当前文档使用 Markdown 源码/人类可读预览；
   待审变更使用只读 before/after Diff 并明确接受/拒绝；历史只读，载入只覆盖本地草稿，
   显式保存才产生新版本。永久右栏删除。文档型变更可以复用同一套 Diff 交互基础设施，
   但不得因此复活跨领域的通用 Proposal/Governance 状态机。
6. **Persona 内容审查不是安全审批。** 用户自由编辑直接保存；Agent/系统提出的人格修改在
   Persona 中做专用内容审查，不进入聊天 Approval Center，不复用通用 Governance，也不
   拆成 approve/apply 两步。
7. **首次初始化只围绕 Laputa 核心文件。** 仅当四份 Persona 与 WORLD 五份权威全部不存在
   时出现；一次原子直写五份文件及首批历史，不创建 Proposal/Approval/Governance。全存在
   后永久不再出现，部分存在、空或损坏进入修复状态。引导语义分别覆盖 Agent 是谁、用户
   与 Agent 的关系、初见承诺、用户对未来努力方向的偏好，以及用户当前工作/旅行环境。
8. **Persona/WORLD 保存完整心路历程。** 每次成功的真实变化都追加不可变完整快照与文本
   Diff；不自动裁剪，也不把全部历史注入 Prompt。
9. **Evolution 只管理 Skill 演进。** SOP 是特殊 Skill、Skill 前置产物还是独立可复用资产，
   必须以更新后的 GenericAgent 实际设计和验证结果为依据；在研究完成前不实现任何晋升
   状态机。旧 AutoDream → SOP/Skill 链路不再修补。
10. **本轮采用 clean break。** 被删除的旧链路不做文件、数据库、API 或 GUI 兼容；开始
    破坏性代码删除前，必须从验证过的准确提交建立保护性分支。保护分支只用于追溯与恢复，
    不能成为运行时 fallback。

## 工作编排与门禁

```text
今天的产品决策（已冻结）
          |
          v
R0 当前系统/数据依赖盘点
   |---------+----------------+----------------|
   v         v                v                v
R1 Evolution R2 STM/Context   R3 Persona       R4 Clean-break
GenericAgent 分层研究          文档工作区研究    数据安全研究
   |---------+----------------+----------------|
          Research Gate（用户评审）
                    |
                    v
          D0 总体权威与生命周期设计
            /          |           \
          D1           D2           D3
       Persona       Memory/STM   Evolution
            \          |           /
          D4 接口、GUI、删除与验收设计
                    |
          Architecture Gate（用户批准）
                    |
                    v
       保护性分支 -> 分批实施 -> 集成/真机验收
```

### Research：现在要做的研究

#### R0 — 当前系统、数据与耦合全景盘点

这是其他研究包的共同输入，先完成事实核对，不提出目标架构。

- 追踪 Persona、WORLD、BML、`memory_md`、session checkpoint、AutoDream、Evolution、
  Proposal、Governance、Approval 在 Core/Laputa/Manager/Tauri/GUI/CLI/Prompt/测试中的
  真实读写与事件链路；
- 列出生产数据目录、schema、版本、持久化所有权、启动时恢复和失败处理；
- 形成“保留 / 改名 / 删除 / 待决策”符号与数据矩阵；
- 将当前 `object Object`、Evolution 数据加载失败、`governance ledger failed: approval
  request not found` 固化为旧架构失败基线，判断其实际触发链和数据影响；
- 输出可由后续 deletion proof 直接使用的路径、符号、路由、DTO、事件和测试清单。

完成物：`current-state-map.md`、`dependency-and-data-inventory.md`、
`legacy-failure-baseline.md`。

**进展（2026-08-13）：** 研究包已落盘
[`../cognitive-r0-current-state-2026-08/README.md`](../cognitive-r0-current-state-2026-08/README.md)。
三份完成物齐全；引用 R1 Evolution 切片，不重复 Evolution 细表。状态：
`Research package complete / Gate pending user review`。

#### R1 — 最新 GenericAgent Evolution 模型研究

- 研究前先确认 `.workspace/GenericAgent` 的远端、工作区状态和本地未提交内容；安全同步主分支，
  记录准确 upstream commit，不用“最新”作为不可复现依据；
- 逐提交审查近期 Evolution 变化，核对触发、Action-Verified 证据、L0–L4、SOP/Skill 产物、
  发现、复用、编辑、禁用、版本和可见性；
- 用真实任务做最小实验，区分代码宣称、测试覆盖与可观察行为；
- 对比 Diva 现有 Skill runtime、AutoDream、Proposal、Laputa 与 GUI，给出可采纳、不适合
  采纳和必须重新设计的证据；
- 回答 SOP/Skill 关系，但不得在证据不足时制造新的晋升层级。

完成物：`genericagent-upstream-baseline.md`、`evolution-behavior-experiments.md`、
`diva-evolution-gap.md`、`sop-skill-recommendation.md`。

**进展（2026-08-13）：** 研究包已落盘  
[`../cognitive-r1-genericagent-evolution-2026-08/README.md`](../cognitive-r1-genericagent-evolution-2026-08/README.md)。  
本地 GA 锁定 `ee5a474`；远程 tip（API）`f06d550`；P0 静态实验完成；P1 活体因无密钥阻断。  
含 R0 Evolution 切片。全量 R0 见
[`../cognitive-r0-current-state-2026-08/README.md`](../cognitive-r0-current-state-2026-08/README.md)。
状态：`Research package complete / Gate pending user review`。

#### R2 — STM 与上下文分层研究

- 联合盘点 C1–C5 Context Assembly、BML provider、session lifecycle、canonical checkpoint、
  Plan/background task、Skill/SOP、Frozen Core、WORLD、Garden 与 GenericAgent；
- 比较 STM 物理权威、scope、并发合并、自动触发、失败恢复、预算/淘汰、历史/撤销和
  证据指针方案；
- 验证新 session、resume、多 channel、subagent、cron/background job、崩溃恢复与多进程
  一致性的需求；
- 研究 STM 与 BML/Skill 的晋升证据边界，以及 Layer 1 在 stable prefix、per-turn context、
  auto/reactive compact 前后的装配约束。

完成物：`stm-options-and-experiments.md`、`context-assembly-constraints.md`、
`stm-failure-and-concurrency-matrix.md`。

**进展（2026-08-13）：** 研究包已落盘
[`../cognitive-r2-stm-context-2026-08/README.md`](../cognitive-r2-stm-context-2026-08/README.md)。
三份完成物齐全。产品 STM 在代码中不存在；现成三条链路（CanonicalCheckpoint、
SessionCheckpoint/`WorkingMemory`、BmlStartupIndex）已拆名。Research Hold 只给选项，
未选物理权威或装配位置。状态：`Research package complete / Gate pending user review`。

#### R3 — Persona 文档权威、历史与工作区技术研究

- 盘点 Frozen Core、Prompt 投影、section version、WORLD、首次初始化、旧 JSON Proposal 和
  persona retirement 的真实实现；
- 比较 Markdown revision、完整快照、文本 Diff、CAS、stale change request、回滚和
  append-only 历史的存储/计算方案；
- 验证 Markdown 编辑、预览、Diff、历史浏览、草稿恢复、窄屏布局、无障碍与危险 HTML/
  链接处理所需技术能力；
- 明确哪些是可复用的文档编辑基础设施，哪些状态必须保持 Persona 领域专用。

完成物：`persona-authority-inventory.md`、`revision-diff-options.md`、
`markdown-workspace-technical-evaluation.md`。

#### R4 — Clean-break 数据安全、删除和恢复研究

R4 以 R0–R3 的事实为输入，不设计兼容层。

- 确定保护性分支的准确基线、创建时机、命名、验证和恢复演练；
- 分析旧 Persona JSON、`memory_md`、AutoDream/Evolution、Proposal/Governance 数据被删除
  后的用户影响；
- 决定是否只提供一次性的人工导出/备份说明；禁止把导入、迁移或 fallback 带回新 runtime；
- 设计将来需要的符号、路由、数据、GUI 和 Prompt 零残留证明方法；
- 建立破坏性发布说明、失败回滚和真实桌面测试的事实基础。

完成物：`clean-break-impact-report.md`、`protection-branch-protocol.md`、
`deletion-proof-catalog.md`。

### Research Gate：何时允许开始设计

只有同时满足以下条件，才可将状态改为 `Research Complete / Architecture Authorized`：

- R0–R4 的完成物齐全，引用的源码和上游版本可复现；
- 每个关键结论区分“源码事实、实验结果、推断、建议”；
- Persona、Memory、STM、SessionCheckpoint、Evolution、Skill、SOP、Approval 的边界没有
  未声明重叠；
- clean-break 的数据损失面和恢复路径已明确展示给用户；
- 所有未解决问题显式列出，没有由实现者默认选择；
- 用户完成研究评审并授权进入架构设计。

### Design：研究完成后才做的架构设计

以下只是待产出的设计包名称和验收问题，不是当前目标架构：

#### D0 — 总体认知领域与权威图

定义 Persona/WORLD、BML、STM、SessionCheckpoint、Evolution/Skill、Approval 的单一权威、
scope、生命周期、写入者、历史、Prompt/Context 投影和禁止依赖，并以 ADR 冻结跨域不变量。

#### D1 — Persona、WORLD、首次初始化与历史架构

设计 Markdown 权威、revision/Diff、直接保存、专用 change request、Frozen Core、首次原子
初始化、incomplete repair、永久历史，以及对应 Manager/Tauri/GUI 契约。

#### D2 — Memory、BML、STM 与上下文装配架构

设计 BML CRUD、STM 权威和自动维护、SessionCheckpoint 分离、Layer 1 装配、并发/失败恢复、
历史、GUI Memory/STM 信息架构及跨会话一致性。

#### D3 — Evolution、SOP 与 Skill 架构

基于 R1 证据设计 Evolution 的唯一领域模型、触发、验证、可管理性、SOP/Skill 关系、
接受/拒绝或发布语义、Skill runtime 接线和 Evolution GUI；人格与普通 Memory 明确排除。

#### D4 — Clean-break 接口、删除、发布与验收设计

整合 D1–D3，产出 API/事件/错误模型、实施依赖图、精确删除矩阵、保护性分支方案、提交切片、
发布提示、恢复演练、自动测试和真实桌面验收矩阵。只有 D4 通过评审，才可进入实施。

### Architecture Gate：何时允许开始实施

- D0–D4 均通过用户评审并形成 ADR/实施计划；
- 每个域只有一个生产权威，禁止路径和删除目标可机器检查；
- 已决定全部 Research Hold 问题，不存在由编码阶段临时猜测的核心状态机；
- clean-break 的用户数据影响、无兼容政策和恢复方式得到再次确认；
- 已选定验证通过的基线提交，获准创建保护性分支；
- 实施被拆成可独立回滚、独立验证、独立提交的切片。

## 实施阶段的暂定外形（非架构承诺）

架构评审前不领取生产代码范围。通过 Architecture Gate 后，按 D4 确认的依赖切片执行，
原则上包括：

1. 建立并验证保护性分支，保存删除前基线；
2. 先建立新权威与窄接口，再切断旧调用者；
3. 分域完成 Persona、Memory/STM、Evolution 的后端和 GUI；
4. 删除旧数据模型、路由、事件、Prompt、GUI、迁移和 fallback；
5. 运行零残留证明、全仓 gate、真实桌面纵向验收和恢复演练。

具体先后、文件范围和提交数量由 D4 决定，本记录不提前固定。

## 与今天其他工作的关系

- `TODOLIST.md` 的过期记录归档已经完成，是独立的仓库治理结果；本 EPIC 只编排仍有效的
  新产品工作，不把历史快照重新激活。
- 已完成的 C1–C5 Context 工作不重做，但其装配实现是 R2 必须核对的现状输入；任何旧文档
  若把 search → mount 作为当前流程，仍以 C5e 自动激活决策为准。
- M3 三模式危险工具审批和 Chat Approval Center 继续作为独立运行时安全能力；本 EPIC
  只移除 Memory、STM、Persona 与旧 Evolution 对审批/治理的误用，不取消真正的危险操作
  授权。
- 已完成的旧链路恢复提交保留为历史和失败基线。它们不阻止 clean break，也不代表必须
  维护旧 Persona/Evolution/Memory governance 接口兼容。

## 旧故障的处置

今天人工测试发现的三个症状是本 EPIC 的输入证据：

- Persona 主内容显示 `[object Object]`；
- Evolution 数据加载失败且页面内容挤作一团；
- `governance ledger failed: approval request not found`。

这些症状暴露出 JSON/文本边界、跨域 proposal/governance 耦合和旧 Evolution 数据模型问题。
除非它们阻断数据导出、研究取证或危险工具审批，本 EPIC 不再沿旧 Persona/Evolution/
Memory governance 架构做临时修补。最终验收必须证明：

- Persona 正文全链路为 Markdown，不可能把对象隐式渲染成字符串；
- Evolution 不加载或投影 Persona/Memory/旧 AutoDream proposal；
- Memory CRUD、STM、Persona 初始化与直接保存不查询 Governance Ledger；
- Chat Approval Center 的危险工具授权仍独立可用，不能因删除领域治理而退化。

## EPIC 最终完成定义

只有满足以下条件才可关闭 `LAPUTA-COGNITIVE-WORKSPACE-RESET`：

- 所有研究包、设计包和用户评审门禁有完整记录；
- Persona、Memory/STM、Evolution、Chat Approval 四个工作区职责与代码依赖一致；
- 旧 Persona JSON、`memory_md`、AutoDream Evolution、Memory governance 残留通过自动扫描
  和运行时纵向测试证明为零；
- Persona 初始化/编辑/Diff/历史、BML CRUD、跨会话 STM、Skill Evolution 管理与危险工具
  审批均通过自动测试和真实桌面测试；
- 保护性分支、破坏性发布说明和恢复演练可用；
- `TODOLIST.md` 中只关闭有证据通过的子项，未通过项保留具体失败证据。
