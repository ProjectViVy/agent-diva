//! EventCollector module for capturing AgentEvent streams from the agent loop.
//!
//! This module collects `AgentEvent` streams from a tokio `mpsc::UnboundedReceiver`
//! and classifies them into structured data for assertion evaluation.
//! It is a key component of the end-to-end testing framework.

use agent_diva_core::bus::events::AgentEvent;
use std::collections::HashMap;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::time::timeout;

/// Record of a single tool call (started → optionally finished)
#[derive(Debug, Clone, Default)]
pub struct ToolCallRecord {
    pub tool_name: String,
    pub input: Option<String>,
    pub result: Option<String>,
    pub is_error: Option<bool>,
}

/// All events collected from a single agent loop invocation
#[derive(Debug, Clone, Default)]
pub struct CollectedEvents {
    pub iteration_indices: Vec<usize>,
    pub assistant_deltas: Vec<String>,
    pub reasoning_deltas: Vec<String>,
    pub tool_calls: Vec<ToolCallRecord>,
    pub final_response: Option<String>,
    pub errors: Vec<String>,
    /// Full timeline of events in order
    pub timeline: Vec<AgentEvent>,
}

/// Collects and classifies all `AgentEvent`s from a channel receiver.
///
/// # Important
///
/// `collect()` will block forever if the sender is not dropped.
/// The caller **must** drop all senders before calling this function.
pub struct EventCollector;

impl EventCollector {
    pub fn new() -> Self {
        Self
    }

    /// Collect all events from the receiver within the given timeout.
    ///
    /// Loops on `rx.recv()` until `None` is returned (all senders dropped).
    /// Classifies each event variant into the appropriate field in [`CollectedEvents`].
    /// If the timeout expires before the sender is dropped, an error is returned.
    pub async fn collect(
        &self,
        rx: &mut mpsc::UnboundedReceiver<AgentEvent>,
        timeout_duration: Duration,
    ) -> Result<CollectedEvents, String> {
        let mut collected = CollectedEvents::default();
        // Track tool calls by call_id to match started -> finished
        let mut pending_tool_calls: HashMap<String, ToolCallRecord> = HashMap::new();

        let result = timeout(timeout_duration, async {
            while let Some(event) = rx.recv().await {
                collected.timeline.push(event.clone());

                match event {
                    AgentEvent::IterationStarted {
                        index,
                        max_iterations: _,
                    } => {
                        collected.iteration_indices.push(index);
                    }
                    AgentEvent::AssistantDelta { text } => {
                        collected.assistant_deltas.push(text);
                    }
                    AgentEvent::ReasoningDelta { text } => {
                        collected.reasoning_deltas.push(text);
                    }
                    AgentEvent::ToolCallDelta {
                        name: _,
                        args_delta: _,
                    } => {
                        // Deltas are intermediate; reference is via ToolCallStarted/Finished
                    }
                    AgentEvent::ToolCallStarted {
                        name,
                        args_preview,
                        call_id,
                    } => {
                        let record = ToolCallRecord {
                            tool_name: name,
                            input: Some(args_preview),
                            result: None,
                            is_error: None,
                        };
                        pending_tool_calls.insert(call_id, record);
                    }
                    AgentEvent::ToolCallFinished {
                        name: _,
                        result,
                        is_error,
                        call_id,
                    } => {
                        if let Some(mut record) = pending_tool_calls.remove(&call_id) {
                            record.result = Some(result);
                            record.is_error = Some(is_error);
                            collected.tool_calls.push(record);
                        } else {
                            // Orphan ToolCallFinished (no matching started) – create partial record
                            collected.tool_calls.push(ToolCallRecord {
                                tool_name: String::new(),
                                input: None,
                                result: Some(result),
                                is_error: Some(is_error),
                            });
                        }
                    }
                    AgentEvent::FinalResponse { content } => {
                        collected.final_response = Some(content);
                    }
                    AgentEvent::Error { message } => {
                        collected.errors.push(message);
                    }
                }
            }
        })
        .await;

        match result {
            Ok(()) => {
                // Flush any orphan ToolCallStarted records (no matching ToolCallFinished)
                for (_, record) in pending_tool_calls.drain() {
                    collected.tool_calls.push(record);
                }
                Ok(collected)
            }
            Err(_elapsed) => {
                Err("EventCollector timed out: the sender was not dropped within the expected duration. Ensure all senders are dropped before calling collect().".to_string())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::mpsc;

    /// Build a (tx, rx) pair for testing
    fn make_channel() -> (mpsc::UnboundedSender<AgentEvent>, mpsc::UnboundedReceiver<AgentEvent>) {
        mpsc::unbounded_channel()
    }

    // -----------------------------------------------------------------------
    // Test 1: Collect all 8 event variants
    // -----------------------------------------------------------------------
    #[tokio::test]
    async fn test_collect_all_variants() {
        let (tx, mut rx) = make_channel();
        let collector = EventCollector::new();

        // Send each variant
        tx.send(AgentEvent::IterationStarted {
            index: 0,
            max_iterations: 3,
        })
        .expect("send IterationStarted");

        tx.send(AgentEvent::AssistantDelta {
            text: "Hello".to_string(),
        })
        .expect("send AssistantDelta");

        tx.send(AgentEvent::ReasoningDelta {
            text: "Let me think...".to_string(),
        })
        .expect("send ReasoningDelta");

        tx.send(AgentEvent::ToolCallDelta {
            name: Some("read_file".to_string()),
            args_delta: r#"{"path":"/tmp/x"}"#.to_string(),
        })
        .expect("send ToolCallDelta");

        tx.send(AgentEvent::ToolCallStarted {
            name: "read_file".to_string(),
            args_preview: r#"{"path":"/tmp/x"}"#.to_string(),
            call_id: "call-1".to_string(),
        })
        .expect("send ToolCallStarted");

        tx.send(AgentEvent::ToolCallFinished {
            name: "read_file".to_string(),
            result: "file content".to_string(),
            is_error: false,
            call_id: "call-1".to_string(),
        })
        .expect("send ToolCallFinished");

        tx.send(AgentEvent::FinalResponse {
            content: "Here is the file content.".to_string(),
        })
        .expect("send FinalResponse");

        tx.send(AgentEvent::Error {
            message: "Something went wrong".to_string(),
        })
        .expect("send Error");

        // Drop sender so recv() returns None
        drop(tx);

        let collected = collector
            .collect(&mut rx, Duration::from_secs(1))
            .await
            .expect("collect should succeed");

        // Verify all fields
        assert_eq!(collected.iteration_indices, vec![0]);
        assert_eq!(collected.assistant_deltas, vec!["Hello"]);
        assert_eq!(collected.reasoning_deltas, vec!["Let me think..."]);
        assert_eq!(collected.tool_calls.len(), 1);
        assert_eq!(collected.tool_calls[0].tool_name, "read_file");
        assert_eq!(
            collected.tool_calls[0].input.as_deref(),
            Some(r#"{"path":"/tmp/x"}"#)
        );
        assert_eq!(
            collected.tool_calls[0].result.as_deref(),
            Some("file content")
        );
        assert_eq!(collected.tool_calls[0].is_error, Some(false));
        assert_eq!(
            collected.final_response.as_deref(),
            Some("Here is the file content.")
        );
        assert_eq!(collected.errors, vec!["Something went wrong"]);

        // Timeline should have 8 events (we sent 8, none filtered)
        assert_eq!(collected.timeline.len(), 8);
    }

    // -----------------------------------------------------------------------
    // Test 2: Sender drop causes collect to complete quickly
    // -----------------------------------------------------------------------
    #[tokio::test]
    async fn test_collect_sender_drop() {
        let (tx, mut rx) = make_channel();
        let collector = EventCollector::new();

        tx.send(AgentEvent::FinalResponse {
            content: "done".to_string(),
        })
        .expect("send FinalResponse");

        // Drop sender – collect should return immediately
        drop(tx);

        let collected = collector
            .collect(&mut rx, Duration::from_millis(500))
            .await
            .expect("collect should succeed after sender drop");

        assert_eq!(collected.final_response.as_deref(), Some("done"));
    }

    // -----------------------------------------------------------------------
    // Test 3: Timeout when sender is not dropped
    // -----------------------------------------------------------------------
    #[tokio::test]
    async fn test_collect_timeout() {
        let (tx, mut rx) = make_channel();
        let collector = EventCollector::new();

        tx.send(AgentEvent::IterationStarted {
            index: 0,
            max_iterations: 5,
        })
        .expect("send IterationStarted");

        // Do NOT drop tx – collect should time out
        let result = collector
            .collect(&mut rx, Duration::from_millis(50))
            .await;

        assert!(result.is_err(), "expected timeout error");
        let err = result.unwrap_err();
        assert!(
            err.contains("timed out"),
            "error should mention timeout: {err}"
        );

        // Drop sender to clean up for the test
        drop(tx);
    }

    // -----------------------------------------------------------------------
    // Test 4: Tool call tracking (started + finished with matching call_id)
    // -----------------------------------------------------------------------
    #[tokio::test]
    async fn test_tool_call_tracking() {
        let (tx, mut rx) = make_channel();
        let collector = EventCollector::new();

        tx.send(AgentEvent::ToolCallStarted {
            name: "search_web".to_string(),
            args_preview: r#"{"query":"rust async"}"#.to_string(),
            call_id: "call-42".to_string(),
        })
        .expect("send ToolCallStarted");

        tx.send(AgentEvent::ToolCallFinished {
            name: "search_web".to_string(),
            result: "3 results found".to_string(),
            is_error: false,
            call_id: "call-42".to_string(),
        })
        .expect("send ToolCallFinished");

        drop(tx);

        let collected = collector
            .collect(&mut rx, Duration::from_secs(1))
            .await
            .expect("collect should succeed");

        assert_eq!(collected.tool_calls.len(), 1);
        let record = &collected.tool_calls[0];
        assert_eq!(record.tool_name, "search_web");
        assert_eq!(record.input.as_deref(), Some(r#"{"query":"rust async"}"#));
        assert_eq!(record.result.as_deref(), Some("3 results found"));
        assert_eq!(record.is_error, Some(false));
    }

    // -----------------------------------------------------------------------
    // Test 5: Orphan ToolCallStarted (no matching ToolCallFinished)
    // -----------------------------------------------------------------------
    #[tokio::test]
    async fn test_orphan_tool_call() {
        let (tx, mut rx) = make_channel();
        let collector = EventCollector::new();

        tx.send(AgentEvent::ToolCallStarted {
            name: "run_shell".to_string(),
            args_preview: "echo hello".to_string(),
            call_id: "orphan-1".to_string(),
        })
        .expect("send ToolCallStarted");

        // No corresponding ToolCallFinished
        drop(tx);

        let collected = collector
            .collect(&mut rx, Duration::from_secs(1))
            .await
            .expect("collect should succeed");

        // Orphan tool call should appear with no result
        assert_eq!(collected.tool_calls.len(), 1);
        let record = &collected.tool_calls[0];
        assert_eq!(record.tool_name, "run_shell");
        assert_eq!(record.input.as_deref(), Some("echo hello"));
        assert!(record.result.is_none());
        assert!(record.is_error.is_none());
    }
}
