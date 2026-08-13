# GMH-10 Iteration Summary

Added the domain-neutral `agent_diva_core::governance` contract for subjects,
capabilities, resources, risks, content digests, audit correlation, generic
approval requests, approval receipts, and stable validation errors.

The change is additive: Plan, Sandbox, and Memory retain their own payload and
existing approval types. Runtime policy, persistence, transport, and GUI
behavior are unchanged.
