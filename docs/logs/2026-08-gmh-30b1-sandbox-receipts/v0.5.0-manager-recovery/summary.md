# Summary

Manager bootstrap now opens the shared workspace `.laputa/governance.db`,
constructs the production Sandbox coordinator with that authority, and
reconciles incomplete command approvals before the runtime starts. Recovery is
bounded and request-id paginated, workspace/capability filtered, and revokes
Pending plus unconsumed Allowed states without reconstructing waiters or
command payloads.
