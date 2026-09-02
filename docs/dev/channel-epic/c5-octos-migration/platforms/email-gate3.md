# Email C5-V Gate 3 evidence — EM-02/EM-03/EM-04 audit

状态：`partial`（本文件只做证据审计，未改变 manifest/JSON 状态）。

审计基线：固定 Octos checkout
`C:\Users\Administrator\Desktop\morediva\.workspace\octos`，
`5ea987813de4fd2afdd1d78f2106ad2868f0d923`。DIVA 侧以当前 native adapter
`agent-diva-channels/src/adapters/email.rs` 为准；旧的
`agent-diva-channels/src/email.rs` 不作为 C5 native adapter 证据。

## Disposition summary

| Row | Frozen matrix target | Final disposition | Audit basis |
| --- | --- | --- | --- |
| EM-02 | Email consent / auto-reply / TLS | `partial` | DIVA has pre-transport consent, address, auto-reply, allowlist and SMTP TLS branches, but policy/TLS failure matrices are absent; `imap_use_ssl` is not read by the native IMAP path. |
| EM-03 | Email multipart attachments | `partial` | DIVA preserves stronger multipart and typed attachment behavior than Octos, but only one image fixture is covered; MIME/size rejection and raw SMTP serialization/response are not proven, and invalid MIME is silently coerced to `application/octet-stream`. |
| EM-04 | Email mark-seen / health / cancel | `partial` | Code orders Fabric admission before `STORE \\Seen`, but there is no raw IMAP ordering spy, busy/cancel/reconnect transcript, or real IMAP/SMTP health probe; blocking tasks can be detached and mark-seen failure is not retried. |

The three rows remain `partial`; no row is eligible for `verified` under the C5
rule requiring code, fixture, test source, and a previously recorded passing
result to prove the complete request/response, receipt/error, and lifecycle.

## EM-02 — consent, auto-reply, allowlist, and TLS

### Source and endpoint comparison

| Concern | Octos at the pinned SHA | Current DIVA native adapter |
| --- | --- | --- |
| Consent / auto-reply | `crates/octos-bus/src/email_channel.rs::EmailConfig` (20-31) has no consent or auto-reply field. `EmailChannel::send` (83-97) sends directly through `smtp_send`; `imap_poll` only applies `should_skip_self_reply` after parsing (226-239). | `agent-diva-core/src/config/schema.rs::EmailConfig` (873-918) retains `consent_granted`, `auto_reply_enabled`, `smtp_use_tls`, `smtp_use_ssl`, and `imap_use_ssl`. `EmailAdapter::execute_send` (491-632) rejects missing consent with `consent_required` (522-529), disables reply sends with `auto_reply_disabled` (540-547), validates the recipient (531-532), and validates the selected sender (564). |
| Allowlist / self-reply | `EmailChannel::is_allowed` (99-103) is empty-list allow-all and otherwise exact-match. In the pinned `imap_poll` path, the message loop calls `should_skip_self_reply` (232-239) but does not call `is_allowed` before constructing/sending the inbound message. `should_skip_self_reply` itself is (439-453), comparing canonical sender against `from_address` or `username` only when the subject starts with `Re:`. | `process_email` (251-271) applies normalized `email_sender_allowed` (264-267) and `should_skip_self_reply` (268-271) before attachment storage or Fabric admission. DIVA compares `from_address`, `smtp_username`, and `imap_username` (1274-1290), which is intentionally stronger than the Octos two-address check. |
| IMAP TLS | `imap_poll` builds a rustls connector, resolves the server name, opens TCP, and performs the TLS handshake (106-126), then logs in/selects (130-141). There is no configurable IMAP plaintext branch. | `fetch_messages_blocking` and `mark_seen_blocking` always construct `native_tls::TlsConnector` and call `imap::connect` (1026-1045, 1071-1086). The DIVA `imap_use_ssl` configuration field exists (schema 889-890) but is not read in either function: this is an implementation gap against the retained configurable TLS target. |
| SMTP TLS | `smtp_send` chooses implicit TLS only when `smtp_port == 465`, otherwise STARTTLS (275-314; branches 295-309), and has no consent gate or invalid-address policy beyond builder parse errors. | `smtp_send_blocking` retains explicit `smtp_use_ssl` → `SmtpTransport::relay`, `smtp_use_tls` → `starttls_relay`, and plaintext fallback branches (1094-1173, 1151-1169). The branches are code-present but no fake SMTP transcript asserts which branch was selected or how TLS failure is typed. |

### Fixtures, tests, and exact results already available

- Fixtures: `agent-diva-channels/tests/fixtures/c5/email/plain.eml` and
  `reply.eml`. Both are deterministic RFC 822 messages under `.test`; the
  reply carries `Subject: Re: Project update`, `Message-ID`, `In-Reply-To`, and
  `References`.
- `adapters::email::tests::self_reply_and_allowlist_are_fail_closed`
  (`agent-diva-channels/src/adapters/email.rs:1502-1516`) proves the pure
  allowlist and self-reply predicates only: denied sender is false, case
  normalization works, a self `Re:` sender is skipped, and a non-reply user
  is not skipped. It does not invoke `process_email` with a transport spy.
- `adapters::email::tests::unsupported_commands_return_without_network_or_smtp_side_effect`
  (`:1662-1680`) proves `Typing` returns
  `UnsupportedCapability { capability: InteractionTyping }` before a transport
  call. It does not cover `consent_required`, `auto_reply_disabled`, an empty
  recipient, malformed `from`, or the Markdown/Card unsupported branches.
- The existing iteration record
  `docs/logs/2026-09-channel-epic/v0.2.2-c5-capability-evidence/verification.md`
  records 11 Email adapter tests as passed. This audit worker did not rerun
  those tests.

### Request, response, error, and lifecycle audit

| Dimension | Current evidence and exact behavior | Missing proof / impact |
| --- | --- | --- |
| Request / response | Ingress policy is local and returns `Ok(false)` for disallowed/self-reply mail, so it should not reach AttachmentStore, Fabric, IMAP `STORE`, or SMTP. Egress content is checked by `text_body_from_parts` (864-911); Markdown/Card return typed `UnsupportedCapability`. Address failure returns `Execution { code: "invalid_email_address" }` (841-851). Consent and auto-reply failures return `consent_required` / `auto_reply_disabled`. | No fake transport counter or ordered transcript proves each rejection has zero IMAP/SMTP calls. No TLS handshake success/failure response fixture exists. |
| Timeout | Fabric admission has `FABRIC_ADMISSION_DEADLINE = 1s` (34-36, 362-370). | No IMAP connect/read timeout, SMTP connect/send timeout, or TLS-handshake timeout is configured or evidenced. `spawn_blocking` is cancellation-selectable but not time-bounded. |
| Cancel / stop | Unsupported policy checks happen before attachment reads and before spawning SMTP (519-556). `execute_send` watches `self.shutdown` while awaiting the blocking SMTP task (600-623). | No test records cancellation at each policy/TLS boundary. A shutdown can detach an active blocking SMTP task; this does not prove that the underlying transaction has stopped before `stop` returns. |
| Retry | IMAP poll errors and SMTP task/send failures are mapped as retryable (`imap_poll_failed`, `imap_task_failed`, `smtp_task_failed`, `smtp_send_failed`). | There is no automatic SMTP retry or idempotency-key transcript, and no retry-after policy for these Email errors. |
| Dedup | Ingress uses `ParsedEmail::dedup_key` (60-66) and checks it before policy (256-259); rejected mail is not inserted into `processed`, allowing later reconsideration. | EM-02 has no integration assertion that a denied/self-reply message is not marked processed or sent. |
| Health / reconnect | A successful poll marks health healthy (452); poll errors mark degraded (470-472), and loop cancellation/stop marks down (458-461, 476-485). Each poll creates a fresh IMAP session. | `ProbeHealth` returns a local accepted receipt without probing IMAP or SMTP (763-766), and no reconnect/TLS failure transcript exists. |

**EM-02 disposition:** `partial` with both `evidence_gap` and
`implementation_gap`. The retained DIVA policy code is present, but the
consent/empty-recipient/malformed-address/TLS matrix is not evidenced, and
`imap_use_ssl` is currently configuration-dead in the native IMAP and
mark-seen paths. Octos has no consent/auto-reply contract that can fill this
gap by inference.

## EM-03 — multipart attachments

### Source and endpoint comparison

| Concern | Octos at the pinned SHA | Current DIVA native adapter |
| --- | --- | --- |
| Inbound media | `crates/octos-bus/src/email_channel.rs::ParsedEmail` (38-46) has no attachment field. `imap_poll` emits `media: vec![]` (247-253). The shared `crates/octos-bus/src/media.rs::download_media_with_cap` (24-92) is a generic URL downloader, but `email_channel.rs` does not call it. | `parse_email_bytes` (940-1011) parses `mail_parser` attachments, preserves filename/MIME/bytes (973-998), and `process_email` stores them through `ChannelAttachmentStore::put` (280-306). Image/audio/video/other MIME prefixes become `ContentPart::Image/Audio/Video/File` (308-325). |
| Text body | Octos has the required `extract_text_body` recursive symbol (408-419): first `text/plain`, recursively through `subparts`, otherwise `None`. | There is no one-to-one DIVA `extract_text_body` symbol. `parse_email_bytes` uses `parsed.body_text(0)` and falls back to `parsed.body_html(0)` (962-966), then applies UTF-8-safe `truncate_utf8` (967, 1022-1024). This behavior is code-present but has no nested HTML/plain MIME fixture in the assigned evidence. |
| Outbound MIME | Octos `smtp_send` builds only `ContentType::TEXT_PLAIN` and a plain body (280-291); it has no attachment or multipart wire path. The shared `crates/octos-bus/src/channel.rs::Channel::send_with_id` (78-83) delegates to `send` and returns `None` by default, so Octos supplies no Email platform receipt ID. | `outbound_attachments` reads and validates content-addressed references (634-692). `smtp_send_blocking` builds `MultiPart::mixed()` with a plain body and one MIME attachment per stored part (1128-1149), while preserving `In-Reply-To`, `References`, and a generated `Message-ID` (1100-1125). |
| MIME/size safety | Octos email has no attachment validation. The generic Octos downloader bounds response bytes at `DEFAULT_MAX_MEDIA_BYTES` (media.rs 10-21, 69-92), but that is not an Email RFC 822 parser contract. | Inbound attachment bytes over `MAX_ATTACHMENT_BYTES = 25 MiB` are rejected as `Execution { code: "attachment_too_large" }` before `AttachmentStore::put` (32-36, 280-287). Outbound stored bytes are size-checked and reference-validated (658-681). There is no MIME grammar/allowlist validation: `smtp_send_blocking` silently falls back to `application/octet-stream` for a parse failure (1137-1140). |

### Fixtures, tests, and exact results already available

- `agent-diva-channels/tests/fixtures/c5/email/multipart.eml` is a
  `multipart/mixed` message with one `image/png` attachment named `pixel.png`
  and four decoded bytes (`AAECAw==`). No audio, video, generic file, malformed
  MIME, or oversized fixture exists in the Email-owned directory.
- `adapters::email::tests::multipart_fixture_extracts_typed_attachment`
  (`:1492-1499`) proves one parsed image attachment has MIME `image/png`, name
  `pixel.png`, and bytes `[0, 1, 2, 3]`.
- `adapters::email::tests::fake_smtp_receives_reply_headers_multipart_and_real_receipt_id`
  (`:1594-1658`) puts one stored image into a `ChannelEnvelopeV1`, invokes
  `ChannelCommand::Send`, and asserts `In-Reply-To`, `References`, MIME/name/
  bytes, and receipt ID `smtp-fixture-1`.
- The `FakeEmailTransport::send` seam (`:1363-1401`) records an in-memory
  `SmtpMessage` and manufactures `smtp-fixture-N`; this is a deterministic
  adapter seam receipt, not a captured SMTP server response or a provider-
  assigned message ID. Production `smtp_send_blocking` returns the locally
  generated RFC Message-ID (1170-1173).
- `adapters::email::tests::inbound_admission_happens_before_processed_marker`
  (`:1525-1553`) additionally proves a parsed image/text envelope reaches
  Fabric with typed thread/correlation fields; it is not a raw MIME wire test.

### Request, response, error, and lifecycle audit

| Dimension | Current evidence and exact behavior | Missing proof / impact |
| --- | --- | --- |
| Request / response | Inbound request is an RFC 822 byte buffer parsed by `MessageParser`; attachment metadata and bytes are sent to `AttachmentStore::put`. Outbound fake request is `SmtpMessage { from, to, subject, body, in_reply_to, references, attachments, message_id }`; fake response is `Ok("smtp-fixture-1")`, converted to an `Accepted` receipt. | No raw SMTP commands, multipart boundary, `Content-Disposition`, encoded body, server `250` response, or transport-level receipt is captured. |
| Timeout | Ingress Fabric admission is bounded to 1s; attachment store and SMTP operations are awaited around blocking boundaries. | No MIME parse, attachment store, SMTP connect, or SMTP send timeout is asserted. |
| Cancel / stop | Inbound cancellation is selected around Fabric admission and the fetch task; outbound shutdown is selected around the blocking send. | No cancellation test proves attachment reads or a partially built multipart send cannot leak a late side effect. Blocking work is detached when shutdown wins. |
| Retry | Attachment-store failure maps to retryable `attachment_store_failed`; SMTP failures map to retryable `smtp_task_failed` / `smtp_send_failed`; invalid/too-large attachment errors are non-retryable. | No retry/idempotency transcript proves a failed multipart send cannot duplicate an already accepted message. |
| Dedup | `process_email` checks Message-ID/UID dedup before storing inbound attachments, then inserts the key only after Fabric admission (256-259, 371-376). | Only the image path is tested; no duplicate multipart or failure-then-retry attachment case exists. |
| Health / reconnect | A successful poll marks the adapter healthy; SMTP success also marks healthy (625). | Attachment/MIME and SMTP multipart behavior has no health or reconnect wire evidence. |

**EM-03 disposition:** `partial` with `evidence_gap` and a concrete
`implementation_gap` for malformed MIME handling. DIVA intentionally retains a
stronger feature than the Octos email adapter, but the current proof covers
only one image and a pre-serialization fake object. The fallback from invalid
MIME to `application/octet-stream` prevents claiming the required typed MIME
rejection contract.

## EM-04 — mark-seen, admission ordering, health, cancellation, and reconnect

### Source and ordering comparison

| Concern | Octos at the pinned SHA | Current DIVA native adapter |
| --- | --- | --- |
| IMAP request sequence | `imap_poll` performs TLS connect/login/select, `SEARCH UNSEEN` (106-147), fetches `RFC822` (154-216), then `session.store(&seq_set, "+FLAGS (\\Seen)")` (218-224), logs out, and only afterwards sends parsed messages to the channel (226-271). | `fetch_messages_blocking` performs TLS connect/login/select, `uid_search("UNSEEN")`, and `uid_fetch("(BODY.PEEK[] UID)")` (1026-1068). `poll_once` wraps that blocking call in `spawn_blocking` (410-441). |
| Policy/admission/STORE order | Octos marks all fetched sequence numbers seen before self-reply filtering and channel send; there is no Fabric admission boundary. | `process_email` applies allowlist/self-reply before attachment storage (261-271), admits through Fabric with a 1s deadline (362-371), inserts the processed key only after admission (373-376), then awaits `mark_seen_after_admission` when configured (377-379). The mark task opens a new IMAP session and calls `uid_store(uid, "+FLAGS (\\Seen)")` (383-408, 1071-1091). |
| Dedup | The pinned Octos `email_channel.rs` has no `MessageDedup` or per-email dedup check. The shared `crates/octos-bus/src/dedup.rs` is a separate TTL/LRU helper (19-74), not used by `EmailChannel`. | `ParsedEmail::dedup_key` prefers Message-ID and falls back to `uid:<uid>` (60-66); fetch filters processed keys (1058-1063), and `process_email` checks/inserts the key around admission (256-259, 371-376). The set is adapter-local and has no TTL/forget operation. |

### Fixtures, tests, and exact results already available

- `agent-diva-channels/tests/fixtures/c5/email/reply.eml` supplies UID
  `uid-reply` in the fake poll and has a valid Message-ID plus reply headers.
- `adapters::email::tests::fake_imap_poll_admits_before_store_seen_and_deduplicates_uid`
  (`:1556-1591`) queues the same parsed message twice, enables `mark_seen`,
  asserts one Fabric admission, processed key `reply@example.test`, and fake
  seen list `["uid-reply"]`; the second poll returns zero and adds no second
  seen marker. This is a fake transport outcome, not an IMAP command
  transcript with a sequence-number/order spy.
- `FakeEmailTransport::fetch_messages` and `mark_seen` (`:1369-1394`) expose
  separate vectors for fetched messages and seen UIDs, but do not record a
  single ordered event stream. The test therefore relies on the adapter code's
  await order rather than asserting wire order directly.
- The existing verification log records the Email adapter test batch as 11
  passed; this audit worker did not rerun it.

### Request, response, error, and lifecycle audit

| Dimension | Current evidence and exact behavior | Missing proof / impact |
| --- | --- | --- |
| Request / response | Exact production IMAP operations are `SEARCH UNSEEN`, `UID FETCH (BODY.PEEK[] UID)`, then, only after successful Fabric admission, a separate `UID STORE +FLAGS (\\Seen)`. Fabric busy maps to `Execution { code: "fabric_busy", retryable: true, retry_after }` (817-824); Fabric cancellation maps to `AdapterError::Stopped` (825). Fake success records one seen UID and one admitted envelope. | No raw IMAP server fixture proves command ordering, UID versus sequence semantics, or behavior when `STORE` fails. No busy/cancel test proves the message remains eligible and unseen. |
| Timeout | `FABRIC_ADMISSION_DEADLINE` is 1s and is passed to `admit_ingress` (34-36, 362-370). | IMAP connect/login/select/fetch/store and SMTP operations have no explicit operation timeout. `spawn_blocking` does not impose an I/O deadline. |
| Cancel / stop | `poll_once` selects adapter/context cancellation while awaiting the blocking fetch (420-441); `run_poll_loop` selects both cancellation tokens while sleeping and marks down (456-488). `mark_seen_after_admission` selects adapter shutdown and detaches the join handle if shutdown wins (390-407). `stop` cancels and marks down (791-795). | The blocking IMAP call is not aborted when the select returns; it may finish after cancellation. There is no test proving no late fetch/STORE/SMTP side effect after stop, nor a stop barrier joining all blocking tasks. |
| Retry / reconnect | Poll errors are marked degraded and the loop continues after the configured fixed interval (464-486); each subsequent poll creates a fresh IMAP connection. A failed mark-seen call marks degraded (390-400). | This is implicit reconnect, not a scripted reconnect/backoff contract. There is no fault transcript, bounded backoff assertion, or mark-seen retry. Because `processed` is inserted before mark-seen (376-378), a mark-seen failure leaves the UID in the processed set and the next fetch filters it (1058-1063); the marker is not retried. |
| Dedup / admission | Duplicate keys are checked before policy and before Fabric; a key is committed only after Fabric succeeds. A policy rejection returns without adding the key (264-271), and a Fabric failure returns before insertion (371). | No test covers duplicate messages in one fetch batch, concurrent polls, Fabric busy, or a failed attachment/admission followed by the same UID. The in-memory set is not the Octos TTL/LRU helper and is lost with the adapter instance. |
| Health | `crates/octos-bus/src/channel.rs::Channel::health_check` (241-247) defaults to `ChannelHealth::Unknown`; Octos `email_channel.rs` does not override it. | `health` starts `Unknown` (143-145), becomes `Healthy` after a successful poll (452), `Degraded` on poll failure (470-472), and `Down` on cancellation/stop (458-461, 478-485, 791-794). `ChannelCommand::ProbeHealth` returns `accepted_receipt("email", "", None, None)` without an IMAP NOOP/SELECT or SMTP connect/auth (763-766). The planned real connectivity probe is an implementation gap, not merely an unexecuted test. |
| Receipt / sensitive data | Ingress preserves UID, Message-ID, thread, and reply metadata in the typed envelope (333-360). Mark-seen has no delivery receipt; fake poll returns an admission count. | No raw server response/error payload is captured. Existing logs use generic failure text, but this audit has no new log execution to verify redaction. |

**EM-04 disposition:** `partial` with `evidence_gap` and
`implementation_gap`. The crucial admission-before-`STORE \\Seen` ordering is
present in code and supported by the fake outcome, but it is not a raw IMAP
wire proof. Health is currently status bookkeeping plus a no-op accepted probe,
blocking cancellation detaches work, reconnect is only a fresh fixed-interval
poll, and a failed mark-seen operation is not retried.

## Preserved adjacent evidence and audit boundary

- The existing EM-01 evidence remains unchanged: `parse_email_bytes`,
  `ParsedEmail::dedup_key`, and `email_thread_topic` are covered by
  `parser_uses_message_id_and_uid_fallback_and_utf8_safe_limit`,
  `octos_thread_precedence_and_subject_normalization_are_preserved`, and the
  fake IMAP test. The pinned Octos anchors are `email_thread_topic`
  (`crates/octos-bus/src/email_channel.rs:333-347`),
  `extract_text_body` (`:408-419`), and self-reply (`:439-453`). This worker
  did not reclassify EM-01.
- `capability-evidence.json` and `evidence-manifest.md` remain authoritative;
  EM-02, EM-03, and EM-04 are intentionally still `partial`.
- No adapter, fixture, test, public contract, Cargo file, manifest, JSON,
  TODO, or LOCK file was changed by this audit. No compilation, build, cargo
  test, clippy, or live network access was performed.
- EML fixtures and fake messages use `.test` identities and non-production
  secrets. This audit does not authorize logging raw message bodies, full
  addresses, passwords, attachment bytes, or approval payloads.
