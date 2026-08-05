//! Conversational ask-user (clarify) HITL coordination.
//!
//! This is the "Conversational Clarify" HITL domain, deliberately separate from
//! governance/approval HITL (Plan / Sandbox Command / Memory apply). The
//! coordinator is a process-local, in-memory oneshot: a question is delivered,
//! the caller blocks until it is answered, cancelled, or expires.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{oneshot, Mutex};
use uuid::Uuid;

/// Default lifetime for a pending ask_user question (10 minutes).
pub const DEFAULT_ASK_USER_TIMEOUT: Duration = Duration::from_secs(600);

/// A single pending question delivered to the user.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AskUserQuestion {
    pub question_id: String,
    pub question: String,
    pub choices: Vec<String>,
    pub allow_other: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<String>,
    pub created_at: DateTime<Utc>,
    pub timeout_seconds: u64,
}

/// Outcome of an ask_user request.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AskUserStatus {
    Answered,
    Cancelled,
    Expired,
}

/// Structured answer returned as the tool result.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AskUserResponse {
    pub question_id: String,
    pub status: AskUserStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selected: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selected_index: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub other_text: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AskUserError {
    NotFound,
    InvalidChoice,
    OtherNotAllowed,
    Expired,
}

struct PendingQuestion {
    question: AskUserQuestion,
    response_tx: oneshot::Sender<AskUserResponse>,
}

#[derive(Default)]
struct AskUserState {
    pending: HashMap<String, PendingQuestion>,
}

/// Process-local coordinator for conversational clarify questions.
#[derive(Clone)]
pub struct AskUserCoordinator {
    state: Arc<Mutex<AskUserState>>,
    timeout: Duration,
}

impl Default for AskUserCoordinator {
    fn default() -> Self {
        Self::new(DEFAULT_ASK_USER_TIMEOUT)
    }
}

impl AskUserCoordinator {
    pub fn new(timeout: Duration) -> Self {
        Self {
            state: Arc::new(Mutex::new(AskUserState::default())),
            timeout,
        }
    }

    /// Effective question lifetime in seconds (also the tool timeout override).
    pub fn timeout_secs(&self) -> u64 {
        self.timeout.as_secs()
    }

    /// Ask a single question and block until answered, cancelled, or expired.
    pub async fn request(
        &self,
        question: impl Into<String>,
        choices: Vec<String>,
        allow_other: bool,
        context: Option<String>,
    ) -> Result<AskUserResponse, AskUserError> {
        let question_id = Uuid::new_v4().to_string();
        let (response_tx, response_rx) = oneshot::channel();
        let question = AskUserQuestion {
            question_id: question_id.clone(),
            question: question.into(),
            choices,
            allow_other,
            context,
            created_at: Utc::now(),
            timeout_seconds: self.timeout.as_secs(),
        };
        self.state.lock().await.pending.insert(
            question_id.clone(),
            PendingQuestion {
                question,
                response_tx,
            },
        );
        match tokio::time::timeout(self.timeout, response_rx).await {
            Ok(Ok(response)) => Ok(response),
            Ok(Err(_)) | Err(_) => {
                self.state.lock().await.pending.remove(&question_id);
                Err(AskUserError::Expired)
            }
        }
    }

    /// List questions awaiting a user response, oldest first.
    pub async fn pending(&self) -> Vec<AskUserQuestion> {
        let state = self.state.lock().await;
        let mut questions: Vec<_> = state.pending.values().map(|p| p.question.clone()).collect();
        questions.sort_by(|a, b| a.created_at.cmp(&b.created_at));
        questions
    }

    /// Resolve a pending question with a choice or free-text answer.
    pub async fn answer(
        &self,
        question_id: &str,
        selected_index: Option<usize>,
        other_text: Option<String>,
    ) -> Result<(), AskUserError> {
        let mut state = self.state.lock().await;
        let Some(pending) = state.pending.remove(question_id) else {
            return Err(AskUserError::NotFound);
        };
        let response = match (selected_index, other_text) {
            (Some(index), _) => match pending.question.choices.get(index) {
                Some(choice) => AskUserResponse {
                    question_id: question_id.to_string(),
                    status: AskUserStatus::Answered,
                    selected: Some(choice.clone()),
                    selected_index: Some(index),
                    other_text: None,
                },
                None => {
                    state.pending.insert(question_id.to_string(), pending);
                    return Err(AskUserError::InvalidChoice);
                }
            },
            (None, Some(text)) if pending.question.allow_other => AskUserResponse {
                question_id: question_id.to_string(),
                status: AskUserStatus::Answered,
                selected: None,
                selected_index: None,
                other_text: Some(text),
            },
            (None, Some(_)) => {
                state.pending.insert(question_id.to_string(), pending);
                return Err(AskUserError::OtherNotAllowed);
            }
            _ => {
                state.pending.insert(question_id.to_string(), pending);
                return Err(AskUserError::InvalidChoice);
            }
        };
        let _ = pending.response_tx.send(response);
        Ok(())
    }

    /// Cancel a pending question without an answer.
    pub async fn cancel(&self, question_id: &str) -> Result<(), AskUserError> {
        let mut state = self.state.lock().await;
        let Some(pending) = state.pending.remove(question_id) else {
            return Err(AskUserError::NotFound);
        };
        let response = AskUserResponse {
            question_id: question_id.to_string(),
            status: AskUserStatus::Cancelled,
            selected: None,
            selected_index: None,
            other_text: None,
        };
        let _ = pending.response_tx.send(response);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Spawn a `request` future owned by its coordinator (JoinHandle is 'static).
    fn spawn_request(
        coordinator: AskUserCoordinator,
        question: &'static str,
        choices: Vec<String>,
        allow_other: bool,
        context: Option<String>,
    ) -> tokio::task::JoinHandle<Result<AskUserResponse, AskUserError>> {
        tokio::spawn(async move {
            coordinator
                .request(question, choices, allow_other, context)
                .await
        })
    }

    /// Wait until a spawned `request` future has registered its question.
    async fn first_pending(coordinator: &AskUserCoordinator) -> AskUserQuestion {
        for _ in 0..100 {
            if let Some(question) = coordinator.pending().await.into_iter().next() {
                return question;
            }
            tokio::task::yield_now().await;
        }
        panic!("no pending question registered");
    }

    #[tokio::test]
    async fn answer_choice_returns_selected_label() {
        let coordinator = AskUserCoordinator::new(Duration::from_secs(30));
        let ask = spawn_request(
            coordinator.clone(),
            "Which backend?",
            vec!["A".into(), "B".into()],
            false,
            None,
        );
        let question_id = first_pending(&coordinator).await.question_id;
        assert_eq!(
            coordinator.answer(&question_id, Some(1), None).await,
            Ok(())
        );
        let response = ask.await.unwrap().unwrap();
        assert_eq!(response.status, AskUserStatus::Answered);
        assert_eq!(response.selected.as_deref(), Some("B"));
        assert_eq!(response.selected_index, Some(1));
    }

    #[tokio::test]
    async fn answer_other_requires_allow_other() {
        let coordinator = AskUserCoordinator::new(Duration::from_secs(30));
        let ask = spawn_request(coordinator.clone(), "Pick one?", vec![], true, None);
        let question_id = first_pending(&coordinator).await.question_id;
        coordinator
            .answer(&question_id, None, Some("custom".into()))
            .await
            .unwrap();
        let response = ask.await.unwrap().unwrap();
        assert_eq!(response.status, AskUserStatus::Answered);
        assert_eq!(response.other_text.as_deref(), Some("custom"));

        let ask2 = spawn_request(coordinator.clone(), "Pick one?", vec![], false, None);
        let question_id2 = first_pending(&coordinator).await.question_id;
        assert_eq!(
            coordinator
                .answer(&question_id2, None, Some("nope".into()))
                .await,
            Err(AskUserError::OtherNotAllowed)
        );
        // Question remains pending after a rejected answer.
        assert_eq!(coordinator.pending().await.len(), 1);
        coordinator.cancel(&question_id2).await.unwrap();
        assert_eq!(
            ask2.await.unwrap().unwrap().status,
            AskUserStatus::Cancelled
        );
    }

    #[tokio::test]
    async fn invalid_choice_keeps_question_pending() {
        let coordinator = AskUserCoordinator::new(Duration::from_secs(30));
        let ask = spawn_request(coordinator.clone(), "Pick?", vec!["A".into()], false, None);
        let question_id = first_pending(&coordinator).await.question_id;
        assert_eq!(
            coordinator.answer(&question_id, Some(5), None).await,
            Err(AskUserError::InvalidChoice)
        );
        assert_eq!(coordinator.pending().await.len(), 1);
        coordinator.cancel(&question_id).await.unwrap();
        assert_eq!(ask.await.unwrap().unwrap().status, AskUserStatus::Cancelled);
    }

    #[tokio::test]
    async fn cancel_unblocks_with_cancelled_status() {
        let coordinator = AskUserCoordinator::new(Duration::from_secs(30));
        let ask = spawn_request(coordinator.clone(), "Proceed?", vec![], false, None);
        let question_id = first_pending(&coordinator).await.question_id;
        coordinator.cancel(&question_id).await.unwrap();
        let response = ask.await.unwrap().unwrap();
        assert_eq!(response.status, AskUserStatus::Cancelled);
        assert!(coordinator.pending().await.is_empty());
    }

    #[tokio::test]
    async fn request_expires_after_timeout() {
        let coordinator = AskUserCoordinator::new(Duration::from_millis(50));
        let ask = spawn_request(coordinator.clone(), "Hurry?", vec![], false, None);
        let question_id = first_pending(&coordinator).await.question_id;
        assert_eq!(ask.await.unwrap(), Err(AskUserError::Expired));
        assert!(coordinator.pending().await.is_empty());
        assert_eq!(
            coordinator.answer(&question_id, Some(0), None).await,
            Err(AskUserError::NotFound)
        );
    }

    #[tokio::test]
    async fn answer_unknown_question_is_not_found() {
        let coordinator = AskUserCoordinator::new(Duration::from_secs(30));
        assert_eq!(
            coordinator.answer("missing", Some(0), None).await,
            Err(AskUserError::NotFound)
        );
    }

    #[test]
    fn default_timeout_is_ten_minutes() {
        assert_eq!(AskUserCoordinator::default().timeout_secs(), 600);
    }
}
