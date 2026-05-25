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
    total_drawers: i64,
    rooms_total: usize,
    edges_total: usize,
}

impl PalaceStatusSnapshot {
    async fn from_toolkit(palace_toolkit: &Arc<Mutex<MemtleToolkit>>) -> Result<Self, String> {
        let toolkit = palace_toolkit.lock().await;
        let status = toolkit.status().await;
        let graph_stats = toolkit.graph_stats().await;

        match (status, graph_stats) {
            (Ok(status), Ok(graph_stats)) => Ok(Self {
                total_drawers: status.total_drawers,
                rooms_total: graph_stats.rooms_total,
                edges_total: graph_stats.edges_total,
            }),
            (status_result, graph_result) => {
                let mut reasons = Vec::new();
                if let Err(err) = status_result {
                    reasons.push(format!("status: {err}"));
                }
                if let Err(err) = graph_result {
                    reasons.push(format!("graph_stats: {err}"));
                }

                Err(reasons.join("; "))
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CachedPalaceSnapshot {
    state: PalaceSnapshotState,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum PalaceSnapshotState {
    Ready(PalaceStatusSnapshot),
    Stale {
        snapshot: PalaceStatusSnapshot,
        last_refresh_error: String,
    },
    Degraded {
        reason: String,
    },
}

impl CachedPalaceSnapshot {
    fn ready(snapshot: PalaceStatusSnapshot) -> Self {
        Self {
            state: PalaceSnapshotState::Ready(snapshot),
        }
    }

    fn degraded(reason: impl Into<String>) -> Self {
        Self {
            state: PalaceSnapshotState::Degraded {
                reason: reason.into(),
            },
        }
    }

    fn from_startup_result(result: Result<PalaceStatusSnapshot, String>) -> Self {
        match result {
            Ok(snapshot) => Self::ready(snapshot),
            Err(reason) => Self::degraded(reason),
        }
    }

    fn refresh_with_result(&mut self, result: Result<PalaceStatusSnapshot, String>) {
        match result {
            Ok(snapshot) => {
                self.state = PalaceSnapshotState::Ready(snapshot);
            }
            Err(last_refresh_error) => {
                let previous_good_snapshot = match &self.state {
                    PalaceSnapshotState::Ready(snapshot)
                    | PalaceSnapshotState::Stale { snapshot, .. } => Some(snapshot.clone()),
                    PalaceSnapshotState::Degraded { .. } => None,
                };

                self.state = match previous_good_snapshot {
                    Some(snapshot) => PalaceSnapshotState::Stale {
                        snapshot,
                        last_refresh_error,
                    },
                    None => PalaceSnapshotState::Degraded {
                        reason: last_refresh_error,
                    },
                }
            }
        }
    }

    fn has_usable_snapshot(&self) -> bool {
        matches!(
            self.state,
            PalaceSnapshotState::Ready(_) | PalaceSnapshotState::Stale { .. }
        )
    }

    fn render_markdown(&self) -> String {
        let mut markdown = String::from("## Memory Palace Overview\n");

        match &self.state {
            PalaceSnapshotState::Ready(snapshot) => {
                markdown.push_str("- snapshot_status: ready\n");
                Self::render_counts(&mut markdown, snapshot);
            }
            PalaceSnapshotState::Stale {
                snapshot,
                last_refresh_error,
            } => {
                markdown.push_str("- snapshot_status: stale\n");
                Self::render_counts(&mut markdown, snapshot);
                markdown.push_str(&format!("- last_refresh_error: {last_refresh_error}\n"));
            }
            PalaceSnapshotState::Degraded { reason } => {
                markdown.push_str("- snapshot_status: degraded\n");
                markdown.push_str(&format!("- reason: {reason}\n"));
            }
        }

        markdown
    }

    fn render_counts(markdown: &mut String, snapshot: &PalaceStatusSnapshot) {
        markdown.push_str(&format!("- active_drawers: {}\n", snapshot.total_drawers));
        markdown.push_str(&format!("- graph_rooms: {}\n", snapshot.rooms_total));
        markdown.push_str(&format!("- graph_tunnels: {}\n", snapshot.edges_total));
    }
}

#[cfg(test)]
mod snapshot_tests {
    use super::*;

    #[test]
    fn test_palace_status_snapshot_renders_ready_markdown() {
        let snapshot = CachedPalaceSnapshot::ready(PalaceStatusSnapshot {
            total_drawers: 7,
            rooms_total: 3,
            edges_total: 5,
        });

        let markdown = snapshot.render_markdown();

        assert!(markdown.contains("## Memory Palace Overview"));
        assert!(markdown.contains("snapshot_status: ready"));
        assert!(markdown.contains("active_drawers: 7"));
        assert!(markdown.contains("graph_rooms: 3"));
        assert!(markdown.contains("graph_tunnels: 5"));
    }

    #[test]
    fn test_cached_palace_snapshot_renders_degraded_markdown() {
        let snapshot = CachedPalaceSnapshot::degraded("status: unavailable");

        let markdown = snapshot.render_markdown();

        assert!(markdown.contains("snapshot_status: degraded"));
        assert!(markdown.contains("reason: status: unavailable"));
        assert!(!markdown.contains("active_drawers"));
    }

    #[test]
    fn test_cached_palace_snapshot_renders_stale_markdown_with_counts() {
        let mut snapshot = CachedPalaceSnapshot::ready(PalaceStatusSnapshot {
            total_drawers: 7,
            rooms_total: 3,
            edges_total: 5,
        });

        snapshot.refresh_with_result(Err("status: database busy".to_string()));
        let markdown = snapshot.render_markdown();

        assert!(markdown.contains("snapshot_status: stale"));
        assert!(markdown.contains("active_drawers: 7"));
        assert!(markdown.contains("graph_rooms: 3"));
        assert!(markdown.contains("graph_tunnels: 5"));
        assert!(markdown.contains("last_refresh_error: status: database busy"));
    }

    #[test]
    fn test_cached_palace_snapshot_startup_success_creates_ready_state() {
        let snapshot = CachedPalaceSnapshot::from_startup_result(Ok(PalaceStatusSnapshot {
            total_drawers: 2,
            rooms_total: 4,
            edges_total: 6,
        }));

        assert!(matches!(snapshot.state, PalaceSnapshotState::Ready(_)));
    }

    #[test]
    fn test_cached_palace_snapshot_startup_failure_creates_degraded_state() {
        let snapshot =
            CachedPalaceSnapshot::from_startup_result(Err("status: offline".to_string()));

        assert!(matches!(
            snapshot.state,
            PalaceSnapshotState::Degraded { ref reason } if reason == "status: offline"
        ));
    }

    #[test]
    fn test_cached_palace_snapshot_refresh_success_replaces_degraded_state() {
        let mut snapshot = CachedPalaceSnapshot::degraded("status: offline");

        snapshot.refresh_with_result(Ok(PalaceStatusSnapshot {
            total_drawers: 8,
            rooms_total: 9,
            edges_total: 10,
        }));

        assert!(matches!(
            snapshot.state,
            PalaceSnapshotState::Ready(PalaceStatusSnapshot {
                total_drawers: 8,
                rooms_total: 9,
                edges_total: 10
            })
        ));
    }

    #[test]
    fn test_cached_palace_snapshot_refresh_failure_after_ready_keeps_last_good_data() {
        let mut snapshot = CachedPalaceSnapshot::ready(PalaceStatusSnapshot {
            total_drawers: 11,
            rooms_total: 12,
            edges_total: 13,
        });

        snapshot.refresh_with_result(Err("graph_stats: unavailable".to_string()));

        assert!(matches!(
            snapshot.state,
            PalaceSnapshotState::Stale {
                snapshot: PalaceStatusSnapshot {
                    total_drawers: 11,
                    rooms_total: 12,
                    edges_total: 13
                },
                ref last_refresh_error,
            } if last_refresh_error == "graph_stats: unavailable"
        ));
    }

    #[test]
    fn test_cached_palace_snapshot_refresh_failure_without_last_good_data_stays_degraded() {
        let mut snapshot = CachedPalaceSnapshot::degraded("status: offline");

        snapshot.refresh_with_result(Err("graph_stats: unavailable".to_string()));

        assert!(matches!(
            snapshot.state,
            PalaceSnapshotState::Degraded { ref reason } if reason == "graph_stats: unavailable"
        ));
    }
}

/// Combines Agent-Diva's Markdown memory with a Mentle palace toolkit.
pub struct HybridMemoryProvider {
    file_manager: Arc<MemoryManager>,
    palace_toolkit: Arc<Mutex<MemtleToolkit>>,
    palace_snapshot: RwLock<CachedPalaceSnapshot>,
}

impl HybridMemoryProvider {
    /// Create a hybrid provider and pre-warm the synchronous palace snapshot.
    pub async fn new(
        file_manager: Arc<MemoryManager>,
        palace_toolkit: Arc<Mutex<MemtleToolkit>>,
    ) -> Self {
        let palace_snapshot = RwLock::new(CachedPalaceSnapshot::from_startup_result(
            Self::fetch_palace_snapshot(&palace_toolkit).await,
        ));

        Self {
            file_manager,
            palace_toolkit,
            palace_snapshot,
        }
    }

    #[cfg(test)]
    fn with_cached_snapshot(
        file_manager: Arc<MemoryManager>,
        palace_toolkit: Arc<Mutex<MemtleToolkit>>,
        palace_snapshot: CachedPalaceSnapshot,
    ) -> Self {
        Self {
            file_manager,
            palace_toolkit,
            palace_snapshot: RwLock::new(palace_snapshot),
        }
    }

    async fn fetch_palace_snapshot(
        palace_toolkit: &Arc<Mutex<MemtleToolkit>>,
    ) -> Result<PalaceStatusSnapshot, String> {
        PalaceStatusSnapshot::from_toolkit(palace_toolkit).await
    }

    async fn refresh_palace_snapshot(&self) {
        let snapshot = Self::fetch_palace_snapshot(&self.palace_toolkit).await;
        self.palace_snapshot.write().refresh_with_result(snapshot);
    }
}

#[async_trait::async_trait]
impl MemoryProvider for HybridMemoryProvider {
    fn system_prompt_block(
        &self,
        request: &SystemPromptRequest,
    ) -> crate::Result<SystemPromptResponse> {
        let file_response = self.file_manager.system_prompt_block(request)?;
        let file_available = file_response.status == super::provider::StartupStatus::Ready;
        let file_markdown = if file_available {
            file_response
                .prompt_block
                .map(|block| block.markdown)
                .unwrap_or_default()
        } else {
            String::new()
        };
        let palace_snapshot = self.palace_snapshot.read();
        let palace_available = palace_snapshot.has_usable_snapshot();
        let palace_markdown = palace_snapshot.render_markdown();
        let markdown = [file_markdown.trim(), palace_markdown.trim()]
            .into_iter()
            .filter(|section| !section.is_empty())
            .collect::<Vec<_>>()
            .join("\n\n");

        if !file_available && !palace_available {
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
                if let Err(err) = self
                    .file_manager
                    .save_memory(&Memory::with_content(memory_update))
                {
                    return Ok(SyncTurnResponse {
                        status: SyncTurnStatus::Failed {
                            reason: format!("failed to persist MEMORY.md: {err}"),
                        },
                    });
                }
                persisted = true;
            }
        }

        if let Some(history_entry) = request.history_entry.as_deref() {
            if !history_entry.trim().is_empty() {
                if let Err(err) = self.file_manager.append_history(history_entry) {
                    return Ok(SyncTurnResponse {
                        status: SyncTurnStatus::Failed {
                            reason: format!("failed to append HISTORY.md: {err}"),
                        },
                    });
                }

                let toolkit = self.palace_toolkit.lock().await;
                let result = toolkit
                    .call_json(
                        "memtle_diary_write",
                        history_diary_write_args(history_entry),
                    )
                    .await;

                match result {
                    Ok(value) if diary_write_succeeded(&value) => {
                        palace_persisted = true;
                    }
                    Ok(value) => {
                        let error = value
                            .get("error")
                            .and_then(serde_json::Value::as_str)
                            .unwrap_or("memtle_diary_write returned an unsuccessful payload");
                        tracing::warn!(
                            error = %error,
                            result = %value,
                            "failed to persist Mentle diary entry; HISTORY.md remains authoritative"
                        );
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

fn diary_write_succeeded(result: &serde_json::Value) -> bool {
    if result.get("success").and_then(serde_json::Value::as_bool) == Some(false) {
        return false;
    }

    if result
        .get("error")
        .and_then(serde_json::Value::as_str)
        .is_some()
    {
        return false;
    }

    result.get("success").and_then(serde_json::Value::as_bool) == Some(true)
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use memtle::toolkit::MemtleToolkit;
    use memtle::tools::AddDrawerArgs;
    use tempfile::TempDir;
    use tokio::sync::Mutex;

    use super::{
        diary_write_succeeded, history_diary_write_args, CachedPalaceSnapshot,
        HybridMemoryProvider, PalaceStatusSnapshot,
    };
    use crate::memory::{
        Memory, MemoryManager, MemoryProvider, PrefetchRequest, PrefetchStatus, SessionEndRequest,
        SessionEndStatus, StartupStatus, SyncTurnRequest, SyncTurnStatus, SystemPromptRequest,
    };

    async fn open_toolkit(temp_dir: &TempDir) -> Arc<Mutex<MemtleToolkit>> {
        Arc::new(Mutex::new(
            MemtleToolkit::open(temp_dir.path().join("palace.db"))
                .await
                .expect("test toolkit should open"),
        ))
    }

    fn ready_snapshot(total_drawers: i64) -> CachedPalaceSnapshot {
        CachedPalaceSnapshot::ready(PalaceStatusSnapshot {
            total_drawers,
            rooms_total: 0,
            edges_total: 0,
        })
    }

    #[test]
    fn history_diary_write_args_match_memtle_required_schema() {
        let args = history_diary_write_args("[2026-05-24 10:00] completed a memory sync");

        assert_eq!(args["agent_name"], "agent-diva");
        assert_eq!(args["entry"], "[2026-05-24 10:00] completed a memory sync");
        assert_eq!(args["topic"], "history");
        assert_eq!(args["wing"], "history");
        assert!(args.as_object().is_some_and(|object| object.len() == 4));
    }

    #[test]
    fn diary_write_succeeded_requires_success_true_without_error() {
        assert!(diary_write_succeeded(&serde_json::json!({
            "success": true,
            "entry_id": "entry-1"
        })));
        assert!(!diary_write_succeeded(&serde_json::json!({
            "success": false,
            "error": "content exceeds maximum length of 100,000 characters"
        })));
        assert!(!diary_write_succeeded(&serde_json::json!({
            "error": "content exceeds maximum length of 100,000 characters",
            "public": true
        })));
        assert!(!diary_write_succeeded(&serde_json::json!({})));
        assert!(!diary_write_succeeded(&serde_json::json!({
            "entry_id": "entry-1"
        })));
    }

    #[tokio::test]
    async fn hybrid_system_prompt_returns_degraded_when_file_and_snapshot_are_empty() {
        let temp_dir = TempDir::new().unwrap();
        let provider = HybridMemoryProvider::with_cached_snapshot(
            Arc::new(MemoryManager::new(temp_dir.path())),
            open_toolkit(&temp_dir).await,
            CachedPalaceSnapshot::degraded("startup fetch failed"),
        );

        let response = provider
            .system_prompt_block(&SystemPromptRequest {
                workspace_root: temp_dir.path().to_path_buf(),
            })
            .unwrap();

        match response.status {
            StartupStatus::Degraded { reason, .. } => {
                assert!(reason.contains("startup continuity unavailable"));
            }
            other => panic!("expected degraded startup, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn hybrid_system_prompt_includes_degraded_snapshot_when_file_memory_exists() {
        let temp_dir = TempDir::new().unwrap();
        let file_manager = Arc::new(MemoryManager::new(temp_dir.path()));
        file_manager
            .save_memory(&Memory::with_content("File memory remains authoritative."))
            .unwrap();
        let provider = HybridMemoryProvider::with_cached_snapshot(
            file_manager,
            open_toolkit(&temp_dir).await,
            CachedPalaceSnapshot::degraded("startup fetch failed"),
        );

        let response = provider
            .system_prompt_block(&SystemPromptRequest {
                workspace_root: temp_dir.path().to_path_buf(),
            })
            .unwrap();

        assert_eq!(response.status, StartupStatus::Ready);
        let prompt = response
            .prompt_block
            .expect("file memory should produce a startup block");
        assert!(prompt
            .markdown
            .contains("File memory remains authoritative."));
        assert!(prompt.markdown.contains("snapshot_status: degraded"));
        assert!(prompt.markdown.contains("reason: startup fetch failed"));
    }

    #[tokio::test]
    async fn hybrid_prefetch_skips_blank_intent_without_querying_palace() {
        let temp_dir = TempDir::new().unwrap();
        let provider = HybridMemoryProvider::with_cached_snapshot(
            Arc::new(MemoryManager::new(temp_dir.path())),
            open_toolkit(&temp_dir).await,
            ready_snapshot(0),
        );

        let response = provider
            .prefetch(PrefetchRequest {
                workspace_root: temp_dir.path().to_path_buf(),
                intent: "   ".to_string(),
                current_room: None,
                user_message: Some("hello".to_string()),
            })
            .await
            .unwrap();

        assert_eq!(response.status, PrefetchStatus::SkippedNoIntent);
        assert!(response.prompt_block.is_none());
    }

    #[tokio::test]
    async fn hybrid_prefetch_returns_palace_search_results() {
        let temp_dir = TempDir::new().unwrap();
        let toolkit = open_toolkit(&temp_dir).await;
        {
            let toolkit = toolkit.lock().await;
            toolkit
                .add_drawer(AddDrawerArgs {
                    wing: "project".to_string(),
                    room: "roadmap".to_string(),
                    content: "Provider boundary work keeps startup prompt assembly synchronous."
                        .to_string(),
                    source_file: Some("s2-a5.md".to_string()),
                    added_by: Some("agent-diva-test".to_string()),
                })
                .await
                .expect("test drawer should be inserted");
        }

        let provider = HybridMemoryProvider::with_cached_snapshot(
            Arc::new(MemoryManager::new(temp_dir.path())),
            toolkit,
            ready_snapshot(1),
        );

        let response = provider
            .prefetch(PrefetchRequest {
                workspace_root: temp_dir.path().to_path_buf(),
                intent: "provider boundary startup prompt".to_string(),
                current_room: Some("roadmap".to_string()),
                user_message: Some("What did we decide about the provider boundary?".to_string()),
            })
            .await
            .unwrap();

        assert_eq!(response.status, PrefetchStatus::Ready);
        let block = response
            .prompt_block
            .expect("search hit should render a recall block");
        assert!(block.contains("## Palace Deep Recall"));
        assert!(block.contains("Query: provider boundary startup prompt"));
        assert!(block.contains("[project/roadmap]"));
        assert!(block.contains("startup prompt assembly synchronous"));
    }

    #[tokio::test]
    async fn hybrid_prefetch_empty_results_still_returns_ready_block() {
        let temp_dir = TempDir::new().unwrap();
        let provider = HybridMemoryProvider::with_cached_snapshot(
            Arc::new(MemoryManager::new(temp_dir.path())),
            open_toolkit(&temp_dir).await,
            ready_snapshot(0),
        );

        let response = provider
            .prefetch(PrefetchRequest {
                workspace_root: temp_dir.path().to_path_buf(),
                intent: "unlikely unmatched query zzzzz".to_string(),
                current_room: None,
                user_message: Some("Search for something absent".to_string()),
            })
            .await
            .unwrap();

        assert_eq!(response.status, PrefetchStatus::Ready);
        let block = response
            .prompt_block
            .expect("empty successful search should still render a recall block");
        assert!(block.contains("No deep factual memories recalled."));
    }

    #[tokio::test]
    async fn hybrid_prefetch_invalid_room_is_recoverable_failure() {
        let temp_dir = TempDir::new().unwrap();
        let provider = HybridMemoryProvider::with_cached_snapshot(
            Arc::new(MemoryManager::new(temp_dir.path())),
            open_toolkit(&temp_dir).await,
            ready_snapshot(0),
        );

        let response = provider
            .prefetch(PrefetchRequest {
                workspace_root: temp_dir.path().to_path_buf(),
                intent: "provider boundary".to_string(),
                current_room: Some("../bad-room".to_string()),
                user_message: Some("Search with bad room".to_string()),
            })
            .await
            .unwrap();

        match response.status {
            PrefetchStatus::Failed { reason } => {
                assert!(reason.contains("room contains invalid characters"));
            }
            other => panic!("expected recoverable prefetch failure, got {other:?}"),
        }
        assert!(response.prompt_block.is_none());
    }

    #[tokio::test]
    async fn hybrid_sync_turn_surfaces_memory_file_failure_as_status() {
        let temp_dir = TempDir::new().unwrap();
        let workspace = temp_dir.path().to_path_buf();
        std::fs::write(workspace.join("memory"), "not a directory").unwrap();

        let provider = HybridMemoryProvider::with_cached_snapshot(
            Arc::new(MemoryManager::new(&workspace)),
            open_toolkit(&temp_dir).await,
            ready_snapshot(0),
        );

        let response = provider
            .sync_turn(SyncTurnRequest {
                workspace_root: workspace,
                memory_update_markdown: Some("cannot persist".to_string()),
                history_entry: None,
            })
            .await
            .unwrap();

        match response.status {
            SyncTurnStatus::Failed { reason } => {
                assert!(reason.contains("failed to persist MEMORY.md"));
            }
            other => panic!("expected sync failure, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn hybrid_sync_turn_surfaces_history_file_failure_as_status() {
        let temp_dir = TempDir::new().unwrap();
        let workspace = temp_dir.path().to_path_buf();
        std::fs::write(workspace.join("memory"), "not a directory").unwrap();

        let provider = HybridMemoryProvider::with_cached_snapshot(
            Arc::new(MemoryManager::new(&workspace)),
            open_toolkit(&temp_dir).await,
            ready_snapshot(0),
        );

        let response = provider
            .sync_turn(SyncTurnRequest {
                workspace_root: workspace,
                memory_update_markdown: None,
                history_entry: Some("[2026-05-24 12:00 UTC] failed append".to_string()),
            })
            .await
            .unwrap();

        match response.status {
            SyncTurnStatus::Failed { reason } => {
                assert!(reason.contains("failed to append HISTORY.md"));
            }
            other => panic!("expected sync failure, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn hybrid_sync_turn_refreshes_snapshot_after_diary_write() {
        let temp_dir = TempDir::new().unwrap();
        let provider = HybridMemoryProvider::with_cached_snapshot(
            Arc::new(MemoryManager::new(temp_dir.path())),
            open_toolkit(&temp_dir).await,
            ready_snapshot(0),
        );

        let response = provider
            .sync_turn(SyncTurnRequest {
                workspace_root: temp_dir.path().to_path_buf(),
                memory_update_markdown: None,
                history_entry: Some("[2026-05-24 12:00 UTC] persisted palace diary".to_string()),
            })
            .await
            .unwrap();

        assert_eq!(response.status, SyncTurnStatus::Persisted);
        assert!(MemoryManager::new(temp_dir.path())
            .load_history()
            .contains("persisted palace diary"));

        let prompt = provider
            .system_prompt_block(&SystemPromptRequest {
                workspace_root: temp_dir.path().to_path_buf(),
            })
            .unwrap()
            .prompt_block
            .expect("hybrid provider should return refreshed snapshot");

        assert!(prompt.markdown.contains("## Memory Palace Overview"));
        assert!(prompt.markdown.contains("active_drawers: 1"));
    }

    #[tokio::test]
    async fn hybrid_sync_turn_keeps_snapshot_when_diary_write_returns_tool_failure() {
        let temp_dir = TempDir::new().unwrap();
        let provider = HybridMemoryProvider::with_cached_snapshot(
            Arc::new(MemoryManager::new(temp_dir.path())),
            open_toolkit(&temp_dir).await,
            ready_snapshot(42),
        );
        let history_prefix = "[2026-05-24 12:00 UTC] rejected palace diary ";
        let history_entry = format!("{history_prefix}{}", "x".repeat(100_001));

        let response = provider
            .sync_turn(SyncTurnRequest {
                workspace_root: temp_dir.path().to_path_buf(),
                memory_update_markdown: None,
                history_entry: Some(history_entry),
            })
            .await
            .unwrap();

        assert_eq!(response.status, SyncTurnStatus::Persisted);
        assert!(MemoryManager::new(temp_dir.path())
            .load_history()
            .contains(history_prefix));

        let prompt = provider
            .system_prompt_block(&SystemPromptRequest {
                workspace_root: temp_dir.path().to_path_buf(),
            })
            .unwrap()
            .prompt_block
            .expect("hybrid provider should keep cached snapshot");

        assert!(prompt.markdown.contains("active_drawers: 42"));
    }

    #[tokio::test]
    async fn hybrid_sync_turn_noop_keeps_cached_snapshot() {
        let temp_dir = TempDir::new().unwrap();
        let provider = HybridMemoryProvider::with_cached_snapshot(
            Arc::new(MemoryManager::new(temp_dir.path())),
            open_toolkit(&temp_dir).await,
            ready_snapshot(42),
        );

        let response = provider
            .sync_turn(SyncTurnRequest {
                workspace_root: temp_dir.path().to_path_buf(),
                memory_update_markdown: Some("   ".to_string()),
                history_entry: Some("   ".to_string()),
            })
            .await
            .unwrap();

        assert_eq!(response.status, SyncTurnStatus::Noop);

        let prompt = provider
            .system_prompt_block(&SystemPromptRequest {
                workspace_root: temp_dir.path().to_path_buf(),
            })
            .unwrap()
            .prompt_block
            .expect("hybrid provider should keep cached snapshot");

        assert!(prompt.markdown.contains("active_drawers: 42"));
    }

    #[tokio::test]
    async fn hybrid_session_end_remains_idempotent_via_file_provider() {
        let temp_dir = TempDir::new().unwrap();
        let provider = HybridMemoryProvider::with_cached_snapshot(
            Arc::new(MemoryManager::new(temp_dir.path())),
            open_toolkit(&temp_dir).await,
            ready_snapshot(0),
        );

        let first = provider
            .on_session_end(SessionEndRequest {
                workspace_root: temp_dir.path().to_path_buf(),
                session_id: Some("session-dup".to_string()),
            })
            .await
            .unwrap();
        assert_eq!(first.status, SessionEndStatus::Triggered);

        let duplicate = provider
            .on_session_end(SessionEndRequest {
                workspace_root: temp_dir.path().to_path_buf(),
                session_id: Some("session-dup".to_string()),
            })
            .await
            .unwrap();
        assert_eq!(duplicate.status, SessionEndStatus::AlreadyHandled);
    }
}
