# Plan Report Runtime Replacement Summary

## Scope

- Replaced legacy GUI plan history rendering with Markdown plan reports.
- Removed the old manager `/api/plans*` gateway surface and legacy manager planning handlers.
- Kept GUI compatibility command names where needed, but backed them with plan report APIs.
- Bound execution TODO tools to approved execution sessions instead of draft PLAN records.
- Updated plan-mode prompting so exploration is read-only and produces a standard Markdown plan report.
- Fixed CI drift found while validating the change set.

## User-visible behavior

- Plan history displays a plan as a single Markdown document.
- Pending plan approval uses the blue plan card with pencil affordance and explicit approve/edit actions.
- Execution TODOs are separate runtime artifacts and are not mixed into plan-history rendering.

## Known follow-up

- Some legacy core planning modules remain compiled for compatibility, but the old gateway routes and old agent plan tools are no longer registered.
- Full CI reached Windows resource/linker limits during `cargo test --all`; focused failures were fixed and rerun.
