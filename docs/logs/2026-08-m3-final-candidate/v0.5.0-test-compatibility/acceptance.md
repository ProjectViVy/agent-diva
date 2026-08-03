# Acceptance

Acceptance is automated: GUI Rust 1.94 all-target Clippy must pass, the affected
Feishu and filesystem tests must remain green, and the full workspace gates must
emit neither of the two previously recorded unused-fixture warnings.

No manual observation is required for this compatibility-only slice.
