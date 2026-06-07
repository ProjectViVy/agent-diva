# Thinking Mode Integration Report for agent-diva-pro

> 调研日期：2026-06-03
> 参考源：Cherry Studio (.workspace/cherry-studio/)、agent-diva-pro 源码
> 结论：**agent-diva-pro 已有 80% 的 thinking 基础设施，需补完的只剩配置层和 GUI 渲染。**

---

## 1. Executive Summary

agent-diva-pro 的 `agent-diva-providers` 和 `agent-diva-agent` 已经内置了 reasoning/thinking 的完整数据流：

- **数据模型** ✅ — `LLMStreamEvent::ReasoningDelta`、`LLMResponse.reasoning_content`、`Message.reasoning_blocks`
- **流式处理** ✅ — LiteLLM 客户端解析 `reasoning_content` 并转换为 `ReasoningDelta` 事件
- **Agent Loop** ✅ — 接收 `ReasoningDelta`、累积 `streamed_reasoning`、存入 Message
- **CLI 渲染** ✅ — `TimelineKind::Thinking` 以 `DarkGray`/`Magenta` 渲染 `<think>...</think>` 块
- **reasoning_effort** ✅ — `LiteLLMClient.default_reasoning_effort` 已支持

**缺失部分（20%）：**
1. ❌ 每模型独立的 reasoning 配置（参考 Cherry Studio 的 `ReasoningConfig`）
2. ❌ GUI 端 thinking 块的折叠/展开渲染
3. ❌ 用户可切换的 "thinking mode" 开关
4. ❌ `ModelCapabilities.reasoning` 字段目前硬编码为 `false`，未根据模型动态判断

---

## 2. Cherry Studio Thinking Architecture (Reference)

### 2.1 Type System

```typescript
// src/shared/data/types/provider.ts
export const REASONING_FORMAT_TYPES = [
  'openai-chat',       // OpenAI Chat Completions reasoning_effort
  'openai-responses',  // OpenAI Responses API reasoning
  'anthropic',         // Anthropic extended thinking
  'gemini',            // Gemini thinkingConfig
  'openrouter',        // OpenRouter reasoning tokens
  'enable-thinking',   // Doubao/Douyin enable_thinking param
  'thinking-type',     // Doubao thinking.type param
  'dashscope',         // Aliyun DashScope reasoning
  'self-hosted'        // Custom endpoints
] as const;

// src/shared/data/types/model.ts
export const ThinkingTokenLimitsSchema = z.object({
  min: z.number().nonnegative().optional(),
  max: z.number().positive().optional(),
  default: z.number().nonnegative().optional()
});

export const ReasoningConfigSchema = z.object({
  type: z.string(),                           // matches REASONING_FORMAT_TYPES
  thinkingTokenLimits: ThinkingTokenLimitsSchema.optional(),
  supportedEfforts: z.array(z.enum([...])).optional(),  // e.g. ['low','medium','high']
  interleaved: z.boolean().optional()         // whether thinking interleaves with text
});

// Runtime form adds defaultEffort
export const RuntimeReasoningSchema = ReasoningConfigSchema
  .required({ supportedEfforts: true })
  .extend({ defaultEffort: z.enum([...]).optional() });
```

### 2.2 Message Block System

```typescript
// src/shared/data/types/message.ts
export enum BlockType {
  THINKING = 'thinking',   // ← thinking block is a first-class type
  MAIN_TEXT = 'main_text',
  TOOL = 'tool',
  CODE = 'code',
  // ... etc
}

export interface ThinkingBlock extends BaseBlock {
  type: BlockType.THINKING
  content: string      // the thinking content
  thinkingMs: number   // how long the model spent thinking
}
```

### 2.3 Streaming Adapter

```typescript
// src/renderer/aiCore/chunk/AiSdkToChunkAdapter.ts
class AiSdkToChunkAdapter {
  // Accumulates reasoningContent during stream
  private async handleReasoningStreamPart(part) {
    final.reasoningContent += chunk.text || '';
    // Emit THINKING_TEXT_DELTA chunk
    this.emitChunk({
      type: ChunkType.THINKING_TEXT_DELTA,
      text: final.reasoningContent || ''
    });
  }
  
  // On stream complete: emit THINKING_COMPLETE with accumulated content
  private emitThinkingCompleteIfNeeded(final) {
    if (final.reasoningContent) {
      this.emitChunk({ type: ChunkType.THINKING_COMPLETE, text: final.reasoningContent });
      final.reasoningContent = '';
    }
  }
}
```

### 2.4 Key Design Decisions

1. **Reasoning config is per-model, not per-provider.** 每个模型可能支持不同的 reasoning format 和 effort level。
2. **Thinking blocks are first-class message blocks.** 与 MAIN_TEXT、TOOL、CODE 并列，有独立的 UI 渲染。
3. **Streaming 区分 `THINKING_TEXT_DELTA` 和 `THINKING_COMPLETE`** — 流式时实时展示增量，完成后发射完成事件。
4. **thinkingMs 记录模型思考耗时** — 用户可见的元数据。

---

## 3. agent-diva-pro Existing Infrastructure (What's Already There)

### 3.1 Provider Layer (`agent-diva-providers/src/base.rs`)

```rust
// L41-42: ModelCapabilities 已有 reasoning 字段
pub struct ModelCapabilities {
    pub vision: bool,
    pub tools: bool,
    pub reasoning: bool,  // ← 已定义，但 model_capabilities_for_model() 未设置
}

// L201: LLMResponse 已有 reasoning_content
pub struct LLMResponse {
    pub reasoning_content: Option<String>,  // ← 已定义
    // ...
}

// L217-231: LLMStreamEvent 已有 ReasoningDelta
pub enum LLMStreamEvent {
    TextDelta(String),
    ReasoningDelta(String),  // ← 已定义
    ToolCallDelta { ... },
    Completed(LLMResponse),
}

// L245-247: Message 已有 reasoning 相关字段
pub struct Message {
    pub reasoning_content: Option<String>,              // ← 已定义
    pub thinking_blocks: Option<Vec<serde_json::Value>>, // ← 已定义
    // ...
}
```

### 3.2 LiteLLM Client (`agent-diva-providers/src/litellm.rs`)

```rust
// L111: StreamDelta 解析 reasoning_content
struct StreamDelta {
    #[serde(default)]
    reasoning_content: Option<String>,  // ← 已解析
    // ...
}

// L60: ResponseMessage 解析 reasoning_content
struct ResponseMessage {
    #[serde(default)]
    reasoning_content: Option<String>,  // ← 已解析
    // ...
}

// L141-143: RequestBuildOptions 已有 reasoning_effort
struct RequestBuildOptions {
    reasoning_effort: Option<String>,  // ← 已定义
    // ...
}

// L163: LiteLLMClient 已有 default_reasoning_effort
pub struct LiteLLMClient {
    default_reasoning_effort: Option<String>,  // ← 已存储
    // ...
}

// L33: ChatCompletionRequest 序列化 reasoning_effort
struct ChatCompletionRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    reasoning_effort: Option<String>,  // ← 已发送给 API
    // ...
}
```

### 3.3 Agent Loop (`agent-diva-agent/src/agent_loop/loop_turn.rs`)

```rust
// L180: 累积流式 reasoning
let mut streamed_reasoning = String::new();

// L207-210: 处理 ReasoningDelta 事件
LLMStreamEvent::ReasoningDelta(delta) => {
    streamed_reasoning.push_str(&delta);
    let event = AgentEvent::ReasoningDelta { text: delta };  // ← 转发给 UI
}

// L253-256: 完成时存入 response
reasoning_content: if streamed_reasoning.is_empty() {
    response.reasoning_content.clone()
} else {
    Some(streamed_reasoning)  // ← 流式累积优先
},

// L489: 创建 assistant message 时携带
reasoning_content: final_reasoning,  // ← 存入消息

// L659-660: 复制 reasoning 到上下文消息
msg.reasoning_content = m.reasoning_content.clone();
msg.thinking_blocks = m.thinking_blocks.clone();
```

### 3.4 Agent Loop Output (`agent-diva-agent/src/agent_loop.rs`)

```rust
// L683-685: 输出时包裹 <think> 标签
if let Some(reasoning) = r.reasoning_content {
    if !reasoning.is_empty() {
        return format!("<think>\n{}\n</think>\n\n{}", reasoning, content);
    }
}
```

### 3.5 CLI Rendering (`agent-diva-cli/src/main.rs`)

```rust
// L908-919: ReasoningDelta 渲染为 TimelineKind::Thinking
AgentEvent::ReasoningDelta { text } => {
    if let Some(item) = self.timeline.last_mut() {
        if matches!(item.kind, TimelineKind::Thinking) {
            item.text.push_str(&text);  // ← 增量追加
            return;
        }
    }
    self.add_line(TimelineKind::Thinking, text);
}

// L1080: 渲染颜色
TimelineKind::Thinking => ("thinking", Color::DarkGray),  // inline view
TimelineKind::Thinking => ("thinking", Color::Magenta),   // debug view
```

---

## 4. Gap Analysis

### Gap 1: No Per-Model Reasoning Configuration

**现状：** `default_reasoning_effort` 是 LiteLLMClient 级别的全局配置，无法按模型差异化。

**Cherry Studio 做法：** 每个 Model 对象有独立的 `reasoning: ReasoningConfig`，包含 type、thinkingTokenLimits、supportedEfforts、defaultEffort。

**建议方案：** 在 `agent-diva-core` 的 provider config 中增加 `model_reasoning_configs: HashMap<String, ReasoningConfig>`，格式兼容 Cherry Studio 的 ReasoningConfig。

### Gap 2: `model_capabilities_for_model()` 未设置 `reasoning`

**现状：** `ModelCapabilities` 有 `reasoning` 字段但硬编码为 `false`。

```rust
// base.rs L55-64: 当前只设置了 vision
pub fn model_capabilities_for_model(model: &str) -> ModelCapabilities {
    let normalized = normalize_model_id(model);
    let mut capabilities = ModelCapabilities::text_only();
    capabilities.vision = matches!(normalized.as_str(), "gpt-4o" | ...);
    capabilities  // ← reasoning 永远是 false!
}
```

**建议方案：** 添加 reasoning 模型白名单，至少包含 `deepseek-chat`、`deepseek-reasoner`、`claude-*`、`gemini-*`。

### Gap 3: No GUI Rendering for Thinking Blocks

**现状：** CLI 有 `TimelineKind::Thinking` 渲染，但 `agent-diva-gui` 没有对应的 Vue 组件。

**建议方案：** 在 GUI 中增加折叠式 `<ThinkingBlock>` 组件，显示 `[思考中...]` 然后折叠展示内容。

### Gap 4: No User-Facing Thinking Mode Toggle

**现状：** thinking 是否启用完全取决于 provider 是否发送 `reasoning_content`，用户无法选择。

**建议方案：** 
- 自动模式：provider 支持 reasoning 的模型自动启用
- 可选手动开关：`/thinking on|off|auto`

---

## 5. Integration Roadmap

### Phase 1: Model Capabilities (1-2h)
**文件：** `agent-diva-providers/src/base.rs`

扩展 `model_capabilities_for_model()`：
```rust
capabilities.reasoning = matches!(
    normalized.as_str(),
    "deepseek-chat" | "deepseek-reasoner" | "claude-3-opus" 
    | "claude-3-5-sonnet" | "gemini-2.0-flash" | "gemini-2.5-pro" | ...
);
```

### Phase 2: Per-Model Reasoning Config (3-4h)
**新增类型：** `agent-diva-core/src/reasoning.rs` (或放 providers crate)
**修改文件：** `agent-diva-providers/src/litellm.rs`

```rust
// 新类型 (兼容 Cherry Studio ReasoningConfig)
pub struct ReasoningConfig {
    pub reasoning_type: String,           // "openai-chat" / "anthropic" / etc.
    pub thinking_token_limits: Option<ThinkingTokenLimits>,
    pub supported_efforts: Option<Vec<String>>,
    pub default_effort: Option<String>,
}

// 在 resolve_model 时查找对应配置
fn get_reasoning_config(&self, model: &str) -> Option<&ReasoningConfig> {
    self.model_reasoning_configs.get(model)
}
```

### Phase 3: Thinking Mode Toggle (2-3h)
**文件：** `agent-diva-agent/src/runtime_control.rs` + CLI 命令

添加运行时控制：
```rust
enum ThinkingMode {
    Auto,   // 默认，模型支持则启用
    On,     // 强制启用
    Off,    // 禁用
}
```

CLI 命令：`/thinking auto|on|off`

### Phase 4: GUI Thinking Block Rendering (4-6h)
**文件：** `agent-diva-gui/src/components/ThinkingBlock.vue`

实现折叠式 thinking 渲染：
```vue
<template>
  <details class="thinking-block">
    <summary>🔍 思考过程 ({{ thinkingMs }}ms)</summary>
    <div class="thinking-content">{{ content }}</div>
  </details>
</template>
```

接收 `AgentEvent::ReasoningDelta` 并逐步构建 ThinkingBlock。

---

## 6. Detailed Injection Points

### 6.1 `agent-diva-providers/src/base.rs`
| 行号 | 元素 | 变更 |
|------|------|------|
| L41 | `ModelCapabilities.reasoning` | 已经在位，无需修改 |
| L55-64 | `model_capabilities_for_model()` | 添加 reasoning 白名单 |
| L201 | `LLMResponse.reasoning_content` | 已经在位 |
| L221 | `LLMStreamEvent::ReasoningDelta` | 已经在位 |
| L245-247 | `Message.reasoning_content` / `thinking_blocks` | 已经在位 |

### 6.2 `agent-diva-providers/src/litellm.rs`
| 行号 | 元素 | 变更 |
|------|------|------|
| L111 | `StreamDelta.reasoning_content` | 已在解析 |
| L141 | `RequestBuildOptions.reasoning_effort` | 已在发送 |
| L163 | `default_reasoning_effort` | 需改为按模型查找 |
| 新增 | ReasoningConfig 类型和查找方法 | 需新增 |

### 6.3 `agent-diva-agent/src/agent_loop/loop_turn.rs`
| 行号 | 元素 | 变更 |
|------|------|------|
| L180 | `streamed_reasoning` | 已在累积 |
| L207-210 | ReasoningDelta 处理 | 已在转发 |
| L253-256 | reasoning_content 赋值 | 已在存储 |
| 新增 | ThinkingMode 检查，Off 时清空 reasoning | 需新增 |

### 6.4 `agent-diva-cli/src/main.rs`
| 行号 | 元素 | 变更 |
|------|------|------|
| L908-919 | ReasoningDelta → TimelineKind::Thinking | 已在渲染 |
| L1080 | DarkGray 渲染 | 可以了 |

### 6.5 `agent-diva-gui/` (新增)
| 文件 | 内容 |
|------|------|
| `src/components/ThinkingBlock.vue` | 折叠式 thinking 展示组件 |
| `src/stores/messageBlocks.ts` | MessageBlock 类型中加入 ThinkingBlock |

---

## 7. Cherry Studio → agent-diva 映射表

| Cherry Studio | agent-diva-pro | 状态 |
|---|---|---|
| `ReasoningConfigSchema` | 待新增 | ❌ |
| `REASONING_FORMAT_TYPES` | 待新增 | ❌ |
| `ThinkingTokenLimitsSchema` | 待新增 | ❌ |
| `REASONING_EFFORT` enum | 全局 `reasoning_effort` | ⚠️ 有但非按模型 |
| `BlockType.THINKING` | `LLMStreamEvent::ReasoningDelta` | ✅ |
| `ThinkingBlock` (content + thinkingMs) | `Message.thinking_blocks` | ⚠️ 字段有但未填充 |
| `ChunkType.THINKING_TEXT_DELTA` | `AgentEvent::ReasoningDelta` | ✅ |
| `ChunkType.THINKING_COMPLETE` | 无对应，但不需要（因 Delta 已足够） | ✅ |
| AI SDK fullStream → Chunk 适配器 | LiteLLM stream → LLMStreamEvent | ✅ |
| Per-model reasoning config | Per-client reasoning_effort | ⚠️ 粒度不够 |
| 折叠式 UI 渲染 | CLI `TimelineKind::Thinking` | ⚠️ GUI 缺失 |

---

## 8. 迁移兼容性清单

从 Cherry Studio 的 v1→v2 迁移映射 (`ChatMappings.ts`) 可以看到历史字段演变：
- `time_thinking_millsec` → `thinkingMs` (字段重命名)
- `thinking_millsec` → `thinkingMs` (字段重命名)
- `reasoning_content` (v1 顶层字段) → `ThinkingBlock.content` (v2 block 系统)

这些对 agent-diva-pro 的设计有参考价值：Message.thinking_blocks 应该是一个可序列化的 struct 而非 `serde_json::Value`。

---

## 9. 总结

**agent-diva-pro 的 thinking 基础设施成熟度：80%**

核心数据模型和流式管道已经完备。主要工作集中在：
1. 配置层的精细化管理（per-model reasoning config）
2. GUI 的 thinking 块渲染
3. 运行时控制开关

**建议优先做 Phase 1+2（配置层），这是"使现有基础设施真正可用"的关键一步。**
