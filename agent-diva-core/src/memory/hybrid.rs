//! Hybrid memory provider for optional Mentle-backed long-term recall.
//!
//! This module is only compiled with the `mentle` feature. The default core
//! build keeps using file-backed memory and does not depend on `memtle`.

use std::sync::Arc;

use memtle::toolkit::MemtleToolkit;
use parking_lot::RwLock;
use tokio::sync::Mutex;

use super::manager::MemoryManager;
use super::provider::{
    MemoryProvider, PrefetchRequest, PrefetchResponse, PrefetchStatus, SessionEndRequest,
    SessionEndResponse, SessionEndStatus, StartupInjectionShape, SyncTurnRequest, SyncTurnResponse,
    SyncTurnStatus, SystemPromptBlock, SystemPromptRequest, SystemPromptResponse,
};
use super::storage::Memory;

#[derive(Debug, Clone, PartialEq, Eq)]
struct PalaceStatusSnapshot {
    total_drawers: Option<i64>,
    rooms_total: Option<usize>,
    edges_total: Option<usize>,
    degraded_reason: Option<String>,
}

impl PalaceStatusSnapshot {
    async fn from_toolkit(palace_toolkit: &Arc<Mutex<MemtleToolkit>>) -> Self {
        let toolkit = palace_toolkit.lock().await;
        let status = toolkit.status().await;
        let graph_stats = toolkit.graph_stats().await;

        match (status, graph_stats) {
            (Ok(status), Ok(graph_stats)) => Self {
                total_drawers: Some(status.total_drawers),
                rooms_total: Some(graph_stats.rooms_total),
                edges_total: Some(graph_stats.edges_total),
                degraded_reason: None,
            },
            (status_result, graph_result) => {
                let mut reasons = Vec::new();
                if let Err(err) = status_result {
                    reasons.push(format!("status: {err}"));
                }
                if let Err(err) = graph_result {
                    reasons.push(format!("graph_stats: {err}"));
                }

                Self {
                    total_drawers: None,
                    rooms_total: None,
                    edges_total: None,
                    degraded_reason: Some(reasons.join("; ")),
                }
            }
        }
    }

    fn render_markdown(&self) -> String {
        let mut markdown = String::from("## Memory Palace Overview\n");

        if let Some(reason) = self.degraded_reason.as_deref() {
            markdown.push_str("- status: degraded\n");
            markdown.push_str(&format!("- reason: {reason}\n"));
            return markdown;
        }

        markdown.push_str("- status: ready\n");
        markdown.push_str(&format!(
            "- active_drawers: {}\n",
            self.total_drawers.unwrap_or_default()
        ));
        markdown.push_str(&format!(
            "- graph_rooms: {}\n",
            self.rooms_total.unwrap_or_default()
        ));
        markdown.push_str(&format!(
            "- graph_tunnels: {}\n",
            self.edges_total.unwrap_or_default()
        ));
        markdown
    }
}

#[cfg(test)]
mod snapshot_tests {
    use super::*;

    #[test]
    fn test_palace_status_snapshot_renders_ready_markdown() {
        let snapshot = PalaceStatusSnapshot {
            total_drawers: Some(7),
            rooms_total: Some(3),
            edges_total: Some(5),
            degraded_reason: None,
        };

        let markdown = snapshot.render_markdown();

        assert!(markdown.contains("## Memory Palace Overview"));
        assert!(markdown.contains("status: ready"));
        assert!(markdown.contains("active_drawers: 7"));
        assert!(markdown.contains("graph_rooms: 3"));
        assert!(markdown.contains("graph_tunnels: 5"));
    }

    #[test]
    fn test_palace_status_snapshot_renders_degraded_markdown() {
        let snapshot = PalaceStatusSnapshot {
            total_drawers: None,
            rooms_total: None,
            edges_total: None,
            degraded_reason: Some("status: unavailable".to_string()),
        };

        let markdown = snapshot.render_markdown();

        assert!(markdown.contains("status: degraded"));
        assert!(markdown.contains("reason: status: unavailable"));
        assert!(!markdown.contains("active_drawers"));
    }
}

/// Combines Agent-Diva's Markdown memory with a Mentle palace toolkit.
pub struct HybridMemoryProvider {
    file_manager: Arc<MemoryManager>,
    palace_toolkit: Arc<Mutex<MemtleToolkit>>,
    palace_snapshot: RwLock<String>,
}

impl HybridMemoryProvider {
    /// Create a hybrid provider and pre-warm the synchronous palace snapshot.
    pub async fn new(
        file_manager: Arc<MemoryManager>,
        palace_toolkit: Arc<Mutex<MemtleToolkit>>,
    ) -> Self {
        let palace_snapshot = RwLock::new(Self::fetch_palace_snapshot(&palace_toolkit).await);

        Self {
            file_manager,
            palace_toolkit,
            palace_snapshot,
        }
    }

    /// Create a hybrid provider with a caller-supplied cached snapshot.
    ///
    /// This is useful for tests and for startup paths that already fetched
    /// Mentle status before constructing the provider.
    pub fn with_snapshot(
        file_manager: Arc<MemoryManager>,
        palace_toolkit: Arc<Mutex<MemtleToolkit>>,
        palace_snapshot: impl Into<String>,
    ) -> Self {
        Self {
            file_manager,
            palace_toolkit,
            palace_snapshot: RwLock::new(palace_snapshot.into()),
        }
    }

    async fn fetch_palace_snapshot(palace_toolkit: &Arc<Mutex<MemtleToolkit>>) -> String {
        PalaceStatusSnapshot::from_toolkit(palace_toolkit)
            .await
            .render_markdown()
    }

    async fn refresh_palace_snapshot(&self) {
        let snapshot = Self::fetch_palace_snapshot(&self.palace_toolkit).await;
        *self.palace_snapshot.write() = snapshot;
    }
}

#[async_trait::async_trait]
impl MemoryProvider for HybridMemoryProvider {
    fn system_prompt_block(
        &self,
        request: &SystemPromptRequest,
    ) -> crate::Result<SystemPromptResponse> {
        let file_response = self.file_manager.system_prompt_block(request)?;
        let file_markdown = file_response
            .prompt_block
            .map(|block| block.markdown)
            .unwrap_or_default();
        let palace_markdown = self.palace_snapshot.read().clone();
        let markdown = [file_markdown.trim(), palace_markdown.trim()]
            .into_iter()
            .filter(|section| !section.is_empty())
            .collect::<Vec<_>>()
            .join("\n\n");

        if markdown.is_empty() {
            return Ok(SystemPromptResponse::degraded(
                "startup continuity unavailable; no long-term memory or palace snapshot available",
            ));
        }

        Ok(SystemPromptResponse::ready(SystemPromptBlock {
            shape: StartupInjectionShape::CompactRenderedMarkdown,
            markdown,
        }))
    }

    async fn prefetch(&self, request: PrefetchRequest) -> crate::Result<PrefetchResponse> {
        if request.intent.trim().is_empty() {
            return Ok(PrefetchResponse {
                status: PrefetchStatus::SkippedNoIntent,
                prompt_block: None,
            });
        }

        let toolkit = self.palace_toolkit.lock().await;
        let args = memtle::tools::SearchArgs {
            query: request.intent.clone(),
            limit: 5,
            wing: None,
            room: request.current_room.clone(),
            context: request.user_message.clone(),
        };

        match toolkit.search(args).await {
            Ok(output) => {
                let mut markdown = format!("## Palace Deep Recall\nQuery: {}\n", request.intent);
                if output.results.is_empty() {
                    markdown.push_str("- No deep factual memories recalled.\n");
                } else {
                    for item in output.results {
                        markdown.push_str(&format!(
                            "- [{}/{}] {}\n",
                            item.wing, item.room, item.content
                        ));
                    }
                }

                Ok(PrefetchResponse {
                    status: PrefetchStatus::Ready,
                    prompt_block: Some(markdown),
                })
            }
            Err(err) => Ok(PrefetchResponse {
                status: PrefetchStatus::Failed {
                    reason: err.to_string(),
                },
                prompt_block: None,
            }),
        }
    }

    async fn sync_turn(&self, request: SyncTurnRequest) -> crate::Result<SyncTurnResponse> {
        let mut persisted = false;
        let mut palace_persisted = false;

        if let Some(memory_update) = request.memory_update_markdown.as_deref() {
            if !memory_update.trim().is_empty() {
                self.file_manager
                    .save_memory(&Memory::with_content(memory_update))?;
                persisted = true;
            }
        }

        if let Some(history_entry) = request.history_entry.as_deref() {
            if !history_entry.trim().is_empty() {
                self.file_manager.append_history(history_entry)?;

                let toolkit = self.palace_toolkit.lock().await;
                let result = toolkit
                    .call_json(
                        "memtle_diary_write",
                        history_diary_write_args(history_entry),
                    )
                    .await;

                match result {
                    Ok(_) => {
                        palace_persisted = true;
                    }
                    Err(err) => {
                        tracing::warn!(
                            error = %err,
                            "failed to persist Mentle diary entry; HISTORY.md remains authoritative"
                        );
                    }
                }

                persisted = true;
            }
        }

        if palace_persisted {
            self.refresh_palace_snapshot().await;
        }

        Ok(SyncTurnResponse {
            status: if persisted {
                SyncTurnStatus::Persisted
            } else {
                SyncTurnStatus::Noop
            },
        })
    }

    async fn on_session_end(
        &self,
        request: SessionEndRequest,
    ) -> crate::Result<SessionEndResponse> {
        let file_response = self.file_manager.on_session_end(request).await?;
        if file_response.status == SessionEndStatus::AlreadyHandled {
            return Ok(file_response);
        }

        Ok(SessionEndResponse {
            status: SessionEndStatus::Triggered,
        })
    }
}

fn history_diary_write_args(history_entry: &str) -> serde_json::Value {
    serde_json::json!({
        "agent_name": "agent-diva",
        "entry": history_entry,
        "topic": "history",
        "wing": "history"
    })
}

#[cfg(test)]
mod tests {
    use super::history_diary_write_args;

    #[test]
    fn history_diary_write_args_match_memtle_required_schema() {
        let args = history_diary_write_args("[2026-05-24 10:00] completed a memory sync");

        assert_eq!(args["agent_name"], "agent-diva");
        assert_eq!(args["entry"], "[2026-05-24 10:00] completed a memory sync");
        assert_eq!(args["topic"], "history");
        assert_eq!(args["wing"], "history");
        assert!(args.as_object().is_some_and(|object| object.len() == 4));
    }
}
