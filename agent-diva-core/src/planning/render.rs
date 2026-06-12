//! Deterministic markdown rendering for plans and todo lists.

use std::fmt::Write;

use super::model::{Plan, PlanStep, TodoItem, TodoList, TodoStatus, TodoPriority};

/// Render a [`TodoList`] as a deterministic markdown checklist.
///
/// Items are grouped by status in a fixed order. Empty groups are omitted.
pub fn render_todo_md(todo_list: &TodoList) -> String {
    let mut out = String::new();

    let groups: &[(&str, TodoStatus)] = &[
        ("In Progress", TodoStatus::InProgress),
        ("Pending", TodoStatus::Pending),
        ("Blocked", TodoStatus::Blocked),
        ("Completed", TodoStatus::Completed),
        ("Canceled", TodoStatus::Canceled),
    ];

    for (label, status) in groups {
        let items: Vec<&TodoItem> = todo_list
            .items
            .iter()
            .filter(|i| i.status == *status)
            .collect();

        if items.is_empty() {
            continue;
        }

        let _ = writeln!(&mut out, "## {label}");
        let _ = writeln!(&mut out);

        for item in &items {
            let checkbox = if item.status == TodoStatus::Completed {
                "- [x]"
            } else {
                "- [ ]"
            };

            let mut line = format!("{checkbox} {}", item.title);

            match item.priority {
                TodoPriority::High => line.push_str(" **[HIGH]**"),
                TodoPriority::Low => line.push_str(" [LOW]"),
                TodoPriority::Normal => {}
            }

            if let Some(reason) = &item.block_reason {
                let _ = write!(&mut line, " — blocked: {reason}");
            }

            if let Some(evidence) = &item.evidence_ref {
                let _ = write!(&mut line, " (evidence: {evidence})");
            }

            let _ = writeln!(&mut out, "{line}");
        }

        let _ = writeln!(&mut out);
    }

    out
}

/// Render a [`Plan`] and its steps as a deterministic markdown document.
pub fn render_plan_md(plan: &Plan, steps: &[PlanStep]) -> String {
    let mut out = String::new();

    let _ = writeln!(&mut out, "# {}", plan.title);
    let _ = writeln!(&mut out);
    let _ = writeln!(&mut out, "**Goal:** {}", plan.goal);
    let _ = writeln!(&mut out, "**Phase:** {}", plan.phase);
    let _ = writeln!(&mut out, "**Status:** {}", plan.status);

    if let Some(strategy) = &plan.strategy {
        let _ = writeln!(&mut out);
        let _ = writeln!(&mut out, "## Strategy");
        let _ = writeln!(&mut out);
        let _ = writeln!(&mut out, "{strategy}");
    }

    if !plan.assumptions.is_empty() {
        let _ = writeln!(&mut out);
        let _ = writeln!(&mut out, "## Assumptions");
        let _ = writeln!(&mut out);
        for a in &plan.assumptions {
            let _ = writeln!(&mut out, "- {a}");
        }
    }

    if !plan.risks.is_empty() {
        let _ = writeln!(&mut out);
        let _ = writeln!(&mut out, "## Risks");
        let _ = writeln!(&mut out);
        for r in &plan.risks {
            let _ = writeln!(&mut out, "- {r}");
        }
    }

    if !plan.open_questions.is_empty() {
        let _ = writeln!(&mut out);
        let _ = writeln!(&mut out, "## Open Questions");
        let _ = writeln!(&mut out);
        for q in &plan.open_questions {
            let _ = writeln!(&mut out, "- {q}");
        }
    }

    if !steps.is_empty() {
        let _ = writeln!(&mut out);
        let _ = writeln!(&mut out, "## Steps");
        let _ = writeln!(&mut out);
        for (i, step) in steps.iter().enumerate() {
            let num = i + 1;
            let _ = writeln!(&mut out, "{num}. **{}** [{}]", step.title, step.status);
            if let Some(rationale) = &step.rationale {
                let _ = writeln!(&mut out, "   Rationale: {rationale}");
            }
            if let Some(expected) = &step.expected_output {
                let _ = writeln!(&mut out, "   Expected: {expected}");
            }
            if let Some(evidence) = &step.evidence_ref {
                let _ = writeln!(&mut out, "   Evidence: {evidence}");
            }
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::planning::ids::*;
    use crate::planning::model::PlanStatus;
    use chrono::Utc;

    fn make_todo(id: &str, title: &str, status: TodoStatus, priority: TodoPriority) -> TodoItem {
        TodoItem {
            id: TodoId(id.to_string()),
            plan_step_id: None,
            title: title.to_string(),
            detail: None,
            status,
            priority,
            evidence_ref: None,
            block_reason: None,
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn test_render_empty_todo() {
        let list = TodoList {
            plan_id: PlanId("p1".to_string()),
            revision: 1,
            items: vec![],
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        let md = render_todo_md(&list);
        assert!(md.trim().is_empty());
    }

    #[test]
    fn test_render_mixed_status() {
        let list = TodoList {
            plan_id: PlanId("p1".to_string()),
            revision: 1,
            items: vec![
                make_todo("t1", "Active task", TodoStatus::InProgress, TodoPriority::Normal),
                make_todo("t2", "Waiting task", TodoStatus::Pending, TodoPriority::High),
                make_todo("t3", "Done task", TodoStatus::Completed, TodoPriority::Low),
            ],
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        let md = render_todo_md(&list);

        assert!(md.contains("## In Progress"));
        assert!(md.contains("## Pending"));
        assert!(md.contains("## Completed"));
        assert!(!md.contains("## Blocked"));
        assert!(!md.contains("## Canceled"));

        assert!(md.contains("- [ ] Active task"));
        assert!(md.contains("- [ ] Waiting task **[HIGH]**"));
        assert!(md.contains("- [x] Done task [LOW]"));
    }

    #[test]
    fn test_render_blocked_with_reason() {
        let mut item = make_todo("t1", "Blocked task", TodoStatus::Blocked, TodoPriority::Normal);
        item.block_reason = Some("Waiting on API".to_string());

        let list = TodoList {
            plan_id: PlanId("p1".to_string()),
            revision: 1,
            items: vec![item],
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        let md = render_todo_md(&list);

        assert!(md.contains("## Blocked"));
        assert!(md.contains("— blocked: Waiting on API"));
    }

    #[test]
    fn test_render_deterministic() {
        let items = vec![
            make_todo("t1", "B task", TodoStatus::Pending, TodoPriority::Normal),
            make_todo("t2", "A task", TodoStatus::Pending, TodoPriority::High),
            make_todo("t3", "C task", TodoStatus::Completed, TodoPriority::Low),
        ];

        let list = TodoList {
            plan_id: PlanId("p1".to_string()),
            revision: 1,
            items: items.clone(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let md1 = render_todo_md(&list);
        let md2 = render_todo_md(&list);
        assert_eq!(md1, md2);
    }

    #[test]
    fn test_render_plan_md() {
        let now = Utc::now();
        let plan = Plan {
            id: PlanId("p1".to_string()),
            title: "Build Widget".to_string(),
            goal: "Ship widget by Friday".to_string(),
            phase: crate::planning::model::PlanPhase::Execute,
            status: crate::planning::model::PlanStatus::InProgress,
            strategy: Some("Iterative development".to_string()),
            assumptions: vec!["No blockers".to_string()],
            risks: vec!["Scope creep".to_string()],
            open_questions: vec!["Need design review?".to_string()],
            verification_verdict: None,
            created_at: now,
            updated_at: now,
        };

        let steps = vec![
            PlanStep {
                id: "s1".to_string(),
                plan_id: PlanId("p1".to_string()),
                ordinal: 0,
                title: "Design".to_string(),
                rationale: Some("Clarify requirements".to_string()),
                expected_output: Some("Spec doc".to_string()),
                status: PlanStatus::Completed,
                evidence_ref: Some("spec-v1.md".to_string()),
                created_at: now,
                updated_at: now,
            },
            PlanStep {
                id: "s2".to_string(),
                plan_id: PlanId("p1".to_string()),
                ordinal: 1,
                title: "Implement".to_string(),
                rationale: None,
                expected_output: Some("Working code".to_string()),
                status: PlanStatus::InProgress,
                evidence_ref: None,
                created_at: now,
                updated_at: now,
            },
        ];

        let md = render_plan_md(&plan, &steps);

        assert!(md.contains("# Build Widget"));
        assert!(md.contains("**Goal:** Ship widget by Friday"));
        assert!(md.contains("**Phase:** Execute"));
        assert!(md.contains("## Strategy"));
        assert!(md.contains("Iterative development"));
        assert!(md.contains("## Assumptions"));
        assert!(md.contains("- No blockers"));
        assert!(md.contains("## Risks"));
        assert!(md.contains("- Scope creep"));
        assert!(md.contains("## Open Questions"));
        assert!(md.contains("- Need design review?"));
        assert!(md.contains("## Steps"));
        assert!(md.contains("1. **Design** [Completed]"));
        assert!(md.contains("2. **Implement** [InProgress]"));
        assert!(md.contains("Rationale: Clarify requirements"));
        assert!(md.contains("Evidence: spec-v1.md"));
    }

    #[test]
    fn test_render_evidence_ref_todo() {
        let mut item = make_todo("t1", "Task with ref", TodoStatus::Completed, TodoPriority::Normal);
        item.evidence_ref = Some("commit-abc123".to_string());

        let list = TodoList {
            plan_id: PlanId("p1".to_string()),
            revision: 1,
            items: vec![item],
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        let md = render_todo_md(&list);
        assert!(md.contains("(evidence: commit-abc123)"));
    }
}
