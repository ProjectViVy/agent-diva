# Acceptance

1. Confirm [`scripts/publish-nano-stack.ps1`](D:\VIVYCORE\agent-diva\scripts\publish-nano-stack.ps1) includes `agent-diva-manager` before `agent-diva-cli` in `$stack`.
2. Run `cargo publish -p agent-diva-manager --dry-run --allow-dirty` and confirm it succeeds.
3. Run `powershell.exe -NoProfile -ExecutionPolicy Bypass -File scripts/publish-nano-stack.ps1 -Mode package -From agent-diva-manager` and confirm both `agent-diva-manager` and `agent-diva-cli` are processed in order.
4. After real publication of `agent-diva-manager`, run `just publish-nano-stack` and confirm `agent-diva-cli` no longer fails with `no matching package named 'agent-diva-manager' found`.
