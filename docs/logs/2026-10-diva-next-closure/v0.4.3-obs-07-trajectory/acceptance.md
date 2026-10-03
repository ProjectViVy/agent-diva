# Acceptance — OBS-07

- [x] Single owner: console reads `vivyChat` from `state/chat-instance`;
      no second `onEvent`/`subscribeRun` subscription exists.
- [x] Snapshot authoritative: `trajectory/session` load replaces wholesale;
      replayed snapshot produces identical stable ids (test).
- [x] Live events dedup by per-run seq cursor; seq != cursor+1 marks the
      run incomplete and forces refetch (test).
- [x] Wait state is run state (approval/question/child/workflow chips),
      never a model spinner.
- [x] gap/lost marks every folded run visibly incomplete; reconnect +
      `trajNeedsRefresh` triggers refetch.
- [x] Session switch/reopen rejects stale snapshots (generation + sessionId
      fencing; test lateSessionResponseIgnored).
- [x] `has_older_runs` banner; no full-history paging claim.
- [x] Usage evidence shown verbatim or `—`; never a fabricated zero.
- [x] No raw thinking/provider payload fields rendered.
- [ ] Owner acceptance: PENDING.
