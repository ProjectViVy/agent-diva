use axum::{
    extract::DefaultBodyLimit,
    routing::{delete, get, patch, post},
    Router,
};
use std::net::SocketAddr;
use tokio::sync::{broadcast, watch};
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

use crate::handlers::{
    add_provider_model_handler, apply_laputa_proposal_handler, cancel_autodream_run_handler,
    chat_handler, create_cron_job_handler, create_laputa_proposal_handler, create_mcp_handler,
    create_provider_handler, decide_laputa_proposal_handler, delete_cron_job_handler,
    delete_mcp_handler, delete_provider_handler, delete_provider_model_handler,
    delete_session_handler, delete_skill_handler, edit_laputa_proposal_handler, events_handler,
    generate_session_title_handler, get_audit_events_handler, get_audit_log_handler,
    get_autodream_run_handler, get_channels_handler, get_config_handler, get_cron_job_handler,
    get_laputa_changelog_handler, get_laputa_proposal_handler, get_laputa_section_handler,
    get_laputa_snapshot_handler, get_mcps_handler, get_provider_handler,
    get_provider_models_handler, get_providers_handler, get_self_evolution_config_handler,
    get_session_history_handler, get_sessions_handler, get_skills_handler, get_tools_handler,
    health_handler, heartbeat_handler, list_autodream_run_events_handler,
    list_autodream_runs_handler, list_cron_jobs_handler, list_laputa_changelog_handler,
    list_laputa_proposals_handler, list_recall_feedback_handler, logs_routes,
    poll_laputa_events_handler, refresh_mcp_status_handler, reset_session_handler,
    resolve_provider_handler, rollback_laputa_changelog_handler, run_cron_job_handler,
    set_cron_job_enabled_handler, set_mcp_enabled_handler, stop_chat_handler,
    stop_cron_job_handler, stream_laputa_events_handler, todo_routes, token_stats_routes,
    transition_laputa_proposal_handler, trigger_autodream_run_handler, update_channel_handler,
    update_config_handler, update_cron_job_handler, update_mcp_handler, update_provider_handler,
    update_self_evolution_config_handler, update_session_title_handler, update_tools_handler,
    upload_file_handler, upload_skill_handler, write_laputa_section_handler,
};
use crate::state::AppState;

pub async fn run_server(
    state: AppState,
    port: u16,
    mut shutdown_rx: broadcast::Receiver<()>,
) -> anyhow::Result<()> {
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    tracing::info!("Listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    run_server_with_listener_and_signal(state, listener, async move {
        let _ = shutdown_rx.recv().await;
        tracing::info!("Server shutting down signal received");
    })
    .await
}

pub async fn run_server_with_listener(
    state: AppState,
    listener: tokio::net::TcpListener,
    mut shutdown_rx: watch::Receiver<bool>,
) -> anyhow::Result<()> {
    let listen_addr = listener.local_addr()?;
    tracing::info!("Listening on {}", listen_addr);

    run_server_with_listener_and_signal(state, listener, async move {
        let _ = shutdown_rx.wait_for(|value| *value).await;
        tracing::info!("Server shutting down signal received");
    })
    .await
}

async fn run_server_with_listener_and_signal<F>(
    state: AppState,
    listener: tokio::net::TcpListener,
    shutdown_signal: F,
) -> anyhow::Result<()>
where
    F: std::future::Future<Output = ()> + Send + 'static,
{
    let app = build_router(state);
    axum::serve(listener, app)
        .with_graceful_shutdown(async move {
            shutdown_signal.await;
        })
        .await?;
    Ok(())
}

/// Build the axum router with all manager HTTP routes.
pub fn build_router(state: AppState) -> Router {
    Router::new()
        .merge(runtime_routes())
        .merge(crate::handlers::command_approval_routes())
        .merge(provider_routes())
        .merge(planning_routes())
        .merge(autodream_routes())
        .merge(laputa_routes())
        .merge(audit_routes())
        .merge(token_stats_routes())
        .merge(todo_routes())
        .merge(misc_routes())
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

fn autodream_routes() -> Router<AppState> {
    Router::new()
        .route(
            "/api/autodream/runs",
            get(list_autodream_runs_handler).post(trigger_autodream_run_handler),
        )
        .route("/api/autodream/runs/:id", get(get_autodream_run_handler))
        .route(
            "/api/autodream/runs/:id/events",
            get(list_autodream_run_events_handler),
        )
        .route(
            "/api/autodream/runs/:id/cancel",
            post(cancel_autodream_run_handler),
        )
}

fn laputa_routes() -> Router<AppState> {
    Router::new()
        .route(
            "/api/laputa/proposals",
            get(list_laputa_proposals_handler).post(create_laputa_proposal_handler),
        )
        .route(
            "/api/laputa/proposals/:id",
            get(get_laputa_proposal_handler).put(edit_laputa_proposal_handler),
        )
        .route(
            "/api/laputa/proposals/:id/transition",
            post(transition_laputa_proposal_handler),
        )
        .route(
            "/api/laputa/proposals/:id/apply",
            post(apply_laputa_proposal_handler),
        )
        .route(
            "/api/laputa/proposals/:id/decision",
            post(decide_laputa_proposal_handler),
        )
        .route("/api/laputa/snapshot", get(get_laputa_snapshot_handler))
        .route("/api/laputa/section/:name", get(get_laputa_section_handler))
        .route(
            "/api/laputa/section/:name/write",
            post(write_laputa_section_handler),
        )
        .route("/api/laputa/changelog", get(list_laputa_changelog_handler))
        .route(
            "/api/laputa/recall-feedback",
            get(list_recall_feedback_handler),
        )
        .route(
            "/api/laputa/changelog/:id",
            get(get_laputa_changelog_handler),
        )
        .route(
            "/api/laputa/changelog/:id/rollback",
            post(rollback_laputa_changelog_handler),
        )
        .route(
            "/api/laputa/events/:kind",
            get(stream_laputa_events_handler),
        )
        .route(
            "/api/laputa/events/:kind/poll",
            get(poll_laputa_events_handler),
        )
}

fn runtime_routes() -> Router<AppState> {
    Router::new()
        .route("/api/chat", post(chat_handler))
        .route("/api/chat/stop", post(stop_chat_handler))
        .route("/api/events", get(events_handler))
        .route("/api/sessions", get(get_sessions_handler))
        .route(
            "/api/sessions/:id",
            get(get_session_history_handler)
                .delete(delete_session_handler)
                .post(delete_session_handler),
        )
        .route(
            "/api/sessions/:id/title",
            patch(update_session_title_handler),
        )
        .route(
            "/api/sessions/:id/generate-title",
            post(generate_session_title_handler),
        )
        .route("/api/sessions/reset", post(reset_session_handler))
        .route(
            "/api/config",
            get(get_config_handler).post(update_config_handler),
        )
        .route(
            "/api/config/self-evolution",
            get(get_self_evolution_config_handler).post(update_self_evolution_config_handler),
        )
        .route(
            "/api/channels",
            get(get_channels_handler).post(update_channel_handler),
        )
        .route(
            "/api/tools",
            get(get_tools_handler).post(update_tools_handler),
        )
        .route(
            "/api/skills",
            get(get_skills_handler).post(upload_skill_handler),
        )
        .route("/api/skills/:name", delete(delete_skill_handler))
        .route(
            "/api/files/upload",
            post(upload_file_handler).layer(DefaultBodyLimit::max(50 * 1024 * 1024)),
        ) // 50MB limit
        .route("/api/mcps", get(get_mcps_handler).post(create_mcp_handler))
        .route(
            "/api/mcps/:name",
            axum::routing::put(update_mcp_handler).delete(delete_mcp_handler),
        )
        .route("/api/mcps/:name/enable", post(set_mcp_enabled_handler))
        .route("/api/mcps/:name/refresh", post(refresh_mcp_status_handler))
        .route(
            "/api/cron/jobs",
            get(list_cron_jobs_handler).post(create_cron_job_handler),
        )
        .route(
            "/api/cron/jobs/:id",
            get(get_cron_job_handler)
                .put(update_cron_job_handler)
                .delete(delete_cron_job_handler),
        )
        .route(
            "/api/cron/jobs/:id/enable",
            post(set_cron_job_enabled_handler),
        )
        .route("/api/cron/jobs/:id/run", post(run_cron_job_handler))
        .route("/api/cron/jobs/:id/stop", post(stop_cron_job_handler))
}

fn provider_routes() -> Router<AppState> {
    Router::new()
        .route(
            "/api/providers",
            get(get_providers_handler).post(create_provider_handler),
        )
        .route("/api/providers/resolve", post(resolve_provider_handler))
        .route(
            "/api/providers/:name",
            get(get_provider_handler)
                .put(update_provider_handler)
                .delete(delete_provider_handler),
        )
        .route(
            "/api/providers/:name/models",
            get(get_provider_models_handler).post(add_provider_model_handler),
        )
        .route(
            "/api/providers/:name/models/:model_id",
            delete(delete_provider_model_handler),
        )
}

fn planning_routes() -> Router<AppState> {
    use crate::handlers::planning::{
        active_plan_execution_handler, append_plan_report_revision_handler,
        approve_plan_report_handler, create_plan_report_handler, list_execution_todos_handler,
        list_plan_reports_handler, update_execution_todo_handler,
    };
    Router::new()
        .route(
            "/api/plan-reports",
            get(list_plan_reports_handler).post(create_plan_report_handler),
        )
        .route(
            "/api/plan-reports/:report_id/revisions",
            post(append_plan_report_revision_handler),
        )
        .route(
            "/api/plan-reports/:report_id/approve",
            post(approve_plan_report_handler),
        )
        .route(
            "/api/plan-executions/active",
            get(active_plan_execution_handler),
        )
        .route(
            "/api/plan-executions/:execution_id/todos",
            get(list_execution_todos_handler),
        )
        .route(
            "/api/plan-executions/:execution_id/todos/:todo_id",
            patch(update_execution_todo_handler),
        )
}

fn audit_routes() -> Router<AppState> {
    Router::new()
        .route("/api/audit/log", get(get_audit_log_handler))
        .route("/api/audit/events", get(get_audit_events_handler))
        .merge(logs_routes())
}

fn misc_routes() -> Router<AppState> {
    Router::new()
        .route("/api/health", get(health_handler))
        .route("/api/heartbeat", get(heartbeat_handler))
}

#[cfg(test)]
mod tests {
    use super::build_router;
    use agent_diva_autodream::DeterministicReflectionEngine;
    use agent_diva_core::evolution::{
        AutoDreamFailureCode, AutoDreamRunState, EvidenceRef, EvidenceSource, EvolutionProposal,
        LaputaSectionName, ProposalState, ProposalType, RiskLevel,
    };
    use agent_diva_core::governance::{
        ApprovalGrant, Decision, GovernanceSubject, GovernanceSubjectKind,
    };
    use agent_diva_laputa::MemoryGovernanceDecision;
    use axum::body::{to_bytes, Body};
    use axum::http::{Request, StatusCode};
    use chrono::{DateTime, Utc};
    use std::sync::Arc;
    use tower::util::ServiceExt;

    use crate::state::{AppState, ManagerCommand};

    const G0_RUNTIME_CONTRACT: &str = include_str!("../tests/fixtures/g0_runtime_contract.json");

    fn ts(seconds: u32) -> DateTime<Utc> {
        DateTime::parse_from_rfc3339(&format!("2026-06-14T00:03:{seconds:02}Z"))
            .unwrap()
            .with_timezone(&Utc)
    }

    fn laputa_proposal(id: &str, patch: &str) -> EvolutionProposal {
        EvolutionProposal {
            id: id.to_string(),
            created_at: ts(2),
            updated_at: ts(2),
            created_by: "autodream".to_string(),
            proposal_type: ProposalType::MemoryPatch,
            target_section: LaputaSectionName::MemoryMd,
            evidence_refs: vec![EvidenceRef {
                id: "ev-1".to_string(),
                source: EvidenceSource::Session,
                uri: "session://ev-1".to_string(),
                excerpt: Some("bounded evidence".to_string()),
                hash: Some("hash-ev-1".to_string()),
                created_at: ts(1),
            }],
            proposed_patch: patch.to_string(),
            risk_level: RiskLevel::Medium,
            state: ProposalState::PendingReview,
            source_run_id: Some("run-1".to_string()),
        }
    }

    #[tokio::test]
    async fn build_router_keeps_health_and_skills_routes_without_overlap() {
        let (api_tx, mut api_rx) = tokio::sync::mpsc::channel(1);
        // get_skills_handler sends GetSkills on api_tx and awaits a oneshot reply; without a
        // consumer the request would hang forever.
        tokio::spawn(async move {
            while let Some(cmd) = api_rx.recv().await {
                if let ManagerCommand::GetSkills(tx) = cmd {
                    let _ = tx.send(Ok(vec![]));
                }
            }
        });
        let temp = tempfile::tempdir().unwrap();
        let state =
            AppState::new(api_tx, agent_diva_core::bus::MessageBus::new(), temp.path()).unwrap();

        let app = build_router(state.clone());

        let health_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/health")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(health_response.status(), StatusCode::SERVICE_UNAVAILABLE);

        let skills_response = app
            .oneshot(
                Request::builder()
                    .uri("/api/skills")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(skills_response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn g0_runtime_route_contract_matches_fixture() {
        let fixture: serde_json::Value = serde_json::from_str(G0_RUNTIME_CONTRACT).unwrap();
        let (api_tx, _api_rx) = tokio::sync::mpsc::channel(1);
        let temp = tempfile::tempdir().unwrap();
        let state =
            AppState::new(api_tx, agent_diva_core::bus::MessageBus::new(), temp.path()).unwrap();
        let app = build_router(state.clone());

        for route in fixture["routes"].as_array().unwrap() {
            let request = Request::builder()
                .method(route["method"].as_str().unwrap())
                .uri(route["path"].as_str().unwrap())
                .body(Body::empty())
                .unwrap();
            let response = app.clone().oneshot(request).await.unwrap();
            assert_eq!(
                response.status().as_u16(),
                route["status_without_runtime"].as_u64().unwrap() as u16,
                "{} {}",
                route["method"],
                route["path"]
            );
        }
    }

    #[tokio::test]
    async fn build_router_exposes_laputa_routes() {
        let (api_tx, _api_rx) = tokio::sync::mpsc::channel(1);
        let temp = tempfile::tempdir().unwrap();
        let state =
            AppState::new(api_tx, agent_diva_core::bus::MessageBus::new(), temp.path()).unwrap();

        let app = build_router(state.clone());

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/laputa/snapshot")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn evolution_workspace_exposes_payload_free_recall_feedback() {
        let (api_tx, _api_rx) = tokio::sync::mpsc::channel(1);
        let temp = tempfile::tempdir().unwrap();
        let state =
            AppState::new(api_tx, agent_diva_core::bus::MessageBus::new(), temp.path()).unwrap();
        let response = build_router(state)
            .oneshot(
                Request::builder()
                    .uri("/api/laputa/recall-feedback?limit=10")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let value: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(value["status"], "ok");
        assert_eq!(value["feedback"], serde_json::json!([]));
    }

    #[tokio::test]
    async fn proposal_decision_retry_finishes_transition_after_decision_crash_window() {
        let (api_tx, _api_rx) = tokio::sync::mpsc::channel(1);
        let temp = tempfile::tempdir().unwrap();
        let state =
            AppState::new(api_tx, agent_diva_core::bus::MessageBus::new(), temp.path()).unwrap();
        state
            .laputa
            .create_proposal(laputa_proposal(
                "proposal-decision-recovery",
                r#"{"facts":[]}"#,
            ))
            .unwrap();
        let proposal = state
            .laputa
            .get_proposal("proposal-decision-recovery")
            .unwrap();
        let pending = state
            .memory_governance
            .submit(&proposal, None, Utc::now())
            .await
            .unwrap();
        state
            .memory_governance
            .decide(
                &proposal,
                pending.request_version,
                MemoryGovernanceDecision {
                    decision: Decision::Allow,
                    grant: ApprovalGrant::Once,
                    actor: GovernanceSubject {
                        kind: GovernanceSubjectKind::User,
                        id: "reviewer".to_string(),
                    },
                    idempotency_key: "decision-before-crash",
                    decided_at: Utc::now(),
                },
            )
            .await
            .unwrap();
        assert_eq!(
            state
                .laputa
                .get_proposal("proposal-decision-recovery")
                .unwrap()
                .state,
            ProposalState::PendingReview
        );
        let app = build_router(state.clone());
        let request = || {
            Request::builder()
                .method("POST")
                .uri("/api/laputa/proposals/proposal-decision-recovery/decision")
                .header("content-type", "application/json")
                .body(Body::from(
                    serde_json::json!({
                        "decision": "allow",
                        "grant": "once",
                        "expected_version": pending.request_version,
                        "idempotency_key": "decision-before-crash"
                    })
                    .to_string(),
                ))
                .unwrap()
        };

        let recovered = app.clone().oneshot(request()).await.unwrap();
        assert_eq!(recovered.status(), StatusCode::OK);
        let replay = app.oneshot(request()).await.unwrap();
        assert_eq!(replay.status(), StatusCode::OK);
        assert_eq!(
            state
                .laputa
                .get_proposal("proposal-decision-recovery")
                .unwrap()
                .state,
            ProposalState::Approved
        );
    }

    #[tokio::test]
    async fn laputa_apply_preserves_typed_schema_incompatible_error_code() {
        let (api_tx, _api_rx) = tokio::sync::mpsc::channel(1);
        let temp = tempfile::tempdir().unwrap();
        let state =
            AppState::new(api_tx, agent_diva_core::bus::MessageBus::new(), temp.path()).unwrap();
        state
            .laputa
            .create_proposal(laputa_proposal("proposal-1", "not-json"))
            .unwrap();
        let proposal = state.laputa.get_proposal("proposal-1").unwrap();
        let now = Utc::now();
        let pending = state
            .memory_governance
            .submit(&proposal, None, now)
            .await
            .unwrap();
        let authorized = state
            .memory_governance
            .decide(
                &proposal,
                pending.request_version,
                MemoryGovernanceDecision {
                    decision: Decision::Allow,
                    grant: ApprovalGrant::Once,
                    actor: GovernanceSubject {
                        kind: GovernanceSubjectKind::User,
                        id: "reviewer".to_string(),
                    },
                    idempotency_key: "test-decision",
                    decided_at: now,
                },
            )
            .await
            .unwrap();
        state
            .laputa
            .transition_proposal("proposal-1", ProposalState::Approved, now)
            .unwrap();

        let app = build_router(state);
        let request_id = authorized.request_id;
        let expected_version = authorized.request_version;
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/laputa/proposals/proposal-1/apply")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::json!({
                            "governance_request_id": request_id,
                            "expected_version": expected_version,
                            "idempotency_key": "apply-test"
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let value: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(value["status"], "error");
        assert_eq!(value["code"], "schema_incompatible");
    }

    #[tokio::test]
    async fn laputa_apply_replays_consumed_result_without_duplicate_execution() {
        let (api_tx, _api_rx) = tokio::sync::mpsc::channel(1);
        let temp = tempfile::tempdir().unwrap();
        let state =
            AppState::new(api_tx, agent_diva_core::bus::MessageBus::new(), temp.path()).unwrap();
        state
            .laputa
            .create_proposal(laputa_proposal("proposal-replay", r#"{"facts":[]}"#))
            .unwrap();
        let proposal = state.laputa.get_proposal("proposal-replay").unwrap();
        let now = Utc::now();
        let pending = state
            .memory_governance
            .submit(&proposal, None, now)
            .await
            .unwrap();
        let authorized = state
            .memory_governance
            .decide(
                &proposal,
                pending.request_version,
                MemoryGovernanceDecision {
                    decision: Decision::Allow,
                    grant: ApprovalGrant::Once,
                    actor: GovernanceSubject {
                        kind: GovernanceSubjectKind::User,
                        id: "reviewer".to_string(),
                    },
                    idempotency_key: "decision-replay",
                    decided_at: now,
                },
            )
            .await
            .unwrap();
        state
            .laputa
            .transition_proposal("proposal-replay", ProposalState::Approved, now)
            .unwrap();

        let app = build_router(state.clone());
        let body = serde_json::json!({
            "governance_request_id": authorized.request_id,
            "expected_version": authorized.request_version,
            "idempotency_key": "apply-replay"
        })
        .to_string();
        let request = || {
            Request::builder()
                .method("POST")
                .uri("/api/laputa/proposals/proposal-replay/apply")
                .header("content-type", "application/json")
                .body(Body::from(body.clone()))
                .unwrap()
        };

        let first = app.clone().oneshot(request()).await.unwrap();
        assert_eq!(first.status(), StatusCode::OK);
        let first: serde_json::Value =
            serde_json::from_slice(&to_bytes(first.into_body(), usize::MAX).await.unwrap())
                .unwrap();
        let second = app.oneshot(request()).await.unwrap();
        assert_eq!(second.status(), StatusCode::OK);
        let second: serde_json::Value =
            serde_json::from_slice(&to_bytes(second.into_body(), usize::MAX).await.unwrap())
                .unwrap();

        assert_eq!(first["changelog"]["id"], second["changelog"]["id"]);
        assert_eq!(first["audit_event"]["id"], second["audit_event"]["id"]);
        assert_eq!(first["governance"], second["governance"]);
        assert_eq!(
            state
                .laputa
                .list_changelog(agent_diva_laputa::ChangelogFilter::default())
                .unwrap()
                .total,
            1
        );
    }

    #[tokio::test]
    async fn autodream_manual_run_executes_worker_and_returns_terminal_failure() {
        let (api_tx, _api_rx) = tokio::sync::mpsc::channel(1);
        let temp = tempfile::tempdir().unwrap();
        let state =
            AppState::new(api_tx, agent_diva_core::bus::MessageBus::new(), temp.path()).unwrap();

        let app = build_router(state.clone());

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/autodream/runs")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"trigger":"manual"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body: serde_json::Value =
            serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap())
                .unwrap();
        assert_eq!(body["run"]["state"], "pending");
        assert_eq!(body["run"]["orchestration"]["phase"], "queued");
        let run_id = body["run"]["id"].as_str().unwrap();
        let terminal = tokio::time::timeout(std::time::Duration::from_secs(5), async {
            loop {
                let status = state.autodream.get_run_status(run_id).unwrap();
                if matches!(
                    status.run.state,
                    AutoDreamRunState::Completed
                        | AutoDreamRunState::Failed
                        | AutoDreamRunState::Cancelled
                ) {
                    break status;
                }
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            }
        })
        .await
        .unwrap();
        assert_eq!(terminal.run.state, AutoDreamRunState::Failed);
        assert_eq!(
            terminal.run.failure_code,
            Some(AutoDreamFailureCode::InputUnavailable)
        );
        assert!(terminal
            .run
            .error
            .as_deref()
            .unwrap_or_default()
            .contains("all mandatory inputs omitted"));
    }

    #[tokio::test]
    async fn autodream_manual_run_executes_worker_and_publishes_proposal() {
        let (api_tx, _api_rx) = tokio::sync::mpsc::channel(1);
        let temp = tempfile::tempdir().unwrap();
        let mut sessions = agent_diva_core::session::SessionManager::new(temp.path());
        let session = sessions.get_or_create("chat:e0");
        session.add_message("user", "verified local E0 session evidence");
        let saved = session.clone();
        sessions.save(&saved).unwrap();
        let mut state =
            AppState::new(api_tx, agent_diva_core::bus::MessageBus::new(), temp.path()).unwrap();
        state.autodream = state
            .autodream
            .clone()
            .with_reflection_engine(Some(Arc::new(
                DeterministicReflectionEngine::evidence_echo(),
            )));
        let app = build_router(state.clone());

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/autodream/runs")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"trigger":"manual"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body: serde_json::Value =
            serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap())
                .unwrap();
        assert_eq!(body["run"]["state"], "pending");
        assert_eq!(body["run"]["orchestration"]["phase"], "queued");
        let run_id = body["run"]["id"].as_str().unwrap();
        let terminal = tokio::time::timeout(std::time::Duration::from_secs(5), async {
            loop {
                let status = state.autodream.get_run_status(run_id).unwrap();
                if matches!(
                    status.run.state,
                    AutoDreamRunState::Completed
                        | AutoDreamRunState::Failed
                        | AutoDreamRunState::Cancelled
                ) {
                    break status;
                }
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            }
        })
        .await
        .unwrap();
        assert_eq!(terminal.run.state, AutoDreamRunState::Completed);
        assert_eq!(terminal.run.proposal_ids.len(), 1);
        assert_eq!(
            state
                .laputa
                .list_proposals(agent_diva_laputa::ProposalFilter::default())
                .unwrap()
                .len(),
            1
        );
    }

    #[tokio::test]
    async fn vertical_autodream_typed_memory_recall_feedback_and_rollback_closes() {
        let (api_tx, _api_rx) = tokio::sync::mpsc::channel(1);
        let temp = tempfile::tempdir().unwrap();
        let journal = agent_diva_core::experience::ExperienceJournal::open(temp.path());
        journal
            .append(&journal.tool_evidence(
                "gui:e7",
                "trace-e7",
                "tool-e7",
                "exec",
                agent_diva_core::experience::OutcomeKind::Succeeded,
            ))
            .unwrap();

        let mut state = AppState::new_with_runtime_memory(
            api_tx,
            agent_diva_core::bus::MessageBus::new(),
            temp.path(),
            agent_diva_sandbox::CommandApprovalCoordinator::default(),
            agent_diva_core::config::schema::MemoryAuthorityMode::Typed,
        )
        .unwrap();
        state.autodream = state
            .autodream
            .clone()
            .with_reflection_engine(Some(Arc::new(
                DeterministicReflectionEngine::evidence_echo(),
            )));
        let app = build_router(state.clone());

        let triggered = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/autodream/runs")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"trigger":"manual"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(triggered.status(), StatusCode::OK);
        let triggered: serde_json::Value =
            serde_json::from_slice(&to_bytes(triggered.into_body(), usize::MAX).await.unwrap())
                .unwrap();
        let run_id = triggered["run"]["id"].as_str().unwrap();
        let terminal = tokio::time::timeout(std::time::Duration::from_secs(5), async {
            loop {
                let status = state.autodream.get_run_status(run_id).unwrap();
                if matches!(
                    status.run.state,
                    AutoDreamRunState::Completed
                        | AutoDreamRunState::Failed
                        | AutoDreamRunState::Cancelled
                ) {
                    break status;
                }
                tokio::time::sleep(std::time::Duration::from_millis(10)).await;
            }
        })
        .await
        .unwrap();
        assert_eq!(terminal.run.state, AutoDreamRunState::Completed);
        let proposal_id = terminal.run.proposal_ids.first().unwrap();
        let proposal = state.laputa.get_proposal(proposal_id).unwrap();
        assert_eq!(proposal.source_run_id.as_deref(), Some(run_id));

        let now = Utc::now();
        let pending = state
            .memory_governance
            .submit(&proposal, None, now)
            .await
            .unwrap();
        let authorized = state
            .memory_governance
            .decide(
                &proposal,
                pending.request_version,
                MemoryGovernanceDecision {
                    decision: Decision::Allow,
                    grant: ApprovalGrant::Once,
                    actor: GovernanceSubject {
                        kind: GovernanceSubjectKind::User,
                        id: "e7-reviewer".into(),
                    },
                    idempotency_key: "e7-decision",
                    decided_at: now,
                },
            )
            .await
            .unwrap();
        state
            .laputa
            .transition_proposal(proposal_id, ProposalState::Approved, now)
            .unwrap();
        let applied = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/laputa/proposals/{proposal_id}/apply"))
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::json!({
                            "governance_request_id": authorized.request_id,
                            "expected_version": authorized.request_version,
                            "idempotency_key": "e7-apply"
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        let applied_status = applied.status();
        let applied: serde_json::Value =
            serde_json::from_slice(&to_bytes(applied.into_body(), usize::MAX).await.unwrap())
                .unwrap();
        assert_eq!(applied_status, StatusCode::OK, "{applied}");
        let changelog_id = applied["changelog"]["id"].as_str().unwrap();

        let workspace_id = agent_diva_core::workspace_identity::canonical_workspace_id(temp.path());
        let store = agent_diva_laputa::TypedMemoryStore::open_existing_canonical(temp.path())
            .await
            .unwrap();
        let recall = agent_diva_laputa::LaputaRecallService::new(store);
        let recall_request = agent_diva_core::memory::RecallRequest {
            query: "command action succeeded".into(),
            scope: agent_diva_core::memory::MemoryScope {
                tenant_id: "local".into(),
                workspace_id,
                session_id: None,
            },
            correlation: agent_diva_core::governance::AuditCorrelation {
                request_id: "e7-recall".into(),
                turn_id: "e7-turn".into(),
                session_id: "gui:e7".into(),
                trace_id: Some("trace-e7".into()),
            },
            now: Utc::now(),
            token_budget: 4_000,
            max_candidates: 8,
            policy: agent_diva_core::memory::RecallPolicy::default_prompt(),
        };
        let recalled = recall.recall_shadow(&recall_request).await.unwrap();
        assert_eq!(
            recalled.outcome.status,
            agent_diva_core::memory::RecallStatus::Ready
        );
        assert_eq!(recalled.outcome.selected_records.len(), 1);
        let selected = &recalled.outcome.selected_records[0];
        agent_diva_laputa::RecallFeedbackStore::new(
            agent_diva_laputa::LaputaStorage::open(temp.path()).unwrap(),
        )
        .commit_pending(
            vec![agent_diva_laputa::PendingRecallFeedback {
                request_id: "e7-recall".into(),
                selected: vec![(
                    selected.id.clone(),
                    selected.provenance.content_digest.clone(),
                )],
                injected: true,
                selected_at: now,
            }],
            agent_diva_laputa::RecallTaskOutcome::Succeeded,
            false,
            Utc::now(),
        )
        .unwrap();

        let rolled_back = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/laputa/changelog/{changelog_id}/rollback"))
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"reason":"e7 automated recovery"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        let rollback_status = rolled_back.status();
        let rollback_body = serde_json::from_slice::<serde_json::Value>(
            &to_bytes(rolled_back.into_body(), usize::MAX).await.unwrap(),
        )
        .unwrap();
        assert_eq!(rollback_status, StatusCode::OK, "{rollback_body}");
        let after_rollback = recall.recall_shadow(&recall_request).await.unwrap();
        assert!(after_rollback.outcome.selected_records.is_empty());
        assert_eq!(
            agent_diva_laputa::RecallFeedbackStore::new(
                agent_diva_laputa::LaputaStorage::open(temp.path()).unwrap()
            )
            .recent(10)
            .unwrap()
            .len(),
            1
        );
    }

    #[tokio::test]
    async fn write_laputa_section_creates_pending_proposal_without_applying() {
        let (api_tx, _api_rx) = tokio::sync::mpsc::channel(1);
        let temp = tempfile::tempdir().unwrap();
        let state =
            AppState::new(api_tx, agent_diva_core::bus::MessageBus::new(), temp.path()).unwrap();

        let app = build_router(state.clone());
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/laputa/section/memory_md/write")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"content":"{\"note\":\"hello\"}","summary":"GUI edit"}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let value: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(value["status"], "ok");
        assert!(value["proposal_id"]
            .as_str()
            .unwrap()
            .starts_with("user-edit-memory_md-"));
        assert_eq!(value["proposal_type"], "memory_patch");
        assert_eq!(value["risk_level"], "medium");
        assert_eq!(value["state"], "pending_review");
        assert!(value["changelog_id"].is_null());
        assert!(value["applied_at"].is_null());

        let section = state
            .laputa
            .read_section(LaputaSectionName::MemoryMd)
            .unwrap();
        assert_eq!(section.content, serde_json::Value::Null);
        let proposal = state
            .laputa
            .get_proposal(value["proposal_id"].as_str().unwrap())
            .unwrap();
        assert_eq!(proposal.state, ProposalState::PendingReview);
        assert_eq!(
            proposal.evidence_refs[0].excerpt.as_deref(),
            Some("GUI edit")
        );
    }

    #[tokio::test]
    async fn write_laputa_section_unknown() {
        let (api_tx, _api_rx) = tokio::sync::mpsc::channel(1);
        let temp = tempfile::tempdir().unwrap();
        let state =
            AppState::new(api_tx, agent_diva_core::bus::MessageBus::new(), temp.path()).unwrap();

        let app = build_router(state);
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/laputa/section/not_a_section/write")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"content":"{}"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let value: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(value["status"], "error");
        assert_eq!(value["code"], "unknown_section");
    }

    #[tokio::test]
    async fn write_laputa_section_malformed_json() {
        let (api_tx, _api_rx) = tokio::sync::mpsc::channel(1);
        let temp = tempfile::tempdir().unwrap();
        let state =
            AppState::new(api_tx, agent_diva_core::bus::MessageBus::new(), temp.path()).unwrap();

        let app = build_router(state);
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/laputa/section/memory_md/write")
                    .header("content-type", "application/json")
                    .body(Body::from("not-json"))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn write_laputa_section_schema_incompatible() {
        let (api_tx, _api_rx) = tokio::sync::mpsc::channel(1);
        let temp = tempfile::tempdir().unwrap();
        let state =
            AppState::new(api_tx, agent_diva_core::bus::MessageBus::new(), temp.path()).unwrap();

        let app = build_router(state);
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/laputa/section/memory_md/write")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"content":"plain text"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let value: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(value["status"], "error");
        assert_eq!(value["code"], "schema_incompatible");
    }
}
