# Task-local execution environment

Linux amd64 / Debian trixie. All tools/resources live under `/workspace/work/memory-loop/tools`; originals and system packages are unchanged. Source `environment.sh` in this workspace and set `npm_config_registry=https://registry.npmjs.org/` for frontend installs.

| Resource | Verified version / binding |
|---|---|
| Go | 1.26.4 linux/amd64; system `go` is GNU Go, so PATH matters |
| just | 1.42.4 |
| PowerShell | 7.5.4; task-local powershell and powershell.exe aliases |
| pnpm | 10.33.2 |
| Node | 24.19.0 |
| GTK | 3.24.49 |
| WebKitGTK | 2.54.0 (4.1 API) |
| libsoup | 3.6.5 (3.0 API) |
| glib | 2.84.4 |
| INOFY | v0.0.0-20260930141905-71e2c9bbe47d; exact cached module linked at ../../INOFY |
| ONNX | Pinned Laputa mentle/models/onnx; file hashes in checkpoint.json |

PowerShell requires task-local XDG_DATA_HOME, XDG_CACHE_HOME and XDG_CONFIG_HOME. Go requires task-local GOPATH as well as GOMODCACHE/GOCACHE, because sumdb otherwise writes under the read-only home. Synthetic consumer builds disable VCS stamping; SDK separately seals their source closure.

Native development packages were downloaded from official Debian trixie/trixie-security repositories using a task-local apt state/cache, then extracted with dpkg-deb. No dpkg install or system write occurred. Local pkg-config files point at the extracted root; development symlinks resolve installed compatible runtime libraries. `ldd` finds the reviewed native binary's dependencies, and its --help process succeeds. This is not display/UI acceptance.

The native reviewed development artifact, its exact source closure and model inventory are identified in checkpoint.json/native-build-report.json. Do not present the old incomplete baseline as passing. A new machine must provision equivalent task resources and rerun the supported wrapper/Inspect, not copy developer memory profiles.

Downloaded native package filenames, sizes and SHA-256 values are recorded in environment-resources.json. This inventory identifies the actual resources; provisioning on another machine still requires extraction/path setup and real reruns.
