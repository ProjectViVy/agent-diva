# Developer documentation (`docs/dev`)

## Entry points (linked from README / user guide)

- [development.md](./development.md) - workflows, tooling, local setup
- [architecture.md](./architecture.md) - high-level crate and data-flow overview
- [migration.md](./migration.md) - Python to Rust migration
- [mentle-integration/09-project-management.md](./mentle-integration/09-project-management.md) - Mentle sprint status and delivery governance
- [mentle-integration/10-package-source-policy.md](./mentle-integration/10-package-source-policy.md) - frozen `memtle` source, version, and upgrade policy
- [mentle-integration/11-s2-a3-published-crate-constraints.md](./mentle-integration/11-s2-a3-published-crate-constraints.md) - Sprint 2 published-crate feature and toolchain constraints
- [mentle-integration/12-s2-a8-sprint2-review-and-s3-interface-baseline.md](./mentle-integration/12-s2-a8-sprint2-review-and-s3-interface-baseline.md) - Sprint 2 review package and Sprint 3 runtime/tool adapter baseline
- [mentle-integration/13-s3-a1-memtle-toolkit-tool-interface.md](./mentle-integration/13-s3-a1-memtle-toolkit-tool-interface.md) - Sprint 3 `MemtleToolkitTool` adapter interface freeze
- [mentle-integration/14-s3-a2-dynamic-tool-registration-model.md](./mentle-integration/14-s3-a2-dynamic-tool-registration-model.md) - Sprint 3 dynamic `memtle_*` tool registration model
- [mentle-integration/15-s3-a3-toolkit-error-mapping.md](./mentle-integration/15-s3-a3-toolkit-error-mapping.md) - Sprint 3 Mentle toolkit error mapping and prompt-routing activation rule
- [nano-runtime-packaging-plan.md](./nano-runtime-packaging-plan.md) - current-state plan for the nano line, shared runtime boundaries, and packaging strategy
- [bug-fixing-lessons-learned.md](./bug-fixing-lessons-learned.md) - detailed case studies of complex bugs and their solutions

## UPSP Integration (`upsp/`)

UPSP-RS (Universal Persona Substrate Protocol - Rust implementation) design documentation:

- [**UPSP-RS Architecture Design**](upsp/upsp-rs-architecture-design.md) - complete architecture design (1500+ lines)
- [**UPSP Documentation Index**](upsp/README.md) - quick navigation and overview

## Archived material (`archive/`)

Long-form design notes, nano/packaging narratives, roadmaps, and research live under [`archive/`](./archive/). Start from the index:

- [**Nano / packaging index**](archive/nano/agent-diva-nano-master-spec.md) - boundaries, literature links, archive pointers
- [**Roadmaps / follow-ups**](archive/roadmaps/) - provider catalog plan, selection follow-ups, SOUL checklist
- [**QA**](archive/qa/blackbox-test-checklist.md) - manual black-box checklist
- [**Research**](archive/research/README.md) - standalone bundle and Windows packaging notes
- [**Architecture reports**](archive/architecture-reports/README.md) - OpenClaw / Zeroclaw / SOUL deep dives
