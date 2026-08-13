# v0.0.8 GUI Paste Boundary Summary

## Background

This note records the follow-up conclusion after checking the current image recognition implementation.

The current multimodal delivery intentionally completed the backend and attachment-based image recognition path first. It did not claim to complete direct clipboard image paste recognition in the GUI.

## Conclusion

- The bottom-layer multimodal chain is partially complete for file attachments:
  GUI file upload -> file service -> attachment file_id -> agent loop image assembly -> OpenAI-compatible vision payload.
- Direct GUI clipboard image paste, such as pasting a screenshot with Ctrl+V into the chat composer, is not implemented in the current code.
- This is a design boundary of the current milestone, not evidence that the backend vision chain is broken.
- The GUI still needs optimization before the product experience can be considered complete for ordinary users.

## Current Expected Behavior

- Users can attach an image file through the GUI attachment button.
- The image is only sent as vision input when the selected model is known to support vision.
- The current hard-coded vision whitelist is conservative and mainly covers `gpt-4o`, `gpt-4o-mini`, `gpt-4.1`, and `gpt-4.1-mini`.
- Text-only or unknown models should not receive image payloads and may return a user-facing unsupported-model message.

## GUI Optimization Direction

Planned GUI work should treat clipboard paste support as a product/UX enhancement:

- Add composer paste handling for clipboard image data.
- Convert pasted image data into the same upload/attachment path used by selected files.
- Show a clear image chip or preview after paste.
- Keep backend enforcement for model vision capability.
- Improve the warning/blocking behavior when the selected model cannot inspect images.

## Impact

No runtime behavior changed in this iteration. This is a documentation-only clarification.
