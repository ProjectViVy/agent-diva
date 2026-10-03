# Verification — DN-2 v0.2.0 (task 5)

- Command: `SENSENOVA_API_KEY=$SHANGTANG_APIKEY python3 drive.py`
  inside `~/diva-dn2-accept/` (driver loads the staged sealed .so via
  ctypes; no mocks, no fake transport).
- Result: `=== ALL CHECKS PASSED ===` (16 checks, see drive.out).
- Transcript: 116 run/event + RPC records in transcript.jsonl.
- Secret hygiene: grep of all evidence for key material → clean.
- Note: `token.sensenova.cn` TLS handshake is slow from this box
  (~30-60s; curl needs `-m 90 --http1.1`). Model calls inside VIVY
  succeed; session auto-title util calls can hit their shorter deadline
  (INFO log, non-blocking).
- Not covered (still pending per index): Windows/amd64 c-shared + FFI
  acceptance runner; GUI-level window reopen against live model
  (projection already unit-verified; reopen semantics proven here via
  review/list + session/messages snapshot readback).
