# Acceptance

1. Open the repo root and confirm `temp/` exists.
2. Confirm the following files are under `temp/` instead of the repo root:
   - `minimax_sync_tts_output.mp3`
   - `siliconflow_tts_output.mp3`
3. Open the root `.gitignore` and confirm `temp/` is ignored.
4. Run `git status --short` and confirm those demo audio outputs no longer appear as untracked files.
5. Confirm source changes, formal tests, `agent-diva-providers/examples/`, and `docs/logs/2026-05-*` remain available for normal commit review.
