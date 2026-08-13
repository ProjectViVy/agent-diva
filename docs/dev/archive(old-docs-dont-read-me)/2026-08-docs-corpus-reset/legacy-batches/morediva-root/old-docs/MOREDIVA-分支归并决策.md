# morediva 分支归并决策

更新日期：2026-06-05
工作目录：C:\Users\Administrator\Desktop\morediva
适用范围：`agent-diva`、`agent-diva-pro`、`agent-diva-selfinprove`、`agent-diva-sandbox`、`agent-diva-vrm-memory-test`
决策目标：降低并行分支混乱度，明确各分支职责、归并方式与后续迁移规则。

## 1. 结论摘要

本轮结论只有一句话：

不是“整支合并”，而是“按内容分流归并”。

具体决策：
1. `agent-diva` 继续作为主线稳定分支，主要承接后端增强、基础设施、安全修复、bugfix。
2. `agent-diva-pro` 继续作为产品功能增强分支，主要承接 GUI、交互、工作流、体验型能力。
3. `agent-diva-selfinprove` 重新定义为“研究/架构孵化线”，不再视为长期并行实现主线；其价值主要通过文档结论、Epic、定向 cherry-pick 向 `main` / `pro` 输送。
4. `agent-diva-sandbox` 重新定义为“历史整合/参考线”，原则上停止继续承载新功能开发，只保留已存在资产供选择性抽取。
5. `agent-diva-vrm-memory-test` 保留为专项参考线，用于 VRM / memory 相关定向取材，不作为当前主归并目标。

## 2. 本次判断依据

### 2.1 当前分支事实快照

| 仓库 | 当前分支 | 相对 `origin/main` | 结论 |
|---|---|---:|---|
| `agent-diva` | `main` | 0 behind / 0 ahead | 当前就是稳定基线 |
| `agent-diva-pro` | `feature/context-compaction` | 11 behind / 54 ahead | 已承载大量功能与产品化实现 |
| `agent-diva-selfinprove` | `autoresearch/agent-diva-autodream-rhythm-session-history-evid-20260531` | 11 behind / 57 ahead | 已偏向研究/设计母本 |
| `agent-diva-sandbox` | `agent-diva-with-sandbox` | 0 behind / 1 ahead | 代码价值大头已被后续整合吸收 |
| `agent-diva-vrm-memory-test` | `vrm-memory-test` | 本轮未作为主归并基线 | 作为 VRM / memory 参考线保留 |

### 2.2 `pro` 与 `selfinprove` 的关键对比

相对 `origin/main` 的变更集合统计：
- `agent-diva-pro` changed files：738
- `agent-diva-selfinprove` changed files：571
- 二者共享 changed files：485
- 仅 `pro` 独有：253
- 仅 `selfinprove` 独有：86

更关键的结构差异：
- `selfinprove` 独有 86 个文件全部为 `.md`
- `selfinprove` 独有 `.rs/.vue/.ts` 数量：0
- `pro` 独有内容中包含：25 个 `.rs`、39 个 `.vue`、6 个 `.ts`

这说明：
- `selfinprove` 的独有价值主要是研究、架构、设计规格、日志沉淀
- `pro` 的独有价值主要是功能实现、GUI、产品化代码、整合产物

因此不适合把 `selfinprove` 当成“另一个实现主线”整支合并；更适合把它当成“研究母本”，把结论按目标分支拆出。

### 2.3 `selfinprove` 当前工作树状态

本轮看到的未提交内容主要仍是研究文档：
- 修改：
  - `docs/dev/genericagent/README.md`
  - `docs/dev/genericagent/autodream-rhythm-distillation-design.md`
  - `docs/dev/genericagent/context-compaction-research.md`
- 未跟踪：
  - `docs/dev/genericagent/candidate-evidence-journal-audit-design.md`
  - `docs/dev/genericagent/shared-memory-rendering-research.md`
  - `docs/logs/2026-05-shared-memory-rendering/*`
  - `autoresearch.sh`
  - `autoresearch_validate.py`

这进一步证明 `selfinprove` 当前仍在“研究迭代态”，而不是“待直接并入产品线的稳定功能态”。

## 3. 各分支重新定位

### 3.1 `agent-diva`（主线）
定位：稳定主干 / 后端主干 / 基础设施主干。

只建议承接：
- bugfix
- 安全与稳定性修复
- provider / manager / core / tooling 增强
- MemoryProvider / MemoryManager 边界内的基础能力
- 可无 GUI 独立落地的后端协议与数据结构

不建议主线先承接：
- 重产品形态的 Journal / Inbox / Review UI
- 实验性自进化工作流全量 UI
- 大量 still-draft 的设计文档全集

### 3.2 `agent-diva-pro`
定位：功能增强主分支 / GUI 与产品工作流承接面。

优先承接：
- GUI / UX / 交互功能
- 节律系统的用户可见工作流
- Journal / Inbox / ReviewCard / EvolutionProposalCard / EvidencePeekCard
- Self-evolution settings
- Thinking / multimodal / pet / advanced workflow 等产品增强

### 3.3 `agent-diva-selfinprove`
定位：研究与架构孵化线，不再视为长期产品实现主线。

保留职责：
- 架构研究
- 设计规格
- 决策文档
- 未来 feature spec
- 实施前的母本文档与结论沉淀

不再承担：
- 长期并行的产品落地职责
- 与 `pro` 并列的功能交付职责
- 大范围直接 merge 进入主线的来源角色

### 3.4 `agent-diva-sandbox`
定位：历史整合参考线 / 安全与 sandbox 资产来源。

建议处理：
- 停止继续承载新功能开发
- 保留现有 crate / docs 供定向抽取
- 已被 `pro` 吸收的能力不再回流重复开发

## 4. selfinprove 这条线“主要是什么”

一句话定义：

`selfinprove` = Agent-Diva 的“自进化 / 记忆 / 节律系统研究线”，负责定义 GenericAgent × Laputa × Mentle × AutoDream 的架构、协议、候选审查与产品形态。

它当前主要由 5 组主题构成：

1. 记忆分层与主体连续性
   - `newedge/architecture.md`
   - `shared-memory-rendering-research.md`
   - `candidate-evidence-journal-audit-design.md`
   - `mentle-laputa-memory-role-decision.md`

2. 节律系统 / AutoDream
   - `autodream-migration-research.md`
   - `autodream-rhythm-distillation-design.md`
   - `compression-research.md`
   - `compression-taxonomy-decision.md`
   - `autonomous-evolution-simplified-architecture-decision.md`

3. SOP / Skill 候选沉淀
   - `newedge/architecture.md` 中的 L3 `sop/*.md`
   - `genericagent-mentle-user-controlled-learning` 总结
   - AutoDream 输出的 `LearningCandidate` / `SOP or Skill candidate` 设计

4. Journal / Inbox / Review 产品面
   - `newedge/ui-design.md`
   - `newedge/agent-diva-pro-self-evolution-ui-research.md`

5. Context compaction 独立线
   - `context-compaction-research.md`
   - 注意：这条线与 AutoDream / rhythm 明确分离，不能混成同一个机制

## 5. “节律系统”和“skill总结”在本项目中的统一定义

为避免后续继续混名，统一采用以下定义：

1. 节律系统（Rhythm / AutoDream）
   = 一个跨 session 的周期性反思与候选生成框架。
   输出包括：
   - `memory_patch_candidate`
   - `learning_candidate`
   - `journal_entry`
   - `evidence_refs`
   - `review_required`

2. skill总结
   = 节律系统产出的 SOP / Skill candidate 审批与沉淀流程。
   不是一个孤立子系统，而是 AutoDream → Candidate → Review → Apply 链路中的 L3 沉淀环节。

结论：
“节律系统”和“skill总结”最终都应落入 `agent-diva-pro` 的产品工作流，但其底层协议、候选模型、审计链、最小后端 contract 可以先回流 `agent-diva` 主线。

## 6. 归并原则（最终版）

### 6.1 允许整合的方式

允许：
- 按主题拆 Epic 后迁移
- 按文件级 cherry-pick
- 按文档结论重新实现
- 先主线后产品面的分层落地

不允许：
- 直接整支 merge `agent-diva-selfinprove` → `agent-diva-pro`
- 直接整支 merge `agent-diva-selfinprove` → `agent-diva`
- 把研究日志、历史阶段文档、产品实现混成一条不可分辨的主线

### 6.2 main / pro 分流规则

进入 `agent-diva` 主线的内容：
- Shared MEMORY rendering MVP
- Candidate / Evidence / Audit 最小协议层
- AutoDream trigger / lock / checkpoint / worker backend skeleton
- Context compaction 的底层 runtime / budget / safety 能力
- 其他无 GUI 依赖的后端与治理能力

进入 `agent-diva-pro` 的内容：
- JournalView / InboxView / Chat card 系统
- Review / approval / apply / rollback 的用户工作流
- Self-evolution settings
- Rhythm run status / health visibility
- SOP / Skill candidate 的产品化承载面
- 需要明确产品交互与导航的信息架构

继续留在 `selfinprove` 的内容：
- 长篇调研原文
- 比较稿 / 决策稿 / 迭代日志
- 未来阶段性 spec
- 未进入实施排期的架构探索

## 7. 建议的第一批迁移主题

### 7.1 先迁入 `agent-diva` 主线
1. Shared MEMORY.md 分层渲染 MVP
2. Candidate / Evidence / Audit 最小 schema 与状态机骨架
3. AutoDream backend skeleton：manual trigger、lock、checkpoint、run status
4. Context compaction 的底层能力与 guardrail

### 7.2 先迁入 `agent-diva-pro`
1. Journal / Inbox / Review 的信息架构
2. Self-evolution MVP UI
3. Rhythm 工作流的产品化入口
4. Skill / SOP candidate 审批与沉淀工作流

### 7.3 暂不迁移，只保留为母本
1. 大部分 `docs/logs/2026-05-*` 调研日志全集
2. `sandbox` 中仅作为历史参考的研究资产
3. 尚未收口为明确实现边界的探索性设计

## 8. 立即执行建议

建议按以下顺序推进：

1. 在 `morediva` 根目录保留本决策文档，作为后续归并的统一规则。
2. 基于 `selfinprove` 产出两组待办：
   - `main` 组：后端协议 / runtime / memory 基础能力
   - `pro` 组：Journal / Inbox / Review / Self-evolution 产品面
3. 给 `selfinprove` 打上“研究母本”标签，不再把它当成长期功能实现主线。
4. 给 `sandbox` 打上“历史参考线”标签，不再继续扩写新功能。
5. 后续所有迁移，优先按“主题卡片 / Epic / 文件级 cherry-pick”进行，不使用整支 merge。

## 9. 本文档的执行性结论

最终执行结论：
- `agent-diva`：稳态主干，继续收后端增强与修复。
- `agent-diva-pro`：功能主承接面，继续收产品与交互增强。
- `agent-diva-selfinprove`：研究孵化线，仅做结论输出，不做整支合并。
- `agent-diva-sandbox`：历史参考线，冻结新增主功能职责。
- `agent-diva-vrm-memory-test`：专项参考线，按需抽取。

后续若与本决策冲突，以“功能归 `pro`、基础设施归 `main`、研究留 `selfinprove`、历史归档进 `sandbox/archive`”为最高优先级规则。
