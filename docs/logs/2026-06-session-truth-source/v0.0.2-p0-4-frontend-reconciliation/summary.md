# v0.0.2 P0-4 Frontend Reconciliation Summary

This iteration completes the GUI slice of `P0-4: Session truth-source fix (Phase A-PRE)`.

- Make `loadSession()` backend-first so successful backend history always overrides local cache.
- Restrict `localStorage` session cache to backend-unavailable fallback only, with a visible stale-state banner.
- Add canonical reconciliation after send completion, stop, reset, delete, and session switch.
- Mark optimistic frontend-only messages as pending/local-only/failed instead of treating them as durable history.
- Move session reset responsibility from `ChatView.vue` into `App.vue` so the current session key/chat id drives backend mutations consistently.
