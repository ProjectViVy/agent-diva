# DN-L sealed DIVA shared library — iteration v0.1.0

Scope: `docs/plans/diva-next/DN-L.md` tasks 1–6 plus close-out.
Deliverables landed on `feat/diva-embedded` in agent-vivy (3 commits,
human-authored): `71e0fb26` fix(app) gatewayless Run + embedded services,
`fd1985fe` feat(embedded) ABI v1 host + c-shared exports + C smoke,
`d187e67b` feat(sdk) `--target shared` pack/inspect + diva recipe +
conformance re-pin.

## What was done

- **Gatewayless lifetime fixed (red→green).**
  `TestGatewaylessRunBlocksUntilContextCancel` proved `App.Run` returned
  early on the gatewayless branch (eager `close(errCh)`); the fix deletes
  that close so Run blocks on ctx alone — gateway path untouched.
- **Sweeper/cron ownership resolved.** New `App.StartEmbeddedServices()`
  starts the interaction sweeper (1s) and the cron scheduler when enabled;
  `internal/embedded.Open` invokes it after `DialControl`, `Host.Close`
  owns all teardown. `TestStartEmbeddedServicesExpiresApprovals` shows a
  real 400 ms approval expiry evicting in embedded mode.
- **`internal/embedded` host.** Open/Call/Poll/Close over a 10 000-event
  drop-oldest queue with sticky gap flag; Poll ≤500 non-blocking; Close
  idempotent via sync.Once; `context.WithoutCancel` so Close is sole owner.
- **ABI v1 c-shared exports** (`cmd/vivy-shared`): `VivyInit`, `VivyCall`,
  `VivyPollEvents`, `VivyShutdown`, `VivyFree` over a uint64 handle table —
  no Go pointer crosses FFI. Envelopes `{ok,value}|{ok,error{kind,…}}`,
  4 MiB input cap, 120 s default call timeout, panic→redacted `internal`.
  `vivy_abi.h` pins `VIVY_ABI_VERSION 1`; version asserted against
  `embedded.ABIVersion` at both Go-test and inspect levels.
- **SDK `--target shared`.** Reuses recipe→assembly→overlay→SealManifest
  pipeline; builds `./cmd/vivy-shared` with `-tags vivy_headless
  -buildmode=c-shared`, stages `vivy_abi.h` next to the generated header;
  `InspectArtifact` detects the shared artifact by filename and verifies
  the ABI header. No manifest schema change.
- **`recipes/diva.vivy.yml`** (profile `diva`) — 20 modules; persona /
  evolution / memory / notebook stay out (pack rejects unused providers;
  all four are DN-4-deferred gaps).
- **`tests/ffi/`**: mock OpenAI-compatible provider + `smoke.c` driving the
  real chain VivyInit→initialize→session→turn→approval(approve)→cancel→
  poll→shutdown→free against both a raw `.so` and the sealed packed
  artifact.
- **Wire corrections applied**: `review/respond` params
  `{review_id, action:"approve"|"deny"}` / `{review_id, action:"answer"|
  "cancel", answer}`; `run/cancel` on inactive run → -32004.

## Verification performed

- `go test ./internal/embedded ./internal/app ./internal/rpc
  ./sdk/internal/... -count=1` — all `ok` (embedded 0.8s, app 8.1s,
  rpc 19.6s, sdk/internal 214.5s, assembly 6.8s, conformance 50.4s),
  no `vivy_headless` tag needed after real vite build became possible.
- Environment provisioned: Go 1.26.8, Node 22.20.0 + pnpm 10.18.3
  (`~/toolchains`), `pnpm install --frozen-lockfile` + `pnpm build` → real
  `ui/dist` — executable-target pack tests now run (not just headless).
- Packed artifact (dev output `/tmp/diva-shared-artifact`): `vivy-shared.so`
  92 MB + `vivy-shared.h` + `vivy_abi.h` + `generation.json` +
  `zz_assembly.go` + `ui/` + `ui-assembly.ts`; generationId
  `322a9cbe24fdac7934f5ec471455d2770c3950054f2fb17194f29e9fcd29e9a4`.
- `TestPackAndInspectSharedTarget`: pack → inspect → tampered `vivy_abi.h`
  fails inspect → corrupted manifest frame fails inspect.
- `TestPackSharedMissingModulePublishesNothing`: failed build leaves no
  published Generation.
- `internal/embedded` race-covered concurrency test (Call+Poll+Close).

## Pending

- windows/amd64 `-buildmode=c-shared` DLL+header + Rust FFI acceptance —
  no Windows runner in this environment (TODOLIST P0-NATIVE-VERIFICATION,
  narrowed to Windows-only).
- `internal/app/dn0_capture_test.go` committed deliberately: the provider
  conformance `sourceSha256` digests the internal/ tree including
  untracked files — an uncommitted harness would desync the sealed digest.
  Digest re-pinned to `cdb4d991…42d96a` (5 occurrences).
