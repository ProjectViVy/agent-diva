use agent_diva_channels::{
    accepted_receipt, build_active_adapters, execution_error, external_message_envelope,
    is_sender_allowed, validate_attachment_reference, AdapterServices, AttachmentStoreError,
    ChannelAttachmentStore, IngressAttachment, StoredAttachment,
};
use agent_diva_core::channel::{
    AttachmentRef, ChannelAddress, ChannelCapability, ChannelCommand, ChannelOrigin, ContentPart,
    Correlation, DeliveryStatus, TypingState,
};
use agent_diva_core::config::Config;
use async_trait::async_trait;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{BTreeSet, HashMap};
use std::sync::{Arc, Mutex};

const TEST_MAX_ATTACHMENT_BYTES: u64 = 16;

#[derive(Default)]
struct MemoryAttachmentStore {
    records: Mutex<HashMap<String, StoredAttachment>>,
}

#[async_trait]
impl ChannelAttachmentStore for MemoryAttachmentStore {
    async fn put(&self, input: IngressAttachment) -> Result<AttachmentRef, AttachmentStoreError> {
        input.validate()?;
        let size_bytes = input.bytes.len() as u64;
        if size_bytes > TEST_MAX_ATTACHMENT_BYTES {
            return Err(AttachmentStoreError::TooLarge {
                size_bytes,
                max_bytes: TEST_MAX_ATTACHMENT_BYTES,
            });
        }

        let sha256 = format!("{:x}", Sha256::digest(&input.bytes));
        let reference = AttachmentRef {
            uri: format!("sha256:{sha256}"),
            media_type: input
                .declared_mime
                .unwrap_or_else(|| "application/octet-stream".to_string()),
            size_bytes,
            sha256,
            file_name: input.file_name,
        };
        let stored = StoredAttachment {
            reference: reference.clone(),
            bytes: input.bytes,
        };
        stored.validate()?;
        self.records
            .lock()
            .expect("test store lock is not poisoned")
            .insert(reference.uri.clone(), stored);
        Ok(reference)
    }

    async fn get(
        &self,
        reference: &AttachmentRef,
    ) -> Result<StoredAttachment, AttachmentStoreError> {
        let stored = self
            .records
            .lock()
            .expect("test store lock is not poisoned")
            .get(&reference.uri)
            .cloned()
            .ok_or_else(|| AttachmentStoreError::NotFound {
                uri: reference.uri.clone(),
            })?;
        if stored.reference != *reference {
            return Err(AttachmentStoreError::InvalidReference {
                diagnosis: "stored metadata differs from requested reference".to_string(),
            });
        }
        stored.validate()?;
        Ok(stored)
    }
}

fn test_services() -> AdapterServices {
    AdapterServices::new(Arc::new(MemoryAttachmentStore::default()))
}

fn ingress(file_name: Option<&str>, bytes: &[u8]) -> IngressAttachment {
    IngressAttachment {
        source_channel: "telegram".to_string(),
        platform_message_id: Some("message-1".to_string()),
        sender_id: Some("sender-1".to_string()),
        file_name: file_name.map(str::to_string),
        declared_mime: Some("image/png".to_string()),
        bytes: bytes.to_vec(),
    }
}

#[tokio::test]
async fn attachment_store_is_content_addressed_and_round_trips() {
    let store = MemoryAttachmentStore::default();
    let reference = store
        .put(ingress(Some("image.png"), b"hello"))
        .await
        .unwrap();
    let expected_digest = format!("{:x}", Sha256::digest(b"hello"));

    assert_eq!(reference.sha256, expected_digest);
    assert_eq!(reference.uri, format!("sha256:{expected_digest}"));
    assert_eq!(reference.size_bytes, 5);
    assert_eq!(store.get(&reference).await.unwrap().bytes, b"hello");
    assert!(validate_attachment_reference(&reference, b"hello").is_ok());

    let mut tampered = reference.clone();
    tampered.sha256 = "0".repeat(64);
    assert!(matches!(
        validate_attachment_reference(&tampered, b"hello"),
        Err(AttachmentStoreError::DigestMismatch { .. })
    ));
}

#[tokio::test]
async fn attachment_store_rejects_paths_and_unbounded_input() {
    let store = MemoryAttachmentStore::default();

    assert!(matches!(
        store.put(ingress(Some("nested/image.png"), b"x")).await,
        Err(AttachmentStoreError::InvalidInput { field: "file_name" })
    ));
    assert!(matches!(
        store.put(ingress(Some("too-large.bin"), &[0_u8; 17])).await,
        Err(AttachmentStoreError::TooLarge {
            size_bytes: 17,
            max_bytes: TEST_MAX_ATTACHMENT_BYTES
        })
    ));

    let mut invalid_mime = ingress(Some("image.png"), b"x");
    invalid_mime.declared_mime = Some("image".to_string());
    assert!(matches!(
        store.put(invalid_mime).await,
        Err(AttachmentStoreError::InvalidInput {
            field: "declared_mime"
        })
    ));
}

#[test]
fn attachment_reference_rejects_mime_size_and_filename_tampering() {
    let digest = format!("{:x}", Sha256::digest(b"hello"));
    let reference = AttachmentRef {
        uri: format!("sha256:{digest}"),
        media_type: "text/plain; charset=utf-8".to_string(),
        size_bytes: 5,
        sha256: digest,
        file_name: Some("hello.txt".to_string()),
    };
    assert!(validate_attachment_reference(&reference, b"hello").is_ok());

    let mut invalid_mime = reference.clone();
    invalid_mime.media_type = "text".to_string();
    assert!(matches!(
        validate_attachment_reference(&invalid_mime, b"hello"),
        Err(AttachmentStoreError::InvalidReference { .. })
    ));

    let mut invalid_size = reference.clone();
    invalid_size.size_bytes = 4;
    assert!(matches!(
        validate_attachment_reference(&invalid_size, b"hello"),
        Err(AttachmentStoreError::InvalidReference { .. })
    ));

    let mut invalid_name = reference;
    invalid_name.file_name = Some("..\\hello.txt".to_string());
    assert!(matches!(
        validate_attachment_reference(&invalid_name, b"hello"),
        Err(AttachmentStoreError::InvalidReference { .. })
    ));
}

#[test]
fn allowlist_contract_is_empty_allow_all_with_wildcard_support() {
    assert!(is_sender_allowed(&[], "any-sender"));
    assert!(is_sender_allowed(&["sender-*".to_string()], "sender-42"));
    assert!(is_sender_allowed(
        &["user-42".to_string()],
        "user-42|display-name"
    ));
    assert!(!is_sender_allowed(&["sender-1".to_string()], "sender-2"));
}

#[test]
fn shared_builders_preserve_external_identity_and_truthful_receipts() {
    let envelope = external_message_envelope(
        ChannelAddress::new("telegram", "chat-1"),
        Correlation::new("telegram:chat-1"),
        vec![ContentPart::Text {
            text: "hello".to_string(),
        }],
        None,
        None,
    );
    assert_eq!(envelope.origin, ChannelOrigin::ExternalUser);
    assert!(envelope.validate().is_ok());

    let receipt = accepted_receipt("telegram", "chat-1", Some("message-9".to_string()), None);
    assert_eq!(receipt.status, DeliveryStatus::Accepted);
    assert_eq!(receipt.platform_message_id.as_deref(), Some("message-9"));

    let error = execution_error(
        "http_429",
        "slow down",
        Some(std::time::Duration::from_secs(1)),
        true,
    );
    assert_eq!(error.code(), "http_429");
    assert!(error.is_retryable());
    assert_eq!(error.retry_after(), Some(std::time::Duration::from_secs(1)));
}

#[test]
fn factory_returns_one_native_adapter_per_enabled_channel() {
    let services = test_services();
    let disabled = Config::default();
    assert!(build_active_adapters(&disabled, services.clone())
        .expect("all disabled channels produce an empty native set")
        .is_empty());

    let mut enabled = Config::default();
    enabled.channels.telegram.enabled = true;
    enabled.channels.qq.enabled = true;
    let adapters = build_active_adapters(&enabled, services)
        .expect("enabled channels must be assembled by their native constructors");
    let names = adapters
        .iter()
        .map(|adapter| adapter.name().to_string())
        .collect::<Vec<_>>();
    assert_eq!(names, vec!["telegram", "qq"]);
}

fn capability_set(values: &[ChannelCapability]) -> BTreeSet<ChannelCapability> {
    values.iter().copied().collect()
}

fn frozen_capabilities(channel: &str) -> BTreeSet<ChannelCapability> {
    use ChannelCapability::*;
    match channel {
        "telegram" => capability_set(&[
            IngressText,
            IngressThread,
            IngressGroup,
            IngressDirect,
            IngressTypedAttachments,
            IngressDedupId,
            EgressText,
            EgressMarkdown,
            EgressChunking,
            EgressReply,
            EgressImage,
            EgressAudio,
            EgressVideo,
            EgressFile,
            InteractionTyping,
            InteractionListening,
            InteractionEdit,
            InteractionDelete,
            InteractionStreamFinalize,
            ReliabilityHealth,
            ReliabilityPacing,
            ReliabilitySupervisedRestart,
        ]),
        "discord" => capability_set(&[
            IngressText,
            IngressMarkdown,
            IngressThread,
            IngressGroup,
            IngressDirect,
            IngressTypedAttachments,
            IngressDedupId,
            EgressText,
            EgressMarkdown,
            EgressChunking,
            EgressReply,
            EgressImage,
            EgressAudio,
            EgressVideo,
            EgressFile,
            EgressCard,
            InteractionTyping,
            InteractionEdit,
            InteractionDelete,
            InteractionReaction,
            InteractionStreamFinalize,
            ReliabilityHealth,
            ReliabilityHeartbeat,
            ReliabilityPacing,
            ReliabilitySupervisedRestart,
        ]),
        "feishu" => capability_set(&[
            IngressText,
            IngressMarkdown,
            IngressThread,
            IngressGroup,
            IngressDirect,
            IngressTypedAttachments,
            IngressDedupId,
            EgressText,
            EgressMarkdown,
            EgressReply,
            EgressImage,
            EgressFile,
            EgressCard,
            InteractionEdit,
            InteractionDelete,
            InteractionStreamFinalize,
            ReliabilityHealth,
            ReliabilityHeartbeat,
            ReliabilityTokenRefresh,
            ReliabilityPacing,
            ReliabilitySupervisedRestart,
        ]),
        "dingtalk" => capability_set(&[
            IngressText,
            IngressMarkdown,
            IngressGroup,
            IngressDirect,
            IngressTypedAttachments,
            IngressDedupId,
            EgressText,
            EgressMarkdown,
            EgressImage,
            EgressAudio,
            EgressVideo,
            EgressFile,
            ReliabilityHealth,
            ReliabilityHeartbeat,
            ReliabilityTokenRefresh,
            ReliabilityPacing,
            ReliabilitySupervisedRestart,
        ]),
        "email" => capability_set(&[
            IngressText,
            IngressThread,
            IngressDirect,
            IngressTypedAttachments,
            IngressDedupId,
            EgressText,
            EgressReply,
            EgressImage,
            EgressAudio,
            EgressVideo,
            EgressFile,
            ReliabilityHealth,
            ReliabilityPacing,
            ReliabilitySupervisedRestart,
        ]),
        "qq" => capability_set(&[
            IngressText,
            IngressGroup,
            IngressDirect,
            IngressDedupId,
            EgressText,
            EgressChunking,
            EgressReply,
            ReliabilityHealth,
            ReliabilityHeartbeat,
            ReliabilityTokenRefresh,
            ReliabilityPacing,
            ReliabilitySupervisedRestart,
        ]),
        other => panic!("unknown frozen channel {other}"),
    }
}

#[test]
fn all_six_native_capability_snapshots_equal_the_frozen_matrix() {
    let mut config = Config::default();
    config.channels.telegram.enabled = true;
    config.channels.discord.enabled = true;
    config.channels.feishu.enabled = true;
    config.channels.dingtalk.enabled = true;
    config.channels.email.enabled = true;
    config.channels.qq.enabled = true;
    let adapters = build_active_adapters(&config, test_services()).unwrap();
    assert_eq!(adapters.len(), 6);
    for adapter in adapters {
        let channel = adapter.name().to_string();
        assert_eq!(
            adapter.capabilities().supported,
            frozen_capabilities(&channel),
            "capability snapshot drifted for {channel}"
        );
    }
}

#[tokio::test]
async fn representative_false_capabilities_fail_before_any_platform_request() {
    let mut config = Config::default();
    config.channels.telegram.enabled = true;
    config.channels.discord.enabled = true;
    config.channels.feishu.enabled = true;
    config.channels.dingtalk.enabled = true;
    config.channels.email.enabled = true;
    config.channels.qq.enabled = true;
    let adapters = build_active_adapters(&config, test_services()).unwrap();
    for adapter in adapters {
        let channel = adapter.name().to_string();
        let command = match channel.as_str() {
            "telegram" => ChannelCommand::React {
                address: ChannelAddress::new("telegram", "chat"),
                correlation: Correlation::new("telegram:chat"),
                target_message_id: "message".to_string(),
                operation: agent_diva_core::channel::ReactionOperation::Add,
                emoji: "thumbsup".to_string(),
                idempotency_key: None,
            },
            "discord" => ChannelCommand::Typing {
                address: ChannelAddress::new("discord", "chat"),
                correlation: Correlation::new("discord:chat"),
                state: TypingState::Listening,
                idempotency_key: None,
            },
            "feishu" => ChannelCommand::React {
                address: ChannelAddress::new("feishu", "chat"),
                correlation: Correlation::new("feishu:chat"),
                target_message_id: "message".to_string(),
                operation: agent_diva_core::channel::ReactionOperation::Add,
                emoji: "THUMBSUP".to_string(),
                idempotency_key: None,
            },
            "dingtalk" => ChannelCommand::Edit {
                address: ChannelAddress::new("dingtalk", "chat"),
                correlation: Correlation::new("dingtalk:chat"),
                target_message_id: "message".to_string(),
                parts: vec![ContentPart::Text {
                    text: "edited".to_string(),
                }],
                idempotency_key: None,
            },
            "email" => ChannelCommand::Typing {
                address: ChannelAddress::new("email", "chat@example.test"),
                correlation: Correlation::new("email:chat@example.test"),
                state: TypingState::Started,
                idempotency_key: None,
            },
            "qq" => ChannelCommand::Send {
                envelope: external_message_envelope(
                    ChannelAddress::new("qq", "chat"),
                    Correlation::new("qq:chat"),
                    vec![ContentPart::Card {
                        schema: "c5.card".to_string(),
                        body: serde_json::json!({"title":"unsupported"}),
                    }],
                    None,
                    None,
                ),
                idempotency_key: None,
            },
            _ => unreachable!(),
        };
        let result = adapter.execute(command).await;
        assert!(
            matches!(
                result,
                Err(agent_diva_channels::AdapterError::UnsupportedCapability { .. })
            ),
            "{channel} did not reject unsupported command before transport: {result:?}"
        );
    }
}

#[test]
fn every_channel_fixture_directory_contains_parseable_contract_data() {
    let fixtures = [
        (
            "telegram",
            include_str!("fixtures/c5/telegram/inbound-text.json"),
        ),
        (
            "discord",
            include_str!("fixtures/c5/discord/gateway-frames.json"),
        ),
        (
            "feishu",
            include_str!("fixtures/c5/feishu/protobuf-frame.json"),
        ),
        (
            "dingtalk",
            include_str!("fixtures/c5/dingtalk/stream-callback.json"),
        ),
        ("email", include_str!("fixtures/c5/email/plain.eml")),
        ("qq", include_str!("fixtures/c5/qq/c2c-message.json")),
    ];
    for (channel, fixture) in fixtures {
        if channel == "email" {
            assert!(fixture.contains("Message-ID:"));
        } else {
            serde_json::from_str::<Value>(fixture)
                .unwrap_or_else(|error| panic!("{channel} fixture is not JSON: {error}"));
        }
    }
}

#[test]
fn capability_evidence_manifest_is_machine_auditable() {
    let manifest: Value =
        serde_json::from_str(include_str!("fixtures/c5/capability-evidence.json"))
            .expect("capability evidence JSON must remain valid");
    assert_eq!(
        manifest["octos_sha"],
        "5ea987813de4fd2afdd1d78f2106ad2868f0d923"
    );
    let rows = manifest["evidence"].as_array().expect("evidence rows");
    assert_eq!(rows.len(), 29);
    let mut ids = BTreeSet::new();
    for row in rows {
        let id = row["id"].as_str().expect("evidence row ID");
        assert!(ids.insert(id), "duplicate evidence row {id}");
        if row["status"] == "verified" {
            assert!(row["fixture"]
                .as_str()
                .is_some_and(|value| !value.is_empty()));
            assert!(row["tests"]
                .as_array()
                .is_some_and(|value| !value.is_empty()));
            assert!(row["request"]
                .as_str()
                .is_some_and(|value| !value.is_empty()));
            assert!(row["response"]
                .as_str()
                .is_some_and(|value| !value.is_empty()));
        }
    }
    assert_eq!(
        manifest["channels"]["qq"]["blocked_decisions"],
        serde_json::json!(["D-013", "D-014"])
    );
}
