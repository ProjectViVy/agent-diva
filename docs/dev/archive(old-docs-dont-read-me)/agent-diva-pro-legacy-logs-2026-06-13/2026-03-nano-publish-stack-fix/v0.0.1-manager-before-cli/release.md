# Release

1. Publish `agent-diva-manager` first:
   - `cargo publish -p agent-diva-manager --allow-dirty`
2. Wait until `agent-diva-manager 0.1.0` is visible on crates.io.
3. Resume the scripted flow:
   - `just publish-nano-stack`
   - or `powershell.exe -NoProfile -ExecutionPolicy Bypass -File scripts/publish-nano-stack.ps1 -Mode publish -From agent-diva-cli`

If `agent-diva-cli` is the only remaining crate after manager becomes visible, resuming from `agent-diva-cli` is sufficient.
