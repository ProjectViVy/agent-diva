# M4+M5 Capability And OpenAI Vision Acceptance

## Acceptance Steps

- Send or simulate a user turn with an image attachment and a vision-capable model such as `gpt-4o`.
- Confirm the provider-bound message contains:
  - a text content part;
  - an `image_url` content part with a `data:image/...;base64,...` URL.
- Confirm provider-bound JSON does not contain `image_file` or `image_data`.
- Send or simulate the same image turn with a text-only model such as `deepseek-chat`.
- Confirm no image payload is sent and the user receives a clear switch-to-vision-model hint.
- Confirm unsupported image MIME, missing file, and oversize image cases produce controlled user-facing errors.
- Confirm session persistence still stores attachment metadata only, not base64 image bytes.
