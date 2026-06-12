# Acceptance

1. Confirm [`scripts/publish-nano-stack.ps1`](../../../scripts/publish-nano-stack.ps1) exists.
2. Confirm [`justfile`](../../../justfile) contains `package-nano-stack` and `publish-nano-stack-dry-run`.
3. Run `just package-nano-stack`.
4. Verify the flow packages `agent-diva-core` first, then stops at `agent-diva-providers`.
5. Verify the stop message explains that upstream crates must already exist on crates.io before higher-level crates can package for upload.
