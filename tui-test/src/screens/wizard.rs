//! Setup Wizard: step-by-step configuration for first-time users.
//!
//! Steps: Welcome -> Provider -> API Key -> Model -> Channel -> Agent -> Summary -> Complete
//! All data is placeholder for demonstration.

use crate::i18n::{TranslationKey, Translator};
use crate::theme;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::{Alignment, Constraint, Flex, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Padding, Paragraph};
use ratatui::Frame;
use std::collections::HashSet;

// ── Step Enum ──────────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum WizardStep {
    Welcome,        // Step 0: Introduction
    ProviderSelect, // Step 1: Choose LLM provider
    ApiKeySetup,    // Step 2: Enter API key
    ModelSelect,    // Step 3: Select models to enable
    ChannelSetup,   // Step 4: Configure at least one channel
    AgentCreate,    // Step 5: Create initial agent
    Summary,        // Step 6: Review configuration
    Complete,       // Step 7: Finish and enter main
}

impl WizardStep {
    pub fn next(self) -> Self {
        match self {
            WizardStep::Welcome => WizardStep::ProviderSelect,
            WizardStep::ProviderSelect => WizardStep::ApiKeySetup,
            WizardStep::ApiKeySetup => WizardStep::ModelSelect,
            WizardStep::ModelSelect => WizardStep::ChannelSetup,
            WizardStep::ChannelSetup => WizardStep::AgentCreate,
            WizardStep::AgentCreate => WizardStep::Summary,
            WizardStep::Summary => WizardStep::Complete,
            WizardStep::Complete => WizardStep::Complete,
        }
    }

    pub fn prev(self) -> Self {
        match self {
            WizardStep::Welcome => WizardStep::Welcome,
            WizardStep::ProviderSelect => WizardStep::Welcome,
            WizardStep::ApiKeySetup => WizardStep::ProviderSelect,
            WizardStep::ModelSelect => WizardStep::ApiKeySetup,
            WizardStep::ChannelSetup => WizardStep::ModelSelect,
            WizardStep::AgentCreate => WizardStep::ChannelSetup,
            WizardStep::Summary => WizardStep::AgentCreate,
            WizardStep::Complete => WizardStep::Summary,
        }
    }

    pub fn index(self) -> usize {
        match self {
            WizardStep::Welcome => 0,
            WizardStep::ProviderSelect => 1,
            WizardStep::ApiKeySetup => 2,
            WizardStep::ModelSelect => 3,
            WizardStep::ChannelSetup => 4,
            WizardStep::AgentCreate => 5,
            WizardStep::Summary => 6,
            WizardStep::Complete => 7,
        }
    }

    pub fn title(self, i18n: &Translator) -> &'static str {
        match self {
            WizardStep::Welcome => i18n.t(TranslationKey::WizardStepWelcome),
            WizardStep::ProviderSelect => i18n.t(TranslationKey::WizardStepProvider),
            WizardStep::ApiKeySetup => i18n.t(TranslationKey::WizardStepApiKey),
            WizardStep::ModelSelect => i18n.t(TranslationKey::WizardStepModel),
            WizardStep::ChannelSetup => i18n.t(TranslationKey::WizardStepChannel),
            WizardStep::AgentCreate => i18n.t(TranslationKey::WizardStepAgent),
            WizardStep::Summary => i18n.t(TranslationKey::WizardStepSummary),
            WizardStep::Complete => i18n.t(TranslationKey::WizardStepComplete),
        }
    }
}

// ── Data Types (placeholder) ────────────────────────────────────────────────

#[derive(Clone)]
pub struct ProviderInfo {
    pub name: String,
    pub display_name: String,
    pub status: ProviderStatus,
    pub api_key_set: bool,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ProviderStatus {
    Active,
    NeedKey,
    Local,
    Disabled,
}

#[derive(Clone)]
pub struct ModelInfo {
    pub id: String,
    pub display_name: String,
    pub provider: String,
    pub context_window: u32,
    pub cost_input: f64,
    pub cost_output: f64,
    pub enabled: bool,
}

#[derive(Clone)]
pub struct ChannelInfo {
    pub name: String,
    pub display_name: String,
    pub category: String,
    pub env_required: Vec<String>,
    pub configured: bool,
}

#[derive(Clone)]
pub struct AgentTemplate {
    pub name: String,
    pub description: String,
}

// ── State ───────────────────────────────────────────────────────────────────

pub struct WizardState {
    pub step: WizardStep,
    pub list_state: ListState,
    pub tick: usize,

    // Step 1: Provider
    pub providers: Vec<ProviderInfo>,
    pub selected_provider: Option<usize>,

    // Step 2: API Key
    pub api_key_input: String,
    pub api_key_visible: bool,
    pub key_test_result: Option<(bool, String)>,

    // Step 3: Models
    pub models: Vec<ModelInfo>,
    pub selected_models: HashSet<usize>,
    pub model_cursor: usize,

    // Step 4: Channel
    pub channels: Vec<ChannelInfo>,
    pub selected_channel: Option<usize>,
    pub channel_env_inputs: Vec<(String, String)>, // (var_name, value)
    pub channel_env_cursor: usize,
    pub channel_input_active: bool,
    pub channel_input_buffer: String,

    // Step 5: Agent
    pub agent_name: String,
    pub agent_templates: Vec<AgentTemplate>,
    pub agent_template_idx: Option<usize>,
    pub agent_custom_mode: bool,

    // Status
    pub status_msg: String,
    pub skip_mode: bool, // Track if user skipped provider setup
}

pub enum WizardAction {
    Continue,
    Back,
    Skip,
    Finish,
    Exit,
}

impl WizardState {
    pub fn new() -> Self {
        Self {
            step: WizardStep::Welcome,
            list_state: ListState::default().with_selected(Some(0)),
            tick: 0,
            providers: placeholder_providers(),
            selected_provider: None,
            api_key_input: String::new(),
            api_key_visible: false,
            key_test_result: None,
            models: placeholder_models(),
            selected_models: HashSet::new(),
            model_cursor: 0,
            channels: placeholder_channels(),
            selected_channel: None,
            channel_env_inputs: Vec::new(),
            channel_env_cursor: 0,
            channel_input_active: false,
            channel_input_buffer: String::new(),
            agent_name: String::new(),
            agent_templates: placeholder_agent_templates(),
            agent_template_idx: Some(0),
            agent_custom_mode: false,
            status_msg: String::new(),
            skip_mode: false,
        }
    }

    pub fn reset(&mut self) {
        self.step = WizardStep::Welcome;
        self.list_state.select(Some(0));
        self.selected_provider = None;
        self.api_key_input.clear();
        self.api_key_visible = false;
        self.key_test_result = None;
        self.selected_models.clear();
        self.model_cursor = 0;
        self.selected_channel = None;
        self.channel_env_inputs.clear();
        self.channel_env_cursor = 0;
        self.channel_input_active = false;
        self.channel_input_buffer.clear();
        self.agent_name.clear();
        self.agent_template_idx = Some(0);
        self.agent_custom_mode = false;
        self.status_msg.clear();
        self.skip_mode = false;
    }

    pub fn tick(&mut self) {
        self.tick = self.tick.wrapping_add(1);
    }

    pub fn handle_key(&mut self, key: KeyEvent, _i18n: &Translator) -> WizardAction {
        // Global: Ctrl+C to exit
        if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
            return WizardAction::Exit;
        }

        match self.step {
            WizardStep::Welcome => self.handle_welcome(key),
            WizardStep::ProviderSelect => self.handle_provider_select(key),
            WizardStep::ApiKeySetup => self.handle_api_key(key),
            WizardStep::ModelSelect => self.handle_model_select(key),
            WizardStep::ChannelSetup => self.handle_channel_setup(key),
            WizardStep::AgentCreate => self.handle_agent_create(key),
            WizardStep::Summary => self.handle_summary(key),
            WizardStep::Complete => self.handle_complete(key),
        }
    }

    fn handle_welcome(&mut self, key: KeyEvent) -> WizardAction {
        match key.code {
            KeyCode::Enter => {
                self.step = WizardStep::ProviderSelect;
                self.list_state.select(Some(0));
                WizardAction::Continue
            }
            KeyCode::Esc => WizardAction::Exit,
            _ => WizardAction::Continue,
        }
    }

    fn handle_provider_select(&mut self, key: KeyEvent) -> WizardAction {
        let total = self.providers.len();
        match key.code {
            KeyCode::Esc => {
                self.step = WizardStep::Welcome;
                WizardAction::Back
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if total > 0 {
                    let i = self.list_state.selected().unwrap_or(0);
                    let next = if i == 0 { total - 1 } else { i - 1 };
                    self.list_state.select(Some(next));
                }
                WizardAction::Continue
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if total > 0 {
                    let i = self.list_state.selected().unwrap_or(0);
                    let next = (i + 1) % total;
                    self.list_state.select(Some(next));
                }
                WizardAction::Continue
            }
            KeyCode::Enter => {
                if let Some(idx) = self.list_state.selected() {
                    self.selected_provider = Some(idx);
                    let provider = &self.providers[idx];
                    self.models = placeholder_models_for_provider(&provider.name);
                    self.selected_models.clear();
                    self.model_cursor = 0;
                    self.api_key_input.clear();
                    self.key_test_result = None;
                    self.step = WizardStep::ApiKeySetup;
                }
                WizardAction::Continue
            }
            KeyCode::Char('s') => {
                // Skip provider setup
                self.skip_mode = true;
                self.selected_provider = None;
                self.models = placeholder_models();
                self.selected_models.clear();
                self.model_cursor = 0;
                self.step = WizardStep::ChannelSetup;
                self.status_msg = "Skipped provider setup (placeholder)".to_string();
                WizardAction::Skip
            }
            _ => WizardAction::Continue,
        }
    }

    fn handle_api_key(&mut self, key: KeyEvent) -> WizardAction {
        match key.code {
            KeyCode::Esc => {
                self.step = WizardStep::ProviderSelect;
                WizardAction::Back
            }
            KeyCode::Char('v') => {
                self.api_key_visible = !self.api_key_visible;
                WizardAction::Continue
            }
            KeyCode::Char('t') => {
                if self.api_key_input.is_empty() {
                    self.key_test_result = Some((false, "Please enter an API key first".to_string()));
                } else {
                    self.key_test_result = Some((true, "Connection successful! (placeholder)".to_string()));
                }
                WizardAction::Continue
            }
            KeyCode::Char('s') => {
                self.step = WizardStep::ModelSelect;
                self.model_cursor = 0;
                WizardAction::Skip
            }
            KeyCode::Char(c) => {
                self.api_key_input.push(c);
                WizardAction::Continue
            }
            KeyCode::Backspace => {
                self.api_key_input.pop();
                WizardAction::Continue
            }
            KeyCode::Enter => {
                if let Some(idx) = self.selected_provider {
                    self.providers[idx].api_key_set = true;
                    self.providers[idx].status = ProviderStatus::Active;
                }
                self.step = WizardStep::ModelSelect;
                self.model_cursor = 0;
                WizardAction::Continue
            }
            _ => WizardAction::Continue,
        }
    }

    fn handle_model_select(&mut self, key: KeyEvent) -> WizardAction {
        let total = self.models.len();
        match key.code {
            KeyCode::Esc => {
                self.step = WizardStep::ApiKeySetup;
                WizardAction::Back
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if total > 0 && self.model_cursor > 0 {
                    self.model_cursor -= 1;
                }
                WizardAction::Continue
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if total > 0 && self.model_cursor < total - 1 {
                    self.model_cursor += 1;
                }
                WizardAction::Continue
            }
            KeyCode::Char(' ') => {
                // Toggle selection
                self.selected_models.insert(self.model_cursor);
                WizardAction::Continue
            }
            KeyCode::Char('x') => {
                // Remove from selection
                self.selected_models.remove(&self.model_cursor);
                WizardAction::Continue
            }
            KeyCode::Char('a') => {
                // Select all
                for i in 0..total {
                    self.selected_models.insert(i);
                }
                WizardAction::Continue
            }
            KeyCode::Char('n') => {
                // Select none
                self.selected_models.clear();
                WizardAction::Continue
            }
            KeyCode::Enter => {
                // At least select first model if nothing selected
                if self.selected_models.is_empty() && total > 0 {
                    self.selected_models.insert(0);
                }
                self.step = WizardStep::ChannelSetup;
                self.list_state.select(Some(0));
                WizardAction::Continue
            }
            KeyCode::Char('s') => {
                self.step = WizardStep::ChannelSetup;
                self.list_state.select(Some(0));
                WizardAction::Skip
            }
            _ => WizardAction::Continue,
        }
    }

    fn handle_channel_setup(&mut self, key: KeyEvent) -> WizardAction {
        let total = self.channels.len();
        if self.channel_input_active {
            match key.code {
                KeyCode::Esc => {
                    self.channel_input_active = false;
                    self.channel_input_buffer.clear();
                }
                KeyCode::Enter => {
                    if !self.channel_input_buffer.is_empty() {
                        if let Some(idx) = self.selected_channel {
                            let env_vars = &self.channels[idx].env_required;
                            if self.channel_env_cursor < env_vars.len() {
                                let var_name = env_vars[self.channel_env_cursor].clone();
                                self.channel_env_inputs.push((var_name, self.channel_input_buffer.clone()));
                                self.channel_env_cursor += 1;
                                self.channel_input_buffer.clear();
                                // Check if all env vars are filled
                                if self.channel_env_cursor >= env_vars.len() {
                                    self.channel_input_active = false;
                                    self.channels[idx].configured = true;
                                    self.status_msg = "Channel configured! (placeholder)".to_string();
                                }
                            }
                        }
                    }
                }
                KeyCode::Char(c) => self.channel_input_buffer.push(c),
                KeyCode::Backspace => { self.channel_input_buffer.pop(); }
                _ => {}
            }
            return WizardAction::Continue;
        }

        match key.code {
            KeyCode::Esc => {
                if self.skip_mode {
                    self.step = WizardStep::Welcome;
                } else {
                    self.step = WizardStep::ModelSelect;
                }
                WizardAction::Back
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if total > 0 {
                    let i = self.list_state.selected().unwrap_or(0);
                    let next = if i == 0 { total - 1 } else { i - 1 };
                    self.list_state.select(Some(next));
                }
                WizardAction::Continue
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if total > 0 {
                    let i = self.list_state.selected().unwrap_or(0);
                    let next = (i + 1) % total;
                    self.list_state.select(Some(next));
                }
                WizardAction::Continue
            }
            KeyCode::Enter => {
                if let Some(idx) = self.list_state.selected() {
                    self.selected_channel = Some(idx);
                    self.channel_env_cursor = 0;
                    self.channel_env_inputs.clear();
                    if !self.channels[idx].env_required.is_empty() {
                        self.channel_input_active = true;
                        self.channel_input_buffer.clear();
                    } else {
                        self.channels[idx].configured = true;
                        self.step = WizardStep::AgentCreate;
                        self.list_state.select(Some(0));
                    }
                }
                WizardAction::Continue
            }
            KeyCode::Char('s') => {
                self.step = WizardStep::AgentCreate;
                self.list_state.select(Some(0));
                WizardAction::Skip
            }
            _ => WizardAction::Continue,
        }
    }

    fn handle_agent_create(&mut self, key: KeyEvent) -> WizardAction {
        let total = self.agent_templates.len();
        match key.code {
            KeyCode::Esc => {
                self.step = WizardStep::ChannelSetup;
                WizardAction::Back
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if total > 0 {
                    let i = self.agent_template_idx.unwrap_or(0);
                    let next = if i == 0 { total - 1 } else { i - 1 };
                    self.agent_template_idx = Some(next);
                }
                WizardAction::Continue
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if total > 0 {
                    let i = self.agent_template_idx.unwrap_or(0);
                    let next = (i + 1) % total;
                    self.agent_template_idx = Some(next);
                }
                WizardAction::Continue
            }
            KeyCode::Char('c') => {
                // Custom mode
                self.agent_custom_mode = true;
                self.agent_name.clear();
                WizardAction::Continue
            }
            KeyCode::Char(c) if self.agent_custom_mode => {
                self.agent_name.push(c);
                WizardAction::Continue
            }
            KeyCode::Backspace if self.agent_custom_mode => {
                self.agent_name.pop();
                WizardAction::Continue
            }
            KeyCode::Enter => {
                if self.agent_custom_mode && self.agent_name.is_empty() {
                    self.status_msg = "Please enter an agent name".to_string();
                    return WizardAction::Continue;
                }
                if self.agent_name.is_empty() {
                    // Use template name
                    if let Some(idx) = self.agent_template_idx {
                        self.agent_name = self.agent_templates[idx].name.clone();
                    }
                }
                self.step = WizardStep::Summary;
                WizardAction::Continue
            }
            KeyCode::Char('s') => {
                self.agent_name = "DefaultAgent".to_string();
                self.step = WizardStep::Summary;
                WizardAction::Skip
            }
            _ => WizardAction::Continue,
        }
    }

    fn handle_summary(&mut self, key: KeyEvent) -> WizardAction {
        match key.code {
            KeyCode::Esc => {
                self.step = WizardStep::AgentCreate;
                WizardAction::Back
            }
            KeyCode::Enter => {
                self.step = WizardStep::Complete;
                WizardAction::Continue
            }
            KeyCode::Char('b') => {
                self.step = WizardStep::AgentCreate;
                WizardAction::Back
            }
            _ => WizardAction::Continue,
        }
    }

    fn handle_complete(&mut self, key: KeyEvent) -> WizardAction {
        match key.code {
            KeyCode::Enter => WizardAction::Finish,
            KeyCode::Esc => {
                self.step = WizardStep::Summary;
                WizardAction::Back
            }
            _ => WizardAction::Continue,
        }
    }
}

// ── Placeholder Data ────────────────────────────────────────────────────────

fn placeholder_providers() -> Vec<ProviderInfo> {
    vec![
        ProviderInfo {
            name: "openrouter".to_string(),
            display_name: "OpenRouter".to_string(),
            status: ProviderStatus::Active,
            api_key_set: true,
        },
        ProviderInfo {
            name: "anthropic".to_string(),
            display_name: "Anthropic".to_string(),
            status: ProviderStatus::NeedKey,
            api_key_set: false,
        },
        ProviderInfo {
            name: "openai".to_string(),
            display_name: "OpenAI".to_string(),
            status: ProviderStatus::NeedKey,
            api_key_set: false,
        },
        ProviderInfo {
            name: "deepseek".to_string(),
            display_name: "DeepSeek".to_string(),
            status: ProviderStatus::NeedKey,
            api_key_set: false,
        },
        ProviderInfo {
            name: "groq".to_string(),
            display_name: "Groq".to_string(),
            status: ProviderStatus::NeedKey,
            api_key_set: false,
        },
        ProviderInfo {
            name: "google".to_string(),
            display_name: "Google Gemini".to_string(),
            status: ProviderStatus::NeedKey,
            api_key_set: false,
        },
        ProviderInfo {
            name: "ollama".to_string(),
            display_name: "Ollama (Local)".to_string(),
            status: ProviderStatus::Local,
            api_key_set: false,
        },
    ]
}

fn placeholder_models() -> Vec<ModelInfo> {
    vec![
        ModelInfo { id: "claude-3-5-sonnet".to_string(), display_name: "Claude 3.5 Sonnet".to_string(), provider: "anthropic".to_string(), context_window: 200_000, cost_input: 3.0, cost_output: 15.0, enabled: false },
        ModelInfo { id: "claude-3-opus".to_string(), display_name: "Claude 3 Opus".to_string(), provider: "anthropic".to_string(), context_window: 200_000, cost_input: 15.0, cost_output: 75.0, enabled: false },
        ModelInfo { id: "gpt-4o".to_string(), display_name: "GPT-4o".to_string(), provider: "openai".to_string(), context_window: 128_000, cost_input: 5.0, cost_output: 15.0, enabled: false },
        ModelInfo { id: "gpt-4o-mini".to_string(), display_name: "GPT-4o Mini".to_string(), provider: "openai".to_string(), context_window: 128_000, cost_input: 0.15, cost_output: 0.6, enabled: false },
        ModelInfo { id: "deepseek-chat".to_string(), display_name: "DeepSeek Chat".to_string(), provider: "deepseek".to_string(), context_window: 64_000, cost_input: 0.14, cost_output: 0.28, enabled: false },
        ModelInfo { id: "llama-3.3-70b".to_string(), display_name: "Llama 3.3 70B".to_string(), provider: "groq".to_string(), context_window: 128_000, cost_input: 0.0, cost_output: 0.0, enabled: false },
        ModelInfo { id: "gemini-2-flash".to_string(), display_name: "Gemini 2 Flash".to_string(), provider: "google".to_string(), context_window: 1_000_000, cost_input: 0.0, cost_output: 0.0, enabled: false },
    ]
}

fn placeholder_models_for_provider(provider: &str) -> Vec<ModelInfo> {
    placeholder_models()
        .into_iter()
        .filter(|m| m.provider == provider || provider == "openrouter")
        .collect()
}

fn placeholder_channels() -> Vec<ChannelInfo> {
    vec![
        ChannelInfo { name: "telegram".to_string(), display_name: "Telegram".to_string(), category: "Messaging".to_string(), env_required: vec!["TELEGRAM_BOT_TOKEN".to_string()], configured: false },
        ChannelInfo { name: "discord".to_string(), display_name: "Discord".to_string(), category: "Messaging".to_string(), env_required: vec!["DISCORD_BOT_TOKEN".to_string()], configured: false },
        ChannelInfo { name: "slack".to_string(), display_name: "Slack".to_string(), category: "Messaging".to_string(), env_required: vec!["SLACK_APP_TOKEN".to_string(), "SLACK_BOT_TOKEN".to_string()], configured: false },
        ChannelInfo { name: "email".to_string(), display_name: "Email".to_string(), category: "Messaging".to_string(), env_required: vec!["EMAIL_PASSWORD".to_string()], configured: false },
        ChannelInfo { name: "feishu".to_string(), display_name: "Feishu/Lark".to_string(), category: "Enterprise".to_string(), env_required: vec!["FEISHU_APP_SECRET".to_string()], configured: false },
        ChannelInfo { name: "dingtalk".to_string(), display_name: "DingTalk".to_string(), category: "Enterprise".to_string(), env_required: vec!["DINGTALK_ACCESS_TOKEN".to_string()], configured: false },
        ChannelInfo { name: "webhook".to_string(), display_name: "Webhook".to_string(), category: "Notifications".to_string(), env_required: vec![], configured: true },
    ]
}

fn placeholder_agent_templates() -> Vec<AgentTemplate> {
    vec![
        AgentTemplate { name: "Coder".to_string(), description: "Code assistant for programming tasks".to_string() },
        AgentTemplate { name: "Researcher".to_string(), description: "Web research and information gathering".to_string() },
        AgentTemplate { name: "Writer".to_string(), description: "Content creation and editing".to_string() },
        AgentTemplate { name: "Analyst".to_string(), description: "Data analysis and reporting".to_string() },
    ]
}

// ── Drawing ─────────────────────────────────────────────────────────────────

pub fn draw(f: &mut Frame, area: Rect, state: &mut WizardState, i18n: &Translator) {
    // Fill background
    f.render_widget(
        Block::default().style(Style::default().bg(theme::BG_PRIMARY)),
        area,
    );

    match state.step {
        WizardStep::Welcome => draw_welcome_step(f, area, state, i18n),
        WizardStep::ProviderSelect => draw_provider_step(f, area, state, i18n),
        WizardStep::ApiKeySetup => draw_api_key_step(f, area, state, i18n),
        WizardStep::ModelSelect => draw_model_step(f, area, state, i18n),
        WizardStep::ChannelSetup => draw_channel_step(f, area, state, i18n),
        WizardStep::AgentCreate => draw_agent_step(f, area, state, i18n),
        WizardStep::Summary => draw_summary_step(f, area, state, i18n),
        WizardStep::Complete => draw_complete_step(f, area, state, i18n),
    }
}

fn draw_welcome_step(f: &mut Frame, area: Rect, _state: &WizardState, i18n: &Translator) {
    let card_h = 16u16.min(area.height);
    let card_w = 70u16.min(area.width.saturating_sub(2));
    let [card_area] = Layout::horizontal([Constraint::Length(card_w)])
        .flex(Flex::Center)
        .areas(area);
    let [card_area] = Layout::vertical([Constraint::Length(card_h)])
        .flex(Flex::Center)
        .areas(card_area);

    let block = Block::default()
        .title(Line::from(vec![Span::styled(
            format!(" {} ", i18n.t(TranslationKey::WizardTitle)),
            theme::title_style(),
        )]))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme::ACCENT))
        .padding(Padding::uniform(1));
    let inner = block.inner(card_area);
    f.render_widget(block, card_area);

    let chunks = Layout::vertical([
        Constraint::Length(2),
        Constraint::Length(3),
        Constraint::Length(2),
        Constraint::Length(4),
        Constraint::Length(1),
    ])
    .split(inner);

    // Welcome message
    f.render_widget(
        Paragraph::new(Line::from(vec![Span::styled(
            i18n.t(TranslationKey::WizardWelcomeMsg),
            Style::default().fg(theme::TEXT_PRIMARY).add_modifier(Modifier::BOLD),
        )]))
        .alignment(Alignment::Center),
        chunks[0],
    );

    // Description
    f.render_widget(
        Paragraph::new(Line::from(vec![Span::styled(
            i18n.t(TranslationKey::WizardWelcomeDesc),
            theme::dim_style(),
        )]))
        .alignment(Alignment::Center),
        chunks[1],
    );

    // Steps overview
    let steps_text = i18n.t(TranslationKey::WizardWelcomeSteps);
    f.render_widget(
        Paragraph::new(Line::from(vec![Span::styled(
            steps_text,
            Style::default().fg(theme::CYAN),
        )]))
        .alignment(Alignment::Center),
        chunks[2],
    );

    // Placeholder note
    f.render_widget(
        Paragraph::new(Line::from(vec![Span::styled(
            i18n.t(TranslationKey::WizardPlaceholderNote),
            Style::default().fg(theme::YELLOW),
        )]))
        .alignment(Alignment::Center),
        chunks[3],
    );

    // Hints
    f.render_widget(
        Paragraph::new(Line::from(vec![Span::styled(
            i18n.t(TranslationKey::WizardHintStart),
            theme::hint_style(),
        )]))
        .alignment(Alignment::Center),
        chunks[4],
    );
}

fn draw_provider_step(f: &mut Frame, area: Rect, state: &mut WizardState, i18n: &Translator) {
    let step_idx = WizardStep::ProviderSelect.index();
    let total_steps = 7;
    draw_step_header(f, area, step_idx, total_steps, state, i18n);

    let content_area = Rect {
        x: area.x + 2,
        y: area.y + 3,
        width: area.width.saturating_sub(4),
        height: area.height.saturating_sub(5),
    };

    let chunks = Layout::vertical([
        Constraint::Length(2), // prompt
        Constraint::Min(3),    // list
        Constraint::Length(1), // hints
    ])
    .split(content_area);

    // Prompt
    f.render_widget(
        Paragraph::new(Line::from(vec![Span::styled(
            i18n.t(TranslationKey::WizardProviderPrompt),
            Style::default().fg(theme::TEXT_PRIMARY),
        )])),
        chunks[0],
    );

    // Provider list
    let items: Vec<ListItem> = state
        .providers
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let (badge, badge_style) = match p.status {
                ProviderStatus::Active => ("\u{2714} Active", Style::default().fg(theme::GREEN)),
                ProviderStatus::NeedKey => ("\u{2718} Need Key", Style::default().fg(theme::YELLOW)),
                ProviderStatus::Local => ("\u{25cf} Local", Style::default().fg(theme::CYAN)),
                ProviderStatus::Disabled => ("\u{25cb} Disabled", Style::default().fg(theme::TEXT_TERTIARY)),
            };
            let highlight = if i == state.list_state.selected().unwrap_or(0) {
                Style::default().fg(theme::ACCENT).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme::TEXT_PRIMARY)
            };
            ListItem::new(Line::from(vec![
                Span::styled(format!("  {:<20}", p.display_name), highlight),
                Span::styled(format!(" {:<12}", badge), badge_style),
            ]))
        })
        .collect();

    let list = List::new(items)
        .highlight_style(theme::selected_style())
        .highlight_symbol("> ");
    f.render_stateful_widget(list, chunks[1], &mut state.list_state);

    // Hints
    f.render_widget(
        Paragraph::new(Line::from(vec![Span::styled(
            i18n.t(TranslationKey::WizardProviderHint),
            theme::hint_style(),
        )])),
        chunks[2],
    );
}

fn draw_api_key_step(f: &mut Frame, area: Rect, state: &mut WizardState, i18n: &Translator) {
    let step_idx = WizardStep::ApiKeySetup.index();
    let total_steps = 7;
    draw_step_header(f, area, step_idx, total_steps, state, i18n);

    let content_area = Rect {
        x: area.x + 2,
        y: area.y + 3,
        width: area.width.saturating_sub(4),
        height: area.height.saturating_sub(5),
    };

    let provider_name = state
        .selected_provider
        .and_then(|i| state.providers.get(i))
        .map(|p| p.display_name.as_str())
        .unwrap_or("?");

    let chunks = Layout::vertical([
        Constraint::Length(2), // provider info
        Constraint::Length(2), // input prompt
        Constraint::Length(1), // input field
        Constraint::Length(2), // test result
        Constraint::Length(1), // hints
    ])
    .split(content_area);

    // Provider info
    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(i18n.t(TranslationKey::WizardApiKeyProvider), Style::default().fg(theme::TEXT_PRIMARY)),
            Span::styled(provider_name, Style::default().fg(theme::CYAN).add_modifier(Modifier::BOLD)),
        ])),
        chunks[0],
    );

    // Input prompt
    f.render_widget(
        Paragraph::new(Line::from(vec![Span::styled(
            i18n.t(TranslationKey::WizardApiKeyPrompt),
            Style::default().fg(theme::TEXT_PRIMARY),
        )])),
        chunks[1],
    );

    // Input field
    let display_key = if state.api_key_visible {
        &state.api_key_input
    } else {
        &state.api_key_input.chars().map(|_| '\u{25cf}').collect::<String>()
    };
    let cursor = "\u{2588}";
    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::raw("  > "),
            Span::styled(display_key, theme::input_style()),
            Span::styled(cursor, Style::default().fg(theme::GREEN).add_modifier(Modifier::SLOW_BLINK)),
        ])),
        chunks[2],
    );

    // Test result
    if let Some((success, msg)) = &state.key_test_result {
        let icon = if *success { "\u{2714}" } else { "\u{2718}" };
        let color = if *success { theme::GREEN } else { theme::RED };
        f.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled(format!("  {icon} "), Style::default().fg(color)),
                Span::styled(msg, Style::default().fg(theme::TEXT_PRIMARY)),
            ])),
            chunks[3],
        );
    }

    // Hints
    f.render_widget(
        Paragraph::new(Line::from(vec![Span::styled(
            i18n.t(TranslationKey::WizardApiKeyHint),
            theme::hint_style(),
        )])),
        chunks[4],
    );
}

fn draw_model_step(f: &mut Frame, area: Rect, state: &mut WizardState, i18n: &Translator) {
    let step_idx = WizardStep::ModelSelect.index();
    let total_steps = 7;
    draw_step_header(f, area, step_idx, total_steps, state, i18n);

    let content_area = Rect {
        x: area.x + 2,
        y: area.y + 3,
        width: area.width.saturating_sub(4),
        height: area.height.saturating_sub(5),
    };

    let chunks = Layout::vertical([
        Constraint::Length(2), // header
        Constraint::Min(3),    // list
        Constraint::Length(1), // count
        Constraint::Length(1), // hints
    ])
    .split(content_area);

    // Header
    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(i18n.t(TranslationKey::WizardModelPrompt), Style::default().fg(theme::TEXT_PRIMARY)),
            Span::styled(format!(" ({})", state.models.len()), theme::dim_style()),
        ])),
        chunks[0],
    );

    // Model list
    let items: Vec<ListItem> = state
        .models
        .iter()
        .enumerate()
        .map(|(i, m)| {
            let check = if state.selected_models.contains(&i) {
                "\u{25c9}"
            } else {
                "\u{25cb}"
            };
            let highlight = if i == state.model_cursor {
                Style::default().fg(theme::ACCENT).add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            let ctx = if m.context_window >= 1000 {
                format!("{:.0}K", m.context_window as f64 / 1000.0)
            } else {
                m.context_window.to_string()
            };
            ListItem::new(Line::from(vec![
                Span::styled(format!("  {check} {:<20}", truncate(&m.display_name, 19)), highlight),
                Span::styled(format!("{:<8} ", truncate(&m.provider, 7)), Style::default().fg(theme::PURPLE)),
                Span::styled(format!("ctx:{:<8} ", ctx), Style::default().fg(theme::TEXT_SECONDARY)),
            ]))
        })
        .collect();

    f.render_widget(List::new(items), chunks[1]);

    // Selection count
    let count = state.selected_models.len();
    f.render_widget(
        Paragraph::new(Line::from(vec![Span::styled(
            format!("  {} model(s) selected", count),
            Style::default().fg(theme::GREEN),
        )])),
        chunks[2],
    );

    // Hints
    f.render_widget(
        Paragraph::new(Line::from(vec![Span::styled(
            i18n.t(TranslationKey::WizardModelHint),
            theme::hint_style(),
        )])),
        chunks[3],
    );
}

fn draw_channel_step(f: &mut Frame, area: Rect, state: &mut WizardState, i18n: &Translator) {
    let step_idx = WizardStep::ChannelSetup.index();
    let total_steps = 7;
    draw_step_header(f, area, step_idx, total_steps, state, i18n);

    let content_area = Rect {
        x: area.x + 2,
        y: area.y + 3,
        width: area.width.saturating_sub(4),
        height: area.height.saturating_sub(5),
    };

    // If input is active, show the input modal
    if state.channel_input_active {
        draw_channel_input(f, content_area, state, i18n);
        return;
    }

    let chunks = Layout::vertical([
        Constraint::Length(2), // prompt
        Constraint::Min(3),    // list
        Constraint::Length(1), // hints
    ])
    .split(content_area);

    // Prompt
    f.render_widget(
        Paragraph::new(Line::from(vec![Span::styled(
            i18n.t(TranslationKey::WizardChannelPrompt),
            Style::default().fg(theme::TEXT_PRIMARY),
        )])),
        chunks[0],
    );

    // Channel list
    let items: Vec<ListItem> = state
        .channels
        .iter()
        .enumerate()
        .map(|(i, c)| {
            let badge = if c.configured {
                "\u{2714} Ready"
            } else if c.env_required.is_empty() {
                "\u{25cf} No config"
            } else {
                "\u{2718} Need setup"
            };
            let badge_style = if c.configured {
                Style::default().fg(theme::GREEN)
            } else if c.env_required.is_empty() {
                Style::default().fg(theme::CYAN)
            } else {
                Style::default().fg(theme::YELLOW)
            };
            let highlight = if i == state.list_state.selected().unwrap_or(0) {
                Style::default().fg(theme::ACCENT).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(theme::TEXT_PRIMARY)
            };
            ListItem::new(Line::from(vec![
                Span::styled(format!("  {:<16}", c.display_name), highlight),
                Span::styled(format!(" {:<14}", c.category), theme::dim_style()),
                Span::styled(badge, badge_style),
            ]))
        })
        .collect();

    let list = List::new(items)
        .highlight_style(theme::selected_style())
        .highlight_symbol("> ");
    f.render_stateful_widget(list, chunks[1], &mut state.list_state);

    // Hints
    f.render_widget(
        Paragraph::new(Line::from(vec![Span::styled(
            i18n.t(TranslationKey::WizardChannelHint),
            theme::hint_style(),
        )])),
        chunks[2],
    );
}

fn draw_channel_input(f: &mut Frame, area: Rect, state: &WizardState, i18n: &Translator) {
    let channel_name = state
        .selected_channel
        .and_then(|i| state.channels.get(i))
        .map(|c| c.display_name.as_str())
        .unwrap_or("?");

    let env_vars = state
        .selected_channel
        .and_then(|i| state.channels.get(i))
        .map(|c| c.env_required.as_slice())
        .unwrap_or(&[]);

    let current_var = if state.channel_env_cursor < env_vars.len() {
        &env_vars[state.channel_env_cursor]
    } else {
        ""
    };

    let total_vars = env_vars.len();
    let current_idx = state.channel_env_cursor + 1;

    let chunks = Layout::vertical([
        Constraint::Length(2), // title
        Constraint::Length(1), // separator
        Constraint::Length(2), // current var
        Constraint::Length(1), // input
        Constraint::Length(1), // hints
    ])
    .split(area);

    // Title
    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(i18n.t(TranslationKey::WizardChannelSetupTitle), Style::default().fg(theme::CYAN).add_modifier(Modifier::BOLD)),
            Span::styled(channel_name, Style::default().fg(theme::ACCENT)),
        ])),
        chunks[0],
    );

    // Separator
    f.render_widget(
        Paragraph::new(Span::styled("\u{2500}".repeat(area.width as usize), theme::dim_style())),
        chunks[1],
    );

    // Current variable
    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(format!("  [{}/{}] ", current_idx, total_vars), Style::default().fg(theme::YELLOW)),
            Span::styled(current_var, Style::default().fg(theme::ACCENT)),
            Span::styled(":", Style::default().fg(theme::TEXT_PRIMARY)),
        ])),
        chunks[2],
    );

    // Input
    let display = if state.channel_input_buffer.is_empty() {
        "paste value..."
    } else {
        &state.channel_input_buffer
    };
    let style = if state.channel_input_buffer.is_empty() {
        theme::dim_style()
    } else {
        theme::input_style()
    };
    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::raw("  > "),
            Span::styled(display, style),
            Span::styled("\u{2588}", Style::default().fg(theme::GREEN).add_modifier(Modifier::SLOW_BLINK)),
        ])),
        chunks[3],
    );

    // Hints
    f.render_widget(
        Paragraph::new(Line::from(vec![Span::styled(
            i18n.t(TranslationKey::WizardChannelInputHint),
            theme::hint_style(),
        )])),
        chunks[4],
    );
}

fn draw_agent_step(f: &mut Frame, area: Rect, state: &mut WizardState, i18n: &Translator) {
    let step_idx = WizardStep::AgentCreate.index();
    let total_steps = 7;
    draw_step_header(f, area, step_idx, total_steps, state, i18n);

    let content_area = Rect {
        x: area.x + 2,
        y: area.y + 3,
        width: area.width.saturating_sub(4),
        height: area.height.saturating_sub(5),
    };

    let chunks = Layout::vertical([
        Constraint::Length(2), // prompt
        Constraint::Min(4),    // templates or input
        Constraint::Length(1), // hints
    ])
    .split(content_area);

    // Prompt
    f.render_widget(
        Paragraph::new(Line::from(vec![Span::styled(
            i18n.t(TranslationKey::WizardAgentPrompt),
            Style::default().fg(theme::TEXT_PRIMARY),
        )])),
        chunks[0],
    );

    if state.agent_custom_mode {
        // Custom name input
        let sub_chunks = Layout::vertical([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(2),
        ])
        .split(chunks[1]);

        f.render_widget(
            Paragraph::new(Line::from(vec![Span::styled(
                "  Enter agent name:",
                Style::default().fg(theme::TEXT_PRIMARY),
            )])),
            sub_chunks[0],
        );

        let display = if state.agent_name.is_empty() {
            "my-agent"
        } else {
            &state.agent_name
        };
        let style = if state.agent_name.is_empty() {
            theme::dim_style()
        } else {
            theme::input_style()
        };
        f.render_widget(
            Paragraph::new(Line::from(vec![
                Span::raw("  > "),
                Span::styled(display, style),
                Span::styled("\u{2588}", Style::default().fg(theme::GREEN).add_modifier(Modifier::SLOW_BLINK)),
            ])),
            sub_chunks[1],
        );
    } else {
        // Template list
        let items: Vec<ListItem> = state
            .agent_templates
            .iter()
            .enumerate()
            .map(|(i, t)| {
                let highlight = if i == state.agent_template_idx.unwrap_or(0) {
                    Style::default().fg(theme::ACCENT).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(theme::TEXT_PRIMARY)
                };
                ListItem::new(Line::from(vec![
                    Span::styled(format!("  {:<12}", t.name), highlight),
                    Span::styled(format!("  {}", t.description), theme::dim_style()),
                ]))
            })
            .collect();

        let list = List::new(items)
            .highlight_style(theme::selected_style())
            .highlight_symbol("> ");
        let mut list_state = ListState::default().with_selected(state.agent_template_idx);
        f.render_stateful_widget(list, chunks[1], &mut list_state);
    }

    // Hints
    let hint_text = if state.agent_custom_mode {
        i18n.t(TranslationKey::WizardAgentHintCustom)
    } else {
        i18n.t(TranslationKey::WizardAgentHint)
    };
    f.render_widget(
        Paragraph::new(Line::from(vec![Span::styled(hint_text, theme::hint_style())])),
        chunks[2],
    );
}

fn draw_summary_step(f: &mut Frame, area: Rect, state: &mut WizardState, i18n: &Translator) {
    let step_idx = WizardStep::Summary.index();
    let total_steps = 7;
    draw_step_header(f, area, step_idx, total_steps, state, i18n);

    let content_area = Rect {
        x: area.x + 2,
        y: area.y + 3,
        width: area.width.saturating_sub(4),
        height: area.height.saturating_sub(5),
    };

    let chunks = Layout::vertical([
        Constraint::Length(2), // title
        Constraint::Min(8),    // summary
        Constraint::Length(1), // hints
    ])
    .split(content_area);

    // Title
    f.render_widget(
        Paragraph::new(Line::from(vec![Span::styled(
            i18n.t(TranslationKey::WizardSummaryTitle),
            Style::default().fg(theme::ACCENT).add_modifier(Modifier::BOLD),
        )])),
        chunks[0],
    );

    // Summary content
    let mut lines: Vec<Line> = Vec::new();

    // Provider
    let provider_line = if state.skip_mode {
        Line::from(vec![
            Span::styled("  Provider: ", Style::default().fg(theme::TEXT_PRIMARY)),
            Span::styled("(skipped)", Style::default().fg(theme::YELLOW)),
        ])
    } else if let Some(idx) = state.selected_provider {
        let p = &state.providers[idx];
        Line::from(vec![
            Span::styled("  Provider: ", Style::default().fg(theme::TEXT_PRIMARY)),
            Span::styled(&p.display_name, Style::default().fg(theme::CYAN)),
        ])
    } else {
        Line::from(vec![
            Span::styled("  Provider: ", Style::default().fg(theme::TEXT_PRIMARY)),
            Span::styled("(none selected)", theme::dim_style()),
        ])
    };
    lines.push(provider_line);

    // Models
    let model_count = state.selected_models.len();
    let model_names: String = state
        .selected_models
        .iter()
        .take(3)
        .map(|&i| state.models.get(i).map(|m| m.display_name.clone()).unwrap_or_default())
        .collect::<Vec<_>>()
        .join(", ");
    let models_text = if model_count > 3 {
        format!("{} ({} more)", model_names, model_count - 3)
    } else if model_count > 0 {
        model_names
    } else {
        "(none selected)".to_string()
    };
    lines.push(Line::from(vec![
        Span::styled("  Models:    ", Style::default().fg(theme::TEXT_PRIMARY)),
        Span::styled(models_text, Style::default().fg(theme::CYAN)),
    ]));

    // Channel
    let channel_line = if let Some(idx) = state.selected_channel {
        let c = &state.channels[idx];
        Line::from(vec![
            Span::styled("  Channel:   ", Style::default().fg(theme::TEXT_PRIMARY)),
            Span::styled(&c.display_name, Style::default().fg(theme::CYAN)),
            Span::styled(if c.configured { " (configured)" } else { "" }, Style::default().fg(theme::GREEN)),
        ])
    } else {
        Line::from(vec![
            Span::styled("  Channel:   ", Style::default().fg(theme::TEXT_PRIMARY)),
            Span::styled("(none selected)", theme::dim_style()),
        ])
    };
    lines.push(channel_line);

    // Agent
    lines.push(Line::from(vec![
        Span::styled("  Agent:     ", Style::default().fg(theme::TEXT_PRIMARY)),
        Span::styled(&state.agent_name, Style::default().fg(theme::CYAN)),
    ]));

    // Separator
    lines.push(Line::from(""));
    lines.push(Line::from(vec![Span::styled(
        i18n.t(TranslationKey::WizardSummaryNote),
        Style::default().fg(theme::YELLOW),
    )]));

    f.render_widget(Paragraph::new(lines), chunks[1]);

    // Hints
    f.render_widget(
        Paragraph::new(Line::from(vec![Span::styled(
            i18n.t(TranslationKey::WizardSummaryHint),
            theme::hint_style(),
        )])),
        chunks[2],
    );
}

fn draw_complete_step(f: &mut Frame, area: Rect, state: &WizardState, i18n: &Translator) {
    let card_h = 12u16.min(area.height);
    let card_w = 50u16.min(area.width.saturating_sub(2));
    let [card_area] = Layout::horizontal([Constraint::Length(card_w)])
        .flex(Flex::Center)
        .areas(area);
    let [card_area] = Layout::vertical([Constraint::Length(card_h)])
        .flex(Flex::Center)
        .areas(card_area);

    let block = Block::default()
        .title(Line::from(vec![Span::styled(
            format!(" {} ", i18n.t(TranslationKey::WizardCompleteTitle)),
            theme::title_style(),
        )]))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme::GREEN))
        .padding(Padding::uniform(1));
    let inner = block.inner(card_area);
    f.render_widget(block, card_area);

    let chunks = Layout::vertical([
        Constraint::Length(2),
        Constraint::Length(2),
        Constraint::Length(2),
        Constraint::Length(1),
    ])
    .split(inner);

    // Success icon
    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("\u{2714} ", Style::default().fg(theme::GREEN).add_modifier(Modifier::BOLD)),
            Span::styled(i18n.t(TranslationKey::WizardCompleteSuccess), Style::default().fg(theme::GREEN).add_modifier(Modifier::BOLD)),
        ]))
        .alignment(Alignment::Center),
        chunks[0],
    );

    // Message
    f.render_widget(
        Paragraph::new(Line::from(vec![Span::styled(
            i18n.t(TranslationKey::WizardCompleteMsg),
            theme::dim_style(),
        )]))
        .alignment(Alignment::Center),
        chunks[1],
    );

    // Agent name
    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("Agent: ", Style::default().fg(theme::TEXT_PRIMARY)),
            Span::styled(&state.agent_name, Style::default().fg(theme::CYAN).add_modifier(Modifier::BOLD)),
        ]))
        .alignment(Alignment::Center),
        chunks[2],
    );

    // Hints
    f.render_widget(
        Paragraph::new(Line::from(vec![Span::styled(
            i18n.t(TranslationKey::WizardCompleteHint),
            theme::hint_style(),
        )]))
        .alignment(Alignment::Center),
        chunks[3],
    );
}

// ── Helpers ─────────────────────────────────────────────────────────────────

fn draw_step_header(
    f: &mut Frame,
    area: Rect,
    step_idx: usize,
    total_steps: usize,
    state: &WizardState,
    i18n: &Translator,
) {
    let title = state.step.title(i18n);
    let progress = format!("[{}/{}]", step_idx, total_steps);

    let header = Paragraph::new(Line::from(vec![
        Span::styled(progress, Style::default().fg(theme::YELLOW)),
        Span::styled(" ", Style::default()),
        Span::styled(title, theme::title_style()),
    ]));
    f.render_widget(header, Rect { x: area.x + 1, y: area.y, width: area.width.saturating_sub(2), height: 2 });
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}\u{2026}", &s[..max.saturating_sub(1)])
    }
}