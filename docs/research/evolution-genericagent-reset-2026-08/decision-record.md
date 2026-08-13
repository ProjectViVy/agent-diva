# Evolution 非兼容重置：讨论决策记录

- 状态：`Research Hold`
- 记录日期：2026-08-13
- 性质：产品边界决策与后续调研任务；不是实施规格

## 背景判断

当前 Evolution 将 AutoDream 运行、Memory 候选、Laputa 提案、人格沉淀、
Governance Ledger、Recall 反馈、审计与回滚集中在同一产品表面。真实桌面测试表明，
这条链路难以理解、难以管理，也容易把记忆、人格和能力进化误认为同一件事。

继续修补现有页面不能解决问题。问题来自领域对象和权威边界，而不是局部排版。

## 已确认决策

### D1：记忆管理完全退出审批体系

- 人格不等于记忆。
- Memory 的增、删、改、查均直接作用于 Memory 权威，不创建 Laputa Proposal，
  不进入 Evolution，也不进入 Governance Ledger。
- 后续实施需要删除 Memory 专属审批、提案优先写入、治理投影、兼容映射和 legacy
  fallback；不保留双轨兼容。
- Memory 的误删保护应由精确目标、软删除、历史版本、撤销或恢复承担，而不是审批。
- 聊天页右侧统一 Approval Center 继续处理危险工具执行等即时运行时授权；它不是
  Memory 生命周期的一部分。

### D2：现有 AutoDream–Evolution 产品链路退役

- 现有“AutoDream 生成 SOP/Skill/Memory 候选，再进入通用 Laputa 提案与治理”的
  链路经真实测试后判定为不可靠且不可管理。
- 该链路与 Laputa 人格沉淀发生概念和界面混淆，后续应非兼容删除，而不是迁移成
  第二套状态机或保留 legacy DTO。
- 当前 Evolution Inbox、Runs、Audit、Policy、Memory health、Recall feedback、
  通用 proposal actions 等实现不得继续被当作新产品设计的基础。

### D3：Evolution 以“可管理性”为核心全面重构

- Evolution 必须重新定义单一业务对象、权威来源和用户任务，再设计页面。
- 新方向以 GenericAgent 的自进化模型为主要参考，但不能照搬当前本地旧快照。
- 在研究完成前，不冻结 SOP、Skill、SOP Candidate、晋升或审批状态机；此前讨论的
  “自动 SOP → 用户选择性固化 Skill”只保留为待验证假设，不视为最终决定。
- Evolution 不再承担普通 Memory 管理，也不得承载 Laputa 人格提案。

### D4：破坏性实施前必须建立保护性分支

- 删除旧 AutoDream–Evolution 链路前，先从删除前的已验证提交建立明确命名的保护性
  分支，保留代码、测试和历史行为供差异核查与必要回看。
- 保护性分支只用于保存旧实现，不作为兼容运行时、fallback 或长期双轨维护来源。
- 当前仅记录该前置条件；本次讨论记录不创建分支、不删除代码。

### D5：AutoDream 必须整理 STM，人格只允许提案；旧实现未通过；先测再接线

- AutoDream 的产品职责是批处理反射：**整理 STM（直写，不走提案）**，并对
  P19 允许的人格文件**生成待审提案**。不是 Evolution 收件箱，也不是 BML /
  Skill 流水线。能改 / 不能改以 Persona **P19 表**和 STM **S5** 为准。
- **未通过**的是当前实现（JSON 提案、默认 MemoryPatch、crate 内无 STM、
  人格 type 未按允许表接线、日志几乎没有），不是「不许对 Laputa 提案」。
- 2026-08-14 已做独立测试：`cargo test -p agent-diva-autodream` 与
  `agent-diva-manager` `autodream_laputa_e2e` 现有套件全绿。测到的是旧合同；
  STM 整理测不到。表征测试见 `agent-diva-autodream/tests/current_contract.rs`。
- 独立诊断轨仍然成立：端到端结构化日志 + 对着产品表的测试。不得在诊断里恢复
  `MemoryPatch`、`SopCreate` 或对 `REDLINE` / `DREAM` / 用户偏好的写入，也不得
  把 STM 整理改成提案。
- D2 仍然成立：旧 AutoDream→通用治理主链退役。新人格提案必须是 Persona 内容
  审查，不是 Governance Ledger。

## GenericAgent 当前核查事实

- 本地参考仓库位于上层 `.workspace/GenericAgent`。
- 本地 `main` 在核查时停于 `ee5a474e5a1b6d203438d7d1dfa21da912268bb8`
  （2026-07-10），已落后于远端。
- 2026-08-13 只读 `ls-remote` 观测到远端 `main` 为
  `63f9db74e63fef54950ed7f6f43e43295fb6b36b`。正式研究开始时必须重新确认，
  不能把本记录中的 SHA 当作永久最新版本。
- 已知 GenericAgent 基线通过 L0–L4、行动验证、任务经验沉淀和按需读取形成能力；
  但其 README 中的 “Skill” 叙事与实际 `memory/*_sop.md` 产物并非严格领域模型。

## 后续专项调研必须回答

1. 更新后的 GenericAgent `main` 相对本地旧快照，在自进化触发、经验筛选、SOP/Skill
   产物、索引、加载、更新、删除、冲突与可观测性方面发生了什么变化？
2. GenericAgent 哪些行为是稳定设计，哪些只是依赖模型提示词和文件命名的偶然实现？
3. 自动沉淀如何证明 Action-Verified，如何避免把失败路径、临时状态、敏感内容和
   项目特例写成长期能力？
4. 同一能力如何去重、合并、修订、降级、删除和恢复？外部修改后如何重新发现？
5. “SOP”与“Skill”在 Diva 中究竟是一个对象、两个阶段，还是仅是内容与载体的关系？
6. 用户所需的“可管理性”包含哪些真实任务：查看、搜索、启停、编辑、验证、删除、
   来源追踪、版本差异、冲突处理或重新测试？
7. 哪些动作无需审批，哪些只是执行时通过统一 Approval Center 做风险授权？
8. 新模型如何与 Laputa 人格权威保持物理和类型上的隔离？
9. 如何迁出或删除旧 AutoDream、proposal、governance、Manager/Tauri/GUI DTO、持久化
   数据和测试，且不留下兼容残骸？
10. 用哪些纵向真实任务、重复任务、失败任务、恶意内容和重启场景证明新模型可靠？

## 调研与验证要求

- 先重新确认并更新 GenericAgent 主分支参考，再做提交级差异和生产代码阅读。
- 同时审查 Diva 当前所有 AutoDream、Laputa proposal、Memory governance、Skill loader、
  Manager/Tauri API、GUI 和持久化依赖，形成删除清单与权威图。
- 研究结论必须包含失败模型、数据迁移/删除策略、测试矩阵、桌面验收和删除证明。
- 不能仅凭 README、概念图或单次成功 demo 决定架构。
- 任何实施必须分为保护性分支、旧链删除、新模型最小纵向闭环、管理 UI 和真机验证，
  每阶段独立验收。

## 当前明确不做

- 不继续修补旧 Evolution UI。
- 不新增旧 proposal 类型或 Governance 状态。
- 不把普通 Memory CRUD 接回审批。
- 不提前实现 SOP Candidate → Skill 晋升模型。
- 不在调研完成前确定新 DTO、数据库 schema、页面布局或兼容迁移。

## 被取代的当前依据

以下内容保留为历史证据，但不再作为当前 Evolution 实施依据：

- `docs/dev/autodream-laputa-product-closure/`
- 归档批次 `architecture/legacy/evo-diva-architecture-2026-06-12.md`
- 归档批次 `architecture/legacy/autodream-architecture-2026-06-12.md`
- 归档批次 `architecture/legacy/skill-sop-unification.md` 中关于 SOP/Skill 最终关系的旧结论
- `TODOLIST.md` 中旧 AutoDream–Laputa–Memory 纵向闭环及其 G2D+ 验收路径

这些记录不得直接删除；后续专项调研应逐项标注“保留、改写或删除”的处置结果。
