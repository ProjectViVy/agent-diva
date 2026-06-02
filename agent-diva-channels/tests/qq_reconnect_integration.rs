use agent_diva_channels::{ChannelHandler, QQHandler};
use agent_diva_core::config::schema::{Config, QQConfig};
use futures::{SinkExt, StreamExt};
use serde_json::{json, Value};
use std::collections::VecDeque;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
use tokio::net::TcpListener;
use tokio::sync::{mpsc, Mutex};
use tokio::task::JoinHandle;
use tokio::time::{timeout, Duration, Instant};
use tokio_tungstenite::{accept_async, tungstenite::Message as WsMessage};
static TEST_ENV_LOCK: std::sync::OnceLock<Mutex<()>> = std::sync::OnceLock::new();

fn test_env_lock() -> &'static Mutex<()> {
    TEST_ENV_LOCK.get_or_init(|| Mutex::new(()))
}

#[derive(Clone)]
struct GatewaySession {
    send_ping_first: bool,
    close_after_ready: bool,
    send_reconnect_after_ready: bool,
    send_invalid_session_after_identify: bool,
    suppress_heartbeat_ack: bool,
    events: Vec<Value>,
}

struct GatewayConnection {
    identify: Value,
    pong_seen: bool,
    connected_at: Instant,
}

struct MockQQGateway {
    ws_url: String,
    app_id: String,
    secret: String,
    connection_count: Arc<AtomicUsize>,
    connections: Arc<Mutex<Vec<GatewayConnection>>>,
    gateway_task: Option<JoinHandle<()>>,
}

impl MockQQGateway {
    async fn spawn(sessions: Vec<GatewaySession>) -> Self {
        let ws_listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("bind mock qq ws listener");
        let ws_addr = ws_listener.local_addr().expect("mock qq ws addr");
        let ws_url = format!("ws://{}", ws_addr);

        let sessions = Arc::new(Mutex::new(VecDeque::from(sessions)));
        let connection_count = Arc::new(AtomicUsize::new(0));
        let conn_count_ref = Arc::clone(&connection_count);
        let connections = Arc::new(Mutex::new(Vec::new()));
        let connections_ref = Arc::clone(&connections);

        let app_id = "test_app_id".to_string();
        let secret = "test_secret".to_string();
        let gateway_task = tokio::spawn(async move {
            loop {
                let session = {
                    let mut guard = sessions.lock().await;
                    guard.pop_front()
                };
                let Some(session) = session else {
                    return;
                };

                let accept_fut = ws_listener.accept();
                let (stream, _) = match timeout(Duration::from_secs(15), accept_fut).await {
                    Ok(Ok(value)) => value,
                    _ => return,
                };

                conn_count_ref.fetch_add(1, Ordering::SeqCst);

                let ws = match accept_async(stream).await {
                    Ok(ws) => ws,
                    Err(_) => return,
                };
                let (mut write, mut read) = ws.split();

                if write
                    .send(WsMessage::Text(
                        json!({
                            "op": 10,
                            "d": {
                                "heartbeat_interval": 250
                            }
                        })
                        .to_string(),
                    ))
                    .await
                    .is_err()
                {
                    return;
                }

                let identify_msg = match timeout(Duration::from_secs(3), read.next()).await {
                    Ok(Some(Ok(WsMessage::Text(text)))) => {
                        serde_json::from_str::<Value>(&text).expect("parse identify payload")
                    }
                    _ => return,
                };

                let mut pong_seen = false;
                if session.send_ping_first {
                    if write
                        .send(WsMessage::Ping(vec![1, 2, 3].into()))
                        .await
                        .is_err()
                    {
                        return;
                    }
                    let pong = match timeout(Duration::from_secs(2), read.next()).await {
                        Ok(Some(Ok(message))) => message,
                        _ => return,
                    };
                    match pong {
                        WsMessage::Pong(payload) if payload.as_ref() == [1, 2, 3] => {
                            pong_seen = true;
                        }
                        WsMessage::Text(text) => {
                            let parsed = serde_json::from_str::<Value>(&text)
                                .expect("parse message after ping");
                            if parsed.get("op") == Some(&json!(1)) {
                                let next = timeout(Duration::from_secs(2), read.next())
                                    .await
                                    .expect("wait pong after heartbeat")
                                    .expect("pong frame present")
                                    .expect("pong frame ok");
                                if let WsMessage::Pong(payload) = next {
                                    pong_seen = payload.as_ref() == [1, 2, 3];
                                }
                            }
                        }
                        _ => {}
                    }
                }

                let ready_event = if identify_msg.get("op") == Some(&json!(6)) {
                    json!({
                        "op": 0,
                        "s": 2,
                        "t": "RESUMED",
                        "d": {
                            "session_id": "session-1"
                        }
                    })
                } else {
                    json!({
                        "op": 0,
                        "s": 1,
                        "t": "READY",
                        "d": {
                            "session_id": "session-1",
                            "user": {
                                "username": "mock-bot"
                            }
                        }
                    })
                };

                connections_ref.lock().await.push(GatewayConnection {
                    identify: identify_msg,
                    pong_seen,
                    connected_at: Instant::now(),
                });

                if session.send_invalid_session_after_identify {
                    let _ = write
                        .send(WsMessage::Text(
                            json!({
                                "op": 9,
                                "d": false
                            })
                            .to_string(),
                        ))
                        .await;
                    continue;
                }

                if write
                    .send(WsMessage::Text(ready_event.to_string()))
                    .await
                    .is_err()
                {
                    return;
                }

                if session.send_reconnect_after_ready {
                    let _ = write
                        .send(WsMessage::Text(
                            json!({
                                "op": 7
                            })
                            .to_string(),
                        ))
                        .await;
                    continue;
                }

                for event in &session.events {
                    if write
                        .send(WsMessage::Text(event.to_string()))
                        .await
                        .is_err()
                    {
                        return;
                    }
                }

                if session.close_after_ready {
                    let _ = write.send(WsMessage::Close(None)).await;
                    continue;
                }

                loop {
                    let next = match timeout(Duration::from_secs(3), read.next()).await {
                        Ok(Some(Ok(message))) => message,
                        Ok(Some(Err(_))) | Ok(None) | Err(_) => break,
                    };

                    match next {
                        WsMessage::Text(text) => {
                            let parsed = match serde_json::from_str::<Value>(&text) {
                                Ok(value) => value,
                                Err(_) => continue,
                            };
                            if parsed.get("op") == Some(&json!(1)) {
                                if !session.suppress_heartbeat_ack {
                                    if write
                                        .send(WsMessage::Text(json!({"op": 11}).to_string()))
                                        .await
                                        .is_err()
                                    {
                                        return;
                                    }
                                }
                            }
                        }
                        WsMessage::Pong(payload) => {
                            if payload.as_ref() == [1, 2, 3] {
                                if let Some(last) = connections_ref.lock().await.last_mut() {
                                    last.pong_seen = true;
                                }
                            }
                        }
                        WsMessage::Close(_) => break,
                        _ => {}
                    }
                }
            }
        });

        Self {
            ws_url,
            app_id,
            secret,
            connection_count,
            connections,
            gateway_task: Some(gateway_task),
        }
    }

    fn qq_config(&self) -> QQConfig {
        QQConfig {
            enabled: true,
            app_id: self.app_id.clone(),
            secret: self.secret.clone(),
            allow_from: vec![],
        }
    }

    async fn shutdown(mut self) {
        if let Some(task) = self.gateway_task.take() {
            let _ = timeout(Duration::from_secs(2), task).await;
        }
    }
}

#[tokio::test]
async fn qq_reconnects_and_resumes_after_server_close() {
    let _guard = test_env_lock().lock().await;
    let gateway = MockQQGateway::spawn(vec![
        GatewaySession {
            send_ping_first: false,
            close_after_ready: true,
            send_reconnect_after_ready: false,
            send_invalid_session_after_identify: false,
            suppress_heartbeat_ack: false,
            events: vec![json!({
                "op": 0,
                "s": 2,
                "t": "C2C_MESSAGE_CREATE",
                "d": {
                    "id": "before-close",
                    "content": "first message",
                    "timestamp": "2026-04-22T00:00:00Z",
                    "author": {
                        "user_openid": "user-1"
                    }
                }
            })],
        },
        GatewaySession {
            send_ping_first: false,
            close_after_ready: false,
            send_reconnect_after_ready: false,
            send_invalid_session_after_identify: false,
            suppress_heartbeat_ack: false,
            events: vec![json!({
                "op": 0,
                "s": 3,
                "t": "C2C_MESSAGE_CREATE",
                "d": {
                    "id": "after-close",
                    "content": "second message",
                    "timestamp": "2026-04-22T00:00:01Z",
                    "author": {
                        "user_openid": "user-1"
                    }
                }
            })],
        },
    ])
    .await;

    let base = Config::default();
    let previous_api_base = std::env::var("QQ_API_BASE_OVERRIDE").ok();
    std::env::set_var("QQ_ACCESS_TOKEN_OVERRIDE", "test_access_token");
    std::env::set_var("QQ_GATEWAY_URL_OVERRIDE", gateway.ws_url.clone());

    let mut handler = QQHandler::new(gateway.qq_config(), base);
    let (inbound_tx, mut inbound_rx) = mpsc::channel(8);
    handler.set_inbound_sender(inbound_tx);

    handler.start().await.expect("start qq handler");

    let first = timeout(Duration::from_secs(3), inbound_rx.recv())
        .await
        .expect("wait first qq inbound")
        .expect("first qq inbound message");
    assert_eq!(
        first.metadata.get("message_id"),
        Some(&json!("before-close"))
    );

    let second = timeout(Duration::from_secs(8), inbound_rx.recv())
        .await
        .expect("wait second qq inbound after reconnect")
        .expect("second qq inbound message");
    assert_eq!(
        second.metadata.get("message_id"),
        Some(&json!("after-close"))
    );

    tokio::time::sleep(Duration::from_millis(300)).await;
    let connections = gateway.connections.lock().await;
    assert_eq!(gateway.connection_count.load(Ordering::SeqCst), 2);
    assert_eq!(connections.len(), 2);
    assert_eq!(connections[0].identify.get("op"), Some(&json!(2)));
    assert_eq!(connections[1].identify.get("op"), Some(&json!(6)));
    drop(connections);

    handler.stop().await.expect("stop qq handler");
    gateway.shutdown().await;

    if let Some(value) = previous_api_base {
        std::env::set_var("QQ_API_BASE_OVERRIDE", value);
    } else {
        std::env::remove_var("QQ_API_BASE_OVERRIDE");
    }
    std::env::remove_var("QQ_ACCESS_TOKEN_OVERRIDE");
    std::env::remove_var("QQ_GATEWAY_URL_OVERRIDE");
}

#[tokio::test]
async fn qq_replies_to_websocket_ping_without_dropping_connection() {
    let _guard = test_env_lock().lock().await;
    let gateway = MockQQGateway::spawn(vec![GatewaySession {
        send_ping_first: true,
        close_after_ready: false,
        send_reconnect_after_ready: false,
        send_invalid_session_after_identify: false,
        suppress_heartbeat_ack: false,
        events: vec![json!({
            "op": 0,
            "s": 2,
            "t": "C2C_MESSAGE_CREATE",
            "d": {
                "id": "ping-session",
                "content": "ping survives",
                "timestamp": "2026-04-22T00:00:02Z",
                "author": {
                    "user_openid": "user-2"
                }
            }
        })],
    }])
    .await;

    let base = Config::default();
    let previous_api_base = std::env::var("QQ_API_BASE_OVERRIDE").ok();
    std::env::set_var("QQ_ACCESS_TOKEN_OVERRIDE", "test_access_token");
    std::env::set_var("QQ_GATEWAY_URL_OVERRIDE", gateway.ws_url.clone());

    let mut handler = QQHandler::new(gateway.qq_config(), base);
    let (inbound_tx, mut inbound_rx) = mpsc::channel(8);
    handler.set_inbound_sender(inbound_tx);

    handler.start().await.expect("start qq handler");

    let inbound = timeout(Duration::from_secs(4), inbound_rx.recv())
        .await
        .expect("wait qq inbound after ping")
        .expect("qq inbound message after ping");
    assert_eq!(
        inbound.metadata.get("message_id"),
        Some(&json!("ping-session"))
    );

    tokio::time::sleep(Duration::from_millis(300)).await;
    let connections = gateway.connections.lock().await;
    assert_eq!(connections.len(), 1);
    assert!(
        connections[0].pong_seen,
        "expected client pong after server ping"
    );
    drop(connections);

    handler.stop().await.expect("stop qq handler");
    gateway.shutdown().await;

    if let Some(value) = previous_api_base {
        std::env::set_var("QQ_API_BASE_OVERRIDE", value);
    } else {
        std::env::remove_var("QQ_API_BASE_OVERRIDE");
    }
    std::env::remove_var("QQ_ACCESS_TOKEN_OVERRIDE");
    std::env::remove_var("QQ_GATEWAY_URL_OVERRIDE");
}

#[tokio::test]
async fn qq_falls_back_to_identify_after_invalid_resume_session() {
    let _guard = test_env_lock().lock().await;
    let gateway = MockQQGateway::spawn(vec![
        GatewaySession {
            send_ping_first: false,
            close_after_ready: false,
            send_reconnect_after_ready: true,
            send_invalid_session_after_identify: false,
            suppress_heartbeat_ack: false,
            events: vec![],
        },
        GatewaySession {
            send_ping_first: false,
            close_after_ready: false,
            send_reconnect_after_ready: false,
            send_invalid_session_after_identify: true,
            suppress_heartbeat_ack: false,
            events: vec![],
        },
        GatewaySession {
            send_ping_first: false,
            close_after_ready: false,
            send_reconnect_after_ready: false,
            send_invalid_session_after_identify: false,
            suppress_heartbeat_ack: false,
            events: vec![json!({
                "op": 0,
                "s": 4,
                "t": "C2C_MESSAGE_CREATE",
                "d": {
                    "id": "after-invalid-session",
                    "content": "fresh identify works",
                    "timestamp": "2026-04-22T00:00:03Z",
                    "author": {
                        "user_openid": "user-3"
                    }
                }
            })],
        },
    ])
    .await;

    let base = Config::default();
    let previous_api_base = std::env::var("QQ_API_BASE_OVERRIDE").ok();
    std::env::set_var("QQ_ACCESS_TOKEN_OVERRIDE", "test_access_token");
    std::env::set_var("QQ_GATEWAY_URL_OVERRIDE", gateway.ws_url.clone());
    std::env::set_var("QQ_WS_TEST_RECONNECT_DELAY_MS", "50");
    std::env::set_var("QQ_WS_TEST_INVALID_SESSION_BACKOFF_MS", "50,100,150,200");

    let mut handler = QQHandler::new(gateway.qq_config(), base);
    let (inbound_tx, mut inbound_rx) = mpsc::channel(8);
    handler.set_inbound_sender(inbound_tx);

    handler.start().await.expect("start qq handler");

    let inbound = timeout(Duration::from_secs(12), inbound_rx.recv())
        .await
        .expect("wait qq inbound after invalid session recovery")
        .expect("qq inbound after invalid session recovery");
    assert_eq!(
        inbound.metadata.get("message_id"),
        Some(&json!("after-invalid-session"))
    );

    tokio::time::sleep(Duration::from_millis(300)).await;
    let connections = gateway.connections.lock().await;
    assert_eq!(gateway.connection_count.load(Ordering::SeqCst), 3);
    assert_eq!(connections.len(), 3);
    assert_eq!(connections[0].identify.get("op"), Some(&json!(2)));
    assert_eq!(connections[1].identify.get("op"), Some(&json!(6)));
    assert_eq!(connections[2].identify.get("op"), Some(&json!(2)));
    drop(connections);

    handler.stop().await.expect("stop qq handler");
    gateway.shutdown().await;

    if let Some(value) = previous_api_base {
        std::env::set_var("QQ_API_BASE_OVERRIDE", value);
    } else {
        std::env::remove_var("QQ_API_BASE_OVERRIDE");
    }
    std::env::remove_var("QQ_ACCESS_TOKEN_OVERRIDE");
    std::env::remove_var("QQ_GATEWAY_URL_OVERRIDE");
    std::env::remove_var("QQ_WS_TEST_RECONNECT_DELAY_MS");
    std::env::remove_var("QQ_WS_TEST_INVALID_SESSION_BACKOFF_MS");
}

#[tokio::test]
async fn qq_reconnect_opcode_resumes_session() {
    let _guard = test_env_lock().lock().await;
    let gateway = MockQQGateway::spawn(vec![
        GatewaySession {
            send_ping_first: false,
            close_after_ready: false,
            send_reconnect_after_ready: true,
            send_invalid_session_after_identify: false,
            suppress_heartbeat_ack: false,
            events: vec![],
        },
        GatewaySession {
            send_ping_first: false,
            close_after_ready: false,
            send_reconnect_after_ready: false,
            send_invalid_session_after_identify: false,
            suppress_heartbeat_ack: false,
            events: vec![json!({
                "op": 0,
                "s": 3,
                "t": "C2C_MESSAGE_CREATE",
                "d": {
                    "id": "resume-after-op7",
                    "content": "resumed after op7",
                    "timestamp": "2026-04-23T00:00:00Z",
                    "author": {
                        "user_openid": "user-7"
                    }
                }
            })],
        },
    ])
    .await;

    let base = Config::default();
    let previous_api_base = std::env::var("QQ_API_BASE_OVERRIDE").ok();
    std::env::set_var("QQ_ACCESS_TOKEN_OVERRIDE", "test_access_token");
    std::env::set_var("QQ_GATEWAY_URL_OVERRIDE", gateway.ws_url.clone());
    std::env::set_var("QQ_WS_TEST_RECONNECT_DELAY_MS", "50");

    let mut handler = QQHandler::new(gateway.qq_config(), base);
    let (inbound_tx, mut inbound_rx) = mpsc::channel(8);
    handler.set_inbound_sender(inbound_tx);

    handler.start().await.expect("start qq handler");

    let inbound = timeout(Duration::from_secs(6), inbound_rx.recv())
        .await
        .expect("wait qq inbound after op7 reconnect")
        .expect("qq inbound after op7 reconnect");
    assert_eq!(
        inbound.metadata.get("message_id"),
        Some(&json!("resume-after-op7"))
    );

    tokio::time::sleep(Duration::from_millis(200)).await;
    let connections = gateway.connections.lock().await;
    assert_eq!(gateway.connection_count.load(Ordering::SeqCst), 2);
    assert_eq!(connections.len(), 2);
    assert_eq!(connections[0].identify.get("op"), Some(&json!(2)));
    assert_eq!(connections[1].identify.get("op"), Some(&json!(6)));
    drop(connections);

    handler.stop().await.expect("stop qq handler");
    gateway.shutdown().await;

    if let Some(value) = previous_api_base {
        std::env::set_var("QQ_API_BASE_OVERRIDE", value);
    } else {
        std::env::remove_var("QQ_API_BASE_OVERRIDE");
    }
    std::env::remove_var("QQ_ACCESS_TOKEN_OVERRIDE");
    std::env::remove_var("QQ_GATEWAY_URL_OVERRIDE");
    std::env::remove_var("QQ_WS_TEST_RECONNECT_DELAY_MS");
}

#[tokio::test]
async fn qq_heartbeat_timeout_reconnects_with_resume() {
    let _guard = test_env_lock().lock().await;
    let gateway = MockQQGateway::spawn(vec![
        GatewaySession {
            send_ping_first: false,
            close_after_ready: false,
            send_reconnect_after_ready: false,
            send_invalid_session_after_identify: false,
            suppress_heartbeat_ack: true,
            events: vec![],
        },
        GatewaySession {
            send_ping_first: false,
            close_after_ready: false,
            send_reconnect_after_ready: false,
            send_invalid_session_after_identify: false,
            suppress_heartbeat_ack: false,
            events: vec![json!({
                "op": 0,
                "s": 3,
                "t": "C2C_MESSAGE_CREATE",
                "d": {
                    "id": "after-heartbeat-timeout",
                    "content": "heartbeat recovery",
                    "timestamp": "2026-04-23T00:00:01Z",
                    "author": {
                        "user_openid": "user-8"
                    }
                }
            })],
        },
    ])
    .await;

    let base = Config::default();
    let previous_api_base = std::env::var("QQ_API_BASE_OVERRIDE").ok();
    std::env::set_var("QQ_ACCESS_TOKEN_OVERRIDE", "test_access_token");
    std::env::set_var("QQ_GATEWAY_URL_OVERRIDE", gateway.ws_url.clone());
    std::env::set_var("QQ_WS_TEST_RECONNECT_DELAY_MS", "50");

    let mut handler = QQHandler::new(gateway.qq_config(), base);
    let (inbound_tx, mut inbound_rx) = mpsc::channel(8);
    handler.set_inbound_sender(inbound_tx);

    handler.start().await.expect("start qq handler");

    let inbound = timeout(Duration::from_secs(8), inbound_rx.recv())
        .await
        .expect("wait qq inbound after heartbeat timeout recovery")
        .expect("qq inbound after heartbeat timeout recovery");
    assert_eq!(
        inbound.metadata.get("message_id"),
        Some(&json!("after-heartbeat-timeout"))
    );

    tokio::time::sleep(Duration::from_millis(200)).await;
    let connections = gateway.connections.lock().await;
    assert_eq!(gateway.connection_count.load(Ordering::SeqCst), 2);
    assert_eq!(connections[0].identify.get("op"), Some(&json!(2)));
    assert_eq!(connections[1].identify.get("op"), Some(&json!(6)));
    drop(connections);

    handler.stop().await.expect("stop qq handler");
    gateway.shutdown().await;

    if let Some(value) = previous_api_base {
        std::env::set_var("QQ_API_BASE_OVERRIDE", value);
    } else {
        std::env::remove_var("QQ_API_BASE_OVERRIDE");
    }
    std::env::remove_var("QQ_ACCESS_TOKEN_OVERRIDE");
    std::env::remove_var("QQ_GATEWAY_URL_OVERRIDE");
    std::env::remove_var("QQ_WS_TEST_RECONNECT_DELAY_MS");
}

#[tokio::test]
async fn qq_invalid_session_storm_uses_incremental_backoff() {
    let _guard = test_env_lock().lock().await;
    let gateway = MockQQGateway::spawn(vec![
        GatewaySession {
            send_ping_first: false,
            close_after_ready: false,
            send_reconnect_after_ready: false,
            send_invalid_session_after_identify: true,
            suppress_heartbeat_ack: false,
            events: vec![],
        },
        GatewaySession {
            send_ping_first: false,
            close_after_ready: false,
            send_reconnect_after_ready: false,
            send_invalid_session_after_identify: true,
            suppress_heartbeat_ack: false,
            events: vec![],
        },
        GatewaySession {
            send_ping_first: false,
            close_after_ready: false,
            send_reconnect_after_ready: false,
            send_invalid_session_after_identify: true,
            suppress_heartbeat_ack: false,
            events: vec![],
        },
    ])
    .await;

    let base = Config::default();
    let previous_api_base = std::env::var("QQ_API_BASE_OVERRIDE").ok();
    std::env::set_var("QQ_ACCESS_TOKEN_OVERRIDE", "test_access_token");
    std::env::set_var("QQ_GATEWAY_URL_OVERRIDE", gateway.ws_url.clone());
    std::env::set_var("QQ_WS_TEST_RECONNECT_DELAY_MS", "20");
    std::env::set_var("QQ_WS_TEST_INVALID_SESSION_BACKOFF_MS", "50,110,170");
    std::env::set_var("QQ_WS_TEST_INVALID_SESSION_COOLDOWN_MS", "1000");

    let mut handler = QQHandler::new(gateway.qq_config(), base);
    let (inbound_tx, _inbound_rx) = mpsc::channel(8);
    handler.set_inbound_sender(inbound_tx);
    handler.start().await.expect("start qq handler");

    timeout(Duration::from_secs(2), async {
        loop {
            if gateway.connection_count.load(Ordering::SeqCst) >= 3 {
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .expect("wait for invalid session reconnect attempts");

    tokio::time::sleep(Duration::from_millis(100)).await;
    let connections = gateway.connections.lock().await;
    assert_eq!(connections.len(), 3);
    assert_eq!(connections[0].identify.get("op"), Some(&json!(2)));
    assert_eq!(connections[1].identify.get("op"), Some(&json!(2)));
    assert_eq!(connections[2].identify.get("op"), Some(&json!(2)));

    let first_gap = connections[1]
        .connected_at
        .duration_since(connections[0].connected_at);
    let second_gap = connections[2]
        .connected_at
        .duration_since(connections[1].connected_at);
    assert!(
        first_gap >= Duration::from_millis(45),
        "expected first invalid-session backoff >= 45ms, got {first_gap:?}"
    );
    assert!(
        second_gap >= Duration::from_millis(100),
        "expected second invalid-session backoff >= 100ms, got {second_gap:?}"
    );
    assert!(
        second_gap > first_gap,
        "expected increasing invalid-session backoff, got {first_gap:?} then {second_gap:?}"
    );
    drop(connections);

    handler.stop().await.expect("stop qq handler");
    gateway.shutdown().await;

    if let Some(value) = previous_api_base {
        std::env::set_var("QQ_API_BASE_OVERRIDE", value);
    } else {
        std::env::remove_var("QQ_API_BASE_OVERRIDE");
    }
    std::env::remove_var("QQ_ACCESS_TOKEN_OVERRIDE");
    std::env::remove_var("QQ_GATEWAY_URL_OVERRIDE");
    std::env::remove_var("QQ_WS_TEST_RECONNECT_DELAY_MS");
    std::env::remove_var("QQ_WS_TEST_INVALID_SESSION_BACKOFF_MS");
    std::env::remove_var("QQ_WS_TEST_INVALID_SESSION_COOLDOWN_MS");
}
