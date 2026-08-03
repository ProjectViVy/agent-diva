# Summary

Replaced lossy fallible line iteration in the AutoDream run-event reader with
explicit I/O error propagation. Malformed JSON remains safely ignored, while a
real line read failure now reaches the caller instead of being silently hidden.
The same Clippy restoration slice also applies the equivalent strict-greater
assertion style required by Rust 1.94 in the AutoDream service integration test.
