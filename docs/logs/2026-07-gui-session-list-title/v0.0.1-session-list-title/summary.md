# Summary

- Extended session metadata and `/api/sessions` payloads to include `last_message`, `message_count`, `title_generated`, `title_manually_set`, and `pinned`.
- Added `POST /api/sessions/:id/generate-title` plus Tauri bridge commands so the first assistant reply can trigger persisted title generation with fallback handling.
- Updated the GUI conversation sidebar to behave as a real session list: new drafts appear immediately, message preview/count update locally, and the final title is backfilled after the first assistant response.
