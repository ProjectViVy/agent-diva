# W5-1 Verification

## Commands run (Linux headless)
- `python3 scripts/build-desktop.py --mode build --development` (post-W4
  commit): sealed ELF, build-report clean (`dirty` absent), generation
  `7070607d…`.
- Fresh profile: `HOME=/tmp/w5-home DEEPSEEK_API_KEY=… VIVY_PROVIDER=deepseek
  VIVY_API_BASE=http://127.0.0.1:18321 xvfb-run -a ./diva` →
  `vivy host opened`, files seeded, `Quitting application…`.
- Crash: SIGKILL → relaunch → `vivy host opened` on same journal.
- Single instance: second launch while running → `organism lease held`.
- `python3 scripts/ci/check_wails_candidate.py docs/plans/diva-next/fixtures/wails-candidate-acceptance.json --build-report … --binary …`
  → `rows: 17 passed=8 pending=9 failed=0` → OK.
- Prior gates still green: `go test -race -tags 'gtk3 vivy_headless'
  ./internal/desktop ./internal/speech ./cmd/diva`; `--mode test`;
  pnpm 551/551; vue-tsc.

## Fixture artifacts
- `wails-candidate-acceptance.json` — 17 rows, per-row requirement +
  outcome + evidence (+owner when pending).
- Old fixtures (`closure-packaged-obs`, `closure-chat-obs`,
  `closure-speech`) preserved untouched.

## Honest gaps
- No GUI driver for webkit2gtk on this VM → interactive rows pending.
- No operator credentials → real-provider rows pending.
- No Windows machine → Windows matrix pending (W0 leg gate unchanged).
