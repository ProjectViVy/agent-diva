# C06 acceptance record

| Check | Result | Evidence |
|---|---|---|
| Real process reaches post-canonical/pre-index boundary | Pass, 10/10 | C06 handshake records child PID, operation ID, target, revision, canonical status, and pending index status |
| Pending index job is durable before crash | Pass, 10/10 | SQLite `index_jobs` row is pending with zero attempts before the process is killed |
| New process recovers the derived index | Pass, 10/10 | Startup drains the outbox; status reports `index_state=ok`; public search returns the same target and revision |
| Canonical memory is not duplicated | Pass, 10/10 | Canonical count remains 2 and target revision remains 1 |
| Receipt reflects recovered index state | Pass, 10/10 | Same operation, target, revision, and applied canonical receipt; `index_status=ready` |
| Unknown DIVA workflow remains safe | Pass, 10/10 | `recovery_required` / `unknown_outcome`; active run remains; watermark stays 0 |
| Restart performs another inference | Pass, 10/10 | Three pre-crash model requests; zero new requests after restart observation window |
| C03–C05 receipt-fix regression | Pass, one sample each | `raw/c03-c05-after-c06-fix-race1.jsonl` |
| Laputa facade tests | Pass | `raw/c06-laputa-facade-tests.txt` |
| VIVY App build and diff check | Pass | `raw/c06-vivy-build.txt` |

No result in this checkpoint promotes S10 from Planned or satisfies final candidate sealing.
