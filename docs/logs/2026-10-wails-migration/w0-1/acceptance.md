# w0-1 — Acceptance

User-visible outcome: none (internal probe + plan fixtures only).

Acceptance = the W0 task rows:

| Row | Status |
|---|---|
| Task 1 pin + compiling probe + bindings round trip + recorded hashes | DONE (Linux) |
| Task 2 native sender contract (window identity, forged/foreign/external denial) | DONE (Linux tests + source-verified header overwrite) |
| Task 3 internal media route, bounded WAV, raw bytes, cancel, revoked cap | DONE (Linux tests) |
| Task 4 lifetime/voice prerequisites on Windows x64 | PENDING — needs Windows machine |
| Task 5 keyring dep + no-fallback; Windows Credential Manager | Dep pinned, Linux leg DONE; Windows leg PENDING |
| Fixture `wails-native-probe.json` | DONE |
| Index/status update | DONE (index W0 row) |

GO/NO-GO: PENDING — blocked on Windows x64 per W0 exit gate. Product
adoption claims are deferred until then.
