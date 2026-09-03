use agent_diva_agent::{AgentLoop, ToolConfig};
use agent_diva_core::bus::AgentEventBus;
use agent_diva_core::channel::{
    AttachmentRef, ChannelAddress, ChannelDirection, ChannelEnvelopeV1, ChannelOrigin,
    ChannelPayloadV1, ContentPart, Correlation, OwnerTurnContextV1, OwnerTurnIntent,
};
use agent_diva_files::handle::FileMetadata;
use agent_diva_files::{FileConfig, FileManager};
use agent_diva_providers::{
    LLMProvider, LLMResponse, LLMStreamEvent, Message, MessageContent, MessageContentPart,
    ProviderEventStream, ProviderResult, ToolChoiceMode,
};
use async_trait::async_trait;
use futures::stream;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[derive(Default)]
struct CapturingStreamProvider {
    captured_messages: Mutex<Vec<Vec<Message>>>,
}

#[async_trait]
impl LLMProvider for CapturingStreamProvider {
    async fn chat(
        &self,
        messages: Vec<Message>,
        _tools: Option<Vec<serde_json::Value>>,
        _tool_choice: ToolChoiceMode,
        _model: Option<String>,
        _max_tokens: i32,
        _temperature: f64,
    ) -> ProviderResult<LLMResponse> {
        self.captured_messages.lock().unwrap().push(messages);
        Ok(LLMResponse {
            content: Some("title".to_string()),
            tool_calls: Vec::new(),
            finish_reason: "stop".to_string(),
            usage: HashMap::new(),
            reasoning_content: None,
        })
    }

    async fn chat_stream(
        &self,
        messages: Vec<Message>,
        _tools: Option<Vec<serde_json::Value>>,
        _tool_choice: ToolChoiceMode,
        _model: Option<String>,
        _max_tokens: i32,
        _temperature: f64,
    ) -> ProviderResult<ProviderEventStream> {
        self.captured_messages.lock().unwrap().push(messages);
        Ok(Box::pin(stream::iter(vec![Ok(LLMStreamEvent::Completed(
            LLMResponse {
                content: Some("done".to_string()),
                tool_calls: Vec::new(),
                finish_reason: "stop".to_string(),
                usage: HashMap::new(),
                reasoning_content: None,
            },
        ))])))
    }

    fn get_default_model(&self) -> String {
        "test-model".to_string()
    }
}

async fn build_file_manager(base: &std::path::Path) -> Arc<FileManager> {
    Arc::new(
        FileManager::new(FileConfig::with_path(base.join("files")))
            .await
            .unwrap(),
    )
}

async fn store_png(file_manager: &FileManager, name: &str) -> String {
    file_manager
        .store(
            &[0x89, b'P', b'N', b'G'],
            FileMetadata {
                name: name.to_string(),
                size: 4,
                mime_type: Some("image/png".to_string()),
                source: Some("test".to_string()),
                created_at: chrono::Utc::now(),
                last_accessed_at: None,
                preview: None,
            },
        )
        .await
        .unwrap()
        .id
}

fn image_envelope(file_id: String, chat_id: &str, content: &str) -> ChannelEnvelopeV1 {
    ChannelEnvelopeV1::new(
        ChannelDirection::Ingress,
        ChannelAddress::new("gui", chat_id),
        Correlation::new(format!("profile/{chat_id}")),
        ChannelOrigin::OwnerFrontend,
        ChannelPayloadV1::Message {
            parts: vec![
                ContentPart::Text {
                    text: content.to_string(),
                },
                ContentPart::Image {
                    attachment: AttachmentRef {
                        uri: file_id.clone(),
                        media_type: "image/png".to_string(),
                        size_bytes: 4,
                        sha256: file_id,
                        file_name: Some("diagram.png".to_string()),
                    },
                },
            ],
            subject: None,
            locale: None,
            context: Some(OwnerTurnContextV1 {
                intent: OwnerTurnIntent::Agent,
                approval_policy: None,
                execution: None,
            }),
        },
    )
}

#[tokio::test]
async fn image_attachment_is_forwarded_as_structured_multimodal_user_message() {
    let bus = AgentEventBus::new();
    let provider = Arc::new(CapturingStreamProvider::default());
    let temp_dir = tempfile::tempdir().unwrap();
    let workspace = temp_dir.path().to_path_buf();
    let file_manager = build_file_manager(temp_dir.path()).await;

    let mut agent = AgentLoop::with_tools(
        bus,
        provider.clone(),
        workspace,
        Some("gpt-4o".to_string()),
        Some(1),
        ToolConfig::default(),
        None,
        file_manager.clone(),
    )
    .await
    .unwrap();

    let file_id = store_png(&file_manager, "diagram.png").await;
    let response = agent
        .process_channel_envelope(
            image_envelope(file_id, "chat-1", "describe this image"),
            None,
        )
        .await
        .unwrap();
    assert!(
        response.is_none(),
        "OwnerFrontend image turns must project through AgentEvents without adapter egress"
    );

    let captured = provider.captured_messages.lock().unwrap();
    let first_call = captured.first().expect("provider should capture a request");
    let user_message = first_call
        .last()
        .expect("captured request should include a current user turn");

    match &user_message.content {
        MessageContent::Parts(parts) => {
            assert_eq!(parts.len(), 2);
            match &parts[0] {
                MessageContentPart::Text { text } => {
                    assert!(text.contains("describe this image"));
                    assert!(text.contains("[Image: diagram.png"));
                }
                other => panic!("expected text part first, got {other:?}"),
            }
            match &parts[1] {
                MessageContentPart::ImageUrl { image_url } => {
                    assert!(image_url.url.starts_with("data:image/png;base64,"));
                }
                other => panic!("expected image_url part, got {other:?}"),
            }
        }
        other => panic!("expected structured multimodal content, got {other:?}"),
    }
}

#[tokio::test]
async fn image_attachment_rejects_non_vision_model_before_provider_call() {
    let bus = AgentEventBus::new();
    let provider = Arc::new(CapturingStreamProvider::default());
    let temp_dir = tempfile::tempdir().unwrap();
    let workspace = temp_dir.path().to_path_buf();
    let file_manager = build_file_manager(temp_dir.path()).await;

    let mut agent = AgentLoop::with_tools(
        bus,
        provider.clone(),
        workspace,
        Some("test-model".to_string()),
        Some(1),
        ToolConfig::default(),
        None,
        file_manager.clone(),
    )
    .await
    .unwrap();

    let file_id = store_png(&file_manager, "blocked.png").await;
    let err = agent
        .process_channel_envelope(image_envelope(file_id, "chat-1", "what is this"), None)
        .await
        .unwrap_err();
    assert!(err.to_string().contains("does not support vision input"));
    assert!(provider.captured_messages.lock().unwrap().is_empty());
}
