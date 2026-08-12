# Persona and Evolution Usability Acceptance

## Persona

1. Open Persona and confirm each section renders content or editable `{}` rather than a blank
   body or `[object Object]`.
2. Confirm a pending proposal is shown in the lifecycle rail and explicitly marked not effective.
3. Simulate a refresh failure and confirm the last successful content stays visible.
4. When governance is unavailable, confirm approval/rejection controls are disabled and a refresh
   recovery message is visible.

## Evolution

1. Open Evolution and confirm proposals render even if one auxiliary event/health request fails.
2. Refresh during a transient proposal error and confirm the last successful list remains visible.
3. Select a proposal and confirm governance actions work after shared-ledger reconciliation.

Automated coverage and a real-workspace API smoke have passed. Visual interaction acceptance in
the current rebuilt window is tracked by the existing M3 manual-smoke TODO.
