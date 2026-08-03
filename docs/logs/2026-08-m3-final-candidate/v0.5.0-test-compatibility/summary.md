# Summary

This compatibility slice closes the remaining Rust 1.94 and test-warning debts
that were explicitly included in the M3 Goal.

- Rewrote the GUI logging fixture with struct update syntax.
- Removed three needless borrows from the Windows gateway process regression.
- Marked two intentionally retained test fixtures as unused without changing
  runtime behavior.

No production approval contract, persistence schema, or user-visible behavior
changed.
