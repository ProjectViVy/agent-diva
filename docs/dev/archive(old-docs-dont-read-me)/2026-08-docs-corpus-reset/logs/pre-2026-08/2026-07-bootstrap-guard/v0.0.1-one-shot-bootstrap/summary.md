# Summary

- Bootstrap now uses `bootstrap_seeded_at` as a one-shot trigger guard.
- Repeated context construction no longer re-enters onboarding when completion has not yet been recorded.
- Corrupt soul state is handled fail-closed instead of silently becoming an empty state.
- Added a prompt boundary preventing autonomous reads or replay of `BOOTSTRAP.md`.

