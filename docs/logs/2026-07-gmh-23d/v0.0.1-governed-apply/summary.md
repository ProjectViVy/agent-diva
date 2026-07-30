# GMH-23D Governed Apply Summary

Implemented the governance seam for Memory proposals. Proposal submission,
editing, approval, rejection, and legacy section apply now share a
digest/version-bound approval ledger and receipt contract. Manager owns actor
and time; Tauri and GUI no longer approve through generic transitions.

Added a non-production typed executor that validates an approve-once receipt
and atomically commits the canonical record, supersedes relations, FTS update,
store/record revisions, and a payload-free apply journal. It is deliberately
not registered as a production write path. No dual-write or GMH-24 cutover was
introduced.

G2D remains open until the required real desktop acceptance and the documented
legacy crash-window recovery hardening are complete.
