# Image Recognition 跨项目对比分析 & agent-diva 实施路径

> 分析日期: 2026-06-01
> 基于三个子代理的深度代码分析: claude-code-vision-analysis.md, hermes-agent-vision-analysis.md, genericagent-opanfang-vision-analysis.md
> 原有计划: docs/dev/multimodal/image-recognition-prephase-analysis-plan.md

---

## 一、兄弟项目图片处理设计对比矩阵

| 维度 | Codex (Python+Rust) | Claude Code (TypeScript) | Hermes Agent (Python) | GenericAgent (Python) | OpenFang (Rust) | **agent-diva (Rust)** |
|------|---------------------|--------------------------|----------------------|----------------------|-----------------|-----------------------|
| **消息内容类型** | InputItem union (Text/Image/LocalImage) | string \| ContentBlockParam[] | str \| List[dict] (OpenAI格式) | str only (images参数被忽略) | ContentBlock::Image enum variant | **String only** ❌ |
| **图片一等公民** | ✅ InputItem::Image/LocalImage | ✅ ContentBlock::Image | ✅ image_url content part | ❌ 接口预留但链路断裂 | ✅ 3个原生驱动映射 | ❌ |
| **存储方式** | ThreadHistory (Rust) + ThreadItem::ImageView | ~/.claude/image-cache/{sessionId}/ | $HERMES_HOME/cache/images/ | temp/uploaded/ (QT) / temp/ (TG) | N/A (内存/inline) | FileHandle (SHA256 content-addressed) ✅ |
| **持久化策略** | ThreadItem::ImageView with path + id | [Image #N] ref in history.jsonl + pasteStore (外存) | image ref 嵌入 message 历史 | 无持久化 (纯临时文件) | inline base64 in session | 待设计 |
| **API序列化** | ContentItem::InputImage { image_url, detail } | Anthropic BetaImageBlockParam (base64) | 多provider: Anthropic/OpenAI/Codex/Gemini/Bedrock | 无 (链路断裂) | 3个原生driver逐一映射 | **无** (content:String) |
| **Provider抽象** | InputModality::Image capability flag | API限制常量 + ImageSizeError | supports_vision flag + ModelCapabilities | 无 | 各driver内置 | **无** |
| **图片预处理** | codex-utils-image (2000px max, resize+LRU cache) | sharp (max 2000px, target 3.75MB, 渐进降级) | Pillow (响应式: 先发原图, 被拒再缩小) | 无 | 5MB硬限制 + 格式白名单 | **无** |
| **设备兼容** | macOS/Linux/Windows 剪贴板 | macOS/Linux/Windows 剪贴板 (imagePaste.ts) | 各平台adapter各自实现 | QT: 文件选择器 / TG: Photo API | N/A | GUI: uploadFile (Vue) |
| **文本+图片同消息** | ✅ 做在 From<Vec<UserInput>> 转为 ContentItem::InputImage | ✅ processTextPrompt合并 | ✅ 同content数组 | ❌ QT: base64嵌入文本 / TG: 仅路径 | ✅ (fix #1043) | ❌ |

---

## 二、各项目核心设计决策分析

### 2.1 Codex — 最完整的类型系统 (最值得参考)

Codex 是唯一在 **Python SDK + Rust Core + Protocol Schema** 三层都严格类型化的项目:

**三层类型定义:**
```
Python SDK:     ImageInput(url) | LocalImageInput(path)  (union type)
Protocol:       InputImage { image_url, detail }  + <image> XML tag wrapping  
Rust Core:      ContentItem::InputImage { image_url, detail }
```

**关键设计:**
- `From<Vec<UserInput>>` 在 **序列化时** 同步读取本地文件 (std::fs::read) — Rust 风格
- `<image>` XML tag 包围图片 URL，确保文本模型也能看到 "这里有张图" 的提示
- token 估计用 32px patches 公式 + 32-entry LRU 缓存 (避免重复解码 base64)
- `strip_images_when_unsupported()` 在模型不支持 image 时自动替换为文本占位符
- `view_image` tool 让模型显式请求查看图片 (工具调用，而非被动接收)

**对 diva 的启示:**
1. 枚举类型 `MessageContentPart` 是最干净的设计
2. 同步读取文件到 base64 可以在 context assembly 阶段做
3. ModelCapabilities::vision 的 guard 是必须的

### 2.2 Claude Code — 最完善的剪贴板集成

Claude Code 的图片管线分 7 步，每一步都有明确的职责边界:

```
剪贴板检测 → sharp缩放编码 → 磁盘存储(image-cache) → processUserInput
→ processTextPrompt组装 → normalizeMessagesForAPI验证 → API发送
```

**关键设计:**
- `PastedContent` 类型有 `content: string` (base64) 字段 — 但图片不经过 pasteStore，独立存为文件
- `[Image #N]` 引用格式在 history 中保持可读性
- `maybeResizeAndDownsampleImageBuffer()` 的渐进降级策略 (PNG最大压缩 → JPEG80→60→40→20 → resize → 二次压缩)
- 每张图片固定 2000 token 估算 (保守但足够)
- bridge 模式下 `normalizeImageBlocks()` 修复 camelCase/snake_case 兼容问题

**对 diva 的启示:**
1. GUI 的图片上传路径已经存在 (uploadFile)，不需要剪贴板检测
2. 但需要类似 `ImageBlockParam` 的归一化层 (处理不同来源的图片格式)
3. API 限制常量 (5MB base64, 100 media/req) 是务实参考

### 2.3 Hermes Agent — 最灵活的路由系统 (最值得借用的架构)

Hermes 的 **图片输入双模式路由** 是 diva 最应该借鉴的设计:

```
decide_image_input_mode(provider, model, cfg)
├── "native"  → build_native_content_parts() → content array
└── "text"   → _enrich_message_with_vision() → vision_analyze → 文本描述
```

**关键设计:**
- `image_input_mode: auto | native | text` 配置项 — 用户可控制
- `ModelCapabilities { supports_vision: bool }` 在 models.dev 数据库
- `_supports_media_in_tool_results()` provider 白名单
- 独立的 `vision_analyze_tool()` 作为 text 回退路径
- 响应式图片大小处理 (先发送，被拒再缩小) — 比 Claude Code 的预处理更省 CPU

**对 diva 的启示:**
1. `ModelCapabilities` 是必须添加的 — 不做 vision guard 会向 text-only 模型发送无效 payload
2. native/text 双模式对 GUI 很有用: vision 模型直接用 native，text 模型给出清晰提示
3. 不需要立即做 vision_analyze 回退 — IMG-PRE 只做 native 就够了

### 2.4 GenericAgent — 反面教材

GenericAgent 的 `put_task(query, images=None)` 定义了 `images` 参数但 `run()` 方法完全忽略。**类型定义存在但链路断裂 = 最危险的半成品状态。**

**对 diva 的警告:**
- 不要只改类型定义 — 必须端到端验证 (GUI upload → InboundMessage → context assembly → API serialization)
- IMG-PRE 的目标是 **最小的完整垂直切片**，不是改一半

### 2.5 OpenFang — Rust 实现参考

OpenFang 的 `ContentBlock::Image` 是 Rust enum 变体:
```rust
Image {
    media_type: String,   // "image/png"
    data: String,         // base64
}
```

三个 driver (Anthropic/Gemini/OpenAI) 都直接映射，每个有自己的序列化逻辑。5MB 硬限制 + 格式白名单。

**对 diva 的启示:**
- Rust enum 序列化比 Python dict 更安全 (编译期穷举检查)
- 每个 provider adapter 各自负责将 `MessageContentPart` 转为原生 JSON — 不需要全局统一格式

---

## 三、agent-diva 当前状态确认

### 3.1 已有的基础设施 ✅

| 模块 | 状态 | 文件 | 备注 |
|------|------|------|------|
| FileAttachment | ✅ 完成 | `agent-diva-core/src/attachment.rs` | `is_image()` 判断、MIME type、SHA256 |
| FileManager | ✅ 完成 | `agent-diva-files` | 内容寻址存储、获取文件字节 |
| GUI uploadFile | ✅ 完成 | `agent-diva-gui/src/components/ChatView.vue` | 用户可附加文件 |
| InboundMessage.media | ✅ 完成 | `agent-diva-core/src/bus/events.rs` | 事件总线携带文件 ID |
| loop_turn 加载附件 | ⚠️ 部分 | `agent-diva-agent/src/agent_loop/loop_turn.rs` | 仅内联小文本文件、非文本发占位符 |

### 3.2 核心缺口 ❌

| 缺口 | 当前状态 | 需要改成 |
|------|---------|---------|
| **Message.content 类型** | `String` (agent-diva-providers/src/base.rs:190) | `enum MessageContent { Text(String), Parts(Vec<MessageContentPart>) }` |
| **Provider 序列化** | 纯字符串发送 | 根据 model 能力序列化 structured content parts |
| **Model capabilities** | 不存在 | 添加 `ModelCapabilities { vision: bool }` |
| **会话历史中的图片** | ChatMessage.content 是 String | 添加 `attachments: Option<Vec<FileAttachmentRef>>` |
| **GUI capability 感知** | 无 | 非 vision 模型时禁用/警告图片上传 |

---

## 四、推荐的 IMG-PRE 实施路径

原有计划定义的 5 个 slice 经受住了跨项目验证。以下是基于对比分析的详细修订:

### IMG-PRE-0: 类型契约 (不变)

**目标:** 定义 `MessageContentPart` 和 `FileAttachmentRef` 的最终形状

**对比验证后的确认:**
```rust
// 参考 Codex 的 InputItem + ContentItem 双层设计
pub enum MessageContentPart {
    Text { text: String },
    ImageUrl { url: String },                          // 远程 URL (ref: Codex ImageInput)
    ImageFile { file_id: String, mime_type: Option<String> },  // 本地文件 (ref: Codex LocalImageInput)
    ImageData { mime_type: String, data_base64: String },      // inline base64 (ref: OpenFang)
}

pub enum MessageContent {
    Text(String),
    Parts(Vec<MessageContentPart>),
}

// 参考 Claude Code 的 PastedContent 设计
pub struct FileAttachmentRef {
    pub file_id: String,
    pub filename: String,
    pub mime_type: Option<String>,
    pub size: u64,
}
```

**位置决策:**
- `MessageContentPart` / `MessageContent` → `agent-diva-providers/src/base.rs` (紧邻 Message)
- `FileAttachmentRef` → `agent-diva-core/src/attachment.rs` (紧邻 FileAttachment)

**兼容性:**
- 保持 `Message::user("text")` 构造函数工作 (映射为 `MessageContent::Text`)
- 旧 JSONL 中 `"content": "some text"` 自动反序列化为 `Text` 变体
- 新消息可写 `"content": {"Parts": [...]}`

### IMG-PRE-1: 会话与消息类型 (修订)

**新增改动 (跨项目发现):**
- 除 `FileAttachmentRef` 外，还需添加 `ModelCapabilities`:
  ```rust
  pub struct ModelCapabilities {
      pub vision: bool,
      pub tools: bool,
      pub reasoning: bool,
  }
  ```
  (参考 Hermes 的 `agent/models_dev.py:401-408`)
- Provider catalog 需要 capability 查询 (参考 Hermes 的 `supports_vision` 三级优先级: config override > per-model > models.dev DB)

### IMG-PRE-2: Agent Loop 图片组装 (修订)

**参考 Codex 做法:**
- `From<Vec<UserInput>>` 风格: 在 context assembly 时把 `InboundMessage.media` 文件 ID 转换为 `MessageContentPart::ImageFile`
- 调用 `FileManager::get_bytes()` 读取图片数据
- 用于 token 计数时: 图片固定 2000 tokens (Claude Code 策略) 或按像素估算 (Codex 策略)

**参考 Hermes 做法:**
- 检测当前模型的 `ModelCapabilities.vision`:
  - 支持 → 构建 `MessageContent::Parts([...text_parts, ...image_parts])`
  - 不支持 → 保持 `MessageContent::Text` + 警告文本 "image content omitted" (Codex 的 `strip_images_when_unsupported`)

### IMG-PRE-3: Provider 序列化 (修订)

**参考所有项目验证后的策略:**

每个 provider adapter 各自把 `MessageContentPart` 转为原生 API JSON:

| Provider | ImageFile 序列化方式 | 参考 |
|----------|---------------------|------|
| OpenAI-compatible | `{"type": "image_url", "image_url": {"url": "data:..."}}` | GenericAgent vision_api, OpenFang |
| Anthropic | `{"type": "image", "source": {"type": "base64", "media_type": "...", "data": "..."}}` | Claude Code, Hermes |
| Ollama (vision models) | 同 OpenAI-compatible 格式 | Hermes chat_completion_helpers |

**模型 ID 安全规则:**
- native provider endpoint: 保持 raw model ID，不加前缀
- LiteLLM-style gateway: 才加 `provider/model` 前缀
- 写 assertion tests

### IMG-PRE-4: GUI 最少改动 (修订)

**参考 Claude Code 和 Hermes 的发现:**
- GUI 不需要改上传逻辑 (uploadFile 已存在)
- 最小改动: 
  1. `ChatView.vue` 识别图片附件并显示缩略图 chip
  2. 发送消息时附带 `media: [fileId, ...]` 到 backend
  3. 非 vision 模型时禁用/灰化图片附加按钮
  4. 历史消息重载时显示 "包含 1 张图片" 标记

---

## 五、兄弟项目的具体代码引用索引

详细的每个项目分析见:
- `docs/dev/multimodal/claude-code-vision-analysis.md` (528 行)
- `docs/dev/multimodal/hermes-agent-vision-analysis.md` (522 行)  
- `docs/dev/multimodal/genericagent-opanfang-vision-analysis.md` (568 行)

### Codex 关键文件

| 功能 | 路径 | 说明 |
|------|------|------|
| InputItem union | `.workspace/codex/sdk/python/src/codex_app_server/_inputs.py` | TextInput / ImageInput / LocalImageInput |
| ContentItem enum | `.workspace/codex/codex-rs/protocol/src/models.rs:403` | InputImage { image_url, detail } |
| UserInput → ContentItem | `.workspace/codex/codex-rs/protocol/src/user_input.rs` | 同步读文件 + 图片处理 |
| 图片处理 | `.workspace/codex/codex-rs/utils/image/src/lib.rs` | MAX_DIMENSION=2048, LRU cache |
| 图片 stripping | `.workspace/codex/codex-rs/core/src/context_manager/normalize.rs:295` | strip_images_when_unsupported |
| view_image tool | `.workspace/codex/codex-rs/core/src/tools/handlers/view_image.rs` | 模型主动请求查看图片 |

### Claude Code 关键文件

| 功能 | 路径 | 说明 |
|------|------|------|
| 7步管线 | `src/utils/processUserInput/processUserInput.ts:290-604` | 完整流程入口 |
| ContentBlock 类型 | `src/services/api/claude.ts:1-19` | BetaImageBlockParam |
| 图片缩放 | `src/utils/imageResizer.ts:179-448` | 渐进降级到 sharp |
| Image #N 引用 | `src/history.ts:58-75` | [Image #N] format |
| validateImagesForAPI | `src/utils/imageValidation.ts:65-104` | 5MB base64 check |
| token 估算 | `src/services/tokenEstimation.ts:467-478` | 2000 tokens/image |

### Hermes Agent 关键文件

| 功能 | 路径 | 说明 |
|------|------|------|
| 路由决策 | `agent/image_routing.py:287-317` | decide_image_input_mode |
| native 内容构建 | `agent/image_routing.py:418-504` | build_native_content_parts |
| ModelCapabilities | `agent/models_dev.py:401-408` | supports_vision 等字段 |
| 图片生成抽象 | `agent/image_gen_provider.py:51-143` | ImageGenProvider ABC |
| vision_analyze tool | `tools/vision_tools.py:716-997` | text 回退路径 |
| 响应式图片缩小 | `tools/vision_tools.py:344-447` | _resize_image_for_vision |
| Anthropic 翻译 | `agent/anthropic_adapter.py:1510-1523` | image_url → image block |

---

## 六、风险与缓解措施 (与原有计划一致，补充新发现)

| 风险 | 原计划缓解 | 跨项目验证补充 |
|------|-----------|--------------|
| 图片字节膨胀会话历史 | 只存 file_id + metadata | 参考 Claude Code: 图片不存 history.jsonl，独立 image-cache |
| text-only provider 崩溃 | MessageContent::Text 兼容路径 | 参考 Codex: strip_images_when_unsupported 替换占位文本 |
| 图片+文本在不同消息 | 合并到同一 user message | 参考 OpenFang fix #1043: 明确修复了此 bug |
| provider model ID 被错误改写 | 测试最终 model 值 | 参考所有项目: 每个 adapter 各自处理，不改动 model 字段 |
| 大图片超限 | max image bytes/config guard | 参考 Claude Code: 5MB base64 limit + 2000px max + 渐进降级 |

---

## 七、总结

跨项目对比验证了原有 IMG-PRE 计划的正确性。关键确认:

1. **`Message.content: String → enum MessageContent` 是正确的首要改动** — Codex/ClaudeCode/Hermes/OpenFang 全部使用 typed content
2. **ModelCapabilities.vision guard 是必须的** — 所有项目都有这个判断
3. **IMG-PRE 保持最小垂直切片是正确的策略** — GenericAgent 的反面案例证明半成品状态最危险
4. **GUI 已有足够的基建 (uploadFile + FileManager)** — 不需要重写前端，只需要串联已有组件

下一步: 开始 IMG-PRE-0，在 `agent-diva-providers/src/base.rs` 中定义 `MessageContentPart` / `MessageContent` 枚举。
