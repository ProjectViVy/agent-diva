# W3-1 acceptance

Owner-side acceptance steps once the Windows candidate exists:

1. Run the sealed `diva` on Windows x64: launch → chat visible → close
   hides to tray (VIVY alive) → reopen resubscribes (no replay) → reload
   refreshes projections with gap → explicit Quit stops producers and
   reports teardown truth → second instance focuses primary only.
2. Confirm privileged calls (VivyCall/DesktopDispatch/MediaToken) reject
   foreign-window and forged-header senders (sender tests + probe).
3. Confirm no old-home discovery, no shared-library loading, no Tauri
   imports remain (W6 removes src-tauri and Rust entirely).
4. Re-audit merged W3 commit (`feat/wails-go-host`) per standing rule —
   CI/subagent green is not acceptance.
