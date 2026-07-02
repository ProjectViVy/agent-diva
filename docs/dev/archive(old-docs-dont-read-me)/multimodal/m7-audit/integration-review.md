# M7 集成兼容性审计报告

> **审计范围**: agent-diva 多模态图片识别功能 (M1~M6)
> **审计日期**: 2026-06-01
> **审计人**: 系统集成与兼容性审计专家
> **涉及 crate**: agent-diva-core, agent-diva-agent, agent-diva-providers, agent-diva-gui

---

## 审计摘要

本报告对 agent-diva 多模态图片识别功能的 12 个关键文件进行集成兼容性审计，覆盖数据类型定义、序列化契约、跨 crate 数据流、前后端接口一致性等维度。共审查 10 个问题，给出逐一结论。

| 序号 | 审查项 | 结论 |
|------|--------|------|
| 1 | 旧 session JSONL 反序列化兼容 | ✅ 通过 |
| 2 | FileAttachmentRef 序列化 round-trip | ✅ 通过 |
| 3 | text-only provider 路径不受影响 | ✅ 通过 |
| 4 | InboundMessage → agent loop → provider 数据流 | ✅ 通过 |
| 5 | GUI 附件 chip 区分图片/普通文件 | ✅ 通过 |
| 6 | GUI vision 白名单与后端 capability check 一致性 | ⚠️ 关注 |
| 7 | 前端警告与后端 guard 的 gap | ⚠️ 关注 |
| 8 | 跨 crate FileAttachment → FileAttachmentRef → ImageFile 转换 | ✅ 通过 |
| 9 | reload session 后附件元数据保留 | ✅ 通过 |
| 10 | GUI payload attachments 格式与后端期望一致性 | ✅ 通过 |

**整体集成兼容性评分: 8.5 / 10**

---

## 逐项审查

### 1. ChatMessage.attachments 新增字段：旧 session JSONL（无 attachments）是否能正常反序列化？

**结论: ✅ 通过**

**证据:**

`agent-diva-core/src/session/store.rs:112-113` 中 `attachments` 字段使用了 `#[serde(skip_serializing_if = "Option::is_none", default)]` 注解：

```rust
#[serde(skip_serializing_if = "Option::is_none", default)]
pub attachments: Option<Vec<FileAttachmentRef>>,
```

- `default` 属性确保反序列化时若 JSON 中不存在 `attachments` 字段，自动填充为 `None`
- `skip_serializing_if = "Option::is_none"` 确保旧消息序列化时不产生冗余的 `null` 字段

`store.rs:209-219` 中有明确的回归测试验证此行为：

```rust
#[test]
fn test_chat_message_deserializes_old_json_without_attachments() {
    let json = r#"{
        "role": "user",
        "content": "hello",
        "timestamp": "2026-06-01T00:00:00Z"
    }"#;
    let message: ChatMessage = serde_json::from_str(json).unwrap();
    assert_eq!(message.attachments, None);
}
```

`SessionManager::load()` (`manager.rs:56-100`) 通过 `serde_json::from_value::<ChatMessage>` 逐行解析 JSONL，对旧格式消息完全兼容。

---

### 2. FileAttachmentRef 序列化 round-trip：serde 是否支持所有字段？

**结论: ✅ 通过**

**证据:**

`attachment.rs:45-59` 中 `FileAttachmentRef` 派生了 `Serialize` 和 `Deserialize`，包含 4 个字段：

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileAttachmentRef {
    pub file_id: String,
    pub filename: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mime_type: Option<String>,
    pub size: u64,
}
```

`store.rs:223-243` 中 round-trip 测试覆盖了序列化和反序列化双向：

```rust
#[test]
fn test_chat_message_attachment_round_trip() {
    // ... 构造 message with attachments
    let json = serde_json::to_string(&message).unwrap();
    assert!(json.contains("\"attachments\""));
    assert!(!json.contains("base64"));
    assert!(!json.contains("bytes"));

    let decoded: ChatMessage = serde_json::from_str(&json).unwrap();
    assert_eq!(decoded.attachments, message.attachments);
}
```

验证要点：
- 序列化产出包含 `file_id`, `filename`, `mime_type`, `size`
- 不包含 `base64`, `bytes`, `preview` 等不应持久化的数据
- 反序列化后数据完整性保持一致

---

### 3. text-only provider 路径完全不受影响？Message::user("text") 老构造仍可用？

**结论: ✅ 通过**

**证据:**

`base.rs:389-400` 中 `Message::user` 接受 `impl Into<MessageContent>`：

```rust
pub fn user(content: impl Into<MessageContent>) -> Self {
    Self {
        role: "user".to_string(),
        content: content.into(),
        // ... 其他字段均为 None
    }
}
```

`base.rs:330-346` 中 `String` 和 `&str` 均实现了 `Into<MessageContent>`，转换结果为 `MessageContent::Text`：

```rust
impl From<String> for MessageContent {
    fn from(value: String) -> Self { Self::Text(value) }
}
impl From<&str> for MessageContent {
    fn from(value: &str) -> Self { Self::Text(value.to_string()) }
}
```

`litellm.rs:482-525` 中 `sanitize_messages` 对 `Text` 和 `Parts` 两种变体均正确处理，text-only 路径不会触碰图片逻辑。

`context.rs:230-243` 中 `build_messages` 仍使用 `MessageContent::Text` 包装当前消息，与旧路径完全兼容。

---

### 4. InboundMessage.media → agent loop → provider message 的数据流是否一致？

**结论: ✅ 通过**

**证据:**

完整数据流追踪：

1. **入口**: `InboundMessage.media: Vec<String>` (`events.rs:65`) — 存储 file_id 列表
2. **组装**: `assemble_current_message_content(&file_manager, &msg.content, &msg.media)` (`loop_turn.rs:53-54`)
   - 图片 MIME → `MessageContentPart::ImageFile { image_file: ImageFile { file_id } }` (`loop_turn.rs:721-726`)
   - 文本 MIME → 内联文本 (`loop_turn.rs:729-743`)
   - 二进制 MIME → 占位符文本 (`loop_turn.rs:754-758`)
3. **上下文构建**: `build_messages_with_content(history, message_content, ...)` (`loop_turn.rs:68-73`) — `MessageContent::Parts` 直接传递到 `Message.user(content)`
4. **Vision 准备**: `prepare_messages_for_openai_vision` (`loop_turn.rs:159-173`)
   - `ImageFile` → 读取文件 → base64 → `ImageUrl { url: data_uri }` (`loop_turn.rs:612-614`)
   - `ImageData` → `ImageUrl { url: data_uri }` (`loop_turn.rs:618-621`)
5. **Provider 序列化**: `LiteLLMClient::build_request` (`litellm.rs:527-553`) — `Message` 被 `serde_json::to_value` 序列化为 OpenAI 兼容格式

`litellm.rs:1522-1562` 中测试验证了最终序列化产出：

```rust
// 最终 JSON: {"type": "image_url", "image_url": {"url": "data:image/png;base64,AAAA"}}
assert_eq!(value["messages"][0]["content"][1]["type"], "image_url");
assert!(!value.to_string().contains("image_file"));
```

`base.rs:254-258` 中 `MessageContent` 使用 `#[serde(untagged)]` 确保 `Text` 序列化为字符串、`Parts` 序列化为数组，与 OpenAI API 格式完全一致。

---

### 5. GUI 附件 chip：是否正确区分图片附件和普通文件？

**结论: ✅ 通过**

**证据:**

`ChatView.vue:129-130` 中定义了图片判断函数：

```typescript
const isImageAttachment = (attachment: FileAttachmentDto) =>
  attachment.mime_type?.toLowerCase().startsWith('image/') ?? false;
```

**消息展示区 chip** (`ChatView.vue:485-497`):

```html
<div
  :class="isImageAttachment(attachment)
    ? 'border-pink-200 bg-pink-50/80 text-pink-700'
    : 'border-gray-200 bg-white/70 text-gray-600'"
>
  <ImageIcon v-if="isImageAttachment(attachment)" :size="13" />
  <FileIcon v-else :size="13" />
</div>
```

**Composer 预览区 chip** (`ChatView.vue:528-547`):

```html
<div
  :class="isImageAttachment(attachment)
    ? 'border-pink-200 bg-pink-50 text-pink-700'
    : 'border-gray-200 bg-gray-100 text-gray-700'"
>
  <ImageIcon v-if="isImageAttachment(attachment)" :size="12" />
  <Paperclip v-else :size="12" />
</div>
```

区分逻辑一致：
- 图片: 粉色边框/背景 + `ImageIcon` 图标 + 标签"图片"
- 普通文件: 灰色边框/背景 + `FileIcon`/`Paperclip` 图标 + 标签"文件"
- 标签文本由 `attachmentLabel()` (`ChatView.vue:147-148`) 生成

---

### 6. GUI vision 模型白名单与后端 capability check 是否一致？

**结论: ⚠️ 关注**

**证据:**

**前端白名单** (`ChatView.vue:119`):

```typescript
const visionModels = new Set(['gpt-4o', 'gpt-4o-mini', 'gpt-4.1', 'gpt-4.1-mini']);
```

**后端白名单** (`base.rs:55-65`):

```rust
pub fn model_capabilities_for_model(model: &str) -> ModelCapabilities {
    let normalized = normalize_model_id(model);
    capabilities.vision = matches!(
        normalized.as_str(),
        "gpt-4o" | "gpt-4o-mini" | "gpt-4.1" | "gpt-4.1-mini"
    );
}
```

**一致性分析:**
- 两者使用相同的模型列表和相同的 `normalize_model_id` 逻辑（剥离 `provider/` 前缀，取最后一段）
- 前端 `normalizeModelId` (`ChatView.vue:121-125`) 与后端 `normalize_model_id` (`base.rs:72-78`) 行为一致

**关注点:**

1. **Claude 模型缺失**: Claude 3.5 Sonnet、Claude 3 Opus 等模型支持 vision，但未列入任一白名单。使用 Claude 模型时，图片附件会被降级为文本处理。
2. **维护负担**: 两处独立维护同一份白名单，存在漂移风险。未来新增 vision 模型需同步修改两处。
3. **Gemini/Grok 等模型**: 同样支持 vision 但未列入。

**建议**: 将 vision 能力检测统一到后端，前端通过 API 查询而非硬编码。

---

### 7. 前端警告（text-only 模型时提示）和后端 guard（阻止发送 image payload）之间有无 gap？

**结论: ⚠️ 关注**

**证据:**

**前端行为** (`ChatView.vue:134-136`):

```typescript
const showVisionWarning = computed(
  () => hasImageAttachments.value && !supportsVisionModel(props.currentModel)
);
```

前端仅显示警告横幅 (`ChatView.vue:549-553`)：

```html
<div v-if="showVisionWarning" class="...amber...">
  当前模型可能无法识别图片，请切换到 gpt-4o / gpt-4.1 系列 vision 模型，或发送文字描述。
</div>
```

**前端不阻止发送** — `handleSend()` (`ChatView.vue:236-247`) 没有任何 vision 模型检查，消息和附件会照常发出。

**后端行为** (`loop_turn.rs:577-599`):

```rust
async fn prepare_messages_for_openai_vision(...) {
    if !messages.iter().any(Message::has_image_content) {
        return Ok(messages);  // 无图片内容则跳过
    }
    if !supports_vision_model(model) {
        return Err(VisionMessagePreparationError::UnsupportedModel { model });
    }
    // ... 解析图片
}
```

后端在检测到图片内容 + text-only 模型时返回错误，错误消息 (`loop_turn.rs:533-534`)：

> "This model cannot inspect images. Please switch to a vision-capable model or send a text description of the image."

**Gap 分析:**

| 维度 | 前端 | 后端 |
|------|------|------|
| 检测时机 | 附件选择时 | LLM 调用前 |
| 阻止方式 | 仅警告（不阻止发送） | 返回错误消息 |
| 用户体验 | 黄色横幅提示 | 对话中出现系统错误消息 |
| 措辞 | "可能无法识别"（语气弱） | "cannot inspect"（语气强） |

**影响**: 用户选择图片附件 + text-only 模型 → 发送 → 后端返回错误 → 用户在对话中看到错误消息。体验不够流畅，但不会导致数据损坏或静默丢失。

**建议**:
1. 前端在 vision 模型白名单匹配时直接阻止发送（禁用发送按钮 + 更明确的提示）
2. 或统一前后端白名单来源，避免维护漂移

---

### 8. 跨 crate 接口契约：FileAttachment → FileAttachmentRef → ImageFile 转换是否正确？

**结论: ✅ 通过**

**证据:**

转换链路：

```
FileAttachment (attachment.rs:96-126)
    ↓ From<&FileAttachment> for FileAttachmentRef (attachment.rs:73-82)
FileAttachmentRef (attachment.rs:45-59)
    ↓ resolve_attachment_refs() (loop_turn.rs:936-962)
    ↓ assemble_current_message_content() (loop_turn.rs:692-791)
MessageContentPart::ImageFile { image_file: ImageFile { file_id } } (base.rs:361)
    ↓ prepare_messages_for_openai_vision() (loop_turn.rs:577-599)
MessageContentPart::ImageUrl { image_url: ImageUrl { url: data_uri } } (base.rs:359)
```

**FileAttachment → FileAttachmentRef** (`attachment.rs:73-82`):

```rust
impl From<&FileAttachment> for FileAttachmentRef {
    fn from(attachment: &FileAttachment) -> Self {
        Self {
            file_id: attachment.file_id.clone(),
            filename: attachment.filename.clone(),
            mime_type: attachment.mime_type.clone(),
            size: attachment.size,
        }
    }
}
```

正确保留 4 个会话所需字段，丢弃 `channel`, `message_id`, `uploaded_by`, `stored_at`, `ref_count` 等运行时元数据。

**FileAttachmentRef → ImageFile** (`loop_turn.rs:721-726`):

```rust
if mime_type.starts_with("image/") {
    image_parts.push(MessageContentPart::ImageFile {
        image_file: ImageFile {
            file_id: handle.id.clone(),
        },
    });
}
```

`ImageFile` (`base.rs:379-382`) 仅携带 `file_id`，这是 OpenAI `image_file` 格式所需的唯一标识符。

**ImageFile → ImageUrl** (`loop_turn.rs:612-614`):

```rust
MessageContentPart::ImageFile { image_file } => {
    let url = resolve_image_file_to_data_uri(file_manager, &image_file.file_id).await?;
    resolved_parts.push(MessageContentPart::ImageUrl {
        image_url: ImageUrl { url },
    });
}
```

完整路径：file_id → FileManager.get() → read bytes → base64 encode → data URI。包含 MIME 类型校验（仅支持 `image/png`, `image/jpeg`, `image/webp`）和大小校验（≤ 5MB）。

---

### 9. 附件元数据在 reload session 后是否正确保留？

**结论: ✅ 通过**

**证据:**

**持久化路径** (`manager.rs:103-126`):

```rust
pub fn save(&self, session: &Session) -> crate::Result<()> {
    // ...
    for msg in &session.messages {
        lines.push(serde_json::to_string(msg)?);  // ChatMessage 完整序列化
    }
    std::fs::write(&path, lines.join("\n"))?;
}
```

`ChatMessage` 序列化包含 `attachments` 字段（当非 None 时）。

**加载路径** (`manager.rs:56-100`):

```rust
if let Ok(msg) = serde_json::from_value::<super::store::ChatMessage>(value) {
    messages.push(msg);  // 包括 attachments 字段
}
```

**端到端测试** (`manager.rs:359-395`):

```rust
#[test]
fn test_save_and_load_session_with_attachment_metadata() {
    // 写入含附件的 session
    session.add_full_message(ChatMessage::with_attachments(
        "user", "please inspect this",
        vec![FileAttachmentRef {
            file_id: "sha256:image123".to_string(),
            filename: "image.png".to_string(),
            mime_type: Some("image/png".to_string()),
            size: 4096,
        }],
    ));
    manager.save(&manager.cache.get(&key).unwrap()).unwrap();

    // 验证 JSONL 内容
    assert!(content.contains("\"attachments\""));
    assert!(content.contains("\"file_id\":\"sha256:image123\""));
    assert!(!content.contains("base64"));
    assert!(!content.contains("bytes"));

    // 清缓存后重新加载
    manager.cache.clear();
    let loaded = manager.get_or_create(&key);
    let attachment = &loaded.messages[0].attachments.as_ref().unwrap()[0];
    assert_eq!(attachment.file_id, "sha256:image123");
    assert_eq!(attachment.filename, "image.png");
    assert_eq!(attachment.mime_type, Some("image/png".to_string()));
    assert_eq!(attachment.size, 4096);
}
```

**GUI 端加载** (`App.vue:423-451`):

```typescript
function mapBackendMessageToUi(msg: BackendChatMessage): Message | null {
    return {
        // ...
        attachments: msg.attachments ?? undefined,  // 直接传递
    };
}
```

`BackendChatMessage.attachments` (`App.vue:101`) 类型为 `FileAttachmentDto[] | null`，与后端 `ChatMessage.attachments: Option<Vec<FileAttachmentRef>>` 的 JSON 结构匹配。

---

### 10. GUI 发送 payload 的 attachments 字段格式与后端期望是否一致？

**结论: ✅ 通过**

**证据:**

**GUI 发送流程** (`App.vue:578-636`):

```typescript
async function sendMessage(content: string, attachments?: FileAttachmentDto[]) {
    const attachmentFileIds = attachments?.map(a => a.file_id);  // 提取 file_id 数组

    await invoke("send_message", {
        message: content,
        channel: currentChannel.value,
        chatId: currentChatId.value,
        attachments: attachmentFileIds,  // string[] 类型
        streamRequestId,
    });
}
```

GUI 将 `FileAttachmentDto[]` 转换为 `string[]`（仅 file_id），传递给 Tauri `send_message` 命令。

**后端接收** (`loop_turn.rs:53-54`):

```rust
let message_content =
    assemble_current_message_content(&self.file_manager, &msg.content, &msg.media).await;
```

`msg.media` 即 `InboundMessage.media: Vec<String>`，存储的是 file_id 列表。

**FileAttachmentDto 类型定义** (`desktop.ts:20-30`):

```typescript
export interface FileAttachmentDto {
    file_id: string;
    filename: string;
    size: number;
    mime_type?: string | null;
    channel?: string;
    message_id?: string | null;
    uploaded_by?: string | null;
    stored_at?: string;
    ref_count?: number;
}
```

此 DTO 用于 GUI 展示（chip 渲染），发送时仅提取 `file_id`。后端通过 `FileManager.get(file_id)` 获取完整文件元数据和内容，不依赖前端传递的其他字段。

**上传路径** (`desktop.ts:186-187`):

```typescript
export const uploadFile = (fileName: string, bytes: number[], channel: string, messageId?: string) =>
    invoke<FileAttachmentDto>("upload_file", { fileName, bytes, channel, messageId });
```

上传返回 `FileAttachmentDto`，前端存储后在发送时提取 `file_id`。整个链路：上传 → 获取 DTO → 展示 chip → 发送 file_id → 后端通过 file_id 读取文件。

---

## 整体评估

### 评分: 8.5 / 10

### 架构优势

1. **向后兼容设计**: `#[serde(default)]` + `skip_serializing_if` 确保旧 session 文件无缝兼容
2. **关注点分离**: `FileAttachment`（运行时完整元数据）与 `FileAttachmentRef`（会话持久化轻量引用）分层清晰
3. **内容寻址存储**: SHA256 作为 file_id，天然去解耦文件名与存储
4. **防御性编程**: vision 准备阶段有 MIME 校验、大小校验、缺失文件处理，错误消息用户友好
5. **数据流干净**: `ImageFile` → `ImageUrl` 转换在 agent loop 层完成，provider 层只处理标准 OpenAI 格式

### 关键建议

| 优先级 | 建议 | 影响范围 |
|--------|------|----------|
| **高** | 将 vision 模型白名单统一到后端 API，前端通过查询获取而非硬编码 | 前端+后端 |
| **高** | 前端在 text-only 模型 + 图片附件时阻止发送（禁用按钮），而非仅警告 | GUI |
| **中** | 补充 Claude、Gemini 等主流 vision 模型到白名单 | 后端 `base.rs` |
| **低** | 考虑将 `supports_vision_model` 暴露为 Tauri 命令，供前端实时查询 | 架构 |

### 风险矩阵

| 风险项 | 概率 | 影响 | 等级 |
|--------|------|------|------|
| 前后端 vision 白名单漂移 | 中 | 中 | 🟡 中 |
| 用户在 text-only 模型发送图片后体验不佳 | 高 | 低 | 🟡 中 |
| 旧 session 文件反序列化失败 | 极低 | 高 | 🟢 低 |
| 跨 crate 类型转换数据丢失 | 极低 | 高 | 🟢 低 |

---

## 附录：审查文件清单

| # | 文件路径 | 关注维度 |
|---|----------|----------|
| 1 | `agent-diva-core/src/attachment.rs` | FileAttachment/FileAttachmentRef 类型定义 |
| 2 | `agent-diva-core/src/session/store.rs` | ChatMessage 扩展 + JSONL 序列化 |
| 3 | `agent-diva-core/src/session/manager.rs` | 会话持久化/加载 |
| 4 | `agent-diva-core/src/bus/events.rs` | InboundMessage.media |
| 5 | `agent-diva-agent/src/agent_loop/loop_turn.rs` | agent loop 附件处理核心 |
| 6 | `agent-diva-agent/src/context.rs` | 上下文组装 |
| 7 | `agent-diva-providers/src/base.rs` | Message/MessageContent 类型 |
| 8 | `agent-diva-providers/src/litellm.rs` | Provider 序列化 |
| 9 | `agent-diva-gui/src/components/ChatView.vue` | GUI 附件 chip 展示 |
| 10 | `agent-diva-gui/src/components/NormalMode.vue` | Composer 附件预览 |
| 11 | `agent-diva-gui/src/App.vue` | 整体集成 + session 加载 |
| 12 | `agent-diva-gui/src/api/desktop.ts` | API 类型定义 |
