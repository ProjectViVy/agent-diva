# Issue #32 P4.3 acceptance

## Engineering acceptance

Vitest regressions prove concurrent subscribers share a listener installation,
each receives one copy of an event, and teardown/retry state is safe across
pending and completed installations.

## Product observation still useful

On a native Wails build, initialize the session, chat, and observability event
consumers concurrently and verify each event reaches each active consumer once.
Close and recreate the VIVY client, then verify old handlers no longer run.
This native-shell observation is not claimed by the unit suite.
