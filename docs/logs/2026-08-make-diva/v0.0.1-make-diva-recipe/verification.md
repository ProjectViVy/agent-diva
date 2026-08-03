# Verification

- `just --list` includes `make-diva`.
- `powershell.exe -NoProfile -File scripts/make-diva.ps1` successfully opened two windows and printed:
  - Opened window [diva-gate] -> just diva-gate
  - Opened window [tauri-dev] -> pnpm tauri dev

Full gateway compile/GUI boot beyond process spawn is manual and environment-dependent.
