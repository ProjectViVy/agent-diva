# Issue #32 P4.3 — Shared native event listener

## Scope

Implemented H6 in the isolated `feat/issue32-desktop-gates` branch. Concurrent
VIVY event subscribers now share one pending Wails listener installation.
Subscribers await installation; an installation error reaches every pending
subscriber, removes failed handlers, and permits a later explicit retry.

Closing during installation detaches a late successful listener once. Closing
after installation is idempotent, and post-close subscription does not install
another native listener. Per-subscriber unsubscription removes only its handler.

## Delivery

Implementation and regressions are committed as `8dbab9be`. No push, merge,
tag write, upload, or release was performed. P4.2's native two-process
acceptance remains separately pending.
