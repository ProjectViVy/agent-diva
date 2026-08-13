# Wave G Remediation Summary

- Closed the five Wave G residuals identified in the 2026-07-05 review pass.
- `LiteLLM` missing usage no longer masquerades as authoritative zero-token usage; both non-streaming and streaming paths emit the same fallback audit visibility.
- `ErrorCategory` now treats timeout as retryable, preserves retryable server/API failure semantics better, and refines sandbox/security categorization.
- `global_tool_timeout_secs` now flows through runtime security loading into `ToolAssembly` and production `ToolRegistry` construction, with timeout failures logging the same structured context style as ordinary tool errors.
- `logging.retention_days = 0` now truly skips retention cleanup, and log-retention tests cover both keep-all and expired-file deletion behavior.
- Feature-gate validation is now a real CI gate via `just ci` and `.github/workflows/ci.yml`, using a cross-platform Python entrypoint and real sandbox parent-feature combinations.
- One unrelated validation residual remains: workspace `just check` still fails on pre-existing `clippy::manual_inspect` findings in `agent-diva-manager/src/skill_service.rs`.
