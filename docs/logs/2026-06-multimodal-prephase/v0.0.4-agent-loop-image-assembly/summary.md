# M3 Agent Loop Image Assembly Summary

## What Changed

- Added structured current-message support to `ContextBuilder` through `build_messages_with_content`.
- Updated the agent loop attachment assembly so image attachments become provider-facing `MessageContentPart::ImageFile` parts.
- Preserved legacy inline text attachment behavior and non-image placeholder text behavior.
- Kept session persistence unchanged: user turns save the original text plus lightweight attachment metadata only.

## Impact

Current user turns with image attachments now reach the provider message pipeline as one `user` message containing both the text prompt and image file references. This iteration does not implement provider JSON serialization, vision capability checks, GUI display changes, or base64/image byte persistence.
