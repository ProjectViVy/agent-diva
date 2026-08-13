# GMH-10 Release

No deployment, configuration migration, storage migration, feature flag, or GUI
release is required. The additive Core API becomes available to later GMH-11
and GMH-12 stories when this commit is integrated.

Rollback is a normal revert of the focused GMH-10 commit. Existing Plan,
Sandbox, Memory, Manager, and GUI behavior does not depend on the new module.
