# Issue #32 P1.3 — strict candidate and publication gates

**State:** locally implemented and contract-verified; native/product acceptance and public promotion remain pending.

**Workspace:** isolated DIVA branch `feat/issue32-desktop-gates` at `/workspace/work/issue32/agent-diva`; paired VIVY worktree `/workspace/agent-vivy/.worktrees/issue32`.

## Delivered

- Added strict v2 acceptance validation in `scripts/ci/check_wails_candidate.py`. It requires all 17 canonical W5 rows and every required subcase on Linux and Windows; exact source, recipe, lock, generation, frontend, toolchain and platform identities; clean source reports; retained artifact hashes; and hash-verified evidence. Historical v1 validation remains available for review, but v1 reports cannot promote.
- Replaced direct tag-triggered publication with three jobs: native candidate build/upload, all-platform acceptance, and explicit promotion. Tag pushes create internal Actions artifacts only. Promotion downloads the selected candidate run, checks the annotated DIVA release tag peel, both paired annotated archive tag peels, and the approved W6 state in an immutable full-SHA acceptance ref. Publish re-downloads and rechecks the same candidate and acceptance bytes, then uploads without rebuilding, signing, creating a release, or overwriting assets.
- Added v2 acceptance example with pending outcomes and placeholder artifact identities. It describes the schema; it is not acceptance evidence and cannot promote.
- Added Python contract suites for strict candidates and workflow ordering. Added `desktop-contract-tests` to aggregate `just ci`.
- Corrected the canonical Laputa tree pin in `build/vivy-sources.lock.json` to the computed SDK-compatible digest `20825d185f8e678df5adfb84d886e1975ae8c37b0fc989414a0ef742b6d340a6` at the already pinned commit `6f2eed2d71c82261333e50883e98b404070a6801`. Added a contract that compares both locked VIVY/Laputa commit and tree identities to checked-out bytes.

## Promotion status

The local synthetic two-platform candidate passes the strict checker, and all incomplete, substituted, pending, tampered, stale-evidence, unapproved-W6, and wrong-archive cases reject. The repository's historical v1 fixture still reports 15 passed, 2 pending, 0 failed under its legacy review command; it is not promotable.

No native candidate was built in this executor. GitHub Actions, installed-product W5 scenarios, real voice tests, actual paired archive refs, and approved W6 disposition remain pending. No candidate was uploaded, no Git ref was written, and no release was published.
