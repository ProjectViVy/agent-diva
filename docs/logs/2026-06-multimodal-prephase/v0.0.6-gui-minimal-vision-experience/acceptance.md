# M6 Acceptance

## User-Facing Acceptance

- Uploading an image file shows an image-specific attachment chip with filename and size.
- Uploading a non-image file shows a file attachment chip with filename and size.
- Sending a message with attachments keeps attachment chips visible in the user message bubble.
- Loading history preserves attachment metadata chips when backend session messages include attachments.
- Adding an image attachment while using an unknown or text-only model shows a clear warning-only vision hint.
- Pure text chat behavior remains unchanged.

## Product Boundaries

- No full image viewer, thumbnail preview, resize/compression, or media manager is included.
- No new provider vision capability is added.
- No backend M1-M5 multimodal chain behavior is rewritten.

## Validation Acceptance

- `npm run build`, targeted Rust tests, `just fmt-check`, and `just check` pass.
- `just test` result is recorded with the unrelated skills loading failure.
