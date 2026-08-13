# Agent-Diva Hooks Architecture

> Generated: 2026-07-03  
> Scope: architecture and pseudocode only; no Rust implementation in this cycle  
> Inputs: `workspace-hooks-comparison.md`, current `agent-diva` source layout, attached implementation plan

## 1. Executive Summary

Agent-Diva should adopt a **typed in-process hook runtime** as the first hooks architecture. The runtime lives in `agent-diva-core`, is explicitly invoked by `agent-diva-agent` and later `agent-diva-manager`, and keeps `MessageBus` as observe-only infrastructure. The first version does not execute shell, HTTP, TOML, prompt, or agent hooks; those remain adapters on top of the core contract after the Rust lifecycle is stable.

The architecture has three layers:

1. **Core hook contract** in `agent-diva-core/src/hooks/`: typed contexts, `HookFlow<T>`, `HookHandler`, `HookRegistry`, ordering, failure policy, and introspection metadata.
2. **Runtime call sites** in `agent-diva-agent`: lifecycle code explicitly runs context, LLM, tool, session hooks at known points in the agent loop.
3. **Domain handlers** in owning crates: planning, future skill/security/observability hooks register typed Rust handlers without weakening memory or bus boundaries.

This is intentionally narrower than Claude Code's broad 27-event surface. It follows the research recommendation: Rust typed reducer semantics from zeroclaw/pi, a single dispatcher shape from Hermes, and Agent-Diva's existing file-hook pipeline style.

## 2. Binding Architecture Decisions

### AD-1: Hooks Are Interceptors, Bus Events Are Observers

`HookRegistry` is the only component allowed to block or transform runtime payloads. `MessageBus`, `AgentEvent`, and `PokeEvent` remain observe-only pub-sub channels.

Prevents:

- GUI, audit, presence, or streaming subscribers accidentally mutating model/tool behavior.
- Hidden execution order dependencies between broadcast subscribers.
- Reusing `MessageBus` as both telemetry bus and policy engine.

### AD-2: MemoryProvider Stays Outside Generic Hooks

`MemoryProvider` keeps its domain lifecycle:

- `system_prompt_block`
- `prefetch`
- `sync_turn`
- `on_session_end`

Generic hooks may observe messages around those calls, but must not replace this provider boundary. Memory writes, recall, and shutdown rhythm remain provider-owned contracts.

Prevents:

- Mixing backend-specific memory semantics into global hooks.
- Letting arbitrary hooks compensate for missed memory writes.
- Breaking the existing Markdown/Laputa/Mentle fallback model.

### AD-3: MVP Supports Rust In-Process Handlers Only

The first architecture accepts `Arc<dyn HookHandler>` registrations. External handlers are deferred:

- P1: synthetic `hooks test`
- P2: config/TOML discovery
- P2: command/shell adapter
- P2: HTTP/prompt/agent adapters

Prevents:

- Introducing trust, timeout, stdout JSON, sandbox, and allowlist decisions before core lifecycle semantics are stable.
- Coupling Agent-Diva to Claude/Codex naming before its own Rust events are coherent.

### AD-4: Safety Guards Run Before User Hooks

Read-only reviewer mode, Plan mode tool restrictions, cron recursion protection, and sandbox/security checks remain non-bypassable. `pre_tool_call` runs only after built-in policy has decided the tool is eligible to execute.

Prevents:

- A hook rewriting arguments to evade Plan mode.
- A hook reviving tools disabled by read-only mode.
- A hook bypassing cron recursion prevention.

### AD-5: Tool Name Is Immutable in MVP

`pre_tool_call` may block or transform JSON arguments, but it cannot change `tool_name`.

Prevents:

- Confusing tool audit trails.
- Breaking policy decisions that already resolved against the original tool name.
- Introducing name aliasing before the registry supports formal matcher semantics.

## 3. Target Module Layout

```text
agent-diva-core/src/hooks/
  mod.rs
  types.rs
  handler.rs
  registry.rs

agent-diva-agent/src/planning/
  hook_handler.rs
```

`agent-diva-core/src/lib.rs` should eventually export:

```rust
pub mod hooks;
```

The hook runtime belongs in `agent-diva-core` because it defines cross-crate contracts and should not depend on provider, tool, manager, or channel crates.

## 4. Core Type Model

### 4.1 Flow and Failure Policy

```rust
pub enum HookFlow<T> {
    Continue(T),
    Block { reason: String },
}

impl<T> HookFlow<T> {
    pub fn map<U>(self, f: impl FnOnce(T) -> U) -> HookFlow<U> {
        match self {
            HookFlow::Continue(value) => HookFlow::Continue(f(value)),
            HookFlow::Block { reason } => HookFlow::Block { reason },
        }
    }
}

pub enum HookFailurePolicy {
    WarnAndContinue,
    BlockOnError,
}
```

`WarnAndContinue` is the default. It preserves Agent-Diva's current "agent loop should keep running when optional infrastructure fails" posture. `BlockOnError` is reserved for intentionally strict policy hooks, such as future enterprise security handlers.

### 4.2 Registration Metadata

```rust
pub struct HookRegistration {
    pub id: String,
    pub priority: i32,
    pub failure_policy: HookFailurePolicy,
    pub events: Vec<HookEventKind>,
}

pub enum HookEventKind {
    SessionStart,
    SessionEnd,
    TransformContext,
    PreLlmCall,
    PostLlmCall,
    PreToolCall,
    PostToolCall,
    TransformToolResult,
}
```

Registration rules:

- `id` must be stable and human-readable, for example `planning.context`.
- Higher `priority` runs earlier.
- Equal priority keeps insertion order.
- `events` powers `hooks list` introspection.

### 4.3 Context Types

Context types should be Agent-Diva-owned domain structs. They must not expose provider SDK structs, MCP schemas, HTTP route structs, or backend-specific memory types.

```rust
pub struct HookRunContext {
    pub trace_id: String,
    pub session_key: String,
    pub channel: String,
    pub chat_id: String,
    pub workspace_root: PathBuf,
}

pub struct SessionStartContext {
    pub run: HookRunContext,
    pub user_message_preview: String,
}

pub struct SessionEndContext {
    pub run: HookRunContext,
    pub reason: SessionEndReason,
}

pub enum SessionEndReason {
    AgentLoopShutdown,
    SessionReset,
    ExplicitStop,
}
```

LLM context:

```rust
pub struct ContextTransform {
    pub run: HookRunContext,
    pub messages: Vec<agent_diva_providers::Message>,
}

pub struct LlmCallContext {
    pub run: HookRunContext,
    pub model: String,
    pub message_count: usize,
    pub tool_names: Vec<String>,
}

pub struct LlmResultContext {
    pub run: HookRunContext,
    pub finish_reason: String,
    pub has_tool_calls: bool,
    pub token_usage: Option<TokenUsage>,
}
```

Tool context:

```rust
pub struct ToolCallContext {
    pub run: HookRunContext,
    pub tool_call_id: String,
    pub tool_name: String,
    pub arguments: serde_json::Value,
}

pub struct ToolResultContext {
    pub run: HookRunContext,
    pub tool_call_id: String,
    pub tool_name: String,
    pub is_error: bool,
    pub result_preview: String,
}

pub struct ToolResultTransform {
    pub run: HookRunContext,
    pub tool_call_id: String,
    pub tool_name: String,
    pub result: String,
    pub is_error: bool,
}
```

The contexts include enough identity for logs and tests, but avoid passing mutable references into `AgentLoop`. Hooks receive owned payloads when they can transform and borrowed payloads when they observe.

## 5. HookHandler Contract

```rust
#[async_trait::async_trait]
pub trait HookHandler: Send + Sync {
    fn registration(&self) -> HookRegistration;

    async fn on_session_start(&self, _ctx: &SessionStartContext) -> crate::Result<()> {
        Ok(())
    }

    async fn on_session_end(&self, _ctx: &SessionEndContext) -> crate::Result<()> {
        Ok(())
    }

    async fn transform_context(
        &self,
        ctx: ContextTransform,
    ) -> crate::Result<HookFlow<ContextTransform>> {
        Ok(HookFlow::Continue(ctx))
    }

    async fn pre_llm_call(&self, _ctx: &LlmCallContext) -> crate::Result<()> {
        Ok(())
    }

    async fn post_llm_call(&self, _ctx: &LlmResultContext) -> crate::Result<()> {
        Ok(())
    }

    async fn pre_tool_call(
        &self,
        ctx: ToolCallContext,
    ) -> crate::Result<HookFlow<ToolCallContext>> {
        Ok(HookFlow::Continue(ctx))
    }

    async fn post_tool_call(&self, _ctx: &ToolResultContext) -> crate::Result<()> {
        Ok(())
    }

    async fn transform_tool_result(
        &self,
        ctx: ToolResultTransform,
    ) -> crate::Result<HookFlow<ToolResultTransform>> {
        Ok(HookFlow::Continue(ctx))
    }
}
```

The trait uses no-op defaults so each domain can implement only its events. This avoids large handler boilerplate while preserving a single registry surface.

## 6. Registry Design

### 6.1 Storage

```rust
pub struct HookRegistry {
    handlers: Arc<RwLock<Vec<RegisteredHook>>>,
}

struct RegisteredHook {
    order: u64,
    registration: HookRegistration,
    handler: Arc<dyn HookHandler>,
}
```

Registration sorts by `(priority desc, order asc)`. The registry can either sort at registration time or sort lazily when building an execution snapshot. A snapshot is preferred for async runtime safety:

```rust
impl HookRegistry {
    pub async fn register(&self, handler: Arc<dyn HookHandler>) {
        let registration = handler.registration();
        let mut handlers = self.handlers.write().await;
        let order = handlers.len() as u64;
        handlers.push(RegisteredHook { order, registration, handler });
    }

    async fn snapshot(&self) -> Vec<RegisteredHookSnapshot> {
        let mut snapshot = self.handlers.read().await.clone();
        snapshot.sort_by(|a, b| {
            b.registration.priority
                .cmp(&a.registration.priority)
                .then_with(|| a.order.cmp(&b.order))
        });
        snapshot
    }
}
```

Implementation note: if cloning `RegisteredHook` is awkward, store only `Arc` fields in the snapshot.

### 6.2 Observe Dispatch

Observe hooks cannot block the agent loop. Errors are logged and execution continues.

```rust
pub async fn run_pre_llm_call(&self, ctx: &LlmCallContext) {
    for hook in self.snapshot().await {
        if !hook.supports(HookEventKind::PreLlmCall) {
            continue;
        }

        if let Err(error) = hook.handler.pre_llm_call(ctx).await {
            tracing::warn!(
                hook_id = %hook.registration.id,
                error = %error,
                "pre_llm_call hook failed"
            );
        }
    }
}
```

### 6.3 Transform Dispatch

Transform hooks form a serial reducer. The output from one handler becomes the input for the next handler.

```rust
pub async fn run_transform_context(
    &self,
    initial: ContextTransform,
) -> HookFlow<ContextTransform> {
    let mut current = initial;

    for hook in self.snapshot().await {
        if !hook.supports(HookEventKind::TransformContext) {
            continue;
        }

        match hook.handler.transform_context(current).await {
            Ok(HookFlow::Continue(next)) => current = next,
            Ok(HookFlow::Block { reason }) => return HookFlow::Block { reason },
            Err(error) => match hook.registration.failure_policy {
                HookFailurePolicy::WarnAndContinue => {
                    tracing::warn!(hook_id = %hook.registration.id, error = %error);
                    current = recover_previous_payload();
                }
                HookFailurePolicy::BlockOnError => {
                    return HookFlow::Block {
                        reason: format!("hook '{}' failed: {}", hook.registration.id, error),
                    };
                }
            },
        }
    }

    HookFlow::Continue(current)
}
```

Real Rust code should avoid `recover_previous_payload()` by matching on a closure that borrows `current` or by storing `previous` before moving it. The design rule is: `WarnAndContinue` keeps the last known good payload.

### 6.4 Tool Block Dispatch

`pre_tool_call` block turns into a tool result error, not a full turn failure.

```rust
match hook_registry.run_pre_tool_call(tool_ctx).await {
    HookFlow::Continue(tool_ctx) => {
        let result = tools.execute(&tool_ctx.tool_name, tool_ctx.arguments).await;
        // normal post_tool_call + transform_tool_result
    }
    HookFlow::Block { reason } => {
        let result = format!("Error: tool '{}' blocked by hook: {}", tool_name, reason);
        let is_error = true;
        // add tool result so the model can recover
    }
}
```

This mirrors the current loop style where policy failures become model-visible tool errors and later tools can still run.

## 7. AgentLoop Integration

### 7.1 Struct and Constructors

`AgentLoop` gains:

```rust
hook_registry: Arc<HookRegistry>,
```

Constructor compatibility:

```rust
pub async fn new(...) -> Result<Self, Box<dyn Error>> {
    Self::new_with_hooks(..., Arc::new(HookRegistry::new())).await
}

pub async fn with_tools_and_memory_provider(...) -> Result<Self, Box<dyn Error>> {
    Self::with_tools_memory_and_hooks(..., None, None).await
}

pub async fn with_hooks(
    ...,
    hook_registry: Arc<HookRegistry>,
) -> Result<Self, Box<dyn Error>> {
    Self::with_tools_memory_and_hooks(..., None, Some(hook_registry)).await
}
```

The inner constructor resolves:

```rust
let hook_registry = hook_registry.unwrap_or_else(|| Arc::new(HookRegistry::new()));
register_builtin_hooks(&hook_registry, &tool_config).await;
```

### 7.2 Built-In Hook Registration

Built-ins should register near runtime assembly, not through global statics.

```rust
async fn register_builtin_hooks(
    registry: &Arc<HookRegistry>,
    tool_config: &ToolConfig,
) {
    if let Some(planning) = &tool_config.planning {
        registry
            .register(Arc::new(PlanningHookHandler::new(planning.store.clone())))
            .await;
    }
}
```

This keeps tests deterministic and allows custom test registries.

### 7.3 Run Context Construction

At the start of `process_inbound_message_inner`, create a reusable context:

```rust
let run_ctx = HookRunContext {
    trace_id: trace_id.clone(),
    session_key: session_key.clone(),
    channel: msg.channel.clone(),
    chat_id: msg.chat_id.clone(),
    workspace_root: self.workspace.clone(),
};
```

The run context should be cloned into event payloads. It is small and makes hook calls testable.

## 8. Turn and LLM Wiring

### 8.1 Context Transform

After existing prompt assembly, planning fallback, mask replacement, cron marker, and memory prefetch, run `transform_context`.

Pseudocode:

```rust
let context_transform = ContextTransform {
    run: run_ctx.clone(),
    messages,
};

messages = match self.hook_registry.run_transform_context(context_transform).await {
    HookFlow::Continue(transformed) => {
        validate_context_transform(transformed.messages)?
    }
    HookFlow::Block { reason } => {
        self.emit_error_event(&msg, event_tx, reason.clone());
        return Ok(Some(OutboundMessage::new(msg.channel, msg.chat_id, reason)));
    }
};
```

Validation rule:

```rust
fn validate_context_transform(messages: Vec<Message>) -> Result<Vec<Message>> {
    if messages.is_empty() {
        bail!("transform_context produced no messages");
    }
    if messages.first().map(|m| m.role.as_str()) != Some("system") {
        bail!("transform_context must preserve first system message");
    }
    Ok(messages)
}
```

### 8.2 LLM Observe Hooks

Before `provider.chat_stream`:

```rust
self.hook_registry
    .run_pre_llm_call(&LlmCallContext {
        run: run_ctx.clone(),
        model: model_to_use.clone(),
        message_count: messages.len(),
        tool_names: tool_defs.iter().filter_map(extract_tool_name).collect(),
    })
    .await;
```

After the stream is normalized into `LLMResponse`:

```rust
self.hook_registry
    .run_post_llm_call(&LlmResultContext {
        run: run_ctx.clone(),
        finish_reason: response.finish_reason.clone(),
        has_tool_calls: response.has_tool_calls(),
        token_usage: turn_token_usage.clone(),
    })
    .await;
```

Hooks should not receive the full streamed delta by default. Delta observation already exists through `AgentEvent`.

### 8.3 Reactive Compaction Interaction

When context overflow triggers reactive compaction, `messages` is rebuilt. The transform hook must run again on the rebuilt messages before the retry.

Pseudocode helper:

```rust
async fn prepare_messages_for_llm(
    &self,
    run_ctx: &HookRunContext,
    messages: Vec<Message>,
) -> Result<Vec<Message>> {
    match self.hook_registry
        .run_transform_context(ContextTransform {
            run: run_ctx.clone(),
            messages,
        })
        .await
    {
        HookFlow::Continue(ctx) => validate_context_transform(ctx.messages),
        HookFlow::Block { reason } => Err(anyhow!(reason)),
    }
}
```

This helper avoids forgetting the hook on the retry path.

## 9. Tool Wiring

### 9.1 Existing Guard Order

Current tool execution order must become:

1. Serialize tool arguments.
2. Enforce read-only reviewer mode.
3. Enforce Plan mode allowed tools.
4. Apply cron context augmentation.
5. Enforce cron recursion denial.
6. Run `pre_tool_call`.
7. Execute tool if not blocked.
8. Run `post_tool_call`.
9. Run `transform_tool_result`.
10. Emit `ToolCallFinished` and append tool result to context.

### 9.2 Tool Call Pseudocode

```rust
let mut params_value = serde_json::to_value(&tool_call.arguments)?;

if read_only_rejected {
    return blocked_tool_result("disabled in reviewer read-only mode");
}

if plan_mode_rejected {
    return blocked_tool_result("disabled in Plan mode");
}

augment_cron_context_if_needed(&mut params_value, &msg);

if is_cron_trigger && tool_call.name == "cron" {
    return blocked_tool_result("cron tool is disabled during cron-triggered execution");
}

let pre = self.hook_registry
    .run_pre_tool_call(ToolCallContext {
        run: run_ctx.clone(),
        tool_call_id: tool_call.id.clone(),
        tool_name: tool_call.name.clone(),
        arguments: params_value,
    })
    .await;

let (result, is_error) = match pre {
    HookFlow::Block { reason } => (
        format!("Error: tool '{}' blocked by hook: {}", tool_call.name, reason),
        true,
    ),
    HookFlow::Continue(ctx) => {
        debug_assert_eq!(ctx.tool_name, tool_call.name);
        match self.tools.execute(&tool_call.name, ctx.arguments).await {
            Ok(text) => (text, false),
            Err(error) => (format!("Error: {}", error), true),
        }
    }
};
```

### 9.3 Tool Result Pseudocode

```rust
self.hook_registry
    .run_post_tool_call(&ToolResultContext {
        run: run_ctx.clone(),
        tool_call_id: tool_call.id.clone(),
        tool_name: tool_call.name.clone(),
        is_error,
        result_preview: truncate_for_hook_preview(&result),
    })
    .await;

let transform = ToolResultTransform {
    run: run_ctx.clone(),
    tool_call_id: tool_call.id.clone(),
    tool_name: tool_call.name.clone(),
    result,
    is_error,
};

let (result, is_error) = match self.hook_registry
    .run_transform_tool_result(transform)
    .await
{
    HookFlow::Continue(next) => (next.result, next.is_error),
    HookFlow::Block { reason } => (
        format!("Error: tool '{}' result blocked by hook: {}", tool_call.name, reason),
        true,
    ),
};
```

`post_tool_call` receives a preview rather than the full result to avoid leaking large outputs into every observer. `transform_tool_result` receives the full text because mutation is its purpose.

## 10. Planning Handler Migration

The existing planning injection should move behind a built-in handler without changing behavior.

### 10.1 Handler Shape

```rust
pub struct PlanningHookHandler {
    store: Arc<dyn PlanningStore>,
}

impl PlanningHookHandler {
    pub fn new(store: Arc<dyn PlanningStore>) -> Self {
        Self { store }
    }
}

#[async_trait::async_trait]
impl HookHandler for PlanningHookHandler {
    fn registration(&self) -> HookRegistration {
        HookRegistration {
            id: "planning.context".to_string(),
            priority: 10_000,
            failure_policy: HookFailurePolicy::WarnAndContinue,
            events: vec![HookEventKind::TransformContext],
        }
    }

    async fn transform_context(
        &self,
        mut ctx: ContextTransform,
    ) -> crate::Result<HookFlow<ContextTransform>> {
        match inject_plan_context(self.store.as_ref()).await {
            Ok(Some(block)) => {
                insert_after_primary_system(&mut ctx.messages, Message::system(block));
            }
            Ok(None) => {}
            Err(error) => {
                tracing::warn!("Planning context injection failed: {}", error);
            }
        }

        Ok(HookFlow::Continue(ctx))
    }
}
```

### 10.2 Plan Mode Fallback

Plan mode fallback text should remain in `loop_turn.rs` until planning runtime is always represented by a handler. It is a safety behavior, not a general hook.

```rust
if plan_mode && !planning_runtime_configured {
    messages.insert(1, Message::system(PLAN_MODE_FALLBACK));
}
```

### 10.3 HOOK-2 Through HOOK-5

Do not force every planning hook into the generic runtime immediately:

- `is_planning_tool_call` stays a domain helper for Plan mode policy.
- `on_planning_tool_complete` remains a stub until event sourcing exists.
- `on_session_save` remains a planning persistence hook until session-save semantics are centralized.
- `is_planning_message` remains a classifier/helper.

## 11. Session Hooks

### 11.1 Session Start

Trigger when a session is first created for a `session_key`, not on every message.

Pseudocode:

```rust
let session_existed = self.sessions.get(&session_key).is_some();
let session = self.sessions.get_or_create(&session_key);

if !session_existed {
    self.hook_registry
        .run_on_session_start(&SessionStartContext {
            run: run_ctx.clone(),
            user_message_preview: preview(&msg.content, 200),
        })
        .await;
}
```

If determining first creation is awkward with the current `SessionManager`, introduce a small helper such as:

```rust
pub enum SessionLookup<'a> {
    Existing(&'a mut Session),
    Created(&'a mut Session),
}

pub fn get_or_create_with_status(&mut self, key: &str) -> SessionLookup<'_>;
```

### 11.2 Session End

Keep existing `MemoryProvider::on_session_end` call. Add hook observation before or after it; recommended order:

1. `HookRegistry::run_on_session_end`
2. `MemoryProvider::on_session_end`

Pseudocode:

```rust
self.hook_registry
    .run_on_session_end(&SessionEndContext {
        run: HookRunContext {
            trace_id: "agent-loop-shutdown".to_string(),
            session_key: "agent-loop-shutdown".to_string(),
            channel: "system".to_string(),
            chat_id: "system".to_string(),
            workspace_root: self.workspace.clone(),
        },
        reason: SessionEndReason::AgentLoopShutdown,
    })
    .await;

self.memory_provider
    .on_session_end(SessionEndRequest {
        workspace_root: self.workspace.clone(),
        session_id: Some("agent-loop-shutdown".to_string()),
    })
    .await;
```

## 12. Introspection

### 12.1 Registry Snapshot

```rust
pub struct HookDescriptor {
    pub id: String,
    pub priority: i32,
    pub failure_policy: HookFailurePolicy,
    pub events: Vec<HookEventKind>,
}

impl HookRegistry {
    pub async fn list(&self) -> Vec<HookDescriptor> {
        self.snapshot()
            .await
            .into_iter()
            .map(|hook| HookDescriptor {
                id: hook.registration.id,
                priority: hook.registration.priority,
                failure_policy: hook.registration.failure_policy,
                events: hook.registration.events,
            })
            .collect()
    }
}
```

### 12.2 Manager Endpoint

Add a read-only route:

```text
GET /hooks
```

Response shape:

```json
{
  "status": "ok",
  "hooks": [
    {
      "id": "planning.context",
      "priority": 10000,
      "failurePolicy": "warnAndContinue",
      "events": ["transformContext"]
    }
  ]
}
```

The manager should not expose handler internals or execute hooks through this endpoint.

### 12.3 CLI Command

CLI shape:

```text
agent-diva hooks list
agent-diva hooks list --json
```

Pseudocode:

```rust
enum Commands {
    Hooks {
        #[command(subcommand)]
        command: HookCommands,
    },
}

enum HookCommands {
    List { #[arg(long)] json: bool },
}
```

Non-JSON output:

```text
ID                 Priority  Failure Policy    Events
planning.context   10000     warn-and-continue transform-context
```

`hooks test` is deferred because it needs stable synthetic payload schemas and, later, external handler sandboxing.

## 13. Config Boundary

Do not add root config fields for executable external hooks in MVP.

Optional in-process enablement config can be introduced later only for built-in hook families:

```toml
[hooks]
enabled = true

[hooks.builtin.planning]
enabled = true
priority = 10000
```

For the MVP, built-in registration can be driven by existing domain config. For example, planning registers only when `tool_config.planning` exists.

## 14. Testing Strategy

### 14.1 Core Tests

Core registry tests:

```rust
#[tokio::test]
async fn transform_hooks_run_by_priority_then_registration_order() {}

#[tokio::test]
async fn transform_hooks_chain_previous_output() {}

#[tokio::test]
async fn block_short_circuits_remaining_transform_hooks() {}

#[tokio::test]
async fn observe_hook_error_warns_and_continues() {}

#[tokio::test]
async fn block_on_error_converts_error_to_block() {}

#[tokio::test]
async fn list_returns_sorted_hook_descriptors() {}
```

### 14.2 Agent Loop Tests

Agent loop tests:

```rust
#[tokio::test]
async fn pre_tool_call_block_does_not_execute_tool() {}

#[tokio::test]
async fn pre_tool_call_can_transform_arguments() {}

#[tokio::test]
async fn transform_tool_result_changes_model_visible_content() {}

#[tokio::test]
async fn read_only_policy_runs_before_user_hooks() {}

#[tokio::test]
async fn plan_mode_policy_runs_before_user_hooks() {}

#[tokio::test]
async fn transform_context_runs_again_after_reactive_compaction() {}
```

### 14.3 Planning Tests

Planning migration tests:

```rust
#[tokio::test]
async fn planning_hook_injects_active_plan_context() {}

#[tokio::test]
async fn planning_hook_is_noop_without_active_plan() {}

#[tokio::test]
async fn plan_mode_fallback_still_works_without_planning_runtime() {}
```

### 14.4 Introspection Tests

```rust
#[tokio::test]
async fn hooks_endpoint_lists_builtin_handlers() {}

#[test]
fn hooks_list_json_is_structured_output() {}
```

## 15. Rollout Plan

### Phase 0: Architecture Only

Deliver this research document with pseudocode and no Rust edits.

Acceptance:

- `docs/research/agent-diva-hooks-architecture.md` exists.
- It covers core API, loop wiring, planning handler, introspection, tests, and deferred external adapters.
- It does not change source code.

### Phase 1: Core API

Implement `agent-diva-core/src/hooks/`:

- `HookFlow<T>`
- `HookFailurePolicy`
- `HookRegistration`
- event/context types
- `HookHandler`
- `HookRegistry`
- registry tests

Run:

```bash
cargo test -p agent-diva-core hooks
```

### Phase 2: Agent Loop Wiring

Implement `hook_registry` field and wire:

- `transform_context`
- `pre_llm_call`
- `post_llm_call`
- `pre_tool_call`
- `post_tool_call`
- `transform_tool_result`
- session start/end observe hooks

Run:

```bash
cargo test -p agent-diva-agent hook
```

### Phase 3: Planning Handler

Add `PlanningHookHandler` and move active plan context injection behind it while preserving behavior.

Run:

```bash
cargo test -p agent-diva-agent planning
```

### Phase 4: Introspection

Add:

- `HookDescriptor`
- manager `GET /hooks`
- CLI `agent-diva hooks list --json`

Run:

```bash
just run -- hooks list --json
```

### Phase 5: Full Validation

Run:

```bash
just fmt-check
just check
just test
```

## 16. Deferred Work

### P1

- `pre_gateway_dispatch` after HTTP, channel bridge, and cron entrypoints are normalized.
- `hooks test` with synthetic payload fixtures.
- Config toggles for built-in hook families.

### P2

- Claude/Codex compatible TOML adapter.
- Shell command handler adapter.
- HTTP handler adapter.
- Prompt/agent handler adapter.
- Trust, allowlist, timeout, stdin/stdout JSON protocol, and sandbox policy for external handlers.

## 17. Non-Goals

This architecture does not:

- Replace `MemoryProvider`.
- Turn `MessageBus` into an interceptor.
- Add shell or config-driven hooks in MVP.
- Rewrite the tools subsystem.
- Reorder existing policy guards after user hooks.
- Make hook handlers global singletons.

## 18. Acceptance Checklist

- [x] Defines a typed `agent-diva-core` hook API with pseudocode.
- [x] Defines registry ordering, block, transform, and failure semantics.
- [x] Defines exact agent loop insertion points.
- [x] Preserves `MemoryProvider` and `MessageBus` boundaries.
- [x] Specifies planning handler migration.
- [x] Specifies hooks introspection.
- [x] Specifies tests and phased validation.
- [x] Leaves external config/shell hooks deferred.
- [x] Contains no actual Rust source changes.
