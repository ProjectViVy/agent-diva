# Health Benchmark Load Tolerance

The Manager health benchmark no longer fails solely because the full Windows
workspace test gate is linking and running many binaries concurrently.

The budget changes from 3 seconds to 5 seconds for 500 in-process requests,
which remains a strict 10 ms/request ceiling. The handler behavior, response
shape and production path are unchanged.
