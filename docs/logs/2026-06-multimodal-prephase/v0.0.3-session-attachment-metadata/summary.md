# Summary

## v0.0.3-session-attachment-metadata

Date: 2026-06-01
Type: implementation / session metadata

## Changes

- Added lightweight conversation attachment references for session history.
- Extended chat messages with optional attachment metadata while keeping old session JSONL compatible.
- Updated turn saving so user messages can persist attachment file references without embedding file bytes.
- Added focused tests for attachment round-trip, old-session compatibility, and session JSONL safety.

## Impact

Historical messages can now record which files were attached to a user turn. This iteration does not send images to the model and does not change provider multimodal serialization.
