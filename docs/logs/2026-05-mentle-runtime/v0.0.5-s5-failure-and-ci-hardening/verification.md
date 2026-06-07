# Verification

## Commands

Sprint 5 validation uses these command groups:

```powershell
cargo fmt --all -- --check
just sprint5-default-check
just mentle-package-policy
cargo check -p agent-diva-agent --features mentle
cargo test -p agent-diva-agent --features mentle mentle
cargo test -p agent-diva-core --features mentle memory
just check
```

## Results

| Command | Result |
|---|---|
| `cargo fmt --all -- --check` | Passed |
| `just sprint5-default-check` | Passed |
| `just mentle-package-policy` | Passed |
| `cargo check -p agent-diva-agent --features mentle` | Passed after adding LLVM to PATH |
| `cargo test -p agent-diva-agent --features mentle mentle` | Passed, 20 tests |
| `cargo test -p agent-diva-core --features mentle memory` | Passed, 39 tests |
| `just check` | Passed |
| `just test` | Failed outside Sprint 5 scope: provider integration tests import `agent_diva_providers::ollama`, which is not exported in the current crate surface |

The `just test` failure occurred in:

- `agent-diva-providers/tests/ollama_streaming.rs`
- `agent-diva-providers/tests/ollama_tools.rs`

The failing errors were unresolved `agent_diva_providers::ollama` imports plus
follow-on type inference errors in those tests. No Sprint 5 Mentle regression
failed in the targeted validation chain.

## Environment Notes

On Windows, Mentle feature checks require `clang-cl.exe` to be discoverable. If
LLVM is installed at `C:\Program Files\LLVM\bin`, prefix the current shell PATH
before running Mentle feature commands:

```powershell
$env:PATH = 'C:\Program Files\LLVM\bin;' + $env:PATH
```
