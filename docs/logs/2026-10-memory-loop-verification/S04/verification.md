# Verification

Original red/setup and green logs are retained in ../raw/. Commands, exit codes and named-test/subtest counts are recorded in evidence.json. No required skip is accepted. Product baseline is blocked and the evidence checker must return 1. For executable reproduction use the plan runbook and environment.md; abbreviated JSON commands are descriptors of recorded invocations.

The executed source predicate exited 1 with MEM-S04-01; request precondition passed. Equivalent reproduction from the DIVA root:

```sh
python3 - <<'PYCODE'
from pathlib import Path
import json
r=Path('docs/logs/2026-10-memory-loop-verification')
fact='synthetic random fact not repeated in assistant response'
assert fact in json.dumps(json.loads((r/'model-requests.json').read_text()))
assert fact in (r/'captured-source.txt').read_text() and json.loads((r/'snapshot.json').read_text())['canonical']['SourceRole']=='user', 'MEM-S04-01'
PYCODE
```

This is one diagnostic predicate, not acceptance of the entire S04 case matrix.
