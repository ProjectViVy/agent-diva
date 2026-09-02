# Email C5-V Gate 3 evidence — EM-02/EM-03/EM-04

状态：partial（Email-owned deterministic evidence is complete in this
worktree; live IMAP/SMTP verification and Lead-owned matrix updates remain
gaps.）

审计基线：

- 固定 Octos checkout：
  C:\Users\Administrator\Desktop\morediva\.workspace\octos
- 固定 SHA：5ea987813de4fd2afdd1d78f2106ad2868f0d923
- DIVA source：agent-diva-channels/src/adapters/email.rs
- 本次只修改 Email adapter、Email-owned C5 fixtures/tests 和本 Gate3 文档；
  未修改公共契约、Cargo、Manager、shared manifest/JSON/TODO/LOCK 或其他
  channel。

## Disposition summary

| Row | Local disposition | Evidence | Remaining gap |
| --- | --- | --- | --- |
| EM-02 | partial — deterministic policy/TLS mode loop closed | execute_send, validate_recipient_address, validate_from_address, process_email, fake IMAP/SMTP events and policy tests | No live TLS handshake/auth failure or provider response; Lead must update shared matrix. |
| EM-03 | partial — MIME/body/typed multipart loop closed | parse_email_bytes, validate_attachment_metadata, strict InvalidMime/size errors, raw EML and SMTP wire fixtures | No real provider multipart delivery/capture; live MIME interoperability remains unverified. |
| EM-04 | partial — admission/STORE retry and tracked lifecycle loop closed | BlockingTaskRegistry, mark_seen_once, retry_pending_seen, health probe, reconnect and stop-drain tests | No live IMAP server fault/reconnect transcript; general SMTP send retry is not implemented by this adapter. |

partial is intentional: a fake transport proves adapter ordering and typed
boundaries, but cannot prove a real provider's TLS handshake, authentication,
server response, or mailbox semantics.

## Octos comparison at the pinned SHA

The pinned Octos crates/octos-bus/src/email_channel.rs provides the comparison
baseline:

- EmailChannel::imap_poll fetches RFC822 bytes and marks fetched messages
  \Seen before channel delivery; it has no Fabric admission boundary.
- smtp_send creates a plain-text message and chooses implicit TLS by port or
  STARTTLS; it has no DIVA consent, typed attachment, or Fabric receipt
  boundary.
- extract_text_body recursively prefers text/plain; Octos has no Email
  attachment field.
- Octos thread topic precedence is References → In-Reply-To → Message-ID →
  normalized subject.

DIVA preserves that thread precedence while adding explicit policy, typed
MIME, Fabric admission, and channel lifecycle behavior.

## EM-02 — policy and security mode evidence

### Source symbols and behavior

- EmailAdapter::execute_send performs content capability validation, then
  consent_granted, recipient validation, auto_reply_enabled, sender
  validation, and only then reads outbound attachments or invokes
  EmailTransport::send.
- validate_recipient_address returns typed missing_recipient for an empty
  target and invalid_email_address for malformed RFC addresses.
- validate_from_address returns typed invalid_from_address.
- process_email applies email_sender_allowed and should_skip_self_reply before
  attachment storage or Fabric admission. An empty allowlist remains allow-all.
- fetch_messages_blocking and mark_seen_blocking branch on the retained
  EmailConfig::imap_use_ssl: connect_imap_tls uses native TLS, while
  connect_imap_plain uses the configured plain IMAP socket.
- build_smtp_transport has explicit smtp_use_ssl → SSL,
  smtp_use_tls → STARTTLS, and plain branches, all with SMTP_IO_TIMEOUT.
- EmailTransportError maps invalid message/MIME/address/size and transport
  failures to typed AdapterError codes; no invalid MIME is coerced to
  application/octet-stream.

### Fixtures and tests

Fixtures under agent-diva-channels/tests/fixtures/c5/email/:

- plain.eml, reply.eml, html.eml, no-message-id.eml, rfc822.eml,
  multipart.eml, multipart-rich.eml, invalid-mime.eml
- imap-transcript.txt with greeting/login/select/search/fetch/store shape
- smtp-transcript.txt with EHLO/MAIL/RCPT/DATA, reply headers, and 250
  acceptance response

The fake transport records ordered imap.fetch, imap.store_seen,
fabric.admission.accepted, smtp.send, and health-probe events. The following
tests provide the local evidence:

- send_policy_rejections_happen_before_fake_smtp covers consent, auto-reply,
  empty recipient, malformed recipient, and malformed sender; every rejection
  has zero fake SMTP events.
- allowlist_rejects_inbound_before_attachment_storage_or_fabric covers
  non-matching allowlist policy with an attachment and no Fabric event.
- fake_transport_records_imap_ssl_and_smtp_security_modes asserts
  imap_use_ssl=false and all SMTP SSL/STARTTLS/plain fake wire modes.
- fixture_transcripts_describe_wire_commands_and_responses asserts the
  channel-owned raw transcript tokens and parses the fake 250 response.
- unsupported_commands_return_without_network_or_smtp_side_effect proves
  unsupported typing returns UnsupportedCapability with no fake event.

### EM-02 lifecycle disposition

Local policy ordering and security-mode selection are complete and tested.
Live certificate, authentication, TLS downgrade, and server rejection
evidence are not available without a real or protocol-faithful external
IMAP/SMTP service; therefore EM-02 remains partial.

## EM-03 — MIME, body, and multipart evidence

### Source symbols and behavior

- parse_email_bytes returns Result<ParsedEmail, EmailTransportError>: parser
  failure or missing sender is InvalidMessage; missing/invalid MIME is
  InvalidMime; oversized parts are TooLarge.
- Body extraction uses mail-parser text first and converts an HTML-only body
  through html_body_to_text, then applies UTF-8-safe truncate_utf8.
- PartType::Message is preserved as message/rfc822; binary, text, HTML, and
  multipart leaf parts retain decoded bytes.
- validate_attachment_metadata checks leaf filename, MIME token shape, and
  MAX_ATTACHMENT_BYTES before the first ChannelAttachmentStore::put.
- process_email_reserved maps image/audio/video/other MIME types to typed
  ContentPart values only after all inbound parts pass validation.
- outbound_attachments validates stored AttachmentRef MIME/name/size before
  smtp_send_blocking; smtp_send_blocking uses ContentType::parse with a typed
  rejection instead of a fallback.
- SmtpMessage carries reply headers, generated Message-ID, body, and
  attachments. The fake fake_smtp_wire renders a multipart/mixed boundary,
  content disposition, base64 bytes, and server 250 response; the receipt ID
  is parsed by parse_smtp_acceptance.

### Fixtures and tests

- parser_handles_html_rfc822_and_uid_fallback_shapes proves HTML-only text
  conversion, no-Message-ID uid:<uid> dedup/platform fallback, and nested
  RFC822 attachment bytes.
- parser_uses_message_id_and_uid_fallback_and_utf8_safe_limit proves
  Message-ID dedup and safe truncation.
- octos_thread_precedence_and_subject_normalization_are_preserved proves
  References → In-Reply-To → Message-ID → subject precedence.
- multipart_fixture_extracts_typed_attachment proves decoded image bytes.
- rich_multipart_is_admitted_as_typed_parts_after_mime_validation proves
  audio/video/file mapping and exactly one Fabric admission.
- malformed_mime_is_typed_and_size_is_rejected_before_transport proves
  invalid inbound MIME and oversize rejection.
- outbound_invalid_mime_is_typed_before_smtp proves invalid stored MIME
  returns invalid_mime and never calls fake SMTP.
- fake_smtp_receives_reply_headers_multipart_and_real_receipt_id proves
  References/In-Reply-To, typed attachment request fields, and receipt ID.

### EM-03 lifecycle disposition

The local parser and multipart boundary are complete with typed errors and
bounded size checks. Real SMTP servers may impose MIME/header interoperability
constraints not covered by the in-memory wire; live delivery remains the
explicit EM-03 gap.

## EM-04 — admission ordering, mark-seen retry, health, cancel, reconnect

### Source symbols and ordering

- BlockingTaskRegistry::run registers every spawn_blocking handle before
  allowing a close race; close_and_drain joins all owned operations.
- poll_once retries pending_seen first, fetches through the tracked blocking
  registry, and marks health degraded on typed fetch/process failure.
- process_email_reserved performs Fabric admission with
  FABRIC_ADMISSION_DEADLINE, then inserts the dedup key, then records pending
  UID and performs UID STORE +FLAGS (\Seen).
- mark_seen_once leaves a failed UID pending; retry_pending_seen retries only
  the STORE and never re-admits the envelope.
- execute_probe_health performs a real production IMAP probe followed by an
  SMTP Transport::test_connection, returning a receipt only after both
  succeed. Probe failures are typed and set Degraded.
- stop cancels the adapter and waits for close_blocking_tasks; loop
  cancellation also drains the registry. Socket and SMTP builders have
  explicit 15-second I/O deadlines, and each IMAP fetch/STORE/probe opens a
  fresh connection.

### Request, response, and lifecycle tests

- inbound_admission_happens_before_processed_marker proves the accepted
  envelope contains external-user origin, thread, and reply metadata.
- fake_imap_poll_admits_before_store_seen_and_deduplicates_uid scripts a
  failed first STORE and successful retry. It asserts one Fabric admission,
  two STORE attempts, degraded → healthy health, and ordered
  fabric.admission.accepted before imap.store_seen.
- health_probe_and_reconnect_failures_are_typed_and_observable scripts probe
  failure then success and IMAP fetch failure then a fresh successful poll,
  asserting degraded → healthy transitions and fresh fake calls.
- blocking_operation_timeout_is_typed_and_drained exercises a short deadline
  and then proves the timed-out spawn_blocking handle is drained by stop.
- failed_fabric_admission_does_not_store_seen proves cancelled admission leaves
  both processed/pending state and fake STORE empty.
- stop_drains_blocking_smtp_and_cancels_waiter_without_late_send_result holds
  a fake SMTP blocking operation, proves stop waits for its registered
  spawn_blocking handle, releases it, observes AdapterError::Stopped, and
  verifies no event is emitted after the stop barrier.

### EM-04 lifecycle disposition

Admission-before-STORE, pending mark-seen retry, health transitions, fresh
reconnect calls, timeout boundaries, and tracked cancellation are locally
closed. A real IMAP server is still needed to validate UID semantics,
\Seen persistence, TLS/auth failures, and network interruption behavior.
This adapter also deliberately does not add an automatic SMTP resend policy;
callers receive retryable typed transport failures and must supply their
idempotency policy.

## Validation

Executed in the Email worktree:

    cargo fmt --all -- --check
    cargo check -p agent-diva-channels
    cargo clippy -p agent-diva-channels --all-targets -- -D warnings
    cargo test -p agent-diva-channels adapters::email::tests --lib
    git diff --check

Results: formatting, check, clippy, and diff check passed; focused tests
reported 23 passed and 0 failed. Git emitted only the repository's normal
LF-to-CRLF working-copy warnings. The workspace future-incompatibility warning
for the already-resolved imap-proto v0.10.2 dependency is non-blocking and
was not changed here.

## Boundary and handoff

- Shared capability-evidence.json, evidence-manifest.md, TODOLIST.md,
  LOCK.md, decision docs, and iteration logs remain untouched for Lead.
- Lead must review this local evidence, update shared C5 status only after
  confirming ownership boundaries, and perform live IMAP/SMTP validation.
- No real credentials, message bodies, attachment bytes, or production
  addresses are included in the fixtures or fake transcript.
