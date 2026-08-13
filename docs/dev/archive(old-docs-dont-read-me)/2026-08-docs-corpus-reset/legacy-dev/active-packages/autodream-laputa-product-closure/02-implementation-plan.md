# 实施方案

## 1. 总体策略

采用“纵向闭环优先”的八个切片。每个切片都必须能独立测试、记录四件套日志并单独提交；不得先堆齐内部模块，最后才接 GUI。

### E0：真实基线与产品承诺校正

- 为现有 AutoDream、提案、typed apply、Recall 建立端到端 characterization。
- 把当前不可执行或仅占位的 GUI 状态明确显示为 `unavailable/degraded`。
- 冻结稳定 DTO 基线和 reason code 命名。
- 输出一个无需真实 API 的 deterministic fake provider E2E。

### E1：Experience Journal

- 在 `agent-diva-core` 定义 `ExperienceEvidence`、`OutcomeKind`、`VerificationState`、`ExperienceBatch`。
- AgentLoop 仅在真实行动后记录结果摘要、工具类别、成功/失败、用户纠正和证据引用；禁止保存 secret、完整工具输出或 Memory 原文。
- 建立确定性 digest、workspace/session 隔离、容量和 retention。
- 增加从现有 session store 的离线回填；默认 dry-run，可回滚。

### E2：可恢复 AutoDream Orchestrator

- 将普通手动、阈值、日/周/月触发统一为一个持久化队列与状态机：
  `queued → gathering → reflecting → validating → publishing → completed`。
- 每个阶段有 checkpoint、attempt、deadline、cancel token 和稳定 reason code。
- 重启后从最后已提交阶段恢复；发布提案采用确定性 key，禁止重复 proposal。
- Manager trigger 返回已入队 run，并由受监督 worker 真正执行。

### E3：Reflection Engine 与候选质量门

- 新增 provider-neutral `ReflectionEngine` trait；生产实现复用现有 provider resolver，测试使用 deterministic fake。
- 输入只包含有界证据摘要、已存在 Memory 的脱敏索引和明确 schema。
- 输出 `MemoryCandidate[]`：类型、内容、证据、置信度、适用 scope、敏感度、预期收益、失效条件。
- `CandidateGate` 拒绝无证据、仅 compaction 证据、重复、矛盾、低价值、越 workspace、越容量和潜在 prompt injection。
- 候选必须选择真实 `ProposalType`，不再固定 `journal_note`。

### E4：提案与统一治理

- 将候选一对一或按同一原子事实合并为 Laputa proposal。
- 接入 GMH-30/31：Plan/Sandbox/Memory 共用 approval coordinator、typed reason code 和事件序列。
- 编辑候选后撤销旧 receipt 并生成新 digest/version。
- 拒绝 digest 写入 bounded suppression，避免相同候选被下一次 run 原样重提；内容显著变化可重新提案。

### E5：typed apply、Recall 与反馈

- 把批准后的 proposal 映射为 canonical `MemoryRecord`，保留 AutoDream run、experience evidence 和治理 correlation。
- apply、changelog、audit、revision、FTS 更新保持单事务或可恢复 journal。
- Recall 记录“候选命中/被选/注入/用户纠正/任务结果”计数，不记录原始 payload。
- 下一轮 AutoDream 可以用这些反馈降权、修订或提出 tombstone/supersede 提案。

### E6：一体化 Evolution Workspace

- 单一页面展示 run 阶段、输入覆盖、候选、提案、治理、写入 revision、Recall 反馈。
- 提供“立即反思”“取消”“重试失败阶段”“批准并应用”“编辑后批准”“拒绝”“回滚”。
- 无模型、provider 失败、typed store 不健康、无可蒸馏证据时给出可操作解释。
- 默认不会自动写 Memory；自动运行可以生成提案，但 apply 始终服从治理策略。

### E7：可靠性、发布与最终人工验收

- 完成 canonical workspace identity、GMH-40 单一副作用 seam、GMH-42 observability、GMH-51 恢复演练。
- 执行真实 provider 的非敏感 smoke（需用户明确授权读取桌面 `keys.txt`）。
- 先通过全自动纵向 E2E，再执行真实桌面六场景及“会话→AutoDream→Recall”第七场景。
- 最终清理不可用旧入口、过期文案、兼容壳和 dead code。

## 2. 主要文件变更地图

| 路径 | 计划变更 |
|---|---|
| `agent-diva-core/src/evolution/types.rs` | run/candidate/evidence/feedback 稳定类型 |
| `agent-diva-core/src/memory/record.rs` | AutoDream provenance 与 supersede/tombstone 契约补强 |
| `agent-diva-agent/src/agent_loop/` | 结果证据采集与单一副作用 seam |
| `agent-diva-autodream/src/worker.rs` | 替换固定占位候选，拆成 orchestrator 阶段 |
| `agent-diva-autodream/src/inputs.rs` | Experience Journal 有界收集、去重、过滤 |
| `agent-diva-autodream/src/outputs.rs` | 确定性 proposal 发布与部分失败恢复 |
| `agent-diva-autodream/src/reflection.rs` | 新增 provider-neutral 反思接口 |
| `agent-diva-autodream/src/candidates.rs` | 新增 schema 与 quality gate |
| `agent-diva-laputa/src/governed_apply.rs` | 统一 coordinator 契约 |
| `agent-diva-laputa/src/typed_store.rs` | feedback/suppression 与恢复 journal |
| `agent-diva-laputa/src/recall.rs` | payload-free 使用反馈 |
| `agent-diva-manager/src/handlers/autodream.rs` | 真正入队/执行/取消/重试/诊断 |
| `agent-diva-manager/src/handlers/laputa.rs` | 统一 reason code 与事件顺序 |
| `agent-diva-gui/src/components/EvolutionView.vue` | 改为纵向闭环 workspace |

## 3. 公共接口草案

```rust
#[async_trait]
pub trait ReflectionEngine: Send + Sync {
    async fn reflect(
        &self,
        input: BoundedReflectionInput,
        cancellation: CancellationToken,
    ) -> Result<ReflectionOutput, ReflectionError>;
}

pub struct MemoryCandidate {
    pub candidate_id: String,
    pub proposal_type: ProposalType,
    pub content: String,
    pub evidence_refs: Vec<EvidenceRef>,
    pub confidence: u8,
    pub scope: MemoryScope,
    pub sensitivity: MemorySensitivity,
    pub expected_value: CandidateValue,
}
```

这只是计划级接口。实现时必须先写契约测试，再冻结字段；不得把 provider response DTO 泄漏到 domain。

## 4. 非目标

- 不恢复 Mentle、legacy Memory fallback 或长期双写。
- 不把 Skill/SOP 可视化编辑器加入本轮。
- 不允许 AutoDream 执行 shell、写源码、直接改 Memory 或自动批准自身提案。
- 不以月报生成等同于自我进化闭环。
- 不追求 GenericAgent 的任意代码自扩展模型。
