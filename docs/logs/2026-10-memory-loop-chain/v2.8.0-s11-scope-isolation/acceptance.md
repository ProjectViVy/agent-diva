# S11 partial developer acceptance record

| Check | Result | Evidence |
|---|---|---|
| Independent Profile A/B data roots | Pass in ordinary and scope-race runs | Two real App/Garden/Mentle fixtures with separate configs |
| A canonical records and scoped search | Pass | A produced two ordinary memories; one bounded result returned |
| Stale revision | Pass in ordinary run | Expand rejected with revision_conflict, zero evidence |
| Foreign session | Pass in ordinary run | B host rejected A session at action authorization (-32009, not authorized) |
| Foreign card | Pass in ordinary run | effect_not_found; evidence page empty |
| Foreign mutation receipt | Pass in ordinary run | effect_not_found; returned receipt is zero-valued |
| Forged cursor | Pass in ordinary run | Rejected with invalid_scope; no entries or continuation cursor |
| Profile B search and recall isolation | Pass in ordinary run | Search empty; one actual recall query had zero candidates; model request contained no A facts; answer was 未知 |
| Scope race regression | Pass, 1 sample | No A facts or memory-context marker in B's actual model request; memory-source trace absent in this race run |
| Hostile memory stays data | Pass in ordinary run | Actual recall evidence preserved the text; actual model request placed it in user data, not system authority |
| Mission, policy, and effects after hostile recall | Pass in ordinary run | Mission unchanged, trigger policy remained disabled at the expected revision, no effect receipt added |
| Injection under race | Blocked by observed source error | Both real source queries returned material read failed; no injection request was produced |
| Valid foreign cursor | Blocked by missing pagination output | Search response had no next_cursor; implementation inspection confirms it is never produced |
| Backend disconnect/read-only/disk-full/index-fault recovery | Not run | Remaining S11 work |
| Workspace A/B authorization matrix | Not run | Current test covers distinct personal data roots only |

The ordinary combined command ran both actual-App tests once: 2 pass, 0 fail, 0 skip, exit 0. This table records developer checks only; S11 remains Planned.
