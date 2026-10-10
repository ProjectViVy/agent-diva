# 复现命令

以下命令从 VIVY 任务 checkout 执行。实际 App 用 loopback 合成模型和独立 test-local SQLite 根；不是 live model 或生产数据。先创建 package-mode Go overlay：

```bash
source /workspace/work/memory-loop/tools/environment.sh
python3 - <<'PY'
import json
source = "/workspace/work/memory-loop/tools/recall-diagnostic-overlay/overlay.json"
target = "/tmp/diva-memory-loop-closeout-overlay.json"
with open(source, encoding="utf-8") as f:
    overlay = json.load(f)
overlay.setdefault("Replace", {})[
    "/workspace/work/memory-loop/agent-vivy/internal/app/default_generation_test.go"
] = ""
with open(target, "w", encoding="utf-8") as f:
    json.dump(overlay, f, indent=2)
PY
```

S08/S09 targeted actual-App suite:

```bash
export VIVY_CAPTURE_COGNITIVE_FIXTURE=/workspace/work/memory-loop/logs/chain-s08-s09-final-suite.json
go test -overlay /tmp/diva-memory-loop-closeout-overlay.json -json ./internal/app \
  -run '^(TestMemoryLoopRecallAfterProcessRestart|TestMemoryLoopRecallNegativeControls|TestMemoryLoopCorrectionAndDeletionInModelInput|TestMemoryLoopPublicCorrectionAndTombstoneAfterRestart|TestMemoryLoopRecallDeadlineDegradesSafely)$' \
  -count=1
```

S11 ordinary hostile-memory path and race-only injection/default-deadline controls:

```bash
export VIVY_CAPTURE_COGNITIVE_FIXTURE=/workspace/work/memory-loop/logs/chain-s11-injection.json
go test -overlay /tmp/diva-memory-loop-closeout-overlay.json -json ./internal/app -run '^TestMemoryLoopMemoryInjection$' -count=1
go test -overlay /tmp/diva-memory-loop-closeout-overlay.json -race -json ./internal/app -run '^TestMemoryLoopMemoryInjection$' -count=1
go test -overlay /tmp/diva-memory-loop-closeout-overlay.json -race -json ./internal/app -run '^TestMemoryLoopRecallDeadlineDegradesSafely$' -count=1
```

Garden cause-chain and unchanged wire behavior:

```bash
cd /workspace/work/memory-loop/laputa/garden
GOSUMDB=off go test -count=1 ./agentapi
```

The package-level invocation is required to honor mutually exclusive `race` and `!race` build tags. Passing named `.go` files directly to `go test` bypasses those package constraints and may compile both variants simultaneously. The overlay only hides the generated-default-only test referencing a method absent from the DIVA integration assembly; production sources are not replaced by this entry.
