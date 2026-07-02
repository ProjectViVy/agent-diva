//! YAML scenario types for the E2E testing framework.
//!
//! These types define the schema for loading test scenarios from YAML files,
//! supporting multi-message conversations, file setup, and rich assertions.

use serde::{Deserialize, Serialize};

/// A single E2E test scenario loaded from YAML.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct E2EScenario {
    /// Unique scenario name (used for identification in reports).
    pub scenario: String,
    /// Optional human-readable description.
    pub description: Option<String>,
    /// Setup configuration (defaults to empty).
    #[serde(default)]
    pub setup: E2ESetup,
    /// The conversation messages to send.
    pub messages: Vec<E2EMessage>,
    /// Assertions to verify against the run output.
    pub assertions: Vec<E2EAssertion>,
}

/// Scenario setup configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct E2ESetup {
    /// Files to create before the test runs (useful for file_ops scenarios).
    #[serde(default)]
    pub create_file: Vec<CreateFileEntry>,
    /// Override the working directory (default: a temporary directory).
    pub working_dir: Option<String>,
    /// Per-scenario timeout in seconds (default: 30).
    #[serde(default = "default_timeout")]
    pub timeout_secs: u64,
    /// Override the default model for this scenario.
    pub model_override: Option<String>,
}

impl Default for E2ESetup {
    fn default() -> Self {
        Self {
            create_file: vec![],
            working_dir: None,
            timeout_secs: 30,
            model_override: None,
        }
    }
}

fn default_timeout() -> u64 {
    30
}

/// An entry describing a file to create during scenario setup.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateFileEntry {
    /// Relative or absolute path for the file.
    pub path: String,
    /// Content to write into the file.
    pub content: String,
}

/// A single message in the scenario conversation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct E2EMessage {
    /// Sender identifier (e.g., "user", "system", "agent").
    pub sender: String,
    /// Message content (markdown or plain text).
    pub content: String,
    /// Optional channel identifier for multi-channel scenarios.
    pub channel: Option<String>,
    /// Optional chat/thread identifier.
    pub chat_id: Option<String>,
}

/// Assertion types for verifying scenario results.
///
/// These are serialised as a tagged YAML union via `#[serde(tag = "type")]`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum E2EAssertion {
    /// Assert that the response text contains the given substring.
    #[serde(rename = "response_contains")]
    ResponseContains {
        /// Substring to search for in the response.
        value: String,
        /// Optional description of what is being checked.
        description: Option<String>,
    },
    /// Assert that the response text matches the given regex pattern.
    #[serde(rename = "response_matches")]
    ResponseMatches {
        /// Regex pattern to match against the response.
        pattern: String,
        /// Optional description of what is being checked.
        description: Option<String>,
    },
    /// Assert that a specific tool was called at least `min_times`.
    #[serde(rename = "tool_called")]
    ToolCalled {
        /// Name of the tool that must have been called.
        name: String,
        /// Minimum number of invocations (default: 1).
        #[serde(default = "default_min_times")]
        min_times: usize,
        /// Optional description of what is being checked.
        description: Option<String>,
    },
    /// Assert that a file exists at the given path.
    #[serde(rename = "file_exists")]
    FileExists {
        /// Path to the file that should exist.
        path: String,
        /// Optional description of what is being checked.
        description: Option<String>,
    },
    /// Assert that no errors occurred during the run.
    #[serde(rename = "no_errors")]
    NoErrors {
        /// Optional description of what is being checked.
        description: Option<String>,
    },
    /// Assert that a judge evaluates the response as passing.
    #[serde(rename = "judge")]
    Judge {
        /// Description of the judge criteria.
        description: String,
    },
}

fn default_min_times() -> usize {
    1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_yaml_scenario_deserializes_correctly() {
        let yaml = r#"
scenario: test_basic_chat
description: A simple hello-world scenario
setup:
  timeout_secs: 45
messages:
  - sender: user
    content: Hello, how are you?
    channel: test
    chat_id: "123"
assertions:
  - type: response_contains
    value: doing
  - type: no_errors
"#;

        let scenario: E2EScenario = serde_yaml::from_str(yaml).expect("Failed to parse YAML scenario");

        assert_eq!(scenario.scenario, "test_basic_chat");
        assert_eq!(scenario.description.as_deref(), Some("A simple hello-world scenario"));
        assert_eq!(scenario.setup.timeout_secs, 45);
        assert_eq!(scenario.setup.create_file.len(), 0);
        assert!(scenario.setup.working_dir.is_none());
        assert!(scenario.setup.model_override.is_none());

        assert_eq!(scenario.messages.len(), 1);
        assert_eq!(scenario.messages[0].sender, "user");
        assert_eq!(scenario.messages[0].content, "Hello, how are you?");
        assert_eq!(scenario.messages[0].channel.as_deref(), Some("test"));
        assert_eq!(scenario.messages[0].chat_id.as_deref(), Some("123"));

        assert_eq!(scenario.assertions.len(), 2);
    }

    #[test]
    fn test_assertion_variants_can_be_constructed() {
        let response_contains = E2EAssertion::ResponseContains {
            value: "hello".into(),
            description: Some("greeting present".into()),
        };
        assert!(matches!(response_contains, E2EAssertion::ResponseContains { .. }));

        let response_matches = E2EAssertion::ResponseMatches {
            pattern: r"hello\s+world".into(),
            description: None,
        };
        assert!(matches!(response_matches, E2EAssertion::ResponseMatches { .. }));

        let tool_called = E2EAssertion::ToolCalled {
            name: "bash".into(),
            min_times: 2,
            description: None,
        };
        assert!(matches!(tool_called, E2EAssertion::ToolCalled { .. }));

        let file_exists = E2EAssertion::FileExists {
            path: "/tmp/test.txt".into(),
            description: None,
        };
        assert!(matches!(file_exists, E2EAssertion::FileExists { .. }));

        let no_errors = E2EAssertion::NoErrors {
            description: None,
        };
        assert!(matches!(no_errors, E2EAssertion::NoErrors { .. }));

        let judge = E2EAssertion::Judge {
            description: "response is helpful".into(),
        };
        assert!(matches!(judge, E2EAssertion::Judge { .. }));
    }

    #[test]
    fn test_assertion_variants_serialize_and_deserialize() {
        let yaml = r#"
type: tool_called
name: bash
min_times: 3
"#;
        let assertion: E2EAssertion = serde_yaml::from_str(yaml).expect("Failed to parse tool_called assertion");
        match assertion {
            E2EAssertion::ToolCalled { name, min_times, description } => {
                assert_eq!(name, "bash");
                assert_eq!(min_times, 3);
                assert!(description.is_none());
            }
            other => panic!("Expected ToolCalled, got {other:?}"),
        }
    }

    #[test]
    fn test_setup_defaults_are_correct() {
        let setup = E2ESetup::default();
        assert!(setup.create_file.is_empty());
        assert!(setup.working_dir.is_none());
        assert_eq!(setup.timeout_secs, 30);
        assert!(setup.model_override.is_none());
    }

    #[test]
    fn test_setup_deserializes_with_defaults() {
        let yaml = r#"
scenario: minimal
messages:
  - sender: user
    content: hi
assertions: []
"#;
        let scenario: E2EScenario = serde_yaml::from_str(yaml).expect("Failed to parse minimal scenario");
        assert_eq!(scenario.setup.timeout_secs, 30);
        assert!(scenario.setup.create_file.is_empty());
    }

    #[test]
    fn test_create_file_entry_deserializes() {
        let yaml = r#"
path: /tmp/test.txt
content: hello world
"#;
        let entry: CreateFileEntry = serde_yaml::from_str(yaml).expect("Failed to parse CreateFileEntry");
        assert_eq!(entry.path, "/tmp/test.txt");
        assert_eq!(entry.content, "hello world");
    }

    #[test]
    fn test_file_exists_assertion_with_minimal_fields() {
        let yaml = r#"
type: file_exists
path: output.txt
"#;
        let assertion: E2EAssertion = serde_yaml::from_str(yaml).expect("Failed to parse file_exists");
        match assertion {
            E2EAssertion::FileExists { path, description } => {
                assert_eq!(path, "output.txt");
                assert!(description.is_none());
            }
            other => panic!("Expected FileExists, got {other:?}"),
        }
    }
}
