# M4+M5 Capability And OpenAI Vision Summary

## Changed

- Added conservative provider model capabilities with explicit vision support checks.
- Added image-part detection helpers on provider message content.
- Added agent-side vision preparation before provider calls:
  - text-only models are blocked before sending image payloads;
  - `ImageFile { file_id }` is resolved through `FileManager`;
  - PNG/JPEG/WebP images under 5 MB are converted to OpenAI-compatible `image_url` data URIs;
  - `ImageData` is normalized to `image_url` instead of leaking `image_data`.
- Added LiteLLM request-shape tests for non-streaming and streaming OpenAI-compatible image content.

## Impact

The backend/provider path can now carry image attachments from the M3 agent loop assembly into OpenAI-compatible vision request JSON for explicitly supported vision models. Unknown and text-only models default to non-vision and receive a clear user-facing explanation instead of an invalid image payload.

## Out Of Scope

- GUI upload or model-selection UX.
- Audio, video, TTS, and image generation.
- Image resize/compression.
- Full provider-specific multimodal support outside the OpenAI-compatible LiteLLM path.
