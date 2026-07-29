use agent_diva_core::session::TokenUsage;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct IterationPass {
    pub index: usize,
    pub summary_only: bool,
}

/// Owns the bounded model-loop counters, including the single summary-only pass.
#[derive(Debug, Default)]
pub(crate) struct IterationBudget {
    completed: usize,
    summary_bonus_remaining: usize,
    summary_nudge_injected: bool,
}

impl IterationBudget {
    pub(crate) fn can_continue(&self, max_iterations: usize) -> bool {
        self.completed < max_iterations + self.summary_bonus_remaining
    }

    pub(crate) fn begin_pass(&mut self, max_iterations: usize) -> IterationPass {
        self.completed += 1;
        let summary_only = self.completed > max_iterations;
        if summary_only {
            self.summary_bonus_remaining = 0;
        }
        IterationPass {
            index: self.completed,
            summary_only,
        }
    }

    pub(crate) fn take_summary_nudge(&mut self) -> bool {
        if self.summary_nudge_injected {
            false
        } else {
            self.summary_nudge_injected = true;
            true
        }
    }

    pub(crate) fn grant_summary_bonus_if_exhausted(&mut self, max_iterations: usize) -> bool {
        if self.completed >= max_iterations && self.summary_bonus_remaining == 0 {
            self.summary_bonus_remaining = 1;
            true
        } else {
            false
        }
    }
}

/// Data handed from model iteration to finalization.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct IterationOutcome {
    pub content: Option<String>,
    pub reasoning: Option<String>,
    pub token_usage: Option<TokenUsage>,
    pub stopped_for_plan_approval: bool,
}

impl IterationOutcome {
    pub(crate) fn resolved_content(&self) -> &str {
        self.content.as_deref().unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retains_text_and_plan_barrier_state_for_finalization() {
        let outcome = IterationOutcome {
            content: Some("done".into()),
            stopped_for_plan_approval: true,
            ..Default::default()
        };
        assert_eq!(outcome.resolved_content(), "done");
        assert!(outcome.stopped_for_plan_approval);
    }

    #[test]
    fn grants_and_consumes_exactly_one_summary_only_pass() {
        let mut budget = IterationBudget::default();
        assert!(budget.can_continue(1));
        assert_eq!(
            budget.begin_pass(1),
            IterationPass {
                index: 1,
                summary_only: false,
            }
        );
        assert!(budget.grant_summary_bonus_if_exhausted(1));
        assert!(budget.can_continue(1));
        assert_eq!(
            budget.begin_pass(1),
            IterationPass {
                index: 2,
                summary_only: true,
            }
        );
        assert!(!budget.can_continue(1));
        assert!(budget.take_summary_nudge());
        assert!(!budget.take_summary_nudge());
    }
}
