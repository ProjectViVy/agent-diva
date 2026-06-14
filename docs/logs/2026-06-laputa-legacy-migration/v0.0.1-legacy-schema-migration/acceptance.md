# Acceptance

Date: 2026-06-14

## Steps

1. Create a workspace containing legacy templates such as `memory/MEMORY.md`, `SOUL.md`, `IDENTITY.md`, `USER.md`, `PROFILE.md`, `TASK.md`, or `BOOTSTRAP.md`.
2. Open `LaputaStorage` and run `LaputaMigration::new(storage).run(Default::default())`.
3. Confirm legacy files still exist at their original paths.
4. Confirm copied backups exist under `.laputa/legacy/{timestamp}/`.
5. Confirm supported material appears in the mapped `.laputa/sections/*.json` files.
6. Confirm unsupported/TBD material has `status=tbd` and explicit metadata.
7. Confirm `.laputa/state.json` has schema version `1.1.0` after successful migration.
