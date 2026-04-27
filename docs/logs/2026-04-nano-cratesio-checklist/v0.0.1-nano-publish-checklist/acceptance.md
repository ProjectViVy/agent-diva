# Acceptance

1. Open [docs/dev/nano-crates-io-checklist.md](../../../../docs/dev/nano-crates-io-checklist.md).
2. Confirm the document lists the current minimum `agent-diva-nano` internal crate closure as:
   - `agent-diva-files`
   - `agent-diva-core`
   - `agent-diva-tooling`
   - `agent-diva-providers`
   - `agent-diva-tools`
   - `agent-diva-agent`
   - `agent-diva-nano`
3. Confirm the document gives an explicit recommended publish order for the above closure.
4. Confirm the document separates:
   - the immediate transition plan for publishing the current closure
   - the longer-term target of narrowing nano behind shared runtime/control-plane layers
5. Confirm the document includes a concrete `path` to crates.io `version` dependency migration section for `.workspace/agent-diva-nano/Cargo.toml`.
