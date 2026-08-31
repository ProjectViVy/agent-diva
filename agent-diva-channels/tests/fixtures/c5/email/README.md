# Email C5 fixtures

These RFC 822 fixtures are redacted, deterministic protocol shapes used by the
native Email adapter tests. They cover plain text, Octos-compatible thread
headers, and a typed MIME attachment. No real address, password, message body,
or production traffic is included.

The adapter must keep the following ordering in production:

1. Parse sender and apply consent/allowlist/self-reply policy.
2. Admit the resulting envelope to Fabric.
3. Mark the IMAP UID `\\Seen` only after admission succeeds.

`tests/fixtures/c5/capability-evidence.json` is assembled by the C5 lead after
all channel adapters land; this directory is the Email-owned evidence source.
