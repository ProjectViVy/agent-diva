//! Ollama tool calling integration tests

use agent_diva_providers::base::{LLMProvider, Message};
use agent_diva_providers::ollama::OllamaProvider;
use serde_json::json;

#[test]
fn test_serialize_tool_call() {
    // Test that tool definitions serialize correctly
    let tool_def = json!({
        "type": "function",
        "function": {
            "name": "get_weather",
            "description": "Get weather for a location",
            "parameters": {
                "type": "object",
                "properties": {
                    "location": {
                        "type": "string",
                        "description": "City name"
                    }
                },
                "required": ["location"]
            }
        }
    });

    // Verify serialization
    let serialized = serde_json::to_string(&tool_def).unwrap();
    assert!(serialized.contains("get_weather"));
    assert!(serialized.contains("function"));
}

#[tokio::test]
async fn test_tool_calling_basic() {
    if std::env::var("OLLAMA_TEST_SKIP").unwrap_or_default() == "true" {
        eprintln!("Skipping Ollama tool calling test (OLLAMA_TEST_SKIP=true)");
        return;
    }

    let provider = OllamaProvider::new(None, "llama3.2".to_string());

    let messages = vec![Message {
        role: "user".to_string(),
        content: "What's the weather in Beijing?".to_string(),
        name: None,
        tool_call_id: None,
        tool_calls: None,
        reasoning_content: None,
        thinking_blocks: None,
    }];

    let tools = vec![json!({
        "type": "function",
        "function": {
            "name": "get_weather",
            "description": "Get current weather for a location",
            "parameters": {
                "type": "object",
                "properties": {
                    "location": {
                        "type": "string",
                        "description": "City name"
                    },
                    "unit": {
                        "type": "string",
                        "enum": ["celsius", "fahrenheit"]
                    }
                },
                "required": ["location"]
            }
        }
    })];

    let result = provider.chat(messages, Some(tools), None, 100, 0.7).await;

    match result {
        Ok(response) => {
            // Response should either have content or tool calls
            if !response.tool_calls.is_empty() {
                assert_eq!(response.tool_calls[0].name, "get_weather");
                assert!(response.tool_calls[0].arguments.contains_key("location"));
            } else {
                assert!(response.content.is_some());
            }
        }
        Err(e) => {
            eprintln!(
                "Test skipped (Ollama may not be running or model not available): {}",
                e
            );
        }
    }
}
