# Developer documentation (`docs/dev`)

## Entry points (linked from README / user guide)

- [development.md](./development.md) - workflows, tooling, local setup
- [architecture.md](./architecture.md) - high-level crate and data-flow overview
- [agent-plan/plan-mode-architecture.md](./agent-plan/plan-mode-architecture.md) - Plan Mode MVP runtime architecture
- [migration.md](./migration.md) - Python to Rust migration
- [mentle-integration/09-project-management.md](./mentle-integration/09-project-management.md) - Mentle sprint status and delivery governance
- [mentle-integration/10-package-source-policy.md](./mentle-integration/10-package-source-policy.md) - frozen `memtle` source, version, and upgrade policy
- [mentle-integration/11-s2-a3-published-crate-constraints.md](./mentle-integration/11-s2-a3-published-crate-constraints.md) - Sprint 2 published-crate feature and toolchain constraints
- [mentle-integration/12-s2-a8-sprint2-review-and-s3-interface-baseline.md](./mentle-integration/12-s2-a8-sprint2-review-and-s3-interface-baseline.md) - Sprint 2 review package and Sprint 3 runtime/tool adapter baseline
- [mentle-integration/13-s3-a1-memtle-toolkit-tool-interface.md](./mentle-integration/13-s3-a1-memtle-toolkit-tool-interface.md) - Sprint 3 `MemtleToolkitTool` adapter interface freeze
- [mentle-integration/14-s3-a2-dynamic-tool-registration-model.md](./mentle-integration/14-s3-a2-dynamic-tool-registration-model.md) - Sprint 3 dynamic `memtle_*` tool registration model
- [mentle-integration/15-s3-a3-toolkit-error-mapping.md](./mentle-integration/15-s3-a3-toolkit-error-mapping.md) - Sprint 3 Mentle toolkit error mapping and prompt-routing activation rule
- [mentle-integration/16-s3-a4-a6-mentle-runtime-assembly.md](./mentle-integration/16-s3-a4-a6-mentle-runtime-assembly.md) - Sprint 3 runtime ownership and assembly boundary
- [mentle-integration/17-s3-a7-test-and-verification-baseline.md](./mentle-integration/17-s3-a7-test-and-verification-baseline.md) - Sprint 3 minimum QA and verification baseline
- [mentle-integration/18-s3-a8-sprint3-review-package.md](./mentle-integration/18-s3-a8-sprint3-review-package.md) - Sprint 3 review package and Sprint 4 entry baseline
- [mentle-integration/19-s4-a1-sprint4-entry-audit.md](./mentle-integration/19-s4-a1-sprint4-entry-audit.md) - Sprint 4 AgentLoop entry audit
- [mentle-integration/20-s4-a8-adapter-runtime-compatibility-review.md](./mentle-integration/20-s4-a8-adapter-runtime-compatibility-review.md) - Sprint 4 adapter/runtime compatibility review
- [mentle-integration/21-s4-a9-regression-test-baseline.md](./mentle-integration/21-s4-a9-regression-test-baseline.md) - Sprint 4 regression test baseline
- [mentle-integration/22-s4-a10-mentle-feature-build-env.md](./mentle-integration/22-s4-a10-mentle-feature-build-env.md) - Sprint 4 Mentle feature build environment record
- [mentle-integration/23-s4-a11-sprint4-iteration-log.md](./mentle-integration/23-s4-a11-sprint4-iteration-log.md) - Sprint 4 iteration log summary
- [mentle-integration/24-s4-a12-sprint4-review-package.md](./mentle-integration/24-s4-a12-sprint4-review-package.md) - Sprint 4 review package and architecture sign-off
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
