# 关键代码示例

以下为架构草案，不是可直接复制的实现。

## 1. 状态机

```rust
pub enum AutoDreamStage {
    Queued,
    Gathering,
    Reflecting,
    Validating,
    Publishing,
    Completed,
    Failed,
    Cancelled,
}
```

transition 必须由 domain policy 验证，不能由 handler 直接赋值。

## 2. 确定性候选

```rust
pub fn candidate_id(
    workspace_id: &str,
    proposal_type: ProposalType,
    normalized_content: &str,
    evidence_digests: &[ContentDigest],
) -> CandidateId {
    // 使用稳定 canonical serialization 后计算 digest。
    // 不包含时间戳、provider request ID 等非确定字段。
}
```

## 3. 受治理发布

```rust
let candidates = reflection_engine.reflect(input, cancellation).await?;
let accepted = candidate_gate.validate(candidates, memory_index).await?;
let proposals = proposal_publisher.publish_once(run_id, accepted).await?;
```

`publish_once` 只创建 proposals，不写 typed Memory；apply 仍必须消费有效 receipt。

## 4. Payload-free feedback

```rust
pub struct RecallFeedbackEvent {
    pub record_id: String,
    pub turn_id: String,
    pub selected: bool,
    pub injected: bool,
    pub outcome: RecallOutcome,
    pub latency_ms: u64,
    pub estimated_tokens: u32,
}
```

禁止加入 content、prompt、tool output 或 secret 字段。
