# W5-1 Summary — Integration/recovery/observability closure (Linux headless leg)

## What ran
Composed the final sealed candidate on the W4 commit and executed every
matrix row reachable without a GUI driver, real provider credentials, or a
Windows box; the rest are recorded as owned pending rows in
`docs/plans/diva-next/fixtures/wails-candidate-acceptance.json`.

## Candidate (Task 1)
- host `a477ffc8` (tree clean), vivy `d9dfdf49`, laputa `6f2eed2`,
  Wails v3.0.0-beta.27, go1.26.4, tags `gtk3 vivy_headless`.
- generation `7070607d0c03c0b7ab3ddd14b92dfce926d5ed41425d65226b28fe9fe761e4e0`
- binary sha256 `2745163f…`; consumer.mod/sum + checksums emitted;
  build-report regenerated post-commit (no dirty files).

## Passed rows (8/17)
- Fresh Next profile: seeds `.config/DIVA/vivy.yaml` + `.vivy/vivy.db` +
  `garden.db` + `speech/` on first boot, host opens sealed generation, no
  ReadFrozen failure, clean quit. (W-R9)
- Single instance: second launch → `organism lease held` (-32086);
  Wails SingleInstance UniqueID also armed. (W-R5)
- Crash → restart: SIGKILL then relaunch reopens same journal; lease
  released on death. (W-R5)
- Clean quit releases workspace. (W-R5)
- Logs: `.vivy/logs/vivy.log.<date>` JSON sink + rotation. (W-R2)
- Scripted voice: w4-1 speech race suite (cancel/late-settle/provider
  errors/keyring-unavailable/shutdown). (W-R4)
- Voice lifecycle at service level: hide→invalidate, stale_context
  discard, lease-gated asset delete. (W-R4/W-R5)
- Candidate composition/Inspect row itself. (W-R2)

## Pending rows (9/17) — all owned
First turn, grants catalog, chat matrix, cognitive controls, hide/reopen,
WebView reload, event-loss recovery, real STT/TTS, and the whole Windows
matrix → owner E2E / Windows leg / operator credentials.

## Tooling
`scripts/ci/check_wails_candidate.py` validates fixture shape (rows,
evidence, owners), cross-checks build-report generation/host commit/
dirty state and optional binary sha256; `--require-all-passed` is the
promotion gate for later legs.

## Residuals (non-blocking)
ACTMEM/console/pause-text backlog items keep their owning-backlog
dispositions; pet dispatch noop deferred; bindings drift check noted.
Old closure fixtures preserved untouched.
