---
name: agent-diva-gui-pm-ui
description: Product and UI design guidance for the Agent Diva desktop GUI (agent-diva-gui). Use when drafting PRDs, design specs, feature requirements, UI improvements, or design system updates for the Tauri+Vue desktop app. Covers product scope, user flows, component structure, themes (default/dark/love), and Tailwind patterns. Trigger when working on agent-diva-gui as PM or UI designer.
---

# Agent Diva GUI — Product & UI Design

Product and UI design guidance for the Agent Diva desktop control app. Use this skill when drafting PRDs, design specs, or UI changes for `agent-diva-gui`.

## Quick Reference

| Role | Primary Reference | Use When |
|------|------------------|----------|
| Product Manager | [product-context.md](references/product-context.md) | PRD, feature scope, user flows, acceptance criteria |
| UI Designer | [ui-design-system.md](references/ui-design-system.md) | Themes, components, tokens, Tailwind patterns |
| Both | [component-map.md](references/component-map.md) | Component hierarchy, file locations, responsibilities |

## Tech Stack

- **Frontend**: Vue 3 (Composition API), Vite 6, Tailwind CSS 3, vue-i18n
- **Desktop**: Tauri 2
- **Icons**: lucide-vue-next
- **Markdown**: markdown-it + highlight.js

## PM Workflow

1. **Scope features** — Read [product-context.md](references/product-context.md) for product scope, personas, and feature list.
2. **Align with WBS** — GUI build docs live in `docs/app-building/`. Key: `README.md`, `wbs-gui-cross-platform-app.md`, `wbs-validation-and-qa.md`.
3. **Define acceptance** — For GUI changes, include: `just ci` pass, GUI smoke (start app + key path), i18n keys if user-facing.
4. **Iteration logs** — Per AGENTS.md, record in `docs/logs/<theme>/v<version>-<slug>/` with `summary.md`, `verification.md`, `acceptance.md`.

## UI Designer Workflow

1. **Design system** — Read [ui-design-system.md](references/ui-design-system.md) for themes, chat-shell, chat-bubble, app-shell, and Tailwind utilities.
2. **Component map** — Use [component-map.md](references/component-map.md) to find where to add or modify UI.
3. **Implement** — Vue SFCs in `agent-diva-gui/src/components/`, styles in `src/styles.css`. Add i18n keys in `src/locales/en.ts` and `zh.ts`.
4. **Theme consistency** — New UI must support `theme-default`, `theme-dark`, `theme-love`. Use existing `.chat-*`, `.app-*` classes.

## Key Paths

| Purpose | Path |
|---------|------|
| App entry | `agent-diva-gui/src/App.vue` |
| Main layout | `agent-diva-gui/src/components/NormalMode.vue` |
| Chat UI | `agent-diva-gui/src/components/ChatView.vue` |
| Settings | `agent-diva-gui/src/components/SettingsView.vue` |
| Styles | `agent-diva-gui/src/styles.css` |
| i18n | `agent-diva-gui/src/locales/en.ts`, `zh.ts` |
| Build docs | `docs/app-building/README.md` |

## Validation

- Run `just ci` before delivery.
- For GUI changes: start app (`pnpm tauri dev` or built binary), verify key flows.
- Per `gui-changes-need-gui-smoke` rule: record GUI smoke in `verification.md`.
