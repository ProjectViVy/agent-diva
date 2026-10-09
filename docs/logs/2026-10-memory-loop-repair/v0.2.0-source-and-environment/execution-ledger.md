# SDD ledger — plan: docs/plans/diva-next/memory-loop/README.md

## Execution authorization
User requested development on 2026-10-09. Implement sequentially; no push or publication.

## Preflight rulings
- Exact pinned VIVY/Laputa worktrees created; do not silently replace with sibling HEADs.
- S01 remains blocked until build/model/runtime prerequisites are satisfied. Independent S03 checker work is allowed by S03; no downstream product gate is bypassed.
- Go toolchain is task-local. Existing /usr/bin/go is GNU Go, not the compiler.
- Cloud skill companion scripts are not available via skills.read (resource read failed); maintain this equivalent ledger directly and record task transitions here.
- Worktree setup is authorized by development instruction and platform autonomy guidance. Original checkouts stay intact except the required DIVA lock claim.
- Test-only fixtures must use full composition. Missing stage observation is an error, not synthetic success.

## First checkpoint tasks (historical; superseded below)
- S01: BLOCKED. Exact pinned worktrees/toolchain prepared. Asset entry regression fixed and committed. Real sealed pack reaches missing native GTK/WebKit dependencies; no product binary.
- S02: BLOCKED. Real backend fixture delivered with pinned ONNX; agentapi 64 and Garden 479 pass. Standalone/e2e regressions non-green and S01 absent.
- S03: BLOCKED. Evidence checker 24 + asset 1 pass. SDK input guard 2 pass and full DIVA composition ack smoke 1 pass. Reflection/recall/reflected/process Restart pending.
- S04: NOT_STARTED formal acceptance; actual diagnostic MEM-S04-01 reproduced (user fact lost / role absent). Product fix not included.
- S05–S13: NOT_STARTED, predecessor gates absent.

## First checkpoint review and rulings (historical)
- Final fresh read-only reviewer required by executing-plans. Four findings fixed with regressions; no substantive remaining preparation findings.
- Runtime instruction regression sees managed /tmp/.git; preserve host mounts. No full-CI claim.
- Shared SDK integration artifact is not a Wails/native candidate. Product baseline fields remain null, validator intentionally exits 1.
- Same-process close/open does not count as Restart; unsupported stages return explicit errors.
- No push, merge or release. Preserve isolated branches for handoff.

## Commits and handoff
See docs/logs/2026-10-memory-loop-verification/handoff.md for focused commits, evidence and next actions. Durable copy of this ledger is stored as execution-ledger.md in that evidence root.

## Continuous repair checkpoint (2026-10-09)
- Ruling: developer admission is separate from Story acceptance — actual ack/App/canonical can validate source repairs while reflection/process/live acceptance waits — risk if wrong: acceptance dependency gaps; retain original gates.
- Ruling: task-local native SDK libraries and tools are preferable to asking for a new machine — no system/HOME changes — risk if wrong: environment is not portable; preserve versions and rerun from exact candidates.
- S04 repair: admitted durable user rows, role/source schema, no 8KiB assistant-summary substitution. Capture tests RED then GREEN; real SDK-packed composition now preserves random user fact. Formal V05–V09 acceptance pending.
- Native supported mode-test: exit 0, sealed consumer go race tests passed. This is not a released binary or Windows UI acceptance.
- Mentle: indirect renameio graph closure fixed, 530 pass / 5 MCP skips. Skips are being repaired rather than counted as successful verification.
- Garden process e2e: explicit pinned model directory; 2 pass.
- Runtime/App affected regression: 1048 pass / 3 skip; just ci running after official registry environment override.

## Fresh whole-branch review and fix pass
- Fresh reviewer `/root/continuous_repair_review`: 3 Important findings, 0 Critical, no additional substantive Minor.
- Final: fixed MCP inherited palace access — TestMCPIgnoresInheritedPalaceOverrides RED→GREEN; Mentle suite 536 pass / 0 skip.
- Final: fixed canonical observation gap — process test RED→GREEN; direct canonical content equals accepted source with actual run/session/user envelope.
- Final: fixed accepted legacy receipt replay conflict — observer unit + Garden authorized lookup RED→GREEN; actual factory/Journal/ingest close/reopen rejoin preserves original receipt; direct changed-payload conflict retained. Full affected/CI rerun continues.
- Ruling: recover the original accepted receipt without automatic legacy source rewriting — durable effect identity and source history must not change under the same event — risk if wrong: old missing user facts remain absent until a separate governed repair; tracked in VIVY TODO and S04/S09.
- Ruling: review's declined broader claims remain unverified — no new authority escalation or supervisor capture found, but privacy exhaustiveness, crash cuts, recall, Windows and live-model acceptance require their own cases — risk if wrong: partial evidence could be mistaken for full closure; no Story is marked Done.
- Additional CI issues repaired: linked-parent Codeface test wrote user home; use explicit isolated data root. NotifyInput test raced automatic wake with a second manual attempt; wait for actual automatic workflow and its completed window instead. Production admission semantics unchanged.

- Required reviewed-source just ci: UI 70 files / 527 tests, vet and business packages passed; executable conformance rejected stale internal-source binding. Refresh exactly five canonical sourceSha256 fields using supported source-hash helper after freezing the clean internal tree; independent filesystem hash agrees. Fresh-cache executable Provider/Host reproduction is running. Never promote a source-bound evidence bundle without matching executable results.

## Source-bound final candidate checkpoint
- Actual executable conformance reproduction passed, exit 0, 159.90 seconds, using a fresh task-local cache. Exactly five internal source identities were advanced; provider/check/pass fields were preserved. Independent digest agrees with the supported helper.
- VIVY source and evidence committed at 2c184453; Laputa at 92d0b634. Shared pack/Inspect identity guards and actual App/random-fact/canonical/two-process regressions passed again on the rebuilt shared generation. Native build/Inspect and --help passed on a separate rebuilt development generation. Do not combine the two generation identities for Story acceptance.
- Original tracked source lock and historical incomplete baseline remain unchanged. Formal S01–S03 gates and S04–S13 acceptance statuses are preserved.
- Next executable work: S04 remaining V05–V09 negative cases and governed old-source repair, S03 reflection/recall/reflected and reusable Restart protocol, then S05–S11 as their actual inputs become available. External-only dependencies are native Windows and live provider/model configuration. This checkpoint does not close the full memory-loop plan.

## Final blocker/repair handoff
- Required frozen-source `just ci`: exit 0, recorded in vivy-full-ci-final-exit.json. All recipe stages, including SDK executable conformance, headless compile and independent plugin/face modules, ran successfully. Rebuilt supported native host race test: exit 0.
- Plan package static checker: 5 Epics / 13 Stories / 28 cases / acyclic dependencies / 24 Markdown documents / 43 source paths, pass.
- Relevant Rulings: repair vs acceptance admission; task-local native/tool provisioning; original accepted receipt recovery without silent legacy source backfill; limited review claims remain outside full privacy/crash/recall/platform/live acceptance. Risks and owner next actions remain in the live package.
- No full Story Done, release, push or merge. Preserve isolated branches and the original historical baseline. Next developer work is the S03/S04 route in acceptance.md and the live README, then S05–S11.
