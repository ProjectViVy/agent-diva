# Acceptance

Run python3 -m unittest discover -s scripts/ci -p test_check_memory_loop_evidence.py. Validate a Story with --story Sxx --evidence-root docs/logs/2026-10-memory-loop-verification. Exit 0 only means complete consistent recorded evidence, not product execution; exit 1 is incomplete/failed and 2 malformed input.
