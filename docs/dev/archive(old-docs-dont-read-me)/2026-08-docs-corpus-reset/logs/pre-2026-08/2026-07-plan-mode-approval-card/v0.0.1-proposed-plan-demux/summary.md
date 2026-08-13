# Plan approval card demux (v0.0.1)

## Problem

Plan mode often emitted the plan as a normal assistant message. `PlanApprovalCard` never appeared because end-of-turn logic required hard section completeness (`validate_report_markdown`) before `PlanReportReadyForApproval`.

## Approach

Codex-aligned soft demux on the existing report path:

1. Prefer line-oriented `<proposed_plan>…</proposed_plan>` extraction
2. Freeform fallback when text looks like a plan report
3. Normalize bare section labels and missing H1
4. Always `create_report` + emit approval event when a body resolves
5. Section completeness is soft (chat note + card banner); approve uses minimum readiness only

## User-visible behavior

- Tagged or freeform plan-like replies show the blue **计划 · 待审批** card
- Incomplete sections show a warning but still allow 批准并开始执行
- Exploration-only chatter does not create junk reports
- Tagged plan body is stripped from the chat bubble to avoid double presentation
