# Acceptance

- focused health benchmark passes;
- full workspace tests explicitly skip the timing-only benchmark;
- the aggregate gate executes the benchmark in its dedicated lane;
- 500 health requests must still complete in less than 5 seconds;
- the final E7 aggregate gate passes.
