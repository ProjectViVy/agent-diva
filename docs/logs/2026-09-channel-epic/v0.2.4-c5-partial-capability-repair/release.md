# Release

This is an isolated repair iteration on `feat/channel-epic`. It is not a production release and does not merge `dev`, change Manager production assembly, or push any branch.

The implementation/evidence commits are integrated locally, with the final small lint and live
harness documentation fixes at `9579e577` and `6c06a069`. The repair is ready for review as an
offline C5 implementation increment, not as a C5 release: 17 rows remain `partial`, one QQ row
is `blocked/unsupported`, and QQ live/D-013/D-014 plus external platform proof remain required.

Release readiness requires:

1. focused channel tests and shared TCK passing;
2. the workspace gates listed in `verification.md` either passing or explicitly documented as pre-existing blockers;
3. evidence records matching the actual source and fixture output;
4. the repair lock released with a handoff note;
5. QQ live/D-013/D-014 status stated conservatively.

No release artifact or real credential is stored in the repository.

The repair LOCK remains held until this iteration's final documentation and handoff are committed;
it must be released before the worktree is handed off. No production manager or C6 action is
authorized by this release record.
