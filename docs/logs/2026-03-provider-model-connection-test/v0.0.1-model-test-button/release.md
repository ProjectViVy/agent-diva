# Release

- No special migration is required.
- Ship with the next normal desktop GUI/Tauri build so the updated provider settings view and the new Tauri command are included together.
- If workspace-wide tests are required before release, stop the running `target\debug\agent-diva.exe` process and rerun `just test`.
