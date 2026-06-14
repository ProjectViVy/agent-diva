# Release

## v0.0.3-session-attachment-metadata

Date: 2026-06-01

## Deployment

No standalone deployment step is required. The change ships with the next normal Rust workspace build.

## Rollback

Revert the session attachment metadata code and this iteration log directory. Existing session files that already contain `attachments` remain readable by this version; older binaries may ignore or fail on the new field depending on their serde settings.

## Operator Notes

This iteration changes persisted session shape by adding an optional field only. It does not require data migration.
