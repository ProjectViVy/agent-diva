# Summary

## Change

- Scoped Focus Mode behavior to the embedded pet page only (`activeMenu === 'pet'`).
- Pet page now hides the topbar unconditionally and uses full-height content.
- Pet page defaults to a collapsed/hidden sidebar so the pet surface occupies the window.
- The pet page menu button now toggles only the sidebar; it does not restore the topbar.
- Other pages keep the regular sidebar plus topbar layout.

## Impact

- No Rust or Tauri API changes.
- No external `NormalMode` props or emits changed.
- Added component coverage for the pet focus layout contract.
