# Issue #32 P4.4 acceptance

## Engineering acceptance

Accepted locally: backend capability state is projected into provider specs;
deferred and unknown providers cannot trigger configuration or execution RPCs;
supported unconfigured providers can still be configured; and read/delete
paths remain available. Focused regressions, the full frontend suite, typecheck,
and production build pass.

## Product observation still required

On the exact P7 candidate, inspect the provider list and exercise a supported
unconfigured provider, a configured provider, and a deferred provider. Confirm
selection/test/refresh behavior matches backend state after reload and that
credentials remain write-only. This native candidate observation is not
claimed by frontend tests.
