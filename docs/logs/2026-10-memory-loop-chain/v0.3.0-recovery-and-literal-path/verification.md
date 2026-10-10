# Verification

| Check | Actual result | Raw log |
|---|---|---|
| chain-unresolved-settlement-red | 0 pass / 4 fail / 0 skip; fail | raw/chain-unresolved-settlement-red.jsonl |
| chain-unresolved-identity-red | 0 pass / 5 fail / 0 skip; fail | raw/chain-unresolved-identity-red.jsonl |
| chain-work-receipt-red | 0 pass / 3 fail / 0 skip; fail | raw/chain-work-receipt-red.jsonl |
| chain-rejected-app-red | 0 pass / 1 fail / 0 skip; fail | raw/chain-rejected-app-red.jsonl |
| chain-rejected-app-regression | 1 pass / 0 fail / 0 skip; pass | raw/chain-rejected-app-regression.jsonl |
| chain-sqlite-path-identity-red | 0 pass / 4 fail / 0 skip; fail | raw/chain-sqlite-path-identity-red.jsonl |
| chain-sqlite-literal-final-regression | 158 pass / 0 fail / 0 skip; pass | raw/chain-sqlite-literal-final-regression.jsonl |
| chain-recovery-final-runtime | 81 pass / 0 fail / 0 skip; pass | raw/chain-recovery-final-runtime.jsonl |
| chain-recovery-final-composition | 12 pass / 0 fail / 0 skip; pass | raw/chain-recovery-final-composition.jsonl |
| chain-recovery-final-default-app | 133 pass / 0 fail / 11 skip; pass | raw/chain-recovery-final-default-app.jsonl |
| chain-work-receipt-regression | 67 pass / 0 fail / 0 skip; pass | raw/chain-work-receipt-regression.jsonl |
| chain-garden-work-receipt-regression | 480 pass / 0 fail / 0 skip; pass | raw/chain-garden-work-receipt-regression.jsonl |
| chain-cancellation-ledger-regression | 10 pass / 0 fail / 0 skip; pass | raw/chain-cancellation-ledger-regression.jsonl |

Source identities and SHA-256 hashes are in checkpoint.json. The default App run validates its untouched inventory; its eleven conditional skips are not DIVA acceptance. The diagnostic DIVA selection compiles current App files except default_generation_test.go under the prior generated overlay. Final frozen-source just ci, executable conformance reproduction, SDK pack/Inspect and native source identity are still required.

Re-run from VIVY after sourcing the task environment. Use the anchored runtime regex documented in the paired VIVY iteration, and add TestMemoryLoopRejectedEffectRemainsVisibleAndFenced to the previous composition selection. Set VIVY_CAPTURE_COGNITIVE_FIXTURE to an owned task log path. Run the untouched ./internal/app package separately. Library and Garden raw records come from go test -json ./... -count=1 at their module roots. Local loopback permission is required.
