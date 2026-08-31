# C5 Octos capability migration planning summary

This documentation-only iteration freezes the implementation and handoff contract for
CHANNEL-EPIC C5.

- Pins the local Octos reference at
  `5ea987813de4fd2afdd1d78f2106ad2868f0d923` (`v2.0.3-rc.9`, Apache-2.0).
- Defines a repeatable Agent Diva/Octos scan, provenance policy, and a 28-capability matrix for all
  six active platforms.
- Freezes the native adapter module shape, attachment-service seam, identity/admission ordering,
  access, command, receipt, health/probe, dependency, and C5/C6 boundaries.
- Provides implementation specifications for Telegram, Discord, Feishu, DingTalk, Email, and QQ.
- Provides conflict-safe agent ownership, merge order, capability evidence/TCK, fixture layout,
  cross-review, rollback, and real QQ smoke instructions.
- Splits the C5 backlog into planning, implementation, and validation without marking C5 complete.

No Rust/TypeScript/config behavior changed. The six adapters, product assembly, real QQ validation,
legacy deletion, `dev` merge, and push remain outside this iteration.
