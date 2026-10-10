# S03 development candidate and sealed consumer test repair

The sealed consumer run exposed a build-script defect: `go test ./...` treated
preserved Go test logs under `docs/logs/**/raw` as real packages. The first run
failed because those archived tests imported VIVY internal packages from
outside the VIVY module. `scripts/build-desktop.py` now discovers package paths
with `go list`, excludes DIVA `docs/`, and passes the remaining six host package
paths explicitly to `go test`. Two new package-selection tests and the
existing test-asset cleanup test passed (3/3). The committed repair is DIVA
`71af4a0e`.

After the repair, the exact-source sealed consumer build and `-race` suite
returned exit 0. The final stage listed seven paths, excluded the one archived
docs package, and tested all six real host package targets. `go build ./...`
also passed. The earlier red test, initial race failure, unit-test output, and
final passing command are preserved under `raw/`.

A complete Linux **development** go-host candidate was then rebuilt from
DIVA `71af4a0e`, VIVY `15d3e469`, and Laputa `4da565e`, and passed SDK
`inspect-artifact`. Its development generation is
`e7e26e20a2fb2f587c608ea263f679d5ebdaff6900e63bbc741f9c919dd63baa`; the
binary SHA-256 is
`78fb1948206d4591804cc366871844f552bee8f39361fafc340f691355ad4f6e`. It
contains 22 modules, including `vivy/diva-memory` and `vivy/diva-cognitive`,
and is retained at
`/workspace/work/memory-loop/artifacts/diva-full-dev-20261010-final`. It is
explicitly `release: false`; its generation is not a formal memory-loop
`candidate_id` and does not satisfy the blocked platform baseline.

The real App composition fixture is built from the separate VIVY shared
integration artifact, as prescribed by the runbook. That artifact inspected
as generation
`d963302d3aea3c458e0089f2903788295ef7241ce6d45286454dfcb4004d382d`; its
overlay tests passed 2/2, and `TestMemoryLoopFixtureUsesRealComposition`
passed (1.39 seconds). A random user-only fact reached the actual provider
request and canonical memory with persisted source role `user`.

An attempt to feed the full go-host package into the shared-overlay generator
was rejected by its recipe/dependency identity check. The go-host artifact
binds the DIVA consumer dependency closure; the overlay generator accepts the
separate shared VIVY package. The failed check is retained in `raw/` and is not
counted as a product failure or as an acceptance pass.

All these results are developer observations. Formal S03 and downstream Story
states remain pending the same-candidate baseline, frozen-source attestation,
and other required gates. No live model or Windows run is claimed.
