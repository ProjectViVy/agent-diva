# 关键代码示例

> 以下均为适配当前代码的设计草图，不是从 deep 分支复制的实现，也不是已授权 API。

## 1. Turn 快照

```rust
/// 本轮开始时一次性冻结的决议；不持有完整 Session 或 AgentLoop。
pub(crate) struct TurnSnapshot {
    pub session_key: String,
    pub model: String,
    pub plan_phase: Option<PlanPhase>,
    pub execution_session_id: Option<String>,
    pub trace_id: String,
}
```

`TurnSnapshot` 的目标是让 turn 起点、tool 后与 runtime-control 后使用同一 phase 推导，不是新增持久化状态。

## 2. 现有工具路径的收口 seam

```rust
pub(crate) async fn execute_tool_step(
    registry: &ToolRegistry,
    snapshot: &TurnSnapshot,
    call: ToolCall,
) -> Result<ToolStepOutcome, TurnError> {
    validate_call_against_phase(snapshot.plan_phase.as_ref(), &call)?;
    let result = registry
        .execute(&call.name, call.arguments)
        .await
        .map_err(TurnError::tool)?;
    Ok(ToolStepOutcome::from_result(call, result))
}
```

真实实现必须复用当前 ToolRegistry、Plan/Mask policy 和 Sandbox。示例不表示可以在此重复一份权限判断；`validate_call_against_phase` 应委托现有 policy。

## 3. 薄 Manager handler

```rust
pub async fn chat_handler(
    State(state): State<AppState>,
    Json(request): Json<ChatRequest>,
) -> Result<Json<ChatAccepted>, ApiError> {
    let accepted = state
        .chat_service()
        .start(request)
        .await
        .map_err(ApiError::from)?;
    Ok(Json(accepted))
}
```

Handler 不构造 provider、AgentLoop 或 store，不推导 Plan terminal state。

## 4. GUI domain adapter

```ts
export async function sendChat(input: SendChatInput): Promise<ChatAccepted> {
  assertCapability("chat.send", "MANAGER");
  const payload = await managerTransport.invoke("send_message", input);
  return parseChatAccepted(payload);
}
```

组件只调用 `sendChat`，不直接 `invoke("send_message")`，也不解析 Rust DTO。

## 5. Server-owned projection

```ts
async function approvePlan(input: ApprovalInput) {
  approvalState.value = { kind: "submitting" };
  await planApi.approve(input);
  try {
    planProjection.value = await planApi.getActive(input.sessionKey);
    approvalState.value = { kind: "idle" };
  } catch (error) {
    approvalState.value = {
      kind: "committed_refresh_failed",
      message: toUserMessage(error),
    };
  }
}
```

`committed_refresh_failed` 不应自动重试 approval，因为服务端动作可能已经提交。

## 6. 安全审查

- 示例和后续实现不使用 `unsafe`；
- 非测试代码不新增 `unwrap/expect`；
- 所有外部输入在 transport/domain 边界验证；
- secret 不进入 DTO fixture、日志或 UI state；
- path 和 effectful tool 仍受现有 sandbox/policy 约束。
