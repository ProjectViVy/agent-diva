//! Cross-crate flag: whether `memory_distill` ran successfully this session.
//!
//! Consolidation reads this flag to skip when an explicit distill path was
//! already used (inventory §10.1 #4 / Wave 5 S4).

use std::cell::Cell;

thread_local! {
    static EXPLICIT_DISTILL_RAN: Cell<bool> = const { Cell::new(false) };
}

/// Mark that `memory_distill` completed successfully in this session.
pub fn mark_distill_ran() {
    EXPLICIT_DISTILL_RAN.with(|c| c.set(true));
}

/// Check whether `memory_distill` ran in this session.
pub fn distill_ran_this_session() -> bool {
    EXPLICIT_DISTILL_RAN.with(|c| c.get())
}

/// Reset the flag (test helper only).
pub fn reset_distill_flag() {
    EXPLICIT_DISTILL_RAN.with(|c| c.set(false));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flag_defaults_to_false() {
        reset_distill_flag();
        assert!(!distill_ran_this_session());
    }

    #[test]
    fn mark_sets_flag() {
        reset_distill_flag();
        mark_distill_ran();
        assert!(distill_ran_this_session());
        reset_distill_flag();
    }
}
