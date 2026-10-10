# S08 V19 automatic authority boundary — developer checkpoint

Recorded: 2026-10-10. This is development evidence, not formal acceptance or a sealed candidate.

## Result

The actual VIVY App/Garden/Mentle test now places unique synthetic markers in the persisted WORLD document and ACTMEM Work, starts a real primary turn, and inspects the captured provider request. Neither marker is automatically injected, and the ordinary Mentle ContextHost source is observed for the same session. Afterward, explicit WORLD and ACTMEM reads return the markers through `module.action.invoke`; each exact request and response is retained in `raw/s08-focused-suite.txt`.

The test also confirms that `diva.cognitive.persona.read` and `diva.cognitive.actmem.read` do not appear as model-callable tools in the actual provider request. VIVY's `diva-cognitive/module.go` declares these as human control-plane actions, not Agent tools. Therefore this checkpoint closes the automatic-injection evidence gap but **does not close V19**: the explicit reads are auditable host actions, not reads issued by the Agent through a model-visible tool.

## Verification

The S08 focused package-mode run passed all 3 top-level tests and 9 profile/control subtests (12 PASS, 0 FAIL, 0 SKIP), exit code 0, elapsed 69.6 seconds. It includes three process-restart recall profiles, six empty/disabled controls, and the new authority-boundary test. The raw output is `raw/s08-focused-suite.txt`.

The first attempt was blocked before any test assertion because the sandbox denied `httptest` binding to `[::1]:0`. Turn-scoped local network permission was granted for the loopback-only synthetic model; the identical focused command then passed. No external network, live model, or user profile was used. The successful log includes three cron-startup cancellation warnings while empty-profile fixtures shut down; all named tests passed.

## Remaining gate

S08 remains Planned. V19 still needs a designed and implemented model-visible read tool with a real provider request/tool-call/result trace and the existing host-bound authority checks. The S01–S03 formal prerequisites and same-candidate acceptance also remain open. No product runtime behavior changed in this checkpoint.

VIVY test commit: `bec6cc7d867d8cf4000156937c5aca24f64846e4`.
