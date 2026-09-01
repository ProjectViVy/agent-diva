# Email C5-V Gate 3 evidence

状态：`partial`。`EmailTransport` 是 adapter-local 的 blocking seam：生产实现仍调用 IMAP/
SMTP libraries，测试用 fake IMAP/SMTP transcript，不连接外部邮箱。

| 目标 | 源码 symbol / endpoint | fixture 与 Rust 测试 | 精确证据 | 生命周期/剩余项 |
| --- | --- | --- | --- | --- |
| IMAP/RFC822/thread/dedup | `parse_email_bytes`, `ParsedEmail::dedup_key`, `email_thread_topic`, `poll_once` | `plain.eml`, `reply.eml`; `parser_uses_message_id_and_uid_fallback_and_utf8_safe_limit`, `octos_thread_precedence_and_subject_normalization_are_preserved`, `fake_imap_poll_admits_before_store_seen_and_deduplicates_uid` | Message-ID 优先；缺失 ID 使用 `uid:<uid>`；References/In-Reply-To/thread root 保留 | 未建立 raw IMAP socket server，fake seam 证明仍为 adapter-level transcript |
| policy/consent/auto-reply | `email_sender_allowed`, `should_skip_self_reply`, `execute_send` | `plain.eml`, `reply.eml`; `self_reply_and_allowlist_are_fail_closed`, unsupported command test | allowlist/self-reply/consent 在 attachment read/SMTP 前拒绝；invalid address typed error | consent/TLS/empty recipient matrix 需单独 fake SMTP assertions |
| typed multipart ingress | `process_email`, `outbound_attachments` | `multipart.eml`; `multipart_fixture_extracts_typed_attachment`, `inbound_admission_happens_before_processed_marker` | MIME/file name/bytes 进入 AttachmentStore；Fabric envelope 使用 Image/File typed part | audio/video/size/MIME reject 分支仍 partial |
| SMTP reply/multipart | `execute_send`, `SmtpMessage`, `SystemEmailTransport::send`; SMTP transport | `multipart.eml`; `fake_smtp_receives_reply_headers_multipart_and_real_receipt_id` | fake SMTP 收到 to/subject/body、In-Reply-To、References、multipart bytes；返回 `smtp-fixture-1` 后 Accepted receipt | 未连接真实 SMTP server；wire serialization 仍需独立 protocol fixture |
| mark-seen/admission | `process_email`, `mark_seen_after_admission`; IMAP `STORE \Seen` | `reply.eml`; `fake_imap_poll_admits_before_store_seen_and_deduplicates_uid` | fake `seen` 只在 Fabric poll 成功后收到 `uid-reply`；第二 poll 不重复 | blocking task cancel/detach、NOOP/reconnect/health probe 仍 partial |
| unsupported/health | `capabilities_snapshot`, `execute`, `stop` | `capabilities_do_not_advertise_false_email_operations`, `unsupported_commands_return_without_network_or_smtp_side_effect` | Markdown/group/chunk/interaction/heartbeat 等 false capability 不调用 transport | ProbeHealth 目前是 local accepted marker，不能替代 IMAP/SMTP connectivity proof |

敏感信息：EML 与 fake message 只使用 `.test` 域名和测试 secret；日志不记录原始邮件或审批载荷。
