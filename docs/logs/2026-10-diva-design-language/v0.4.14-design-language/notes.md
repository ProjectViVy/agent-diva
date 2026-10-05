# Design language implementation

Binding specification: the user-approved DIVA frontend design-language plan, 2026-10-05.

Preserve all navigation, panel layout, per-theme sidebar/message geometry, scrolling, breakpoints, events, configuration and avatar runtime behavior. Preserve love/dark/default/miku identities and theme persistence. Unify Vue-native controls and semantic CSS; include welcome and pet control surfaces. Remove floating decorations and visual compatibility bridges.

Tasks: 1. Semantic tokens and shared controls. 2. Migrate all interface consumers and remove duplicate/dead styles. 3. Verify all themes, layout, behavior and native desktop; independent review; document and commit.

Pre-flight: tokens produced by task 1 are consumed by all templates and scoped styles in task 2. Rename old accent-as-primary consumers before defining accent-as-subtle-surface. Theme-specific geometry remains in layout authority, including Miku 280/60px sidebar and 75% message width.

Baseline: full Vitest suite, 67 files / 551 tests passed. Existing unmocked localhost:3000 ECONNREFUSED logs do not fail the suite. Node dependencies are reused through an ignored junction; package manifests and lockfiles remain unchanged.

## Full-page visual and state audit

The second audit covered the main chat/composer, provider split view, settings index and provider, MCP, console/diagnostics, channels, masks, network/compaction/speech, Skills, Cron, welcome flow, planning/approval controls, and transparent pet controls. A source-wide review found no remaining raw locale keys or overlapping nested input frames. Follow-up fixes:

- Composer keeps one shared outer frame and uses a blended focus border; toolbar divider remains part of the same shell.
- Search inputs with leading icons reserve text space. MCP URL/search fields use the wrapper as the only border and retain focus feedback.
- MCP zero counts are neutral; degraded turns red only when a degraded server exists.
- Diagnostics no longer shows “no records” after a read failure.
- Speech save failures preserve the user's draft and error. Reloading server state is explicit and confirms before replacing unsaved edits.
- Corrected warning/approval button semantics, hover affordances, pet slider focus, and several missing Chinese/English locale strings. Empty/error states no longer contradict one another.

Visual checks used the local fixture preview for Love, Dark, Default and Miku identities. Love/Dark/Default main chat and composer, Love provider detail, and Miku MCP zero-state were visible at the current ~1090 CSS px viewport. Earlier pass screenshots covered representative settings, welcome, console, Cron, Skills, channels/masks and pet overlay. Layout and theme-specific width rules were reviewed in source diff; this browser control surface has no viewport override, so a literal 1024 px screenshot was unavailable. Preview-only diagnostics and MCP operations are explicitly fixture-limited.

## Verification

- `pnpm test`: **69 files / 564 tests passed**. The expected unmocked localhost:3000 ECONNREFUSED logs remain non-fatal.
- `pnpm build`: **passed** (`vue-tsc --noEmit` + Vite). Vite retains its existing >500 kB chunk advisory.
- The sealed Go-host package was built as a development artifact and launched as a visible `DIVA` window. It is marked `release:false`. The verified Go 1.26.8 archive was kept under ignored `.scratch/`; the global PATH was unchanged. Native drag-region and transparent-pet mouse interactions could not be observed because the available CUA surface exposes browser tabs but no native window controls.
