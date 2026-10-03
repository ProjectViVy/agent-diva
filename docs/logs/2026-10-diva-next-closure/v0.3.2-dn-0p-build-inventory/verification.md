# v0.3.2 — DN-0P verification

Commands run from build-only staging (`~/staging/dn-0p`, sibling worktrees at
the pinned commits):

- `go list -m -json all` → `modules.json` (sha256 a6177f91…, 271 modules).
- `go list -deps -json ./cmd/vivy-shared` → `deps-vivy-shared.json`
  (sha256 7627865e…; 1081 packages, 149 modules, 4 cgo packages).
- `go run ./sdk stage-ui --recipe recipes/default.vivy.yml --out
  ui/src/generated` — staged the gitignored ui projection (required before ui
  vitest; resolves `./ui/<module>/src/index.tsx` imports).
- `go run ./sdk pack --recipe recipes/diva.vivy.yml --target shared --output
  out-shared` — produced `vivy-shared.so` (92,290,312 B, sha256 fe5b334f…),
  `vivy-shared.h`, `vivy_abi.h`, `generation.json` (generationId 733127c6…,
  compiler plg-p9).
- `go run ./sdk inspect-artifact out-shared` → `inspect.json` (sha256
  f6be0967…).
- `ldd out-shared/vivy-shared.so` → libc + ld-linux only.
- `nm -D` → exactly the five C exports.
- `go build ./...` in laputa/embed-smoke (pass), mentle (pass), garden
  (console embed `dist` failure captured, not patched).
- `GOOS=windows GOARCH=amd64 CGO_ENABLED=1 go build -buildmode=c-shared
  ./cmd/vivy-shared` → fails on `-mthreads` (no mingw-w64); recorded pending.
- Story gate: `go test ./sdk/internal/... ./internal/embedded
  ./cmd/vivy-shared -count=1` — all packages ok (sdk/internal 211.6s incl.
  full-ui conformance, 57.4s conformance pkg, embedded + vivy-shared ok).
- `python3 -m json.tool docs/plans/diva-next/fixtures/closure-build-inputs.json`
  — valid.
