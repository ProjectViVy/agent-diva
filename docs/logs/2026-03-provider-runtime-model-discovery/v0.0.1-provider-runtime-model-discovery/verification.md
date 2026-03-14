# Verification

## Commands

- `cargo fmt --all`
- `cargo check`
- `cargo test --all`
- `cargo fmt --all --check`
- `pnpm.cmd -C agent-diva-gui build`
- CLI smoke:

```powershell
$temp = Join-Path $env:TEMP ('agent-diva-smoke-' + [guid]::NewGuid().ToString())
New-Item -ItemType Directory -Path (Join-Path $temp 'instance') -Force | Out-Null
New-Item -ItemType Directory -Path (Join-Path $temp 'workspace') -Force | Out-Null
$configPath = Join-Path $temp 'instance\config.json'
$config = [ordered]@{
  agents = [ordered]@{
    defaults = [ordered]@{
      workspace = (Join-Path $temp 'workspace')
      model = 'openai/gpt-4o'
    }
  }
  providers = [ordered]@{
    anthropic = [ordered]@{
      api_key = 'sk-test'
    }
  }
} | ConvertTo-Json -Depth 10
[System.IO.File]::WriteAllText($configPath, $config, (New-Object System.Text.UTF8Encoding($false)))
cargo run -q -p agent-diva-cli -- --config $configPath provider models --provider anthropic --static-fallback --json
```

## Results

- `cargo check`: passed.
- `cargo test --all`: passed.
- `cargo fmt --all --check`: passed.
- `pnpm.cmd -C agent-diva-gui build`: passed after running outside the sandbox; output bundle generated successfully.
- CLI smoke result: returned `source = static_fallback`, `runtime_supported = false`, and bundled Anthropic models as expected.

## Notes

- `just fmt-check` could not be used in this environment because `just.exe` was not runnable from PowerShell. Equivalent validation was performed with `cargo fmt --all --check`.
- PowerShell emitted the local execution-policy warning for `profile.ps1` during cargo commands, but commands still completed successfully.
