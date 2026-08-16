//! CLI-facing unified approval contract.

use crate::client::ApiClient;
use anyhow::{bail, Result};
use dialoguer::{theme::ColorfulTheme, Select};
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const APPROVAL_REQUIRED_NONINTERACTIVE: &str = "approval_required_noninteractive";
pub const APPROVAL_QUEUE_UNAVAILABLE: &str = "approval_queue_unavailable";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ApprovalView {
    pub request_id: String,
    pub version: u64,
    pub domain: String,
    pub capability: String,
    pub resource: ApprovalResource,
    pub risk: String,
    pub status: String,
    pub expires_at: String,
    #[serde(default)]
    pub evidence: Vec<Value>,
    #[serde(default)]
    pub actions: Vec<String>,
    #[serde(default)]
    pub presentation: Option<Value>,
    #[serde(default)]
    pub reason_code: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ApprovalResource {
    pub workspace_id: String,
    #[serde(default)]
    pub session_id: Option<String>,
    pub kind: String,
    pub resource_id: String,
    #[serde(default)]
    pub boundary: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalListPage {
    pub approvals: Vec<ApprovalView>,
    pub next_cursor: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApprovalChoice {
    AllowOnce,
    AllowSession,
    AllowRule,
    Deny,
    Cancel,
    Skip,
}

impl ApprovalChoice {
    pub fn parse(value: &str) -> Result<Self> {
        match value {
            "allow-once" => Ok(Self::AllowOnce),
            "allow-session" => Ok(Self::AllowSession),
            "allow-rule" => Ok(Self::AllowRule),
            "deny" => Ok(Self::Deny),
            "cancel" => Ok(Self::Cancel),
            _ => bail!("invalid approval decision: {value}"),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct ApprovalCliResult {
    pub ok: bool,
    pub reason_code: Option<String>,
    pub approval: Option<ApprovalView>,
}

pub async fn apply_choice(
    client: &ApiClient,
    approval: &ApprovalView,
    choice: ApprovalChoice,
) -> Result<ApprovalView> {
    let key = format!("cli-{}-v{}", approval.request_id, approval.version);
    match choice {
        ApprovalChoice::Cancel => {
            client
                .cancel_approval(&approval.request_id, approval.version, &key)
                .await
        }
        ApprovalChoice::AllowOnce => {
            client
                .decide_approval(
                    &approval.request_id,
                    approval.version,
                    &key,
                    "allow",
                    "once",
                )
                .await
        }
        ApprovalChoice::AllowSession => {
            client
                .decide_approval(
                    &approval.request_id,
                    approval.version,
                    &key,
                    "allow",
                    "session",
                )
                .await
        }
        ApprovalChoice::AllowRule => {
            client
                .decide_approval(
                    &approval.request_id,
                    approval.version,
                    &key,
                    "allow",
                    "rule",
                )
                .await
        }
        ApprovalChoice::Deny => {
            client
                .decide_approval(&approval.request_id, approval.version, &key, "deny", "once")
                .await
        }
        ApprovalChoice::Skip => bail!("skipped approval"),
    }
}

pub fn print_human(approval: &ApprovalView) {
    println!(
        "{} v{} [{} / {} / {}]",
        approval.request_id, approval.version, approval.domain, approval.risk, approval.status
    );
    println!("  capability: {}", approval.capability);
    println!("  scope: {}", approval.resource.workspace_id);
    println!("  expires_at: {}", approval.expires_at);
    if let Some(presentation) = &approval.presentation {
        if let Some(title) = presentation.get("title").and_then(Value::as_str) {
            println!("  summary: {title}");
        }
        if let Some(summary) = presentation.get("summary").and_then(Value::as_str) {
            println!("  detail: {summary}");
        }
    }
    println!("  evidence: {} item(s)", approval.evidence.len());
}

pub async fn review_pending(
    client: &ApiClient,
    session: Option<&str>,
) -> Result<Vec<ApprovalView>> {
    let page = client.list_approvals(Some("pending"), session).await?;
    let mut resolved = Vec::new();
    for approval in page.approvals {
        print_human(&approval);
        let mut labels = vec!["Deny", "Cancel", "Skip"];
        let mut choices = vec![
            ApprovalChoice::Deny,
            ApprovalChoice::Cancel,
            ApprovalChoice::Skip,
        ];
        if approval.actions.iter().any(|action| action == "allow") {
            labels.insert(0, "Allow once");
            choices.insert(0, ApprovalChoice::AllowOnce);
            if approval.domain == "command" {
                labels.insert(1, "Allow session (5 minutes)");
                choices.insert(1, ApprovalChoice::AllowSession);
                if approval
                    .presentation
                    .as_ref()
                    .and_then(|value| value.get("suggested_prefix"))
                    .is_some()
                {
                    labels.insert(2, "Allow rule");
                    choices.insert(2, ApprovalChoice::AllowRule);
                }
            }
        }
        let selected = match Select::with_theme(&ColorfulTheme::default())
            .with_prompt("Explicit decision (empty input never allows)")
            .items(&labels)
            .default(labels.len() - 1)
            .interact()
        {
            Ok(selected) => selected,
            Err(error) => {
                let _ = apply_choice(client, &approval, ApprovalChoice::Cancel).await;
                return Err(error.into());
            }
        };
        let choice = choices[selected];
        if choice != ApprovalChoice::Skip {
            resolved.push(apply_choice(client, &approval, choice).await?);
        }
    }
    Ok(resolved)
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{body_json, method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[test]
    fn decision_parser_never_treats_empty_as_allow() {
        assert!(ApprovalChoice::parse("").is_err());
        assert_eq!(
            ApprovalChoice::parse("allow-once").unwrap(),
            ApprovalChoice::AllowOnce
        );
    }

    fn approval(domain: &str) -> Value {
        serde_json::json!({
            "request_id": format!("{domain}-1"),
            "version": 1,
            "domain": domain,
            "capability": match domain { "command" => "command_execute", "plan" => "plan_execute", _ => "memory_apply" },
            "resource": {
                "workspace_id": "workspace",
                "session_id": "cli:test",
                "kind": domain,
                "resource_id": format!("{domain}-resource"),
                "boundary": null
            },
            "risk": "high",
            "status": "pending",
            "expires_at": "2026-08-03T12:05:00Z",
            "evidence": [],
            "actions": ["allow", "deny", "cancel"],
            "presentation": { "title": format!("{domain} approval") },
            "reason_code": null
        })
    }

    #[tokio::test]
    async fn unified_list_parses_command_and_plan_domains() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/api/approvals"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "approvals": [approval("command"), approval("plan")],
                "next_cursor": null
            })))
            .mount(&server)
            .await;
        let page = ApiClient::new(Some(format!("{}/api", server.uri())))
            .list_approvals(Some("pending"), Some("cli:test"))
            .await
            .unwrap();
        assert_eq!(page.approvals.len(), 2);
        assert_eq!(page.approvals[0].domain, "command");
        assert_eq!(page.approvals[1].domain, "plan");
    }

    #[tokio::test]
    async fn explicit_allow_once_uses_version_and_idempotency_key() {
        let server = MockServer::start().await;
        let body = serde_json::json!({
            "expected_version": 1,
            "idempotency_key": "cli-plan-1-v1",
            "decision": "allow",
            "grant": "once"
        });
        Mock::given(method("POST"))
            .and(path("/api/approvals/plan-1/decisions"))
            .and(body_json(body))
            .respond_with(ResponseTemplate::new(200).set_body_json(approval("plan")))
            .mount(&server)
            .await;
        let client = ApiClient::new(Some(format!("{}/api", server.uri())));
        let view: ApprovalView = serde_json::from_value(approval("plan")).unwrap();
        let resolved = apply_choice(&client, &view, ApprovalChoice::AllowOnce)
            .await
            .unwrap();
        assert_eq!(resolved.request_id, "plan-1");
    }

    #[tokio::test]
    async fn command_and_plan_decisions_share_one_cli_contract() {
        let server = MockServer::start().await;
        for (domain, choice, decision) in [
            ("command", ApprovalChoice::Deny, "deny"),
            ("plan", ApprovalChoice::AllowOnce, "allow"),
        ] {
            Mock::given(method("POST"))
                .and(path(format!("/api/approvals/{domain}-1/decisions")))
                .and(body_json(serde_json::json!({
                    "expected_version": 1,
                    "idempotency_key": format!("cli-{domain}-1-v1"),
                    "decision": decision,
                    "grant": "once"
                })))
                .respond_with(ResponseTemplate::new(200).set_body_json(approval(domain)))
                .expect(1)
                .mount(&server)
                .await;
            let client = ApiClient::new(Some(format!("{}/api", server.uri())));
            let view = serde_json::from_value(approval(domain)).unwrap();
            assert_eq!(
                apply_choice(&client, &view, choice).await.unwrap().domain,
                domain
            );
        }
    }
}
