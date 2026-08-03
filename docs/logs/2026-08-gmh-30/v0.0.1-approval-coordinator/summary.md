# Summary

Added the GMH-30A domain-neutral approval coordinator in `agent-diva-core`.
The coordinator composes the existing pure policy evaluator and append-only
governance ledger, giving Plan, Sandbox, and Memory one pending-review boundary
without persisting domain payloads or creating parallel authority.
