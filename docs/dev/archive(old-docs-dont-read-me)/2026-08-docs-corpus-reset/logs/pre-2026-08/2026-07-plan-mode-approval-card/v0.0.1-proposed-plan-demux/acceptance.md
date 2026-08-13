# Acceptance

| AC | Result |
|----|--------|
| 1. Tagged `<proposed_plan>` creates report + approval event path | Pass (core extract + loop demux) |
| 2. Freeform almost-plan (bare 目标/范围…) yields card after normalize | Pass (looks_like + normalize tests) |
| 3. Exploration-only turn does not create junk report | Pass (`exploration_chatter_is_not_a_plan`) |
| 4. Tagged body stripped from FinalResponse | Pass (`strip_proposed_plan_block`) |
| 5. Approve min gate allows non-empty title+body without full sections | Pass (`assert_report_ready_for_approval` + store) |
| 6. Plan mode remains read-only for tools | Pass (`tool_assembly` plan mode test) |
| GUI incomplete plan still shows approve CTA | Pass (PlanApprovalCard test) |

Manual desktop smoke remains recommended after restarting the GUI against this build.
