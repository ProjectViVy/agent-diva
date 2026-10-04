# W4-1 Acceptance

| Requirement (W4) | Evidence |
|---|---|
| T1 behavioral vectors | wav_test.go 10 invalid + bound cases; CAS conflict, late-settle, cancel/settle-distinct cases in service_test.go |
| T2 native stores + provider HTTP | keyring seam (no plaintext, presence-only), atomic config CAS, asset caps+leases+reconcile, verified endpoints/timeouts/caps/redaction in providers.go |
| T3 cancellable service | registry admit/cancel/settle/deliverable; identity checked at admission+return; hide/reload/quit invalidation wired; late success → stale_context discard (test) |
| T4 binary media adapter | 4 routes on internal mux; grant before body reads; 2 KiB meta + byte caps; typed JSON errors; no listener |
| T5 preserved frontend API | same 8 commands + media paths + `speech:diagnostic`; pnpm 551/551, vue-tsc clean |
| T6 failure-path verification | scripted httptest providers (error/slow/cancel); keyring failure fake; real-provider rows pending credentials |

GO (Linux leg). Windows leg pending W0 Windows probe acceptance.
