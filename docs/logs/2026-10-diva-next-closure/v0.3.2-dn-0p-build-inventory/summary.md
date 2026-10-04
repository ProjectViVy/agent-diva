# v0.3.2 — DN-0P build inventory

Delivered `docs/plans/diva-next/fixtures/closure-build-inputs.json` and linked it
from contract ledger C2-5.

Contents: exact pins (VIVY `1db8b55`, Laputa `dc6066e` + per-directory tree
hashes for laputa/garden/mentle/embed-smoke, INOFY `71e2c9b`), toolchain
(go1.26.4 linux/amd64, gcc 11.4.0, CGO=1, node v24.19.0, pnpm 11.21.0), the full
transitive replace map (no go.work needed), module graph (271 build-list
modules; 1081 dep packages / 149 dep modules of `cmd/vivy-shared`), native
inventory (4 cgo packages; ldd → libc only; TLS/SQLite pure Go; no ONNX or
native tokenizer in the baseline closure), staged pack/inspect commands, and
baseline artifact SHA-256s (`.so` fe5b334f…, 92,290,312 B).

Build-only staging at `~/staging/dn-0p`; reference checkout go.mod files were
not modified. Windows/amd64 build/load is recorded pending (no runner, no
mingw-w64). Deferred-minor: garden `console` embed needs a `dist` build product;
captured, not patched.
