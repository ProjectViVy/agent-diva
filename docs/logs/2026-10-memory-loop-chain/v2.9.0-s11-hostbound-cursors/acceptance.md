# S11 partial developer acceptance record

| Check | Result | Evidence |
|---|---|---|
| Independent Profile A/B data roots | Pass in ordinary and scope-race runs | Separate real App/Garden/Mentle compositions and local SQLite roots |
| A canonical records and scoped search | Pass | Bounded result page and continuation cursor from A |
| Stale revision | Pass in ordinary run | Expand rejected with `revision_conflict`, no evidence |
| Foreign session, card, and mutation receipt | Pass in ordinary run | Host authorization denial or `effect_not_found`; no protected data returned |
| Forged and valid foreign cursor | Pass in ordinary and scope-race runs | Forged token rejected; B rejected A's valid token with `invalid_scope`, empty items and no next cursor |
| Cursor opacity and authentication | Pass in Garden adapter tests | Ciphertext does not expose offset/inner cursor; tampering, query change, and another adapter instance are rejected |
| B search and recall isolation | Pass in ordinary run | Search empty; recall probe observed; no A content in actual model request |
| Scope isolation under race | Pass, 1 sample | No A content in model input; ContextHost source was not invoked in this sample |
| Hostile memory stays data | Pass in ordinary run | Actual recall evidence reached the model as user data, not system authority |
| Mission, policy, and effects after hostile recall | Pass in ordinary run | Mission unchanged; trigger policy unchanged; no effect receipt added |
| Injection under race | Open | Final race output and error detail are recorded in `defects.md` |
| Backend recovery matrix | Not run | Disconnect/read-only/disk-full/index fault and recovery remain open |
| Workspace A/B matrix | Not run | Current product fixes cognitive profile identity to `diva`; workspace-specific conclusion remains blocked |

These are developer checks only. S11 remains Planned; S09 is a direct prerequisite and remains Planned.
