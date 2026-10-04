# W0-3 Release — sealed candidate, round 2 (Windows x64)

## Final artifact (evidence artifact for w0-3)

`repos/artifacts/diva-go-host-r2/diva.exe`

| field | value |
|-------|-------|
| sha256 | `dfc85ae1e962b46e7bb8ada8aa555022b9d7d110b34f29fbb242c25f946683fa` |
| generationId | `8616618d1cde3914c85d5bd7b42a00e4b5dd3ddafc8ef4bbaf8dfa24639f5e30` |
| build-report schema | `vivy.go-host-report/v1` |
| host (agent-diva) | `780237a54f53cb6c3b3130cfe15d6ba6d6ebc25f` (`feat/wails-go-host`) |
| vivy | `42c263f2bd84336d2120279dfa454c0110f1d1a7` (`feat/wails-migration`) |
| laputa | `6f2eed2d71c82261333e50883e98b404070a6801` |
| toolchain | go1.26.8, wails v3.0.0-beta.27, windows/amd64, cgo false |
| release | false (development build) |

`consumerModfileSHA256 768879d2…`, `dependencyLockSHA256 f8b02f83…`
(see `build-report.json`). Staging copies tracked files only, so the
local probe dirs `cmd/turnprobe/` and `garden/cmd/frozencap/` are listed
as `dirty` inputs in the report but carry no content into the exe.

## Intermediate diagnostic build (superseded)

`repos/artifacts/diva-go-host/diva.exe` sha256
`be4d01b4ed7cd1f17381e31e60bb40b333ce0b048ae53eebbb18b75d540e865f` —
same tree plus a one-line `internal/rpc/control.go` debug patch
surfacing real `internal error` messages (used to expose the F5 error
text in the GUI). **Reverted before the final build**; behavior parity
verified by re-running the same flows on the clean artifact.

## Contents of the prior leg

w0-2 artifact (sha256 `623bfc05…2800`, generation `a24a62a2…d033`,
host `81c381f8`, vivy `3862b768`) recorded in `w0-2/release.md`; its
binary embedded only `.gitkeep` assets — superseded by this round.
