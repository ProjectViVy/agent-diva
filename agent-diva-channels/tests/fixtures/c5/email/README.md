# Email C5 fixtures

These RFC 822 fixtures are redacted, deterministic protocol shapes used by the
native Email adapter tests. They cover plain and HTML bodies, Octos-compatible
thread headers, missing Message-ID/UID fallback, nested RFC822, rich multipart
media, and malformed MIME. No real address, password, message body, or
production traffic is included.

The adapter must keep the following ordering in production:

1. Parse sender and apply consent/allowlist/self-reply policy.
2. Validate every MIME part before attachment storage.
3. Admit the resulting envelope to Fabric.
4. Mark the IMAP UID `\\Seen` only after admission succeeds; retry that STORE
   without re-admitting or re-sending the message.

`imap-transcript.txt` and `smtp-transcript.txt` define the command/response
shape asserted by the in-memory fake transport. The fake also records whether
IMAP implicit SSL and SMTP SSL/STARTTLS/plain mode were selected.

`tests/fixtures/c5/capability-evidence.json` is assembled by the C5 lead after
all channel adapters land; this directory is the Email-owned evidence source.
