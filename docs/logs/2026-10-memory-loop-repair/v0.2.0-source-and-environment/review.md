# Fresh review and final fix pass

Read-only reviewer `/root/continuous_repair_review` assessed VIVY 968371cd..f574f0a9 and Laputa 5007f62f..7b020a7 plus direct consumers. Three Important findings, zero Critical and no additional substantive Minor findings:

1. Inherited MCP palace overrides could defeat test isolation. Adversarial fresh-path regression RED -> GREEN; child filters only MEMPALACE configuration variables. Mentle full suite now 536 pass / 0 skip.
2. Canonical fact survival was inferred from ingestion content. Process test RED -> GREEN; actual canonical content is now read by RecordID and must equal the ingestion envelope (including run/session/user role). Both separate processes pass.
3. An old acceptance could conflict with the upgraded source hash before observer cursor recovery. Observer and Garden lookup tests RED -> GREEN; scoped receipt lookup preserves the original acceptance without reapplying. Actual factory/Journal/Garden close/reopen recovery and direct changed-payload rejection are tested. Historical missing source is not silently backfilled.

Declined broader judgments remain bounded: no new authority escalation, supervisor capture or fresh payload instability was found; exhaustive privacy, crash cuts, actual recall, Windows and live-model acceptance still require their dedicated cases. Unchanged weak legacy MCP notification assertions are not treated as new acceptance proof. No second review pass is inferred.

Additional CI fixture defects (managed ancestor VCS, codeface user-home writes and a NotifyInput double-admission race) were reproduced and corrected with focused evidence. Complete final frozen-source `just ci` exited 0; raw log and recorded exit code are in this evidence root.
