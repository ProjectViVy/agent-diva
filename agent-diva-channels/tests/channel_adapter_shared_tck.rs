use agent_diva_channels::{
    accepted_receipt, build_active_adapters, execution_error, external_message_envelope,
    is_sender_allowed, validate_attachment_reference, AdapterServices, AttachmentStoreError,
    ChannelAttachmentStore, IngressAttachment, StoredAttachment,
};
use agent_diva_core::channel::{
    AttachmentRef, ChannelAddress, ChannelOrigin, ContentPart, Correlation, DeliveryStatus,
};
use agent_diva_core::config::Config;
use async_trait::async_trait;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
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
fn factory_never_returns_a_silent_noop_for_enabled_channels() {
    let services = test_services();
    let disabled = Config::default();
    assert!(build_active_adapters(&disabled, services.clone())
        .expect("all disabled channels produce an empty native set")
        .is_empty());

    let mut enabled = Config::default();
    enabled.channels.telegram.enabled = true;
    enabled.channels.qq.enabled = true;
    let error = match build_active_adapters(&enabled, services) {
        Ok(_) => panic!("an enabled channel must not produce a silent no-op"),
        Err(error) => error,
    };
    assert_eq!(
        error.to_string(),
        "native adapters are not available yet for enabled channel(s): [\"telegram\", \"qq\"]"
    );
}
