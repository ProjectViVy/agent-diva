# Render `<proposed_plan>` chat history as plan message blocks

## Problem

After plan generation/approval, earlier agent messages that still contained raw
`<proposed_plan>…</proposed_plan>` tags were rendered as plain markdown text
(tags visible / hard to read).

## Fix

GUI demuxes agent/system message bodies:

1. Text before the tags → normal markdown preface
2. Inner body → **计划 / Plan** message block (`ProposedPlanBlock` + `PlanDocument`)
3. Text after the tags → normal markdown epilogue

Soft display normalize promotes bare section labels (`目标`, `范围`, …) and a
title line so freeform plan bodies still look structured.

Incomplete tags while streaming stay as ordinary text until the closing tag
arrives.

## Files

- `agent-diva-gui/src/components/planning/proposedPlanMessage.ts`
- `agent-diva-gui/src/components/planning/ProposedPlanBlock.vue`
- `agent-diva-gui/src/components/planning/AgentMessageBody.vue`
- `agent-diva-gui/src/components/ChatView.vue`
