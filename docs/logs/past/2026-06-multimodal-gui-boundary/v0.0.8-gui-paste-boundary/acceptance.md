# v0.0.8 GUI Paste Boundary Acceptance

## Acceptance Statement

The project accepts the following clarification:

- "Unable to paste an image directly into the GUI and have it recognized" is true for the current implementation.
- This is an expected design boundary of the current multimodal milestone.
- The delivered path is attachment-based image recognition, not clipboard paste recognition.
- GUI paste support and related user experience improvements are planned follow-up work.

## User/Product Acceptance

Current expected user flow:

1. Select a vision-capable model.
2. Attach a PNG/JPEG/WebP image through the attachment button.
3. Send the message with the image attachment.
4. The backend prepares a vision payload only if the selected model supports vision.

Not currently accepted as implemented:

1. Pasting screenshots or images directly into the composer.
2. Automatically converting pasted clipboard image data into an attachment.
3. Showing pasted image previews before upload.

## Follow-Up Acceptance Targets

Future GUI work should be accepted only when:

- Pasted clipboard image data is captured in the composer.
- Pasted images are uploaded through the existing file attachment path.
- The composer displays the pasted image as an attachment chip or preview.
- Text-only model behavior is clear before sending, preferably blocking or strongly guiding the user instead of relying only on backend errors.
