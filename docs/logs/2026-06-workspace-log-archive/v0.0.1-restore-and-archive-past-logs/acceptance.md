# Restore and Archive Past Logs Acceptance

## Acceptance Steps

1. Inspect `docs/logs/past/` and confirm historical iteration directories are present.
2. Inspect `docs/logs/` and confirm current active iteration logs remain available.
3. Run `git status --short --untracked-files=all` and confirm the archive change is separable from unrelated existing worktree changes.
