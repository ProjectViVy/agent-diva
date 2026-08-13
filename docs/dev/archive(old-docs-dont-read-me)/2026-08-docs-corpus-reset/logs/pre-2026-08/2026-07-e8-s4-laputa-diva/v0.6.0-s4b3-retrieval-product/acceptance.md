# S4b-3 Acceptance

1. Search local profile memory and confirm only matching profile plus exact-session records are returned.
2. Confirm tombstones and `RemoteOnly` records never appear.
3. Confirm result order is stable, section-diverse, and bounded to eight.
4. Query Manager status and confirm Garden/sync are `not_configured/local_only`.
5. Confirm GUI calls status/search/section through the `@/api` barrel and has no direct `invoke`.
6. Resolve `RG-E8-S4b-MS1`, then rerun `cargo +1.80.0 check` before closing the parent S4b release gate.
