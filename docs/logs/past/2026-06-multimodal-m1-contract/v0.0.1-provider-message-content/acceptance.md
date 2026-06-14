# M1 Provider Message Content Acceptance

## Activity Acceptance

- M1-A1: `MessageContent` supports `Text` and `Parts`; `Message::user("text")` remains valid.
- M1-A2: `MessageContentPart` supports `Text`, `ImageUrl`, `ImageFile`, and `ImageData`, including local `file_id` and data URI representation.
- M1-A3: Provider base tests cover old JSON string reads and new structured writes.
- M1-A4: LiteLLM preserves structured content, and Ollama keeps text-only behavior through lossy text extraction.
- M1-A5: Type placement and compatibility strategy are recorded in this iteration log.

## Out Of Scope Confirmed

- No image upload implementation.
- No local file lookup or file existence validation.
- No data URI MIME/base64 validation.
- No GUI, CLI, channel, or session-storage multimodal entrypoint.
