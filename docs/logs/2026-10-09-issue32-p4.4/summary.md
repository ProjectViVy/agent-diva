# Issue #32 P4.4 — Provider capability UI

## Scope

Implemented C8 in the isolated `feat/issue32-desktop-gates` branch. Provider
execution controls now follow the capability exposed by the active VIVY
backend profile and catalog. Deferred, unknown, missing, and ambiguous profile
states fail closed; supported unconfigured providers remain configurable.
The provider list and model browsing stay available for deferred entries, and
operators can still delete them.

The UI disables provider selection, test, refresh, and save actions when the
backend denies execution, with a localized explanation that includes the
backend capability state. API guards reject direct calls before any upsert,
refresh, or model-select RPC. Doctor readiness now follows the active
configured provider instead of an unrelated ready profile.

## Delivery

Implementation and regressions are committed as `3879cb3f`
(`fix(settings): honor backend provider capabilities`). Evidence, acceptance,
and the DIVA issue backlog update are recorded in the following commit.
P7's real-candidate observations remain open. No push, merge, tag write,
upload, or release was performed.
