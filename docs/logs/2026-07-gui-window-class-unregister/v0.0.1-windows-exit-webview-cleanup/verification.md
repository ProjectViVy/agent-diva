# Verification

- Root-cause check:
  - Win32 error `1412` resolves to `类仍有打开的窗口`, which matches a shutdown path that exits while WebView windows still exist.
- Validation executed:
  - `cargo test -p agent-diva-gui`
- Validation results:
  - The GUI crate compiled with the shutdown fix in place.
  - `cargo test -p agent-diva-gui` still fails on the pre-existing `embedded_server::tests::embedded_gateway_serves_health_endpoint` assertion (`502` vs expected `200`), unrelated to the Windows window-destruction path.
- Remaining manual smoke test:
  - Run `pnpm --dir agent-diva-gui tauri dev` on Windows.
  - Open and close the app normally.
  - If applicable, also open `desktop-pet` once and then quit.
  - Confirm the terminal no longer prints `Failed to unregister class Chrome_WidgetWin_0. Error = 1412`.
