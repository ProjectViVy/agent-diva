# Summary — `just make-diva`

## What changed

- Added Windows recipe `just make-diva`.
- Added launcher script `scripts/make-diva.ps1` that opens two new PowerShell windows:
  1. `just diva-gate` (gateway backend)
  2. `pnpm tauri dev` in `agent-diva-gui` (GUI frontend)

## Impact

- Local Windows developer workflow only.
- Does not change runtime binaries, CI, or production packaging.
