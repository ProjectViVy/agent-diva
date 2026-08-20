//! Vertical contract for the skills.sh marketplace routes.
//!
//! Covers search validation/mapping and snapshot install through
//! `SkillService`, with the marketplace base URL pointed at a local
//! wiremock server via `AGENT_DIVA_SKILLS_MARKETPLACE_URL`.

use std::path::Path;

use agent_diva_core::bus::MessageBus;
use agent_diva_manager::{build_router, marketplace::MARKETPLACE_BASE_URL_ENV, AppState};
use agent_diva_sandbox::CommandApprovalCoordinator;
use axum::{
    body::{to_bytes, Body},
    http::{Method, Request, StatusCode},
    Router,
};
use serde_json::{json, Value};
use tempfile::TempDir;
use tokio::sync::mpsc;
use tower::ServiceExt;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn allow_loopback_proxy() {
    let existing = std::env::var("NO_PROXY").unwrap_or_default();
    if existing.contains("127.0.0.1") {
        return;
    }
    let merged = format!("127.0.0.1,localhost,{existing}")
        .trim_matches(',')
        .to_string();
    std::env::set_var("NO_PROXY", &merged);
    std::env::set_var("no_proxy", &merged);
}

fn marketplace_state(root: &Path) -> AppState {
    let (api_tx, _api_rx) = mpsc::channel(8);
    AppState::new_with_runtime_memory(
        api_tx,
        MessageBus::new(),
        root,
        CommandApprovalCoordinator::default(),
        agent_diva_core::ask_user::AskUserCoordinator::default(),
    )
    .unwrap()
}

async fn json_request(
    app: &Router,
    method: Method,
    uri: &str,
    payload: Value,
) -> (StatusCode, Value) {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(method)
                .uri(uri)
                .header("content-type", "application/json")
                .body(Body::from(payload.to_string()))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let value = serde_json::from_slice(&body).unwrap();
    (status, value)
}

#[tokio::test]
async fn marketplace_search_and_install_end_to_end() {
    allow_loopback_proxy();
    let upstream = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/api/search"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "query": "commit",
            "skills": [{
                "id": "demo-owner/demo-repo/demo-skill",
                "name": "demo-skill",
                "installs": 42_u64,
                "source": "demo-owner/demo-repo"
            }]
        })))
        .mount(&upstream)
        .await;
    Mock::given(method("GET"))
        .and(path("/api/download/demo-owner/demo-repo/demo-skill"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "files": [
                { "path": "SKILL.md", "contents": "---\nname: demo-skill\ndescription: demo\n---\nbody" },
                { "path": "README.md", "contents": "readme" }
            ],
            "hash": "hash-1"
        })))
        .mount(&upstream)
        .await;
    std::env::set_var(MARKETPLACE_BASE_URL_ENV, upstream.uri());

    let config = TempDir::new().unwrap();
    let app = build_router(marketplace_state(config.path()));

    // Short queries are rejected before hitting upstream.
    let (status, body) = json_request(
        &app,
        Method::GET,
        "/api/skills/marketplace/search?q=x",
        json!({}),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["status"], "error");

    // Search maps upstream rows.
    let (status, body) = json_request(
        &app,
        Method::GET,
        "/api/skills/marketplace/search?q=commit",
        json!({}),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["skills"][0]["id"], "demo-owner/demo-repo/demo-skill");
    assert_eq!(body["skills"][0]["installs"], 42);

    // Malformed install ids are rejected.
    let (status, body) = json_request(
        &app,
        Method::POST,
        "/api/skills/marketplace/install",
        json!({ "id": "not-a-skill-id" }),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    assert_eq!(body["status"], "error");

    // Install downloads the snapshot and writes it into the skill home.
    let (status, body) = json_request(
        &app,
        Method::POST,
        "/api/skills/marketplace/install",
        json!({ "id": "demo-owner/demo-repo/demo-skill" }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "body was: {body}");
    assert_eq!(body["skill"]["slug"], "demo-skill");
    assert!(config.path().join("skills/demo-skill/SKILL.md").is_file());
    assert!(config.path().join("skills/demo-skill/README.md").is_file());

    // The installed skill shows up in the regular skills listing.
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/api/skills")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let listing: Value = serde_json::from_slice(&body).unwrap();
    let slugs: Vec<&str> = listing["skills"]
        .as_array()
        .unwrap()
        .iter()
        .map(|skill| skill["slug"].as_str().unwrap())
        .collect();
    assert!(slugs.contains(&"demo-skill"));

    // Reinstalling the same skill conflicts (marketplace only installs new).
    let (status, body) = json_request(
        &app,
        Method::POST,
        "/api/skills/marketplace/install",
        json!({ "id": "demo-owner/demo-repo/demo-skill" }),
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);
    assert_eq!(body["status"], "error");

    std::env::remove_var(MARKETPLACE_BASE_URL_ENV);
}

#[tokio::test]
async fn marketplace_featured_serves_embedded_snapshot() {
    let config = TempDir::new().unwrap();
    let app = build_router(marketplace_state(config.path()));

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method(Method::GET)
                .uri("/api/skills/marketplace/featured")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let value: Value = serde_json::from_slice(&body).unwrap();
    assert_eq!(value["status"], "ok");
    let skills = value["skills"].as_array().unwrap();
    assert!(!skills.is_empty());
    assert_eq!(value["total"], skills.len());
    assert!(value["generated_at"].as_str().map(str::is_empty) == Some(false));
    for skill in skills {
        assert_eq!(skill["id"].as_str().unwrap().matches('/').count(), 2);
    }
}
