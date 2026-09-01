//! Ignored end-to-end QQ Bot C5-Q harness.
//!
//! This test intentionally uses the public native factory, Fabric, pacing
//! lane, registry and supervisor together. It never supplies a test endpoint;
//! all credentials and target open IDs must come from the caller's process
//! environment.

use agent_diva_channels::{
    build_active_adapters, AdapterContext, AdapterServices, AdapterSupervisor,
    ChannelAttachmentStore, IngressAttachment, StoredAttachment,
};
use agent_diva_channels::{AdapterPacingLane, AdapterRegistry};
use agent_diva_core::channel::{
    AttachmentRef, ChannelAddress, ChannelCommand, ChannelDirection, ChannelEnvelopeV1,
    ChannelOrigin, ChannelPayloadV1, ContentPart, Correlation, DeliveryStatus, FabricIngressItem,
    FabricKernel,
};
use agent_diva_core::config::Config;
use async_trait::async_trait;
use sha2::{Digest, Sha256};
use std::sync::Arc;
use std::time::Duration;
use tokio_util::sync::CancellationToken;

const LIVE_EVENT_TIMEOUT: Duration = Duration::from_secs(120);
const LIVE_DEDUP_WINDOW: Duration = Duration::from_secs(3);

struct UnusedAttachmentStore;

#[async_trait]
impl ChannelAttachmentStore for UnusedAttachmentStore {
    async fn put(
        &self,
        _input: IngressAttachment,
    ) -> Result<AttachmentRef, agent_diva_channels::AttachmentStoreError> {
        Err(agent_diva_channels::AttachmentStoreError::Backend {
            diagnosis: "QQ C5-Q live harness does not accept media".to_string(),
        })
    }

    async fn get(
        &self,
        reference: &AttachmentRef,
    ) -> Result<StoredAttachment, agent_diva_channels::AttachmentStoreError> {
        Err(agent_diva_channels::AttachmentStoreError::NotFound {
            uri: reference.uri.clone(),
        })
    }
}

/// Run manually with `cargo test -p agent-diva-channels --test qq_live_harness
/// -- --ignored --nocapture` after exporting all five `AGENT_DIVA_LIVE_QQ_*`
/// variables. Missing credentials are reported as a blocked gate and do not
/// attempt network access.
#[tokio::test]
#[ignore]
async fn qq_live_vertical_smoke() -> Result<(), Box<dyn std::error::Error>> {
    let required = [
        "AGENT_DIVA_LIVE_QQ_APP_ID",
        "AGENT_DIVA_LIVE_QQ_SECRET",
        "AGENT_DIVA_LIVE_QQ_C2C_OPENID",
        "AGENT_DIVA_LIVE_QQ_GROUP_OPENID",
    ];
    let mut values = Vec::with_capacity(required.len());
    for name in required {
        match std::env::var(name)
            .ok()
            .filter(|value| !value.trim().is_empty())
        {
            Some(value) => values.push(value),
            None => {
                eprintln!("C5-Q blocked: missing {name}");
                return Ok(());
            }
        }
    }
    let [app_id, secret, c2c_open_id, group_open_id] = values
        .try_into()
        .map_err(|_| "QQ live harness environment collection did not contain four values")?;

    let mut config = Config::default();
    config.channels.qq.enabled = true;
    config.channels.qq.app_id = app_id;
    config.channels.qq.secret = secret;

    let services = AdapterServices::new(Arc::new(UnusedAttachmentStore));
    let mut adapters = build_active_adapters(&config, services)?;
    let qq = adapters
        .drain(..)
        .find(|adapter| adapter.name().as_str() == "qq")
        .ok_or("QQ native adapter was not assembled")?;

    let registry = AdapterRegistry::new();
    registry.register(qq.clone()).await?;
    assert!(
        registry
            .supports(
                &qq.name(),
                agent_diva_core::channel::ChannelCapability::EgressText
            )
            .await?
    );

    let (fabric, mut consumer) = FabricKernel::new().into_parts();
    let root_cancel = CancellationToken::new();
    let supervisor = AdapterSupervisor::spawn(
        qq.clone(),
        AdapterContext {
            fabric,
            cancel: root_cancel.clone(),
        },
    );
    let pacing = AdapterPacingLane::spawn(qq);

    let (c2c, group) = collect_live_inbound(&mut consumer, &c2c_open_id, &group_open_id).await?;
    eprintln!(
        "C5-Q inbound accepted: c2c={} group={}",
        short_hash(c2c.correlation.message_id.as_deref().unwrap_or_default()),
        short_hash(group.correlation.message_id.as_deref().unwrap_or_default())
    );

    let mut seen = std::collections::BTreeSet::new();
    seen.insert(c2c.correlation.message_id.clone());
    seen.insert(group.correlation.message_id.clone());
    let duplicate_check = tokio::time::timeout(LIVE_DEDUP_WINDOW, async {
        while let Some(item) = consumer.recv_ingress().await {
            if let FabricIngressItem::Envelope(envelope) = item {
                if seen.contains(&envelope.correlation.message_id) {
                    return Err("QQ replay admitted a duplicate message ID");
                }
            }
        }
        Ok(())
    })
    .await;
    if let Ok(Err(error)) = duplicate_check {
        root_cancel.cancel();
        supervisor.shutdown().await;
        pacing.shutdown().await;
        return Err(error.into());
    }

    let c2c_receipt =
        send_live_final(&pacing, &c2c, "C5-Q C2C final smoke", "qq-live-c2c-final").await?;
    assert_eq!(c2c_receipt.status, DeliveryStatus::Accepted);
    assert!(c2c_receipt.platform_message_id.is_some());
    let group_receipt = send_live_final(
        &pacing,
        &group,
        "C5-Q group final smoke",
        "qq-live-group-final",
    )
    .await?;
    assert_eq!(group_receipt.status, DeliveryStatus::Accepted);
    assert!(group_receipt.platform_message_id.is_some());
    eprintln!(
        "C5-Q outbound accepted: c2c={} group={}",
        short_hash(
            c2c_receipt
                .platform_message_id
                .as_deref()
                .unwrap_or_default()
        ),
        short_hash(
            group_receipt
                .platform_message_id
                .as_deref()
                .unwrap_or_default()
        )
    );

    root_cancel.cancel();
    supervisor.shutdown().await;
    pacing.shutdown().await;
    Ok(())
}

async fn collect_live_inbound(
    consumer: &mut agent_diva_core::channel::FabricConsumer,
    c2c_open_id: &str,
    group_open_id: &str,
) -> Result<(ChannelEnvelopeV1, ChannelEnvelopeV1), Box<dyn std::error::Error>> {
    let deadline = tokio::time::Instant::now() + LIVE_EVENT_TIMEOUT;
    let mut c2c = None;
    let mut group = None;
    while c2c.is_none() || group.is_none() {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
        if remaining.is_zero() {
            return Err("timed out waiting for both QQ C2C and group @ events".into());
        }
        let item = tokio::time::timeout(remaining, consumer.recv_ingress())
            .await
            .map_err(|_| "timed out waiting for a QQ Fabric event")?
            .ok_or("QQ Fabric closed while waiting for live events")?;
        let FabricIngressItem::Envelope(envelope) = item else {
            continue;
        };
        if envelope.address.chat_id == c2c_open_id
            && envelope
                .extensions
                .get("qq.chat_kind")
                .and_then(|value| value.as_str())
                == Some("direct")
        {
            c2c = Some(envelope);
        } else if envelope.address.chat_id == group_open_id
            && envelope
                .extensions
                .get("qq.chat_kind")
                .and_then(|value| value.as_str())
                == Some("group")
        {
            group = Some(envelope);
        }
    }
    Ok((
        c2c.expect("C2C event was set"),
        group.expect("group event was set"),
    ))
}

async fn send_live_final(
    pacing: &AdapterPacingLane,
    inbound: &ChannelEnvelopeV1,
    text: &str,
    idempotency_key: &str,
) -> Result<agent_diva_core::channel::DeliveryReceipt, Box<dyn std::error::Error>> {
    let mut correlation = Correlation::new(inbound.correlation.session_key.clone());
    correlation.reply_to = inbound.correlation.message_id.clone();
    let mut envelope = ChannelEnvelopeV1::new(
        ChannelDirection::Egress,
        ChannelAddress::new("qq", inbound.address.chat_id.clone()),
        correlation,
        ChannelOrigin::Runtime,
        ChannelPayloadV1::Message {
            parts: vec![ContentPart::Text {
                text: text.to_string(),
            }],
            subject: None,
            locale: None,
            context: None,
        },
    );
    if inbound
        .extensions
        .get("qq.chat_kind")
        .and_then(|value| value.as_str())
        == Some("group")
    {
        envelope
            .extensions
            .insert("qq.chat_kind".to_string(), serde_json::json!("group"));
    }
    Ok(pacing
        .handle()
        .submit(
            ChannelCommand::Send {
                envelope,
                idempotency_key: Some(idempotency_key.to_string()),
            },
            Duration::from_secs(5),
            &CancellationToken::new(),
        )
        .await?)
}

fn short_hash(value: &str) -> String {
    let digest = Sha256::digest(value.as_bytes());
    format!("{:x}", digest)[..12].to_string()
}
