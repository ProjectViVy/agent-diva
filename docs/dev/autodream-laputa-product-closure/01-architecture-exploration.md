# AutoDream–Laputa 产品闭环：架构探索

## 1. 目标与结论

本计划的交付目标不是“恢复 Evolution 页面”，而是让用户安装并配置模型后即可获得一条真实、可解释、可治理、可恢复的自我改进链路：

```text
会话与执行结果
  → 证据日志
  → AutoDream 反思与蒸馏
  → 候选验证/去重/风险分类
  → Laputa 提案
  → 人工批准、编辑或拒绝
  → typed Memory 原子写入
  → Recall 在后续任务使用
  → 效果反馈进入下一轮证据
```

当前 Embedded Laputa 已具备 typed SQLite/FTS5、治理申请、receipt、幂等 apply、回滚、完整性检查和 Recall 底座；主要缺口位于链路的前半段和产品组装：AutoDream 还没有真正把会话经验蒸馏为高质量候选，普通手动运行也没有形成可用的后台执行路径。

## 2. GenericAgent 可借鉴的机制

GenericAgent 的核心是“分层记忆 × 极简工具 × 自主执行循环”，其 L0–L4 分层和执行后结晶流程见 [README.md](../../../../.workspace/GenericAgent/README.md:241)。它有四个值得吸收的设计：

1. 每轮保留极短的 working summary，而不是反复注入完整历史，见 [ga.py](../../../../.workspace/GenericAgent/ga.py:541)。
2. 长期记忆只接纳“执行成功且验证过”的事实与经验，见 [ga.py](../../../../.workspace/GenericAgent/ga.py:510)。
3. L1 仅作为存在性索引，详细知识按需读取，见 [memory_cleanup_sop.md](../../../../.workspace/GenericAgent/memory/memory_cleanup_sop.md:3)。
4. 原始会话先压缩、去重、归档，再进入长期历史，且批处理默认 dry-run，见 [compress_session.py](../../../../.workspace/GenericAgent/memory/L4_raw_sessions/compress_session.py:154)。

不能照搬的部分：

- GenericAgent 允许模型直接修改 memory 文件；Agent Diva 必须坚持提案与 receipt 治理。
- GenericAgent 的 SOP 是文件型长期记忆；Agent Diva 已明确产品对象只有 Skill，Memory 记录与 Skill 不混为一体。
- GenericAgent 的反思判断主要由提示词约束；Agent Diva 必须用稳定类型、状态机、schema、幂等键和单一副作用 seam 约束。
- GenericAgent 没有 typed authority、CAS、tombstone 和 rollback 语义，不能作为存储实现模板。

## 3. Agent Diva 当前能力

### 3.1 已闭环的底座

- canonical typed 记录与 provenance：`MemoryRecord`、scope、trust、sensitivity、tombstone，见 [record.rs](../../../agent-diva-core/src/memory/record.rs:103)。
- typed SQLite/FTS5、governed apply journal、备份与完整性，见 [typed_store.rs](../../../agent-diva-laputa/src/typed_store.rs:130)。
- receipt 绑定的 Memory 治理协调器，见 [governed_apply.rs](../../../agent-diva-laputa/src/governed_apply.rs:66)。
- proposal 编辑、状态转换、changelog、audit 与 rollback，见 [proposals.rs](../../../agent-diva-laputa/src/proposals.rs:91)。
- GUI 已有提案 inbox、详情、批准/编辑/拒绝/回滚交互，见 [EvolutionView.vue](../../../agent-diva-gui/src/components/EvolutionView.vue:508)。

### 3.2 尚未形成产品的断点

1. `AutoDreamWorker::propose` 固定产生一个 `journal_note`，正文只是最多三条输入摘录，置信度固定为 60；这不是可用的经验蒸馏，见 [worker.rs](../../../agent-diva-autodream/src/worker.rs:280)。
2. Manager 普通手动触发只创建 run；仅报告型 trigger 会立即执行专用路径，见 [autodream.rs](../../../agent-diva-manager/src/handlers/autodream.rs:12)。
3. 缺少受限 LLM reflection adapter、结构化输出 schema、候选 quality gate 和跨轮效果反馈。
4. proposal type 虽有 14 类稳定枚举，但 AutoDream 没有按内容选择正确类型，也没有把一个反思结果拆成多个独立候选，见 [types.rs](../../../agent-diva-core/src/evolution/types.rs:69)。
5. GUI 能展示治理对象，却没有稳定的运行阶段、进度、输入覆盖、候选拒绝原因和 degraded 状态。
6. G2D 目前只验证提案执行底座，无法证明“会话最终变成可召回记忆”的全链路。

## 4. 目标组件

| 组件 | 责任 | authority |
|---|---|---|
| `ExperienceJournal` | 记录脱敏、可引用的会话结果与验证证据 | Session/event store |
| `AutoDreamOrchestrator` | 调度、恢复、取消、预算与阶段推进 | AutoDream run store |
| `ReflectionEngine` | 通过 provider 生成结构化候选，不执行工具 | 无 authority |
| `CandidateGate` | schema、证据、重复、敏感度、风险与价值校验 | 无 authority |
| `ProposalPublisher` | 确定性地创建 Laputa proposals | Proposal repository |
| `ApprovalCoordinator` | 统一决策、receipt、过期、撤销与消费 | Governance ledger |
| `TypedMemoryWriter` | 唯一生产 Memory 写入 | Embedded Laputa |
| `RecallFeedback` | 记录命中、使用、纠正与效果，不含原文 | Metrics/evidence |
| `EvolutionWorkspace` | 运行、候选、审批、记忆结果的一体化 GUI | 无 authority |

## 5. 关键交互

```text
AgentLoop ──append──> ExperienceJournal
   │                       │
   │ recall                ▼
   └────────────── Typed Laputa <── governed apply <── ApprovalCoordinator
                             ▲                              ▲
                             │                              │
                    ProposalPublisher <── CandidateGate <── ReflectionEngine
                             ▲                              ▲
                             └──────── AutoDreamOrchestrator┘
```

任何降级都必须显式：反思失败不能产生提案；提案失败不能写 Memory；Recall 失败不能静默切换 legacy；人工拒绝不能被后续 run 自动复活为同 digest 候选。
