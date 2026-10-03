# VIVY runtime staging

Populated by `just shell-stage-runtime <sealed-artifact-dir>` (rsync copy of the
DN-L pack output). Bundled wholesale via `bundle.resources` in
`tauri.conf.json` and resolved at runtime from the app resource dir — never
CWD or a search path.

Committed: `vivy_abi.h`, `vivy-shared.h`, `generation.json` (ABI pin + seal
provenance). Ignored: `vivy-shared.{so,dll,dylib}`, `zz_assembly.go`,
`ui-assembly.ts`, `ui/` (regenerated payload).
