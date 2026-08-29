# HQ-03 Release

HQ-03 is committed locally on `dev` and has not been pushed or externally released.

The configuration change is backward compatible. Existing files that omit
`agents.defaults.session_admission` receive these defaults:

```json
{
  "max_queue_depth": 2,
  "wait_timeout": 30,
  "idle_ttl": 600
}
```

The values are seconds except `max_queue_depth`, and zero is accepted only for queue depth. Saved
configuration may begin persisting the additive object.

Rollback is a focused revert of `011db1f3`. No database, session-history, BML, Persona, or MessageBus
data migration is required. After rollback, production Bus consumption returns to the HQ-02 serial
compatibility path and clients stop receiving the additive correlation/admission fields.
