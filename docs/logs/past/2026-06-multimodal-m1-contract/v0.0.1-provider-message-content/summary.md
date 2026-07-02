# M1 Provider Message Content Summary

## What Changed

- Added `MessageContent` in `agent-diva-providers/src/base.rs` so provider messages can carry either legacy text or structured multimodal parts.
- Added `MessageContentPart` variants for text, image URL, local/file-backed image IDs, and data URI image payloads.
- Kept existing text constructors such as `Message::user("text")` working by converting text inputs into `MessageContent::Text`.
- Updated LiteLLM request handling to preserve structured content while continuing to serialize text-only messages as JSON strings.
- Updated Ollama conversion to use lossy text extraction for structured content because M1 does not send image payloads to text-only providers.

## Impact

The provider layer can now express text plus image-bearing content blocks without changing session storage, channel input, CLI, GUI, upload, or model-routing behavior. Existing pure-text provider flows continue to use the legacy string shape.

## Contract Decision

The type contract lives in `agent-diva-providers/src/base.rs` because `Message` is the provider-facing request type. M1 intentionally does not change `agent-diva-core::session::ChatMessage`, keeping durable chat history compatibility outside this stage.
