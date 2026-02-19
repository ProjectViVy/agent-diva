use futures::{SinkExt, StreamExt};
use agent_diva_channels::{ChannelHandler, WhatsAppHandler};
use agent_diva_core::bus::OutboundMessage;
use agent_diva_core::config::WhatsAppConfig;
use serde_json::{json, Value};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
use tokio::net::TcpListener;
use tokio::sync::{mpsc, oneshot};
use tokio::task::JoinHandle;
use tokio::time::{timeout, Duration};
use tokio_tungstenite::{accept_async, tungstenite::Message as WsMessage};

struct BridgeSession {
    messages: Vec<Value>,
    close_after_send: bool,
}

struct MockBridge {
    url: String,
    connection_count: Arc<AtomicUsize>,
    outbound_rx: mpsc::UnboundedReceiver<Value>,
    shutdown_tx: Option<oneshot::Sender<()>>,
    task: Option<JoinHandle<()>>,
}

impl MockBridge {
    async fn spawn(sessions: Vec<BridgeSession>) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind mock bridge listener");
        let addr = listener.local_addr().expect("get mock bridge address");
        let url = format!("ws://{}", addr);

        let connection_count = Arc::new(AtomicUsize::new(0));
        let conn_count_ref = Arc::clone(&connection_count);

        let (outbound_tx, outbound_rx) = mpsc::unbounded_channel();
        let (shutdown_tx, mut shutdown_rx) = oneshot::channel();

        let task = tokio::spawn(async move {
            for session in sessions {
                let accept_fut = listener.accept();
                let (stream, _) = tokio::select! {
                    _ = &mut shutdown_rx => return,
                    accepted = accept_fut => match accepted {
                        Ok(v) => v,
                        Err(_) => return,
                    }
                };

                conn_count_ref.fetch_add(1, Ordering::SeqCst);

                let ws = match accept_async(stream).await {
                    Ok(ws) => ws,
                    Err(_) => return,
                };
                let (mut write, mut read) = ws.split();

                for msg in session.messages {
                    if write.send(WsMessage::Text(msg.to_string())).await.is_err() {
                        return;
                    }
                }

                if session.close_after_send {
                    let _ = write.send(WsMessage::Close(None)).await;
                    continue;
                }

                loop {
                    tokio::select! {
                        _ = &mut shutdown_rx => {
                            let _ = write.send(WsMessage::Close(None)).await;
                            return;
                        }
                        ws_msg = read.next() => {
                            match ws_msg {
                                Some(Ok(WsMessage::Text(text))) => {
                                    if let Ok(parsed) = serde_json::from_str::<Value>(&text) {
                                        let _ = outbound_tx.send(parsed);
                                    }
                                }
                                Some(Ok(WsMessage::Close(_))) | None => break,
                                Some(Err(_)) => break,
                                _ => {}
                            }
                        }
                    }
                }
            }
        });

        Self {
            url,
            connection_count,
            outbound_rx,
            shutdown_tx: Some(shutdown_tx),
            task: Some(task),
        }
    }

    async fn recv_outbound(&mut self) -> Value {
        timeout(Duration::from_secs(3), self.outbound_rx.recv())
            .await
            .expect("wait outbound message")
            .expect("receive outbound message")
    }

    fn connection_count(&self) -> usize {
        self.connection_count.load(Ordering::SeqCst)
    }

    async fn shutdown(mut self) {
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }
        if let Some(task) = self.task.take() {
            let _ = timeout(Duration::from_secs(2), task).await;
        }
    }
}

fn message_payload(
    id: &str,
    sender: &str,
    content: &str,
    is_group: Value,
    protocol_version: Option<u8>,
    media: Option<Value>,
) -> Value {
    let mut payload = json!({
        "type": "message",
        "id": id,
        "sender": sender,
        "pn": "",
        "content": content,
        "timestamp": 1700000000,
        "isGroup": is_group
    });
    if let Some(version) = protocol_version {
        payload["protocolVersion"] = json!(version);
    }
    if let Some(media_payload) = media {
        payload["media"] = media_payload;
    }
    payload
}

#[tokio::test]
async fn whatsapp_reconnects_and_keeps_bridge_protocol_after_reconnect() {
    let mut bridge = MockBridge::spawn(vec![
        BridgeSession {
            messages: vec![
                json!({"type": "status", "status": "connected"}),
                message_payload(
                    "before-reconnect",
                    "11111@s.whatsapp.net",
                    "first connection",
                    json!(false),
                    None,
                    None,
                ),
            ],
            close_after_send: true,
        },
        BridgeSession {
            messages: vec![
                json!({"type": "status", "status": "connected"}),
                message_payload(
                    "after-reconnect",
                    "11111@s.whatsapp.net",
                    "second connection",
                    json!(false),
                    Some(2),
                    Some(json!({"mediaType": "audio", "mime": "audio/ogg"})),
                ),
            ],
            close_after_send: false,
        },
    ])
    .await;

    let config = WhatsAppConfig {
        enabled: true,
        bridge_url: bridge.url.clone(),
        allow_from: vec![],
    };
    let mut handler = WhatsAppHandler::new(config);
    let (inbound_tx, mut inbound_rx) = mpsc::channel(8);
    handler.set_inbound_sender(inbound_tx);

    handler.start().await.expect("start whatsapp handler");

    let first = timeout(Duration::from_secs(3), inbound_rx.recv())
        .await
        .expect("wait first inbound")
        .expect("first inbound message");
    assert_eq!(
        first.metadata.get("message_id"),
        Some(&json!("before-reconnect"))
    );

    let second = timeout(Duration::from_secs(12), inbound_rx.recv())
        .await
        .expect("wait second inbound after reconnect")
        .expect("second inbound message");
    assert_eq!(
        second.metadata.get("message_id"),
        Some(&json!("after-reconnect"))
    );
    assert_eq!(second.metadata.get("protocol_version"), Some(&json!(2)));

    handler
        .send(OutboundMessage::new(
            "whatsapp",
            "11111@s.whatsapp.net",
            "pong",
        ))
        .await
        .expect("send outbound after reconnect");

    let outbound = bridge.recv_outbound().await;
    assert_eq!(outbound.get("type"), Some(&json!("send")));
    assert_eq!(outbound.get("to"), Some(&json!("11111@s.whatsapp.net")));
    assert_eq!(outbound.get("text"), Some(&json!("pong")));
    assert!(
        bridge.connection_count() >= 2,
        "expected at least two bridge connections"
    );

    handler.stop().await.expect("stop whatsapp handler");
    bridge.shutdown().await;
}

#[tokio::test]
async fn whatsapp_accepts_bridge_v1_and_v2_message_shapes() {
    let bridge = MockBridge::spawn(vec![BridgeSession {
        messages: vec![
            json!({"type": "status", "status": "connected"}),
            json!({
                "type": "message",
                "id": "v1",
                "sender": "22222@s.whatsapp.net",
                "pn": "",
                "content": "legacy payload",
                "timestamp": 1700000010,
                "is_group": true
            }),
            json!({
                "type": "message",
                "id": "v2",
                "sender": "22222@s.whatsapp.net",
                "pn": "",
                "content": "[Voice Message]",
                "timestamp": 1700000020,
                "isGroup": false,
                "protocolVersion": 2,
                "media": {
                    "mediaType": "audio",
                    "mime": "audio/ogg",
                    "localPath": "C:/missing/audio.ogg"
                }
            }),
        ],
        close_after_send: false,
    }])
    .await;

    let config = WhatsAppConfig {
        enabled: true,
        bridge_url: bridge.url.clone(),
        allow_from: vec![],
    };
    let mut handler = WhatsAppHandler::new(config);
    let (inbound_tx, mut inbound_rx) = mpsc::channel(8);
    handler.set_inbound_sender(inbound_tx);

    handler.start().await.expect("start whatsapp handler");

    let first = timeout(Duration::from_secs(3), inbound_rx.recv())
        .await
        .expect("wait v1 inbound")
        .expect("receive v1 inbound");
    assert_eq!(first.metadata.get("message_id"), Some(&json!("v1")));
    assert_eq!(first.metadata.get("is_group"), Some(&json!(true)));
    assert!(first.metadata.get("protocol_version").is_none());

    let second = timeout(Duration::from_secs(3), inbound_rx.recv())
        .await
        .expect("wait v2 inbound")
        .expect("receive v2 inbound");
    assert_eq!(second.metadata.get("message_id"), Some(&json!("v2")));
    assert_eq!(second.metadata.get("protocol_version"), Some(&json!(2)));
    assert_eq!(
        second
            .metadata
            .get("media")
            .and_then(|media| media.get("mediaType")),
        Some(&json!("audio"))
    );
    assert_eq!(
        second.metadata.get("transcription_status"),
        Some(&json!("failed"))
    );

    handler.stop().await.expect("stop whatsapp handler");
    bridge.shutdown().await;
}
