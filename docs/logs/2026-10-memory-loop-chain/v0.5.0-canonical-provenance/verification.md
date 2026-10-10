# Verification

| Check | Actual named results | Exit | Log |
|---|---|---|---|
|chain-source-provenance-red|0 pass /4 fail /0 skip|1|raw/chain-source-provenance-red.jsonl|
|chain-source-provenance-regression|6 pass /0 fail /0 skip|0|raw/chain-source-provenance-regression.jsonl|
|chain-source-update-provenance-red|0 pass /1 fail /0 skip|1|raw/chain-source-update-provenance-red.jsonl|
|chain-native-provenance-update-red|0 pass /1 fail /0 skip|1|raw/chain-native-provenance-update-red.jsonl|
|chain-native-provenance-update-regression|69 pass /1 fail /0 skip|1|raw/chain-native-provenance-update-regression.jsonl|
|chain-native-provenance-update-final|70 pass /0 fail /0 skip|0|raw/chain-native-provenance-update-final.jsonl|
|chain-provenance-native-baseline|69 pass /0 fail /0 skip|0|raw/chain-provenance-native-baseline.jsonl|
|chain-provenance-garden-final-union|486 pass /0 fail /0 skip|0|raw/chain-provenance-garden-final-union.jsonl|
|chain-provenance-final-mentle|537 pass /0 fail /0 skip|0|raw/chain-provenance-final-mentle.jsonl|
|chain-real-app-source-provenance|2 pass /0 fail /0 skip|0|raw/chain-real-app-source-provenance.jsonl|
|chain-provenance-final-composition|13 pass /0 fail /0 skip|0|raw/chain-provenance-final-composition.jsonl|
|chain-provenance-default-app|133 pass /0 fail /12 skip|0|raw/chain-provenance-default-app.jsonl|

Run Mentle and Garden go test -json ./... -count=1 after sourcing the task environment, with local loopback permission. Actual composition follows v0.4 with TestMemoryLoopReflectionProvenanceAfterRestart added. Run default ./internal/app separately; its12 conditional skips are not DIVA acceptance. Final required just ci/conformance reproduction/new pack/Inspect/native identity/fresh review remain pending for final frozen source.

An intermediate native regression failed because its test expected an outbox job after normal synchronous indexing had consumed it. The corrected negative derived-index test disables only the disposable projection and reads the real pending job. Both logs remain unchanged. OpenCatalogService is the supported writable canonical/BM25 development seam, not an injected Backend. Actual App positives use configured real ONNX composition.
