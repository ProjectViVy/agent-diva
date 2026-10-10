# Manual cognitive trigger override

The real App test `TestMemoryLoopManualTriggerOverridesBusyAndIntervalGates`
passed under `-race`: 1 pass, 0 skips, exit 0, 74.75 seconds. It exercised the
actual timer, foreground model request, DIVA module action, canonical source,
and cognitive owner.

After the interval policy deferred a new source, the test held a third
foreground request open and invoked the public manual trigger. The trigger
admitted the original pending window while the foreground owner was busy. The
actual provider emitted exactly seven requests: one foreground plus three
reconcile/reflect pairs. The manual pair covered the same `[after, through]`
window, advanced the watermark to source 2 once, and created no repeated
requests. Releasing the foreground call persisted source 3 once; the 120-second
automatic interval remained active, so that later source stayed pending and
did not create an automatic request.

The observation includes nine hashed artifacts. Final status was idle with no
active run, watermark and pending-through both at 2, and the three original
sources preserved. Model counts were seven requests and seven completed
response handlers; the fixture does not claim client-side response
consumption. This is developer evidence, not formal Story acceptance or a
frozen-candidate attestation.

Two earlier exit-1 logs are retained alongside the passing race log. They show
a test assertion mistake: it searched for reconcile and reflect in each
individual provider request, although the actual protocol sends one stage per
request. The test now checks the paired requests against the same original
window; no product code changed for this case. The assertion correction is in
VIVY commit `15d3e469`.
