# M6 GUI Minimal Vision Experience Summary

## What Changed

- GUI message attachment state now keeps file metadata instead of only file IDs.
- Optimistic user messages keep attachment chips after sending, while backend send payload still uses `file_id[]`.
- Session history mapping now reads attachment metadata from backend messages.
- Chat bubbles and composer attachment previews distinguish image attachments from ordinary files and show filename plus size.
- Composer now shows a warning-only hint when image attachments are pending under a model not known to support vision.
- Composer upload failures now surface as a visible inline error.

## Impact

- Scope is limited to `agent-diva-gui` frontend types/rendering and attachment DTO compatibility.
- M1-M5 backend provider routing, model ID behavior, OpenAI-compatible vision serialization, and session byte storage are unchanged.

## Notes

- The frontend vision model list mirrors the conservative backend whitelist: `gpt-4o`, `gpt-4o-mini`, `gpt-4.1`, `gpt-4.1-mini`.
- The frontend warning does not block sending; backend remains the enforcement source.
