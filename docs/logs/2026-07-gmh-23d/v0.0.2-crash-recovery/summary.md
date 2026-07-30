# GMH-23D Crash Recovery Summary

Legacy proposal apply now persists a payload-bearing local recovery journal
before authority mutation, advances it after authority commit, and records the
consumed governance result. Retries recover the original apply outcome instead
of repeating section, changelog, audit, or rollback writes.

The journal binds governance request, proposal, idempotency key, proposal
digest, and expected governance version. Changed bindings fail with conflict;
missing committed or consumed results fail closed.

Production typed Memory writes and GMH-24 cutover remain disabled. G2D remains
open until the required real desktop acceptance passes.
