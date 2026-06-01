# M4+M5 Capability And OpenAI Vision Release

## Method

Release by merging this backend/provider package after the targeted validation and workspace lint gates pass.

## Operational Notes

- Unknown models are treated as non-vision by default.
- First supported model allowlist is intentionally conservative.
- Image bytes are only read at provider-call preparation time and are not written to session JSONL.
- Native provider model IDs remain protected by existing raw-model routing tests.

## Rollback

Revert the M4/M5 code changes and this iteration log directory. M1/M2/M3 remain independent and can continue to store/assemble image metadata without sending images to providers.
