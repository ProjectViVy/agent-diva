# Issue #32 P4.2 acceptance

## Engineering acceptance

Local headless lifecycle and speech regressions, `go vet`, and speech race
passed. Source inspection of Wails v3 beta.27 confirmed that services shut down
in reverse registration order and the single-instance manager is cleaned up
only after service shutdown completes. The runtime drain barrier preserves
that ordering for teardown workers that outlive the bounded runtime callback.

## Product acceptance still required

Run on native Linux and Windows candidates after the canonical SDK pins and
native toolchains are usable:

1. Start a primary with a valid profile, hide its window, and start a secondary
   with a different valid config. Confirm one focus/reopen request and no
   secondary-created Journal/profile.
2. Force a primary composition failure, then launch a corrected configuration;
   confirm the singleton and organism lease can be acquired by the new process.
3. Exercise a blocked event-pump drain beyond the caller budget. Confirm the
   process keeps the singleton lock until the producer, speech tasks and host
   close finish; confirm no show/reopen occurs after admission closes.
4. Confirm graceful quit permits immediate relaunch and crash restart follows
   the existing organism lease/TTL contract.

No native candidate was accepted by this iteration.
