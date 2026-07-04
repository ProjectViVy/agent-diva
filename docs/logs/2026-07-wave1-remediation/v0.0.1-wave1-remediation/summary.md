# Wave 1 Remediation Summary

## Scope

- Close the Wave 1 `Do not ship` blockers on E2E false-green behavior, token-budget enforcement, supervised-run cancellation semantics, and security production wiring.

## Changes

- Integrated the child-agent fixes already landed on branch:
  - `497bbcb` `fix: harden e2e judge and failure handling`
  - `0ab55a6` `fix: enforce supervised run cancellation semantics`
  - `1de984d` `fix: wire token budget enforcement`
- Wired `check_security()` into the main inbound-message hot path after attachment expansion and before session/provider work.
- Changed `check_security()` resolution so injection and hierarchy-conflict findings outrank PII sanitize, while still preserving PII findings for audit/reporting.
- Added security audit emission from the core decision path.
- Removed the misleading unconditional `SkillLoaded` audit from `validate_skill_md()`.
- Enforced skill-upload security checks in `agent-diva-manager` and only emit `SkillLoaded` after a successful install with review-tier provenance.
- Sanitized successful tool output before returning it through `ToolRegistry`.

## Outcome

- Wave 1 release blockers are remediated on the reviewed hot paths.
- One residual limitation remains: budget configuration is loaded from workspace or home security files instead of being explicitly constructed by manager config wiring.
