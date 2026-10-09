# Verification record

## Passed locally

- `python -m py_compile scripts/ci/check_wails_candidate.py scripts/ci/test_wails_candidate.py scripts/ci/test_desktop_release_contract.py scripts/ci/test_desktop_ci_contract.py`
- `python -m unittest discover -s scripts/ci -p 'test_*.py' -v` — **35 tests passed**.
- Strict candidate contracts — **13 tests passed**, including one complete synthetic Linux/Windows candidate and checks for missing/unknown rows, missing subcases, pending/failed results, absent platform/artifacts, short commits, dirty sources, changed recipe/generation/frontend/locks/tools/platform, binary/installer tampering, stale evidence, v1 rejection, and archive/W6 prerequisites.
- Release workflow contracts — **4 tests passed**, including gate dependency, same-run candidate downloads, no rebuild/overwrite in publish, annotated tag peel checks and paired archive/W6 gates.
- Desktop CI/build contracts — **12 tests passed**, including exact checked-out source tree pins and SDK file-size hash framing.
- `python scripts/ci/check_desktop_boundary.py --selftest` — 4/4 forbidden fixtures rejected.
- `python scripts/ci/check_desktop_boundary.py` — clean.
- `python scripts/ci/check_wails_candidate.py docs/plans/diva-next/fixtures/wails-candidate-acceptance.json` — historical report shape valid; 15 passed / 2 pending / 0 failed; output explicitly says it is not promotable.
- PyYAML parsed `.github/workflows/ci.yml` (5 jobs) and `.github/workflows/desktop-release.yml` (3 jobs).
- `git diff --check` — clean.
- Recomputed source identities: VIVY `9ba59caa4816a7f66e27c50b7948ddeb09a2df9c` / tree `905d787eeceae38ff75eb75ee3217cd0a2aca40d5b7182156d9262ce72313a10`; Laputa `6f2eed2d71c82261333e50883e98b404070a6801` / tree `20825d185f8e678df5adfb84d886e1975ae8c37b0fc989414a0ef742b6d340a6`.

## Not run here

- `just ci` and native `python scripts/build-desktop.py --mode build` could not run because this execution image does not provide the Go toolchain or `just`; the earlier canonical host attempt also stopped at missing GTK3/WebKit2GTK development packages. The Python contract command that backs `desktop-contract-tests` ran directly and passed.
- GitHub Actions Linux/Windows candidate jobs and actual SDK executable inspection were not run.
- Installed-product acceptance, microphone/playback, real provider voice, remote annotated archive tags, and owner-approved W6 state were not available as evidence. Promotion remains blocked until they are supplied.
- `actionlint` is not installed; both workflow files were parsed with PyYAML and their local contract tests passed.
