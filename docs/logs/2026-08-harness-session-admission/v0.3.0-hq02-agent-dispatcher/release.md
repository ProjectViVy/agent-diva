# HQ-02 Release

HQ-02 is committed locally on `dev` and is not pushed or externally released.

The dispatcher and worker ownership boundary are active, but production MessageBus consumption intentionally remains serialized. Enabling concurrent Manager/CLI/GUI requests is deferred to HQ-03 after backward-compatible request/trace correlation and stable admission outcomes are available.

Rollback is the focused revert of `b53c619f` followed by `9302f8e7`; no data or configuration migration is required.
