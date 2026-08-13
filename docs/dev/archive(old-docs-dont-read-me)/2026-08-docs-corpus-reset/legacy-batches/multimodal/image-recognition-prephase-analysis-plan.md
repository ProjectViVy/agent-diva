# Image Recognition Prephase Analysis Plan

## Background

Before the Phase A-PRE full-link session durability work, there is a useful
side branch for image recognition capability. The immediate goal is not broad
"multimodal everything"; it is specifically:

```text
Users can attach or send images, and agent-diva can pass those images to a
vision-capable model as first-class input instead of degrading them into text
placeholders.
```

This should be done before Phase A-PRE only if it stays small and foundational:
define the message/session/provider shape needed for image inputs, then ship a
minimal vertical slice. Large channel rewrites, video/audio, image generation,
and realtime media should remain out of scope.

## Initial Findings

### Current agent-diva State

agent-diva already has several useful building blocks:

- `agent-diva-core/src/attachment.rs` defines `FileAttachment` and MIME helpers
  such as `is_image`.
- `agent-diva-files` stores uploaded files in content-addressed storage and can
  identify common image MIME types.
- `agent-diva-manager/src/file_service.rs` and GUI `uploadFile` already support
  file uploads and return file IDs.
- `agent-diva-gui/src/components/ChatView.vue` already allows attaching files
  to a message.
- `agent-diva-core/src/bus/events.rs` carries `InboundMessage.media` and
  `OutboundMessage.media`.
- `agent-diva-agent/src/agent_loop/loop_turn.rs` loads attachments, but current
  behavior only inlines small text files and emits a textual placeholder for
  non-text files.

The main gap is structural: `agent-diva-providers::Message` currently stores
`content: String`. That forces images to become text markers before provider
routing. Vision models need structured content blocks.

### Reference Project Observations

The `.workspace` examples point toward a clear design direction:

- Codex models user input as typed parts: `Text`, `Image { url }`, and
  `LocalImage { path }`. It also preserves local image references in thread
  history and analytics counts image inputs separately.
- Claude Code uses typed message content (`string | ContentBlock[]`) and keeps
  attachment/progress messages as explicit transcript items, not just inline
  text.
- OpenFang notes a recent fix where multimodal user messages combine text and
  image blocks into one LLM message so the model sees both together.
- GenericAgent has a simple `put_task(query, images=None)` shape, which is easy
  to reason about but too loose for agent-diva's typed Rust boundaries.
- Hermes has rich media/channel handling, but it is broader than this slice.
  Its relevant lesson is to keep media storage, channel upload/download, and
  model-facing vision input as separate layers.

## Product Shape

The first image-recognition slice should support these user-visible behaviors:

1. GUI user attaches one or more images with a text prompt.
2. Agent receives the prompt and images in the same turn.
3. If the active model/provider supports vision, the request is sent as
   structured text + image content.
4. If the active model/provider does not support vision, the user gets a clear
   fallback message instead of a confusing placeholder-only response.
5. Session history preserves enough metadata to reload the image attachment and
   explain what was sent.

The core experience to validate:

```text
User: "这张图里是什么？" + image.png
Assistant: provides image-based analysis, not "use read_file tool".
```

## Non Goals

- No audio transcription work.
- No video understanding.
- No image generation.
- No OCR-only pipeline as the primary path.
- No full media gallery or asset manager.
- No large Phase A-PRE session durability rewrite in this branch.
- No channel-by-channel media rewrite beyond the minimum needed to preserve
  image references.
- No attempt to make every provider vision-capable.

## Proposed Architecture

### 1. Introduce Model-Facing Content Blocks

Add a provider-neutral message content model, likely in
`agent-diva-providers` or `agent-diva-core` depending on ownership:

```rust
pub enum MessageContentPart {
    Text { text: String },
    ImageUrl { url: String },
    ImageFile { file_id: String, mime_type: Option<String> },
    ImageData { mime_type: String, data_base64: String },
}

pub struct Message {
    pub role: String,
    pub content: MessageContent,
    // existing tool/reasoning fields remain
}

pub enum MessageContent {
    Text(String),
    Parts(Vec<MessageContentPart>),
}
```

Keep `Message::user("text")` working by mapping it to `Text`. This preserves
existing text-only call sites while allowing image-capable paths to opt in.

### 2. Preserve Images In Session History

Extend `ChatMessage` with optional attachment metadata rather than burying image
references inside `content`:

```rust
pub struct ChatMessage {
    pub role: String,
    pub content: String,
    pub attachments: Option<Vec<FileAttachmentRef>>,
}
```

For the first slice, store file IDs, filename, MIME type, and size. Avoid
storing base64 image bytes in session JSONL.

### 3. Convert Attachments During Context Assembly

Move image conversion out of raw prompt text concatenation:

```text
InboundMessage.media file IDs
  -> FileManager metadata lookup
  -> text attachments may still inline
  -> image attachments become MessageContentPart::ImageFile or ImageData
  -> provider adapter serializes to native API format
```

This lets the model see text and image blocks in one user message, matching the
OpenFang lesson.

### 4. Add Provider Capability Metadata

Provider/model catalog entries need a lightweight capability flag:

```rust
pub struct ModelCapabilities {
    pub vision: bool,
    pub tools: bool,
    pub reasoning: bool,
}
```

The first pass can use static rules for known models and a conservative default
of `vision = false`. This prevents accidentally sending image blocks to text-only
models.

### 5. Implement OpenAI-Compatible Vision Serialization First

The smallest provider slice is OpenAI-compatible chat/completions style:

```json
{
  "role": "user",
  "content": [
    { "type": "text", "text": "这张图里是什么？" },
    {
      "type": "image_url",
      "image_url": {
        "url": "data:image/png;base64,..."
      }
    }
  ]
}
```

Do not rewrite raw model IDs while doing this. The provider model-ID safety rule
still applies: native provider endpoints keep raw model IDs; LiteLLM-style
prefix rewriting only happens for true gateway/aggregator routing.

### 6. GUI Attachment UX Minimum

The GUI already uploads files. The first visible improvement should be narrow:

- show image thumbnails or at least image-specific attachment chips before send;
- include attachment metadata in the local optimistic message;
- after backend reload, show that a historical user message had image
  attachments;
- disable or warn when current provider/model is not vision-capable.

This branch does not need a full media viewer.

## Analysis Plan

### Step 1: Confirm Current Image Path

Files to inspect:

- `agent-diva-gui/src/components/ChatView.vue`
- `agent-diva-gui/src/App.vue`
- `agent-diva-manager/src/handlers.rs`
- `agent-diva-manager/src/file_service.rs`
- `agent-diva-agent/src/agent_loop/loop_turn.rs`
- `agent-diva-core/src/bus/events.rs`
- `agent-diva-core/src/session/store.rs`
- `agent-diva-providers/src/base.rs`
- `agent-diva-providers/src/litellm.rs`
- `agent-diva-providers/src/ollama.rs`

Questions:

- Are GUI attachment file IDs always available to the agent loop?
- Is MIME type recoverable from `FileManager` at prompt assembly time?
- Where should file IDs be persisted so history reload can reconstruct image
  attachments?
- Which provider adapter should be first: OpenAI-compatible, Ollama vision, or
  both?

### Step 2: Define The Typed Message Contract

Decide:

- whether `MessageContentPart` lives in `agent-diva-providers` or
  `agent-diva-core`;
- whether provider messages keep `content: String` plus optional
  `content_parts`, or migrate to an enum;
- how to serialize/deserialize without breaking existing tests;
- how tool calls and assistant reasoning continue to work with text-only
  assistant messages.

Recommendation: use an enum with backwards-compatible constructors and
serialization tests.

### Step 3: Define Session Attachment Persistence

Decide:

- minimal `FileAttachmentRef` fields;
- JSONL compatibility for old messages without attachments;
- whether attachment refs are copied from inbound message to the saved user
  `ChatMessage`;
- how `get_history()` reconstructs model-facing image blocks.

Recommendation: keep binary image bytes out of session JSONL and store only
file IDs plus metadata.

### Step 4: Provider Capability And Fallback

Decide:

- static capability map for built-in providers/models;
- custom provider setting for `vision = true`;
- GUI/API exposure for whether the selected model supports image input;
- fallback behavior for text-only models.

Recommended fallback:

```text
This model cannot inspect images. Please switch to a vision-capable model or
send a text description of the image.
```

### Step 5: OpenAI-Compatible Serialization Spike

Implement or prototype:

- convert `ImageFile` to data URI by reading bytes from `FileManager`;
- serialize `MessageContent::Parts` into OpenAI-compatible content array;
- retain plain string content for text-only messages;
- add tests that assert final outbound JSON contains both text and image blocks
  in the same user message.

### Step 6: GUI Smoke Path

Validate:

- upload an image through GUI;
- send prompt + image;
- backend receives file ID in `InboundMessage.media`;
- provider request contains an image content block;
- session history reload still shows the user message and attachment.

## Recommended Implementation Slices

### IMG-PRE-0: Research And Contract

Deliverables:

- this analysis document;
- code map with exact call sites;
- final `MessageContentPart` and `FileAttachmentRef` shape.

Acceptance:

- no behavior changes;
- contract is clear enough to implement without revisiting scope.

### IMG-PRE-1: Session And Message Types

Deliverables:

- typed message content parts;
- session attachment refs;
- compatibility tests for old text-only messages.

Acceptance:

- existing text-only provider tests still pass;
- session JSONL can read old messages and write new messages with attachments.

### IMG-PRE-2: Agent Loop Image Assembly

Deliverables:

- image attachments become structured image parts;
- non-image files keep current text inline/placeholder behavior;
- text prompt and images remain in one user message.

Acceptance:

- unit test: prompt + one PNG file produces text + image parts;
- text-only attachment behavior remains unchanged.

### IMG-PRE-3: OpenAI-Compatible Vision Request

Deliverables:

- OpenAI-compatible adapter serializes image parts;
- model capability guard prevents sending images to unsupported models;
- tests assert request JSON shape and raw model ID preservation.

Acceptance:

- vision model request contains `content: [{type:text}, {type:image_url}]`;
- native provider model IDs are not prefixed incorrectly.

### IMG-PRE-4: GUI And Manager Minimum

Deliverables:

- image attachment metadata shown in pending and reloaded messages;
- provider capability warning or disabled send path for image + text-only model;
- manual smoke path documented.

Acceptance:

- user can attach an image and ask a question in GUI;
- after reload, the message still shows image attachment metadata.

## Risks And Controls

| Risk | Control |
| --- | --- |
| Image bytes inflate session history | Store file IDs and metadata only |
| Existing text-only providers break | Keep text constructors and string serialization path |
| Image sent without prompt context | Combine text and image blocks in one user message |
| Unsupported models receive invalid payloads | Add model capability gate |
| Native provider model IDs get rewritten | Add request-shape tests for final `model` value |
| GUI local optimistic state diverges | Keep this branch minimal, then let Phase A-PRE harden canonical history |
| Large images exceed provider limits | Add max image bytes/config guard before encoding |
| SVG or uncommon formats fail provider-side | Normalize accepted formats or reject with clear message |

## Validation Plan

Docs-only research validation:

```powershell
rg -n "Image Recognition Prephase" docs
```

Implementation validation for later slices:

```powershell
just fmt-check
just check
just test
cargo test -p agent-diva-providers vision
cargo test -p agent-diva-agent attachment
```

Manual GUI smoke test:

1. Start GUI.
2. Select a vision-capable provider/model.
3. Attach a PNG/JPEG image.
4. Ask "这张图里是什么？".
5. Confirm the provider request includes a structured image block.
6. Confirm the assistant answer reflects image content.
7. Reload the session and confirm the user message still shows the image
   attachment metadata.

## Recommendation

Do this before Phase A-PRE as a compact `IMG-PRE` branch, but keep it to the
contract and one vertical slice:

```text
GUI image upload -> FileManager -> InboundMessage.media -> typed content parts
-> OpenAI-compatible vision serialization -> session attachment metadata.
```

Then return to Phase A-PRE for full durability, canonical history, and GUI
reconciliation. This order is reasonable because Phase A-PRE can then harden
the final image-aware session/message shape instead of hardening a text-only
shape that immediately needs migration.
