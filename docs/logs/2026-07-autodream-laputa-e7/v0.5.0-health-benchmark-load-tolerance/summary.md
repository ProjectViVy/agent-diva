# Health Benchmark Load Tolerance

The Manager health benchmark no longer runs inside the parallel full-workspace
test lane, where Windows scheduling and concurrent binaries dominate timing.
It is an explicit `just health-benchmark-check` gate in the E7 aggregate.

The budget is 5 seconds for 500 in-process requests, a strict 10 ms/request
ceiling. The handler behavior, response shape and production path are unchanged.
