# M3 Agent Loop Image Assembly Acceptance

## Acceptance Steps

- Send a current user message with text plus a PNG attachment.
- Verify the current provider-facing user message is `MessageContent::Parts`.
- Verify the first part is text containing the user's prompt.
- Verify the image is represented as `MessageContentPart::ImageFile { image_file: ImageFile { file_id } }`.
- Verify small text attachments still inline into the text content.
- Verify non-image binary attachments still produce a read-file placeholder.
- Verify missing file IDs produce a clear text fallback and do not panic.
- Verify session storage records only the original user text and attachment metadata.

## Result

Accepted for M3 based on the new unit coverage and targeted validation commands recorded in `verification.md`.
