# HQ-04 Release

HQ-04 is committed locally on `dev` and has not been pushed or externally released.

This batch adds no database migration and changes no saved configuration defaults. The provider
observer API is additive, and existing provider-level retry/final-wire listeners remain available as
a fallback. Manager/CLI/GUI admission observations use the stable HQ-03 wire fields and codes.

Rollback can revert the three implementation commits in reverse order:

1. `0226571e` — desktop backpressure/error UX and Manager/CLI cross-entry assertions.
2. `5176ac18` — session-worker supervision and panic recovery.
3. `2e3553fb` — request-scoped provider observers.

No session history, BML, Persona, MessageBus, or configuration data conversion is required. HQ-05
remains responsible for the final Epic release gate and closure decision.
