# Verification

Validated with:
- `cargo check -p agent-diva-agent -p agent-diva-laputa`
- `cargo test -p agent-diva-agent governance_context_does_not_inject_mentle_recall_by_default`
- `cargo test -p agent-diva-agent test_build_subagent_prompt_includes_applied_laputa_authority`
- `cargo test -p agent-diva-laputa proposals_reject_compaction_only_evidence_on_create_and_edit`

Result: all checks passed.
