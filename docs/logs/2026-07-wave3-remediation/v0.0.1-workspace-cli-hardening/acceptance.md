# Acceptance

1. Run `agent-diva --config-dir <dir> workspace create test-ws` and confirm it succeeds.
2. Run `agent-diva --config-dir <dir> workspace create ..\\escape` and confirm it fails with `Invalid workspace name`.
3. Run `agent-diva --config-dir <dir> workspace list` against a fresh config dir and confirm it prints `No workspaces found.` without creating `workspaces/`.
4. Configure a managed workspace as active, then run `workspace delete <active-name> --force` with a different `--workspace` override and confirm deletion is rejected.
