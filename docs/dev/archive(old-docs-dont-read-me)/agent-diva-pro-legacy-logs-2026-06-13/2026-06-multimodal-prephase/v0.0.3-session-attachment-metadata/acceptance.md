# Acceptance

## v0.0.3-session-attachment-metadata

Date: 2026-06-01

## Acceptance Steps

- Send or simulate a user message with an uploaded image file id.
- Confirm the saved session JSONL user message includes attachment metadata with `file_id`, `filename`, `mime_type`, and `size`.
- Confirm the saved session JSONL does not include image bytes, base64 content, or preview content.
- Confirm older session JSONL messages without `attachments` still load successfully.
- Confirm normal text-only conversation history remains unchanged.
