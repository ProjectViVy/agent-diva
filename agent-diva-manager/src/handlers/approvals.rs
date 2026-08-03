//! Unified approval HTTP and durable SSE projection.

use std::{collections::VecDeque, convert::Infallible, time::Duration};

use agent_diva_core::governance::{ApprovalLedgerError, ApprovalStatus};
use agent_diva_sandbox::ApprovalResolveError;
use axum::{
    extract::{rejection::JsonRejection, rejection::QueryRejection, Path, Query, State},
    http::StatusCode,
    response::{
        sse::{Event, KeepAlive, Sse},
        IntoResponse, Response,
    },
    routing::{get, post},
    Json, Router,
};
use futures::stream;
use serde::{Deserialize, Serialize};

use crate::{
    approval_service::{
        ApprovalEventView, ApprovalListQuery, ApprovalReasonCode, ApprovalService,
        ApprovalServiceError, UnifiedCancelBody, UnifiedDecisionBody,
    },
    state::AppState,
};

#[derive(Debug, Serialize)]
struct ApprovalApiError {
    error: String,
    reason_code: ApprovalReasonCode,
}

#[derive(Debug, Deserialize)]
pub struct ApprovalEventsQuery {
    cursor: Option<String>,
    #[serde(default = "default_event_limit")]
    limit: u32,
}

fn default_event_limit() -> u32 {
    100
}

pub fn approval_routes() -> Router<AppState> {
    Router::new()
        .route("/api/approvals", get(list_approvals_handler))
        .route("/api/approvals/events", get(approval_events_handler))
        .route("/api/approvals/:request_id", get(get_approval_handler))
        .route(
            "/api/approvals/:request_id/decisions",
            post(decide_approval_handler),
        )
        .route(
            "/api/approvals/:request_id/cancel",
            post(cancel_approval_handler),
        )
}

pub async fn list_approvals_handler(
    State(state): State<AppState>,
    query: Result<Query<ApprovalListQuery>, QueryRejection>,
) -> Response {
    let Query(query) = match query {
        Ok(query) => query,
        Err(error) => {
            return extraction_error_response(error, ApprovalReasonCode::ApprovalInvalidQuery)
        }
    };
    let service = match ApprovalService::from_state(&state) {
        Ok(service) => service,
        Err(error) => return error_response(error),
    };
    match service.list(&query).await {
        Ok(page) => Json(page).into_response(),
        Err(error) => error_response(error),
    }
}

pub async fn get_approval_handler(
    State(state): State<AppState>,
    Path(request_id): Path<String>,
) -> Response {
    let service = match ApprovalService::from_state(&state) {
        Ok(service) => service,
        Err(error) => return error_response(error),
    };
    match service.detail(&request_id).await {
        Ok(view) => Json(view).into_response(),
        Err(error) => error_response(error),
    }
}

pub async fn decide_approval_handler(
    State(state): State<AppState>,
    Path(request_id): Path<String>,
    body: Result<Json<UnifiedDecisionBody>, JsonRejection>,
) -> Response {
    let Json(body) = match body {
        Ok(body) => body,
        Err(error) => {
            return extraction_error_response(error, ApprovalReasonCode::ApprovalInvalidBody)
        }
    };
    let service = match ApprovalService::from_state(&state) {
        Ok(service) => service,
        Err(error) => return error_response(error),
    };
    match service.decide(&request_id, &body).await {
        Ok(view) => Json(view).into_response(),
        Err(error) => error_response(error),
    }
}

pub async fn cancel_approval_handler(
    State(state): State<AppState>,
    Path(request_id): Path<String>,
    body: Result<Json<UnifiedCancelBody>, JsonRejection>,
) -> Response {
    let Json(body) = match body {
        Ok(body) => body,
        Err(error) => {
            return extraction_error_response(error, ApprovalReasonCode::ApprovalInvalidBody)
        }
    };
    let service = match ApprovalService::from_state(&state) {
        Ok(service) => service,
        Err(error) => return error_response(error),
    };
    match service.cancel(&request_id, &body).await {
        Ok(view) => Json(view).into_response(),
        Err(error) => error_response(error),
    }
}

struct EventStreamState {
    service: ApprovalService,
    cursor: Option<String>,
    limit: u32,
    queued: VecDeque<ApprovalEventView>,
}

pub async fn approval_events_handler(
    State(state): State<AppState>,
    query: Result<Query<ApprovalEventsQuery>, QueryRejection>,
) -> Response {
    let Query(query) = match query {
        Ok(query) => query,
        Err(error) => {
            return extraction_error_response(error, ApprovalReasonCode::ApprovalInvalidQuery)
        }
    };
    let service = match ApprovalService::from_state(&state) {
        Ok(service) => service,
        Err(error) => return error_response(error),
    };
    if let Err(error) = service.events(query.cursor.as_deref(), query.limit).await {
        return error_response(error);
    }
    let stream = stream::unfold(
        EventStreamState {
            service,
            cursor: query.cursor,
            limit: query.limit,
            queued: VecDeque::new(),
        },
        |mut state| async move {
            loop {
                if let Some(event) = state.queued.pop_front() {
                    state.cursor = Some(event.cursor.clone());
                    let name = event_name(&event.status);
                    let payload = serde_json::to_string(&event).ok()?;
                    let item = Event::default()
                        .id(event.cursor.clone())
                        .event(name)
                        .data(payload);
                    return Some((Ok::<_, Infallible>(item), state));
                }
                match state
                    .service
                    .events(state.cursor.as_deref(), state.limit)
                    .await
                {
                    Ok(events) if !events.is_empty() => state.queued.extend(events),
                    Ok(_) | Err(_) => tokio::time::sleep(Duration::from_millis(250)).await,
                }
            }
        },
    );
    Sse::new(stream)
        .keep_alive(KeepAlive::default())
        .into_response()
}

fn event_name(status: &ApprovalStatus) -> &'static str {
    match status {
        ApprovalStatus::Pending => "approval.requested",
        ApprovalStatus::Allowed
        | ApprovalStatus::Denied
        | ApprovalStatus::Revoked
        | ApprovalStatus::Expired => "approval.resolved",
        ApprovalStatus::Consumed => "approval.updated",
    }
}

fn error_response(error: ApprovalServiceError) -> Response {
    let (status, reason_code) = match error {
        ApprovalServiceError::Ledger(ApprovalLedgerError::NotFound)
        | ApprovalServiceError::Command(ApprovalResolveError::NotFound) => {
            (StatusCode::NOT_FOUND, ApprovalReasonCode::ApprovalNotFound)
        }
        ApprovalServiceError::Ledger(ApprovalLedgerError::VersionConflict)
        | ApprovalServiceError::Command(ApprovalResolveError::VersionConflict) => (
            StatusCode::CONFLICT,
            ApprovalReasonCode::ApprovalVersionConflict,
        ),
        ApprovalServiceError::Ledger(ApprovalLedgerError::IdempotencyConflict)
        | ApprovalServiceError::Command(ApprovalResolveError::IdempotencyConflict) => (
            StatusCode::CONFLICT,
            ApprovalReasonCode::ApprovalIdempotencyConflict,
        ),
        ApprovalServiceError::Ledger(ApprovalLedgerError::Expired)
        | ApprovalServiceError::Command(ApprovalResolveError::Expired) => {
            (StatusCode::CONFLICT, ApprovalReasonCode::ApprovalExpired)
        }
        ApprovalServiceError::Command(ApprovalResolveError::AlreadyResolved) => (
            StatusCode::CONFLICT,
            ApprovalReasonCode::ApprovalAlreadyResolved,
        ),
        ApprovalServiceError::Ledger(ApprovalLedgerError::AlreadyConsumed)
        | ApprovalServiceError::Command(ApprovalResolveError::AlreadyConsumed) => (
            StatusCode::CONFLICT,
            ApprovalReasonCode::ApprovalAlreadyConsumed,
        ),
        ApprovalServiceError::Ledger(ApprovalLedgerError::InvalidTransition)
        | ApprovalServiceError::Command(ApprovalResolveError::InvalidTransition) => (
            StatusCode::UNPROCESSABLE_ENTITY,
            ApprovalReasonCode::ApprovalInvalidTransition,
        ),
        ApprovalServiceError::InvalidGrant
        | ApprovalServiceError::Command(ApprovalResolveError::InvalidGlobalApproval) => (
            StatusCode::UNPROCESSABLE_ENTITY,
            ApprovalReasonCode::ApprovalInvalidGrant,
        ),
        ApprovalServiceError::PayloadUnavailable => (
            StatusCode::CONFLICT,
            ApprovalReasonCode::ApprovalPayloadUnavailable,
        ),
        ApprovalServiceError::Unavailable => (
            StatusCode::SERVICE_UNAVAILABLE,
            ApprovalReasonCode::ApprovalQueueUnavailable,
        ),
        ApprovalServiceError::Ledger(ApprovalLedgerError::Validation(
            agent_diva_core::governance::GovernanceValidationError::InvalidCursor
            | agent_diva_core::governance::GovernanceValidationError::InvalidPageLimit,
        )) => (
            StatusCode::UNPROCESSABLE_ENTITY,
            ApprovalReasonCode::ApprovalInvalidCursor,
        ),
        ApprovalServiceError::Ledger(ApprovalLedgerError::Validation(_)) => (
            StatusCode::UNPROCESSABLE_ENTITY,
            ApprovalReasonCode::ApprovalDigestMismatch,
        ),
        ApprovalServiceError::Ledger(ApprovalLedgerError::Persistence(_))
        | ApprovalServiceError::Command(ApprovalResolveError::Persistence)
        | ApprovalServiceError::Memory(_)
        | ApprovalServiceError::Plan(_) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            ApprovalReasonCode::ApprovalPersistenceFailed,
        ),
    };
    (
        status,
        Json(ApprovalApiError {
            error: error.to_string(),
            reason_code,
        }),
    )
        .into_response()
}

fn extraction_error_response(
    error: impl std::fmt::Display,
    reason_code: ApprovalReasonCode,
) -> Response {
    (
        StatusCode::UNPROCESSABLE_ENTITY,
        Json(ApprovalApiError {
            error: error.to_string(),
            reason_code,
        }),
    )
        .into_response()
}

#[cfg(test)]
mod tests {
    use super::*;
    use agent_diva_core::{
        bus::MessageBus,
        config::schema::MemoryAuthorityMode,
        evolution::{
            EvidenceRef, EvidenceSource, EvolutionProposal, LaputaSectionName, ProposalState,
            ProposalType, RiskLevel,
        },
        governance::{ApprovalCoordinator, GovernanceSubject, SqliteGovernanceLedger},
    };
    use agent_diva_sandbox::{
        CommandApprovalCoordinator, CommandApprovalScope, CommandApprovalStatus,
    };
    use axum::{
        body::{to_bytes, Body},
        http::Request,
    };
    use futures::StreamExt;
    use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
    use std::{path::Path, sync::Arc};
    use tokio::sync::mpsc;
    use tower::ServiceExt;

    async fn state(root: &Path) -> AppState {
        std::fs::create_dir_all(root.join(".laputa")).unwrap();
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(
                SqliteConnectOptions::new()
                    .filename(root.join(".laputa/governance.db"))
                    .create_if_missing(true),
            )
            .await
            .unwrap();
        let governance =
            ApprovalCoordinator::new(Arc::new(SqliteGovernanceLedger::new(pool).await.unwrap()));
        let workspace_id = agent_diva_core::workspace_identity::canonical_workspace_id(root);
        let command = CommandApprovalCoordinator::new(Duration::from_secs(5))
            .governed(governance.clone(), workspace_id.clone());
        let planning = Arc::new(crate::planning_service::PlanningService::governed(
            root.to_path_buf(),
            governance.clone(),
            workspace_id,
        ));
        let (api_tx, _api_rx) = mpsc::channel(1);
        AppState::new_with_runtime_governance(
            api_tx,
            MessageBus::new(),
            root,
            command,
            MemoryAuthorityMode::Legacy,
            governance,
            planning,
        )
        .unwrap()
    }

    #[test]
    fn unified_rust_fixture_matches_public_contract_types() {
        let fixture: serde_json::Value = serde_json::from_str(include_str!(
            "../../tests/fixtures/approval_contract_v1.json"
        ))
        .unwrap();
        let approval: crate::approval_service::ApprovalView =
            serde_json::from_value(fixture["approval"].clone()).unwrap();
        let decision: UnifiedDecisionBody =
            serde_json::from_value(fixture["decision"].clone()).unwrap();
        let event: ApprovalEventView = serde_json::from_value(fixture["event"].clone()).unwrap();
        assert_eq!(
            approval.domain,
            crate::approval_service::ApprovalDomain::Command
        );
        assert_eq!(decision.expected_version, 1);
        assert_eq!(event.cursor, "1");
    }

    async fn pending_id(state: &AppState) -> String {
        tokio::time::timeout(Duration::from_secs(1), async {
            loop {
                if let Some(request) = state.command_approvals.pending(None).await.first() {
                    return request.approval_id.clone();
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap()
    }

    #[tokio::test]
    async fn unified_command_contract_enforces_cas_and_replays_durable_events() {
        let temp = tempfile::tempdir().unwrap();
        let state = state(temp.path()).await;
        let waiter = tokio::spawn({
            let coordinator = state.command_approvals.clone();
            async move {
                coordinator
                    .request(
                        "echo unified".into(),
                        ".".into(),
                        "sandbox denied".into(),
                        CommandApprovalScope {
                            channel: "api".into(),
                            chat_id: "chat".into(),
                            session_key: "api:chat".into(),
                        },
                    )
                    .await
            }
        });
        let request_id = pending_id(&state).await;
        let app = approval_routes().with_state(state.clone());

        let detail = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/api/approvals/{request_id}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(detail.status(), StatusCode::OK);
        let json: serde_json::Value =
            serde_json::from_slice(&to_bytes(detail.into_body(), usize::MAX).await.unwrap())
                .unwrap();
        assert_eq!(json["domain"], "command");
        assert_eq!(json["version"], 1);
        assert_eq!(json["presentation"]["command"], "echo unified");

        let decide = |version, key: &str| {
            Request::builder()
                .method("POST")
                .uri(format!("/api/approvals/{request_id}/decisions"))
                .header("content-type", "application/json")
                .body(Body::from(format!(
                    r#"{{"expected_version":{version},"idempotency_key":"{key}","decision":"allow","grant":"once"}}"#
                )))
                .unwrap()
        };
        let stale = app.clone().oneshot(decide(9, "stale")).await.unwrap();
        assert_eq!(stale.status(), StatusCode::CONFLICT);
        let stale_json: serde_json::Value =
            serde_json::from_slice(&to_bytes(stale.into_body(), usize::MAX).await.unwrap())
                .unwrap();
        assert_eq!(stale_json["reason_code"], "approval_version_conflict");

        assert_eq!(
            app.clone()
                .oneshot(decide(1, "unified-command-allow"))
                .await
                .unwrap()
                .status(),
            StatusCode::OK
        );
        assert_eq!(waiter.await.unwrap().1, CommandApprovalStatus::Approved);
        assert_eq!(
            app.clone()
                .oneshot(decide(1, "unified-command-allow"))
                .await
                .unwrap()
                .status(),
            StatusCode::OK
        );

        let service = ApprovalService::from_state(&state).unwrap();
        let first = service.events(None, 1).await.unwrap();
        assert_eq!(first[0].status, ApprovalStatus::Pending);
        let remaining = service.events(Some(&first[0].cursor), 10).await.unwrap();
        assert_eq!(
            remaining
                .iter()
                .map(|event| event.status.clone())
                .collect::<Vec<_>>(),
            [ApprovalStatus::Allowed, ApprovalStatus::Consumed]
        );
    }

    #[tokio::test]
    async fn unified_cancel_revokes_and_wakes_command_waiter() {
        let temp = tempfile::tempdir().unwrap();
        let state = state(temp.path()).await;
        let waiter = tokio::spawn({
            let coordinator = state.command_approvals.clone();
            async move {
                coordinator
                    .request(
                        "echo cancelled".into(),
                        ".".into(),
                        "sandbox denied".into(),
                        CommandApprovalScope {
                            channel: "api".into(),
                            chat_id: "chat".into(),
                            session_key: "api:chat".into(),
                        },
                    )
                    .await
            }
        });
        let request_id = pending_id(&state).await;
        let app = approval_routes().with_state(state);
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/approvals/{request_id}/cancel"))
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"expected_version":1,"idempotency_key":"cancel-command"}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(waiter.await.unwrap().1, CommandApprovalStatus::Cancelled);
    }

    #[tokio::test]
    async fn unified_event_stream_rejects_invalid_reconnect_cursor() {
        let temp = tempfile::tempdir().unwrap();
        let app = approval_routes().with_state(state(temp.path()).await);
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/approvals/events?cursor=not-a-cursor")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
        let json: serde_json::Value =
            serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap())
                .unwrap();
        assert_eq!(json["reason_code"], "approval_invalid_cursor");

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/approvals/events?limit=0")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
    }

    #[tokio::test]
    async fn unified_routes_return_typed_extraction_errors() {
        let temp = tempfile::tempdir().unwrap();
        let app = approval_routes().with_state(state(temp.path()).await);

        let malformed_body = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/approvals/request/decisions")
                    .header("content-type", "application/json")
                    .body(Body::from("{"))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(malformed_body.status(), StatusCode::UNPROCESSABLE_ENTITY);
        let json: serde_json::Value = serde_json::from_slice(
            &to_bytes(malformed_body.into_body(), usize::MAX)
                .await
                .unwrap(),
        )
        .unwrap();
        assert_eq!(json["reason_code"], "approval_invalid_body");

        let malformed_query = app
            .oneshot(
                Request::builder()
                    .uri("/api/approvals?limit=not-a-number")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(malformed_query.status(), StatusCode::UNPROCESSABLE_ENTITY);
        let json: serde_json::Value = serde_json::from_slice(
            &to_bytes(malformed_query.into_body(), usize::MAX)
                .await
                .unwrap(),
        )
        .unwrap();
        assert_eq!(json["reason_code"], "approval_invalid_query");
    }

    #[tokio::test]
    async fn unified_event_stream_emits_durable_requested_frame_and_cursor() {
        let temp = tempfile::tempdir().unwrap();
        let state = state(temp.path()).await;
        let waiter = tokio::spawn({
            let coordinator = state.command_approvals.clone();
            async move {
                coordinator
                    .request(
                        "echo streamed".into(),
                        ".".into(),
                        "sandbox denied".into(),
                        CommandApprovalScope {
                            channel: "api".into(),
                            chat_id: "chat".into(),
                            session_key: "api:chat".into(),
                        },
                    )
                    .await
            }
        });
        let request_id = pending_id(&state).await;
        let app = approval_routes().with_state(state.clone());
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/approvals/events?limit=1")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.headers()["content-type"], "text/event-stream");
        let mut body = response.into_body().into_data_stream();
        let frame = tokio::time::timeout(Duration::from_secs(1), body.next())
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        let frame = String::from_utf8(frame.to_vec()).unwrap();
        assert!(frame.contains("event: approval.requested"));
        assert!(frame.contains("id: 1"));
        assert!(frame.contains(&format!(r#""request_id":"{request_id}""#)));
        assert!(frame.contains(r#""cursor":"1""#));

        state
            .command_approvals
            .cancel_request(
                &request_id,
                1,
                "stream-test-cancel",
                GovernanceSubject {
                    kind: agent_diva_core::governance::GovernanceSubjectKind::System,
                    id: "test".into(),
                },
            )
            .await
            .unwrap();
        assert_eq!(waiter.await.unwrap().1, CommandApprovalStatus::Cancelled);
    }

    #[test]
    fn unified_event_names_cover_every_governance_status() {
        assert_eq!(event_name(&ApprovalStatus::Pending), "approval.requested");
        assert_eq!(event_name(&ApprovalStatus::Allowed), "approval.resolved");
        assert_eq!(event_name(&ApprovalStatus::Denied), "approval.resolved");
        assert_eq!(event_name(&ApprovalStatus::Revoked), "approval.resolved");
        assert_eq!(event_name(&ApprovalStatus::Expired), "approval.resolved");
        assert_eq!(event_name(&ApprovalStatus::Consumed), "approval.updated");
    }

    #[tokio::test]
    async fn unified_plan_allow_consumes_and_initializes_execution() {
        let temp = tempfile::tempdir().unwrap();
        let state = state(temp.path()).await;
        let planning = state.planning_service.as_ref().unwrap();
        let report = planning
            .create_report(
                "gui:plan",
                "Unified Plan",
                "# Unified Plan\n## 目标\nShip\n## 范围\nManager\n## 计划步骤\n1. Execute\n## 风险与假设\nNone\n## 验证方法\nTests",
                agent_diva_core::planning::PlanRevisionAuthor::Agent,
            )
            .await
            .unwrap();
        let service = ApprovalService::from_state(&state).unwrap();
        let page = service
            .list(&ApprovalListQuery {
                domain: Some(crate::approval_service::ApprovalDomain::Plan),
                status: Some(ApprovalStatus::Pending),
                session: None,
                cursor: None,
                limit: 100,
            })
            .await
            .unwrap();
        let pending = page.approvals.first().unwrap();
        let result = service
            .decide(
                &pending.request_id,
                &UnifiedDecisionBody {
                    expected_version: pending.version,
                    idempotency_key: "unified-plan-allow".into(),
                    decision: agent_diva_core::governance::Decision::Allow,
                    grant: agent_diva_core::governance::ApprovalGrant::Once,
                },
            )
            .await
            .unwrap();
        assert_eq!(result.status, ApprovalStatus::Consumed);
        let replay = service
            .decide(
                &pending.request_id,
                &UnifiedDecisionBody {
                    expected_version: pending.version,
                    idempotency_key: "unified-plan-allow".into(),
                    decision: agent_diva_core::governance::Decision::Allow,
                    grant: agent_diva_core::governance::ApprovalGrant::Once,
                },
            )
            .await
            .unwrap();
        assert_eq!(replay.status, ApprovalStatus::Consumed);
        assert!(planning
            .active_execution(&report.report.session_key)
            .await
            .is_some());
    }

    #[tokio::test]
    async fn unified_memory_deny_updates_governance_and_domain_state() {
        let temp = tempfile::tempdir().unwrap();
        let state = state(temp.path()).await;
        let now = chrono::Utc::now();
        let proposal = state
            .laputa
            .create_proposal(EvolutionProposal {
                id: "unified-memory".into(),
                created_at: now,
                updated_at: now,
                created_by: "test".into(),
                proposal_type: ProposalType::MemoryPatch,
                target_section: LaputaSectionName::MemoryMd,
                evidence_refs: vec![EvidenceRef {
                    id: "evidence-1".into(),
                    source: EvidenceSource::Session,
                    uri: "session://unified".into(),
                    excerpt: None,
                    hash: Some("hash".into()),
                    created_at: now,
                }],
                proposed_patch: r#"{"facts":[]}"#.into(),
                risk_level: RiskLevel::Medium,
                state: ProposalState::PendingReview,
                source_run_id: None,
            })
            .unwrap();
        let pending = state
            .memory_governance
            .submit(&proposal, None, now)
            .await
            .unwrap();
        let service = ApprovalService::from_state(&state).unwrap();
        let result = service
            .decide(
                &pending.request_id,
                &UnifiedDecisionBody {
                    expected_version: pending.request_version,
                    idempotency_key: "unified-memory-deny".into(),
                    decision: agent_diva_core::governance::Decision::Deny,
                    grant: agent_diva_core::governance::ApprovalGrant::Once,
                },
            )
            .await
            .unwrap();
        assert_eq!(result.status, ApprovalStatus::Denied);
        assert_eq!(
            service
                .decide(
                    &pending.request_id,
                    &UnifiedDecisionBody {
                        expected_version: pending.request_version,
                        idempotency_key: "unified-memory-deny".into(),
                        decision: agent_diva_core::governance::Decision::Deny,
                        grant: agent_diva_core::governance::ApprovalGrant::Once,
                    },
                )
                .await
                .unwrap()
                .status,
            ApprovalStatus::Denied
        );
        assert_eq!(
            state.laputa.get_proposal(&proposal.id).unwrap().state,
            ProposalState::Rejected
        );
    }
}
