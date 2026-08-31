# Email adapter specification

Endpoint-level evidence and current-state gaps are in [`email-scan.md`](email-scan.md).

## Sources and invariants

- Diva source: `agent-diva-channels/src/email.rs`; retain explicit consent, IMAP mailbox/SSL,
  SMTP TLS/SSL, auto-reply policy, poll interval, mark-seen, body limit, subject prefix, allowlist,
  MIME parsing, and blocking-I/O isolation.
- Octos source: `octos-bus/src/email_channel.rs`; adapt Message-ID/subject/reply parsing tests and
  clear IMAP/SMTP separation. Do not replace Diva config or weaken consent.

## Target behavior

- Poll through a bounded, cancellation-aware `spawn_blocking` boundary; never block an async worker.
- Admit plain/HTML-normalized text as Text, preserve subject, Message-ID, In-Reply-To/References as
  thread/reply identity, use Message-ID with UID fallback for dedup, and store MIME attachments as
  typed parts.
- Email is direct-only. Send text and any image/audio/video/file as MIME attachments. Reply uses
  frozen In-Reply-To/References, not mutable sender-only state; the envelope subject overrides the
  default prefix only when non-empty.
- SMTP acceptance returns `Accepted` and a generated/preserved Message-ID. No edit/delete/typing,
  chunking, Card, Markdown capability, heartbeat, resume, or token refresh is advertised.
- Health performs bounded IMAP select/NOOP and SMTP connect/auth checks without sending mail.

## Failure and lifecycle rules

- `consent_granted=false`, auto-reply denial, empty recipient, malformed address, oversized MIME,
  attachment read failure, and SMTP rejection are explicit errors/receipts, never `Ok(())` skips.
- Mark an IMAP item seen only after Fabric admission. Busy admission leaves it eligible for retry.
- Poll cancellation must finish without aborting an in-progress blocking library call unsafely;
  subsequent results are fenced after cancellation.
- Passwords, full email addresses, subjects/bodies, and attachment content are redacted from logs.

## Required fixtures

1. RFC 822 plain, HTML, reply/thread, missing Message-ID, duplicate UID, and multipart attachments.
2. Fake IMAP poll/select/fetch/mark-seen with Fabric success, busy, cancellation, and reconnect.
3. Captured SMTP text/reply/multipart messages, generated Message-ID, acceptance, and rejection.
4. Consent/auto-reply/allowlist combinations and malformed/oversized inputs.
5. IMAP/SMTP health healthy/degraded/down and bounded blocking-task lifecycle.
6. All false capabilities returning unsupported with zero SMTP/IMAP side effects.

Done means Email no longer depends on legacy metadata for subject/files/threading and every skip in
the old implementation has become a truthful outcome.
