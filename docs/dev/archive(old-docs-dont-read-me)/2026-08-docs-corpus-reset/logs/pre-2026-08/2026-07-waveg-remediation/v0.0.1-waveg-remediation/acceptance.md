# Wave G Remediation Acceptance

1. Open a LiteLLM path where the provider omits `usage`; confirm downstream responses no longer contain fake `0` usage fields and that `UsageMissingFallback` is emitted.
2. Confirm timeout-triggered tool failures still return `ToolError::Timeout` while logging structured timeout context with tool name and timeout seconds.
3. Set `logging.retention_days = 0` and verify existing gateway logs are preserved after startup.
4. Run `just feature-gate-check` or `python scripts/feature-gate-check.py` on the host platform and confirm the sandbox feature matrix passes.
5. Review `TODOLIST.md` and confirm the five Wave G residuals are closed and replaced by one unrelated validation residual for manager clippy cleanup.
