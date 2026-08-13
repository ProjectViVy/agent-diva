# Summary

- Updated [`scripts/publish-nano-stack.ps1`](D:\VIVYCORE\agent-diva\scripts\publish-nano-stack.ps1) to publish `agent-diva-manager` before `agent-diva-cli`.
- This fixes the release orchestration gap where `agent-diva-cli` default feature `full` requires `agent-diva-manager`, but the publish stack previously skipped that crate entirely.
- Impact scope is limited to the publish helper flow; no runtime behavior changed.
