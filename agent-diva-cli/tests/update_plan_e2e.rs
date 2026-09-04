use std::time::Duration;

use agent_diva_agent::AgentEvent;
use agent_diva_cli::client::ApiClient;
use agent_diva_core::bus::AgentEventBus;
use agent_diva_core::bus::{SessionAdmissionObservation, SessionAdmissionPhase};
use agent_diva_core::channel::{ChannelOrigin, ChannelPayloadV1, ContentPart};
use agent_diva_core::planning::update_plan::{PlanItem, PlanItemStatus, UpdatePlanArgs};
use agent_diva_manager::run_server_with_listener;
use agent_diva_manager::state::{AppState, ManagerCommand};
use tokio::io::AsyncWriteExt;
use tokio::sync::{mpsc, oneshot, watch};
use tokio::time::timeout;

/// End-to-end test for the normal-chat `update_plan` event flow.
///
/// Verifies the complete chain:
/// 1. The CLI `ApiClient` posts a turn to the manager `/api/runtime/turns` endpoint.
/// 2. The manager forwards the request as a `ManagerCommand::Chat`.
/// 3. A mock agent consumer replies with `AgentEvent::ChatPlanUpdate` followed by
///    `AgentEvent::FinalResponse`.
/// 4. The manager SSE stream emits a `turn_plan_updated` event.
/// 5. The CLI `ApiClient` parses the SSE event back into `AgentEvent::ChatPlanUpdate`.
///
/// This complements the existing T5 handler test in `agent-diva-agent` and the T7
/// manager forwarding tests in `agent-diva-manager` by covering the full wire path
/// from the client perspective.
#[tokio::test]
async fn update_plan_end_to_end_client_sse() {
    let (api_tx, mut api_rx) = mpsc::channel::<ManagerCommand>(1);

    let temp_dir = tempfile::tempdir().expect("create temp dir");
    agent_diva_laputa::PersonaService::open(temp_dir.path())
        .expect("open Persona")
        .initialize(agent_diva_laputa::PersonaInitialization {
            identity: "Diva".into(),
            relationship: "Test partner".into(),
            redline: "No unsafe actions".into(),
            user: "Concise output".into(),
            world: "Local E2E".into(),
        })
        .expect("initialize Persona");
    let state = AppState::new(api_tx, AgentEventBus::new(), temp_dir.path()).expect("build state");

    let expected_args = UpdatePlanArgs {
        explanation: Some("e2e update plan".to_string()),
        plan: vec![
            PlanItem {
                step: "analyze request".to_string(),
                status: PlanItemStatus::Completed,
            },
            PlanItem {
                step: "draft response".to_string(),
                status: PlanItemStatus::InProgress,
            },
        ],
    };
    let expected_args_clone = expected_args.clone();

    // Mock agent consumer: emits a plan update then finishes the turn.
    tokio::spawn(async move {
        if let Some(ManagerCommand::Chat(req)) = api_rx.recv().await {
            let envelope = &req.envelope;
            assert_eq!(envelope.origin, ChannelOrigin::OwnerFrontend);
            assert_eq!(envelope.correlation.session_key, "gui:chat-1");
            let request_id = envelope
                .correlation
                .request_id
                .clone()
                .expect("manager chat request should carry request_id");
            match &envelope.payload {
                ChannelPayloadV1::Message {
                    parts,
                    context: Some(context),
                    ..
                } => {
                    assert!(matches!(
                        parts.as_slice(),
                        [ContentPart::Text { text }] if text == "hello"
                    ));
                    assert_eq!(
                        context.intent,
                        agent_diva_core::channel::OwnerTurnIntent::Agent
                    );
                }
                other => panic!("expected typed owner message, got {other:?}"),
            }
            let _ = req.event_tx.send(AgentEvent::SessionAdmission {
                observation: SessionAdmissionObservation {
                    code: None,
                    phase: SessionAdmissionPhase::Queued,
                    session_key: "gui:chat-1".to_string(),
                    request_id: request_id.clone(),
                    trace_id: "trace-e2e".to_string(),
                    queue_depth: 1,
                    wait_latency_ms: 0,
                },
            });
            let _ = req.event_tx.send(AgentEvent::SessionAdmission {
                observation: SessionAdmissionObservation {
                    code: None,
                    phase: SessionAdmissionPhase::Running,
                    session_key: "gui:chat-1".to_string(),
                    request_id,
                    trace_id: "trace-e2e".to_string(),
                    queue_depth: 1,
                    wait_latency_ms: 25,
                },
            });
            let _ = req.event_tx.send(AgentEvent::ChatPlanUpdate {
                args: expected_args_clone,
            });
            let _ = req.event_tx.send(AgentEvent::FinalResponse {
                content: "done".to_string(),
            });
        }
    });

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("bind random port");
    let port = listener.local_addr().expect("local addr").port();

    let (shutdown_tx, shutdown_rx) = watch::channel(false);
    let server_handle =
        tokio::spawn(async move { run_server_with_listener(state, listener, shutdown_rx).await });

    // Wait until the server is actually accepting connections.
    wait_for_server_ready(port).await;

    // Avoid any system proxy routing localhost through an intermediate hop.
    std::env::set_var("NO_PROXY", "127.0.0.1");

    let client = ApiClient::new(Some(format!("http://127.0.0.1:{}/api", port)));
    let (event_tx, mut event_rx) = mpsc::unbounded_channel::<AgentEvent>();
    let (chat_result_tx, chat_result_rx) = oneshot::channel();
    tokio::spawn(async move {
        let result = client
            .chat_with_target("hello".to_string(), Some("gui"), Some("chat-1"), event_tx)
            .await;
        let _ = chat_result_tx.send(result);
    });

    let first_event = match timeout(Duration::from_secs(5), event_rx.recv()).await {
        Ok(Some(event)) => event,
        Ok(None) => {
            let result = timeout(Duration::from_secs(5), chat_result_rx)
                .await
                .expect("timed out waiting for chat task result")
                .expect("chat result sender dropped");
            panic!(
                "event stream closed before first event; chat result: {:#?}",
                result
            );
        }
        Err(_) => panic!("timed out waiting for first event"),
    };

    match first_event {
        AgentEvent::SessionAdmission { observation } => {
            assert_eq!(observation.phase, SessionAdmissionPhase::Queued);
            assert_eq!(observation.queue_depth, 1);
            assert!(!observation.request_id.is_empty());
        }
        other => panic!("expected queued SessionAdmission, got {:?}", other),
    }

    let second_event = timeout(Duration::from_secs(5), event_rx.recv())
        .await
        .expect("timed out waiting for running admission")
        .expect("event stream closed before running admission");
    match second_event {
        AgentEvent::SessionAdmission { observation } => {
            assert_eq!(observation.phase, SessionAdmissionPhase::Running);
            assert_eq!(observation.trace_id, "trace-e2e");
            assert_eq!(observation.wait_latency_ms, 25);
        }
        other => panic!("expected running SessionAdmission, got {:?}", other),
    }

    let plan_event = timeout(Duration::from_secs(5), event_rx.recv())
        .await
        .expect("timed out waiting for plan event")
        .expect("event stream closed before plan event");
    match plan_event {
        AgentEvent::ChatPlanUpdate { args } => {
            assert_eq!(args, expected_args);
        }
        other => panic!("expected AgentEvent::ChatPlanUpdate, got {:?}", other),
    }

    // Ensure the client task finishes cleanly after the final response.
    let chat_result = timeout(Duration::from_secs(5), chat_result_rx)
        .await
        .expect("timed out waiting for chat task result")
        .expect("chat result sender dropped");
    chat_result.expect("chat_with_target failed");

    // Gracefully shut down the manager server.
    shutdown_tx
        .send(true)
        .expect("send shutdown signal to server");
    timeout(Duration::from_secs(5), server_handle)
        .await
        .expect("server shutdown timed out")
        .expect("server task panicked")
        .expect("server returned an error");
}

async fn wait_for_server_ready(port: u16) {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(5);
    let addr = format!("127.0.0.1:{}", port);
    while tokio::time::Instant::now() < deadline {
        match tokio::net::TcpStream::connect(&addr).await {
            Ok(mut stream) => {
                // Immediately close the probe connection.
                let _ = stream.shutdown().await;
                return;
            }
            Err(_) => {
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
        }
    }
    panic!("server did not become ready within 5 seconds");
}
