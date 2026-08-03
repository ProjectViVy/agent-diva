# Acceptance

1. From repo root, run `just make-diva`.
2. Two PowerShell windows appear and stay open (`-NoExit`).
3. One window runs the same path as `just diva-gate`.
4. The other window runs `pnpm tauri dev` under `agent-diva-gui`.
5. Closing a window stops only that process.
