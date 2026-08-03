# Summary

GMH-33 closes the CLI/headless portion of the M3 governance loop.

- Added one unified `approvals` CLI surface for paged list, explicit decision,
  cancel, and batch interactive review across Command, Plan, and Memory.
- Added `agent --approval-mode fail|queue` with fail as the default and stable
  machine-readable approval outcomes through `--json`.
- Local non-interactive Command escalation uses the workspace governance ledger,
  writes a durable denial, and never executes without explicit approval.
- Remote fail mode observes the Manager projection, revokes a newly created
  request, and exits non-zero with `approval_required_noninteractive`.
- Explicit queue returns a queryable Plan/Memory Pending without waiting for
  execution; Pending is not reported as success. Command queue is revoked and
  returns `approval_queue_unavailable` because raw command payload is not durable.
- No command, prompt, Memory patch, credential, or provider payload was added to
  ledger, CLI fixture, or logs.
