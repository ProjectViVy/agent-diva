# Acceptance

1. Confirm [`scripts/publish-nano-stack.ps1`](../../../scripts/publish-nano-stack.ps1) supports `-From`, `-SkipExisting`, `-PollSeconds`, `-TimeoutSeconds`, and `-Registry`.
2. Confirm [`justfile`](../../../justfile) contains `publish-nano-stack`.
3. Run `just publish-nano-stack-dry-run`.
4. Verify the dry-run stops after `agent-diva-core` and explains why downstream crates still fail in dry-run mode.
5. For a real release, run `just publish-nano-stack` after `cargo login`, and verify the script waits for crates.io visibility before moving to the next crate.
