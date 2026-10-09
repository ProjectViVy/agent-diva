# Verification

| Actual check | Result |
|---|---|
| Required VIVY `just ci` | Exit 0; final recorded whole recipe, all required stages passed |
| Capture source tests | RED then GREEN; completed/failed/cancelled, long user fact, missing source, replay |
| Affected VIVY runtime/observer/cognitive/App | 1048 pass / 3 skip / exit 0; not a full-CI claim |
| Real DIVA full App + ONNX/canonical | Random user fact persists; ack-only reply; source role parsed from actual envelope |
| Process continuity | 2 parent tests pass; capture/reopen child phases pass in distinct PIDs |
| Mentle standalone | 536 pass / 0 skip / exit 0; includes 8 actual MCP protocol tests |
| Garden process e2e | 2 pass / 0 skip / exit 0; real backend process included |
| Garden full regression | 480 pass / 0 skip / exit 0 |
| Laputa standalone | 63 pass / 0 skip / exit 0; prior unblock iteration |
| SDK synthetic external consumer | Initial managed VCS failure, focused fix GREEN |
| Supported sealed native host mode-test | Exit 0; sealed consumer race tests passed |
| Supported native development build | Rebuilt at VIVY 2c184453 / Laputa 92d0b634; Inspect succeeded; generation and binary hash in checkpoint.json |

Required VIVY `just ci` final recorded run exited 0 (vivy-full-ci-final-exit.json). It includes formatting, UI typecheck/build and 70 files / 527 tests, i18n, vet, complete Go suite, source-bound conformance, headless compilation and independent plugin/face gates. No Story gate is marked Done from these partial proofs. Logs are raw actual execution records; artifact/model metadata points to local task resources rather than pretending those binaries are embedded in Git evidence.

Fresh review and one fix pass are recorded in review.md; actual canonical content and accepted legacy receipt recovery now have separate positive regressions. Python evidence checker/test-asset regression: 25 pass. The reviewed native build and sealed race test both exit 0; native binary --help exits 0 with resolved runtime libraries.

Final frozen-source replay: shared artifact identity 2 pass / 0 skip; actual App/random fact/canonical and two-process parent tests 2 pass / 0 skip. The rebuilt native supported test exits 0, including its sealed consumer race suite; rebuilt binary --help exits 0. Shared/native generation identities are separate and must not be combined as one Story acceptance run.

The earlier review-full-affected-final.jsonl contained a NotifyInput fixture admission race. Its fix was validated by ten focused repetitions and the subsequent complete runtime package in required CI. That failed log is not counted as a green suite. The MCP inherited-path RED log and explicit source/canonical RED logs similarly remain first-failure evidence.

CLI help output is stored losslessly in native-source-bound-help.json (stdout and SHA-256), so the original tab indentation does not violate Git whitespace checks. The decoded bytes were compared to the raw task log.
