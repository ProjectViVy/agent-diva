# v0.0.1 Notebook Proposal Creation Summary

## Changed

- Replaced Notebook direct solidification actions with proposal creation actions for SOP, Skill, and Memory.
- Added Notebook proposal preview data that includes target section, proposal type, extracted summary, evidence refs, risk level, and review status.
- Added Tauri commands to preview and create Notebook report proposals through the existing Laputa proposal API.
- Added report-to-proposal mapping tests covering SOP, Skill, Memory, ambiguous memory, and no authority-file mutation before apply.

## Impact

- Notebook actions now create `EvolutionProposal` records for Laputa review.
- Authority files are not modified by Notebook proposal creation.
- Created proposals can be opened through the existing Evolution Inbox deep-link path.
