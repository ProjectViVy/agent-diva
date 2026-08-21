# Release: Global Version Bump 0.5.0 -> 0.9.9

## Method

- Version metadata change only; no deployable artifact rebuild required at
  this time.
- The next Windows packaging run (`scripts/package-windows-gui.ps1`) will
  produce installers named with `0.9.9` automatically (NSIS/MSI derive the
  version from the `agent-diva-gui` src-tauri crate manifest).
- Not pushed per repository policy (push only on explicit user request).

## Reason for no immediate release

No behavior change; existing running gateway/GUI binaries remain valid until
the user requests a rebuild/repackage.
