# Verification

Actual App diagnostic overlay, export enabled: TestMemoryLoopEvidenceExportsActualLargeUserSource, TestMemoryLoopExcludesDerivedEvidence and TestMemoryLoopConcurrentWakeKeepsOneOriginalActiveWindow. chain-evidence-export-final.jsonl: 3 pass, zero skip, observed shell exit0, 38.857 seconds. Initial standalone large-source probe: one pass, exit0 (chain-evidence-recorder-first.jsonl).

The external artifact check imports existing check_memory_loop_evidence.py and applies its _file/_path integrity rules to every exported manifest artifact, checks registered bytes, developer kind and explicit non-acceptance. chain-evidence-artifact-check.txt records the actual count. It does not invoke a formal Story gate.

Existing Python evidence checker regression: python3 -m unittest discover -s scripts/ci -p test_check_memory_loop_evidence.py: 24 tests pass, observed exit0 (chain-evidence-checker-final.txt). Report actual current count, not the older planning count.

Overlay is diagnostic; no new final source/artifact claim. Final CI/conformance/SDK pack/Inspect/native identity, fresh whole-phase review and complete local matrix remain open.
