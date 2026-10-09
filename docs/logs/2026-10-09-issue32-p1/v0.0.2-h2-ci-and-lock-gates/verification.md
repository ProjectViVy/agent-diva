# Verification

## Regression-first evidence

Before implementation, `python -m unittest discover -s scripts/ci -p
'test_desktop_*.py' -v` failed as expected: Go-only trigger paths and required
aggregate recipes were missing, the workflow had no Linux/Windows locked-source
jobs, and target derivation/boundary-check functions did not exist.

## Passing local checks

- `python -m unittest discover -s scripts/ci -p 'test_desktop_*.py' -v` — 16
  tests passed, including deterministic Linux/Windows derivation, common pin
  preservation, lock drift, native-target mismatch, path filters, and negative
  boundary fixtures.
- `python scripts/ci/check_desktop_boundary.py --selftest` — all 4 negative
  fixtures rejected and the authorized SDK/native seams accepted.
- `python scripts/ci/check_desktop_boundary.py` — current DIVA tree clean.
- `python -m py_compile scripts/build-desktop.py scripts/ci/check_desktop_boundary.py
  scripts/ci/test_desktop_ci_contract.py scripts/ci/test_desktop_boundary.py` —
  passed.
- PyYAML parsed `.github/workflows/ci.yml`; both platform jobs and the required
  aggregate job are present.
- `git diff --check` — passed.
- `just` is not installed in this execution image, so the aggregate recipe was
  inspected by contract tests but could not be parsed or run locally.
- The source lock records a 40-character inofy commit and 40-character VIVY
  commit. The VIVY source-tree digest was computed from an isolated checkout at
  the pinned commit.
- The preceding VIVY SDK source-closure regression remains green in its focused
  test; the test-mode embedded-index contract passes in this DIVA suite.

## Native checks not run

The local `go` executable in this environment is not the Go toolchain: invoking
`go mod download` returns `Go: Unknown option: mod`. Therefore this session
could not execute source-lock verification or the canonical host race/build
gate. An earlier canonical wrapper attempt in the implementation session
reached native compilation and stopped at unavailable GTK3/WebKit2GTK
development packages. Windows CGO/race acceptance has not run. These results
remain pending the workflow's real Linux and Windows runners.
