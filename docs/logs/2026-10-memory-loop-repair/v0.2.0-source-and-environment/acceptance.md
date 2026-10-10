# Acceptance and continuation

Source `/workspace/work/memory-loop/tools/environment.sh` in this workspace and export `npm_config_registry=https://registry.npmjs.org/` to override the unavailable UI mirror. It uses task-local tools, Go cache, pnpm store, PowerShell XDG directories and extracted native libraries; it changes neither HOME nor system packages.

From VIVY run `just ci`. From DIVA run the supported `scripts/build-desktop.py --mode test` with explicit isolated VIVY/Laputa roots; use `--mode build --development --output <empty directory>` for a development artifact. Never count a required test skip as success.

Continue S04 negative/recovery cases, S03 reflection/recall/process fixture contracts, then S05–S11 according to their actual available inputs. Each repair produces a new candidate and real evidence; formal acceptance follows the original dependency graph. Only S12 native Windows and S13 live-model resources require external provision at present.

Rebuilt final development inputs: VIVY `2c184453`, Laputa `92d0b634`; the DIVA build closure is recorded separately in native-build-report.json. Shared integration and native host generation identities intentionally differ and are separately identified in checkpoint.json. The former verifies real App/canonical/process paths; the latter verifies supported sealed native host compilation and race tests. Neither is the final Windows/live acceptance candidate.

Next owner action: source the recorded environment, read the live README and continuity.md, reproduce the next S04 or S03 contract gap, and implement its smallest owner-side fix. Preserve the old failed baseline; a new acceptance run needs a new complete candidate record, not edits to historical evidence.

The next S03 fixture increment has an existing product route; it does not need Windows or live credentials:

1. Keep the actual App/Journal/bound Garden composition. Extend only the loopback provider response protocol for the existing `[cognitive-infer stage=reconcile|reflect]` tasks. Use Laputa's canonical prompts/schemas (`laputa/evolution/diva/prompts.go`), parse actual request input and cite actual batch source IDs. Do not answer from a test-owned random-fact closure.
2. Read the current policy through `diva.cognitive.policy.get`, then enable the isolated profile with `diva.cognitive.policy.set` and its actual CAS revision. The default policy is disabled; an untriggered workflow is not proof of unavailable infrastructure. Observe real workflow/effect receipts through the existing public control actions.
3. Implement `Wait(reflected)` from committed effects and `Restart` from the established two-process protocol, retaining separate stage/PID evidence. The current explicit unsupported errors are developer work, not external resource deferrals. Existing governed cognitive children and the pinned Eino adapter already own inference; no extra product model loop is required.

These steps are an inspected continuation route, not executed reflection or recall acceptance. S04/S06/S08 still require their own positive, negative and causal assertions.
