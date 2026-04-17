//! Application state machine and event dispatch.
//!
//! Cloned from AgentDiVA TUI mod.rs core state machine logic.

use crate::config::AppConfig;
use crate::draw;
use crate::event::AppEvent;
use crate::i18n::{Language, Translator};
use crate::screens;
use crate::screens::agents::{AgentAction, AgentSelectState};
use crate::screens::welcome::{WelcomeAction, WelcomeState};
use crate::screens::wizard::{WizardAction, WizardState};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::Frame;
use std::sync::mpsc;

// Core types

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    Boot(BootScreen),
    Main,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum BootScreen {
    Welcome,
    Wizard,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Dashboard,
    Agents,
    Chat,
    Sessions,
    Workflows,
    Triggers,
    Memory,
    Channels,
    Skills,
    Hands,
    Extensions,
    Templates,
    Peers,
    Comms,
    Security,
    Audit,
    Usage,
    Settings,
    Logs,
}

pub const TABS: &[Tab] = &[
    Tab::Dashboard,
    Tab::Agents,
    Tab::Chat,
    Tab::Sessions,
    Tab::Workflows,
    Tab::Triggers,
    Tab::Memory,
    Tab::Channels,
    Tab::Skills,
    Tab::Hands,
    Tab::Extensions,
    Tab::Templates,
    Tab::Peers,
    Tab::Comms,
    Tab::Security,
    Tab::Audit,
    Tab::Usage,
    Tab::Settings,
    Tab::Logs,
];

impl Tab {
    pub fn label(self, i18n: &Translator) -> &'static str {
        use crate::i18n::TranslationKey;
        match self {
            Tab::Dashboard => i18n.t(TranslationKey::TabDashboard),
            Tab::Agents => i18n.t(TranslationKey::TabAgents),
            Tab::Chat => i18n.t(TranslationKey::TabChat),
            Tab::Sessions => i18n.t(TranslationKey::TabSessions),
            Tab::Workflows => i18n.t(TranslationKey::TabWorkflows),
            Tab::Triggers => i18n.t(TranslationKey::TabTriggers),
            Tab::Memory => i18n.t(TranslationKey::TabMemory),
            Tab::Channels => i18n.t(TranslationKey::TabChannels),
            Tab::Skills => i18n.t(TranslationKey::TabSkills),
            Tab::Hands => i18n.t(TranslationKey::TabHands),
            Tab::Extensions => i18n.t(TranslationKey::TabExtensions),
            Tab::Templates => i18n.t(TranslationKey::TabTemplates),
            Tab::Peers => i18n.t(TranslationKey::TabPeers),
            Tab::Comms => i18n.t(TranslationKey::TabComms),
            Tab::Security => i18n.t(TranslationKey::TabSecurity),
            Tab::Audit => i18n.t(TranslationKey::TabAudit),
            Tab::Usage => i18n.t(TranslationKey::TabUsage),
            Tab::Settings => i18n.t(TranslationKey::TabSettings),
            Tab::Logs => i18n.t(TranslationKey::TabLogs),
        }
    }

    pub fn index(self) -> usize {
        TABS.iter().position(|&t| t == self).unwrap_or(0)
    }
}

// App struct

pub struct App {
    pub phase: Phase,
    pub active_tab: Tab,
    pub tab_scroll_offset: usize,
    pub should_quit: bool,
    pub event_tx: mpsc::Sender<AppEvent>,
    /// Double Ctrl+C quit: true after first Ctrl+C press.
    pub ctrl_c_pending: bool,
    /// Tick counter when first Ctrl+C was pressed (auto-resets after ~2s).
    ctrl_c_tick: usize,
    /// Global tick counter for Ctrl+C timeout tracking.
    pub tick_count: usize,

    /// Internationalization translator
    pub i18n: Translator,
    /// Application configuration (persisted)
    pub config: AppConfig,

    // Screen states
    pub welcome: WelcomeState,
    pub agents: AgentSelectState,
    pub dashboard: screens::dashboard::DashboardState,
    pub chat: screens::chat::ChatState,
    pub sessions: screens::sessions::SessionsState,
    pub workflows: screens::workflows::WorkflowsState,
    pub triggers: screens::triggers::TriggersState,
    pub memory: screens::memory::MemoryState,
    pub channels: screens::channels::ChannelState,
    pub skills: screens::skills::SkillsState,
    pub hands: screens::hands::HandsState,
    pub extensions: screens::extensions::ExtensionsState,
    pub templates: screens::templates::TemplatesState,
    pub peers: screens::peers::PeersState,
    pub comms: screens::comms::CommsState,
    pub security: screens::security::SecurityState,
    pub audit: screens::audit::AuditState,
    pub usage: screens::usage::UsageState,
    pub settings: screens::settings::SettingsState,
    pub logs: screens::logs::LogsState,
    pub wizard: WizardState,
}

impl App {
    pub fn new(event_tx: mpsc::Sender<AppEvent>) -> Self {
        // Load configuration from disk (or use default)
        let config = AppConfig::load();

        // Initialize translator with configured language
        let i18n = Translator::new(config.language);

        Self {
            phase: Phase::Boot(BootScreen::Welcome),
            active_tab: Tab::Dashboard,
            tab_scroll_offset: 0,
            should_quit: false,
            event_tx,
            ctrl_c_pending: false,
            ctrl_c_tick: 0,
            tick_count: 0,
            i18n,
            config,
            welcome: WelcomeState::new(),
            agents: AgentSelectState::new(),
            dashboard: screens::dashboard::DashboardState::new(),
            chat: screens::chat::ChatState::new(),
            sessions: screens::sessions::SessionsState::new(),
            workflows: screens::workflows::WorkflowsState::new(),
            triggers: screens::triggers::TriggersState::new(),
            memory: screens::memory::MemoryState::new(),
            channels: screens::channels::ChannelState::new(),
            skills: screens::skills::SkillsState::new(),
            hands: screens::hands::HandsState::new(),
            extensions: screens::extensions::ExtensionsState::new(),
            templates: screens::templates::TemplatesState::new(),
            peers: screens::peers::PeersState::new(),
            comms: screens::comms::CommsState::new(),
            security: screens::security::SecurityState::new(),
            audit: screens::audit::AuditState::new(),
            usage: screens::usage::UsageState::new(),
            settings: screens::settings::SettingsState::new(),
            logs: screens::logs::LogsState::new(),
            wizard: WizardState::new(),
        }
    }

    /// Switch language and persist the change
    pub fn switch_language(&mut self, language: Language) {
        self.i18n.set_language(language);
        self.config.set_language(language);
        if let Err(e) = self.config.save() {
            eprintln!("Warning: Failed to save language config: {}", e);
        }
    }

    pub fn start_welcome(&mut self) {
        self.phase = Phase::Boot(BootScreen::Welcome);
    }

    // Event dispatch

    pub fn handle_event(&mut self, ev: AppEvent) {
        match ev {
            AppEvent::Key(key) => self.handle_key(key),
            AppEvent::Tick => self.handle_tick(),
        }
    }

    fn handle_key(&mut self, key: KeyEvent) {
        // Global: Double Ctrl+C to quit (all phases)
        let is_ctrl_c =
            key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL);
        if is_ctrl_c {
            if self.ctrl_c_pending {
                self.should_quit = true;
                return;
            }
            self.ctrl_c_pending = true;
            self.ctrl_c_tick = self.tick_count;
            // In Main phase, don't pass the first Ctrl+C to screen handlers
            if matches!(self.phase, Phase::Main) {
                return;
            }
        } else {
            // Any other key clears the pending Ctrl+C state
            self.ctrl_c_pending = false;
        }

        // Global: Ctrl+Q quit from Main phase
        if matches!(self.phase, Phase::Main) {
            if key.code == KeyCode::Char('q') && key.modifiers.contains(KeyModifiers::CONTROL) {
                self.should_quit = true;
                return;
            }

            // Global: ESC from Main phase returns to Welcome (boot menu)
            if key.code == KeyCode::Esc && key.modifiers.is_empty() {
                self.phase = Phase::Boot(BootScreen::Welcome);
                return;
            }

            // Tab switching: F1-F12 for direct jump
            match key.code {
                KeyCode::F(1) => {
                    self.switch_tab(Tab::Dashboard);
                    return;
                }
                KeyCode::F(2) => {
                    self.switch_tab(Tab::Agents);
                    return;
                }
                KeyCode::F(3) => {
                    self.switch_tab(Tab::Chat);
                    return;
                }
                KeyCode::F(4) => {
                    self.switch_tab(Tab::Sessions);
                    return;
                }
                KeyCode::F(5) => {
                    self.switch_tab(Tab::Workflows);
                    return;
                }
                KeyCode::F(6) => {
                    self.switch_tab(Tab::Triggers);
                    return;
                }
                KeyCode::F(7) => {
                    self.switch_tab(Tab::Memory);
                    return;
                }
                KeyCode::F(8) => {
                    self.switch_tab(Tab::Channels);
                    return;
                }
                KeyCode::F(9) => {
                    self.switch_tab(Tab::Skills);
                    return;
                }
                KeyCode::F(10) => {
                    self.switch_tab(Tab::Templates);
                    return;
                }
                KeyCode::F(11) => {
                    self.switch_tab(Tab::Peers);
                    return;
                }
                KeyCode::F(12) => {
                    self.switch_tab(Tab::Security);
                    return;
                }
                _ => {}
            }

            // Tab cycling: Tab / Shift+Tab
            if key.code == KeyCode::Tab && key.modifiers.is_empty() {
                self.next_tab();
                return;
            }
            if key.code == KeyCode::BackTab {
                self.prev_tab();
                return;
            }

            // Tab cycling: Ctrl+Left/Right
            if key.modifiers.contains(KeyModifiers::CONTROL) {
                match key.code {
                    KeyCode::Left => {
                        self.prev_tab();
                        return;
                    }
                    KeyCode::Right => {
                        self.next_tab();
                        return;
                    }
                    _ => {}
                }
            }

            // Tab cycling: Ctrl+[ / Ctrl+]
            if key.modifiers.contains(KeyModifiers::CONTROL) {
                match key.code {
                    KeyCode::Char('[') => {
                        self.prev_tab();
                        return;
                    }
                    KeyCode::Char(']') => {
                        self.next_tab();
                        return;
                    }
                    _ => {}
                }
            }

            // Fallback: Alt+1-9,0
            if key.modifiers.contains(KeyModifiers::ALT) {
                match key.code {
                    KeyCode::Char('1') => {
                        self.switch_tab(Tab::Dashboard);
                        return;
                    }
                    KeyCode::Char('2') => {
                        self.switch_tab(Tab::Agents);
                        return;
                    }
                    KeyCode::Char('3') => {
                        self.switch_tab(Tab::Chat);
                        return;
                    }
                    KeyCode::Char('4') => {
                        self.switch_tab(Tab::Sessions);
                        return;
                    }
                    KeyCode::Char('5') => {
                        self.switch_tab(Tab::Workflows);
                        return;
                    }
                    KeyCode::Char('6') => {
                        self.switch_tab(Tab::Triggers);
                        return;
                    }
                    KeyCode::Char('7') => {
                        self.switch_tab(Tab::Memory);
                        return;
                    }
                    KeyCode::Char('8') => {
                        self.switch_tab(Tab::Channels);
                        return;
                    }
                    KeyCode::Char('9') => {
                        self.switch_tab(Tab::Skills);
                        return;
                    }
                    KeyCode::Char('0') => {
                        self.switch_tab(Tab::Templates);
                        return;
                    }
                    _ => {}
                }
            }
        }

        // Route to screen handler
        match self.phase {
            Phase::Boot(BootScreen::Welcome) => {
                if let Some(action) = self.welcome.handle_key(key, &self.i18n) {
                    self.handle_welcome_action(action);
                }
            }
            Phase::Boot(BootScreen::Wizard) => {
                let action = self.wizard.handle_key(key, &self.i18n);
                self.handle_wizard_action(action);
            }
            Phase::Main => match self.active_tab {
                Tab::Dashboard => {
                    let action = self.dashboard.handle_key(key);
                    self.handle_dashboard_action(action);
                }
                Tab::Agents => {
                    let action = self.agents.handle_key(key);
                    self.handle_agent_action(action);
                }
                Tab::Chat => {
                    let action = self.chat.handle_key(key);
                    self.handle_chat_action(action);
                }
                Tab::Sessions => {
                    let action = self.sessions.handle_key(key);
                    self.handle_sessions_action(action);
                }
                Tab::Workflows => {
                    let action = self.workflows.handle_key(key);
                    self.handle_workflows_action(action);
                }
                Tab::Triggers => {
                    let action = self.triggers.handle_key(key);
                    self.handle_triggers_action(action);
                }
                Tab::Memory => {
                    let action = self.memory.handle_key(key);
                    self.handle_memory_action(action);
                }
                Tab::Channels => {
                    let action = self.channels.handle_key(key);
                    self.handle_channels_action(action);
                }
                Tab::Skills => {
                    let action = self.skills.handle_key(key);
                    self.handle_skills_action(action);
                }
                Tab::Hands => {
                    let action = self.hands.handle_key(key);
                    self.handle_hands_action(action);
                }
                Tab::Extensions => {
                    let action = self.extensions.handle_key(key);
                    self.handle_extensions_action(action);
                }
                Tab::Templates => {
                    let action = self.templates.handle_key(key);
                    self.handle_templates_action(action);
                }
                Tab::Peers => {
                    let action = self.peers.handle_key(key);
                    self.handle_peers_action(action);
                }
                Tab::Comms => {
                    let action = self.comms.handle_key(key);
                    self.handle_comms_action(action);
                }
                Tab::Security => {
                    let action = self.security.handle_key(key);
                    self.handle_security_action(action);
                }
                Tab::Audit => {
                    let action = self.audit.handle_key(key);
                    self.handle_audit_action(action);
                }
                Tab::Usage => {
                    let action = self.usage.handle_key(key);
                    self.handle_usage_action(action);
                }
                Tab::Settings => {
                    let action = self.settings.handle_key(key, &self.i18n);
                    self.handle_settings_action(action);
                }
                Tab::Logs => {
                    let action = self.logs.handle_key(key);
                    self.handle_logs_action(action);
                }
            },
        }
    }

    fn handle_tick(&mut self) {
        self.tick_count = self.tick_count.wrapping_add(1);
        // Auto-reset Ctrl+C pending after ~2s (40 ticks at 50ms)
        if self.ctrl_c_pending && self.tick_count.wrapping_sub(self.ctrl_c_tick) > 40 {
            self.ctrl_c_pending = false;
        }
        self.welcome.tick();
        self.agents.tick();
        self.chat.tick();
        self.dashboard.tick();
        self.wizard.tick();
    }

    // Tab navigation

    fn next_tab(&mut self) {
        let idx = self.active_tab.index();
        let next = (idx + 1) % TABS.len();
        self.switch_tab(TABS[next]);
    }

    fn prev_tab(&mut self) {
        let idx = self.active_tab.index();
        let prev = if idx == 0 { TABS.len() - 1 } else { idx - 1 };
        self.switch_tab(TABS[prev]);
    }

    fn switch_tab(&mut self, tab: Tab) {
        self.active_tab = tab;
        // Keep active tab visible in the scrollable tab bar
        let idx = tab.index();
        if idx < self.tab_scroll_offset {
            self.tab_scroll_offset = idx;
        }
        self.on_tab_enter(tab);
    }

    /// Called when a tab becomes active
    fn on_tab_enter(&mut self, tab: Tab) {
        // Placeholder - no data loading
        match tab {
            Tab::Dashboard => {}
            Tab::Agents => {}
            Tab::Chat => {}
            _ => {}
        }
    }

    /// Transition from Boot to Main phase.
    fn enter_main_phase(&mut self) {
        self.phase = Phase::Main;
        self.active_tab = Tab::Agents;
    }

    // Screen action handlers

    fn handle_welcome_action(&mut self, action: WelcomeAction) {
        match action {
            WelcomeAction::Exit => self.should_quit = true,
            WelcomeAction::ConnectDaemon => {
                self.enter_main_phase();
            }
            WelcomeAction::InProcess => {
                self.enter_main_phase();
            }
            WelcomeAction::Wizard => {
                self.wizard.reset();
                self.phase = Phase::Boot(BootScreen::Wizard);
            }
            WelcomeAction::SwitchLanguage(lang) => {
                self.switch_language(lang);
            }
        }
    }

    fn handle_wizard_action(&mut self, action: WizardAction) {
        match action {
            WizardAction::Continue => {}
            WizardAction::Back => {}
            WizardAction::Skip => {}
            WizardAction::Finish => {
                self.enter_main_phase();
            }
            WizardAction::Exit => {
                self.phase = Phase::Boot(BootScreen::Welcome);
            }
        }
    }

    fn handle_agent_action(&mut self, action: AgentAction) {
        match action {
            AgentAction::Continue => {}
            AgentAction::Back => {
                // In Main phase, Esc from agents just stays on the tab
            }
            AgentAction::CreatedManifest(_toml) => {
                // Placeholder - show success message
                self.agents.status_msg = "Agent created! (placeholder)".to_string();
                self.agents.sub = screens::agents::AgentSubScreen::AgentList;
            }
            AgentAction::ChatWithAgent { id: _, name } => {
                self.chat.reset();
                self.chat.agent_name = name.clone();
                self.chat.mode_label = "placeholder".to_string();
                self.chat.model_label = "model/placeholder".to_string();
                self.chat.push_message(
                    screens::chat::Role::System,
                    "Placeholder chat mode - enter to send".to_string(),
                );
                self.active_tab = Tab::Chat;
            }
            AgentAction::KillAgent(_id) => {
                self.agents.status_msg = "Agent killed (placeholder)".to_string();
                self.agents.sub = screens::agents::AgentSubScreen::AgentList;
            }
            AgentAction::UpdateSkills { id: _, skills: _ } => {
                self.agents.status_msg = "Skills updated (placeholder)".to_string();
                self.agents.sub = screens::agents::AgentSubScreen::AgentDetail;
            }
            AgentAction::UpdateMcpServers { id: _, servers: _ } => {
                self.agents.status_msg = "MCP servers updated (placeholder)".to_string();
                self.agents.sub = screens::agents::AgentSubScreen::AgentDetail;
            }
            AgentAction::FetchAgentSkills(_id) => {
                // Placeholder - populate with mock data
                self.agents.available_skills = vec![
                    ("coding".to_string(), true),
                    ("research".to_string(), false),
                    ("writing".to_string(), false),
                ];
                self.agents.skill_cursor = 0;
            }
            AgentAction::FetchAgentMcpServers(_id) => {
                // Placeholder - populate with mock data
                self.agents.available_mcp = vec![
                    ("filesystem".to_string(), true),
                    ("github".to_string(), false),
                ];
                self.agents.mcp_cursor = 0;
            }
        }
    }

    fn handle_chat_action(&mut self, action: screens::chat::ChatAction) {
        match action {
            screens::chat::ChatAction::Continue => {}
            screens::chat::ChatAction::Back => {
                self.chat.reset();
                self.switch_tab(Tab::Agents);
            }
            screens::chat::ChatAction::SendMessage(msg) => {
                // Placeholder - simulate streaming response with tool calls
                self.chat.is_streaming = true;
                self.chat.thinking = true;

                // Simulate a tool call
                self.chat.tool_start("file_read");
                self.chat.tool_use_end("file_read", "{\"path\": \"src/main.rs\"}");
                self.chat.tool_result("file_read", "fn main() { println!(\"Hello\"); }", false);

                // Simulate streaming text
                self.chat.append_stream("I've read the file. Here's what I found:\n\nThe main.rs file contains a simple Rust program that prints \"Hello\".\n\nWould you like me to make any changes?");

                // Finalize
                self.chat.finalize_stream();
                self.chat.last_tokens = Some((150, 45));
                self.chat.last_cost_usd = Some(0.0023);
            }
            screens::chat::ChatAction::SlashCommand(cmd) => {
                if cmd == "/help" {
                    self.chat.push_message(
                        screens::chat::Role::System,
                        "/help \u{2022} /clear \u{2022} /exit \u{2022} /tools".to_string(),
                    );
                } else if cmd == "/clear" {
                    self.chat.messages.clear();
                    self.chat.push_message(
                        screens::chat::Role::System,
                        "Chat cleared.".to_string(),
                    );
                } else if cmd == "/tools" {
                    self.chat.push_message(
                        screens::chat::Role::System,
                        "Available tools: file_read, file_write, shell_exec, web_fetch".to_string(),
                    );
                } else {
                    self.chat.push_message(
                        screens::chat::Role::System,
                        format!("Unknown command: {} \u{2014} try /help", cmd),
                    );
                }
            }
            screens::chat::ChatAction::OpenModelPicker => {
                self.chat.model_picker_models = vec![
                    screens::chat::ModelEntry {
                        id: "claude-3-5-sonnet".to_string(),
                        display_name: "Claude 3.5 Sonnet".to_string(),
                        provider: "anthropic".to_string(),
                        tier: "Frontier".to_string(),
                    },
                    screens::chat::ModelEntry {
                        id: "claude-3-opus".to_string(),
                        display_name: "Claude 3 Opus".to_string(),
                        provider: "anthropic".to_string(),
                        tier: "Frontier".to_string(),
                    },
                    screens::chat::ModelEntry {
                        id: "gpt-4o".to_string(),
                        display_name: "GPT-4o".to_string(),
                        provider: "openai".to_string(),
                        tier: "Smart".to_string(),
                    },
                    screens::chat::ModelEntry {
                        id: "gpt-4o-mini".to_string(),
                        display_name: "GPT-4o Mini".to_string(),
                        provider: "openai".to_string(),
                        tier: "Fast".to_string(),
                    },
                    screens::chat::ModelEntry {
                        id: "deepseek-chat".to_string(),
                        display_name: "DeepSeek Chat".to_string(),
                        provider: "deepseek".to_string(),
                        tier: "Balanced".to_string(),
                    },
                    screens::chat::ModelEntry {
                        id: "gemini-2-flash".to_string(),
                        display_name: "Gemini 2 Flash".to_string(),
                        provider: "google".to_string(),
                        tier: "Fast".to_string(),
                    },
                ];
                self.chat.show_model_picker = true;
                self.chat.model_picker_idx = 0;
            }
            screens::chat::ChatAction::SwitchModel(model_id) => {
                self.chat.model_label = model_id.clone();
                self.chat.push_message(
                    screens::chat::Role::System,
                    format!("Switched to {}", model_id),
                );
            }
        }
    }

    fn handle_dashboard_action(&mut self, action: screens::dashboard::DashboardAction) {
        match action {
            screens::dashboard::DashboardAction::Continue => {}
            screens::dashboard::DashboardAction::Refresh => {}
            screens::dashboard::DashboardAction::GoToAgents => {
                self.switch_tab(Tab::Agents);
            }
        }
    }

    fn handle_sessions_action(&mut self, action: screens::sessions::SessionsAction) {
        match action {
            screens::sessions::SessionsAction::Continue => {}
            screens::sessions::SessionsAction::Back => {
                self.switch_tab(Tab::Dashboard);
            }
            screens::sessions::SessionsAction::Refresh => {}
            screens::sessions::SessionsAction::OpenInChat { agent_id, agent_name } => {
                self.chat.reset();
                self.chat.agent_name = agent_name.clone();
                self.chat.mode_label = "session".to_string();
                self.chat.model_label = "placeholder".to_string();
                self.chat.push_message(
                    screens::chat::Role::System,
                    format!("Opened session for agent: {}", agent_name),
                );
                self.active_tab = Tab::Chat;
            }
            screens::sessions::SessionsAction::DeleteSession(_id) => {
                self.sessions.status_msg = "Session deleted (placeholder)".to_string();
            }
        }
    }

    fn handle_memory_action(&mut self, action: screens::memory::MemoryAction) {
        match action {
            screens::memory::MemoryAction::Continue => {}
            screens::memory::MemoryAction::Back => {
                self.switch_tab(Tab::Dashboard);
            }
            screens::memory::MemoryAction::LoadAgents => {}
            screens::memory::MemoryAction::LoadKv(_agent_id) => {}
            screens::memory::MemoryAction::SaveKv { agent_id, key, value } => {
                self.memory.status_msg = format!("Saved {} for agent {}", key, agent_id);
            }
            screens::memory::MemoryAction::DeleteKv { agent_id, key } => {
                self.memory.status_msg = format!("Deleted {} for agent {}", key, agent_id);
            }
        }
    }

    fn handle_channels_action(&mut self, action: screens::channels::ChannelAction) {
        match action {
            screens::channels::ChannelAction::Continue => {}
            screens::channels::ChannelAction::Back => {
                self.switch_tab(Tab::Dashboard);
            }
            screens::channels::ChannelAction::Refresh => {}
            screens::channels::ChannelAction::TestChannel(_name) => {}
            screens::channels::ChannelAction::ToggleChannel(_name, _enabled) => {}
            screens::channels::ChannelAction::SaveChannel(_name, _values) => {}
        }
    }

    fn handle_workflows_action(&mut self, action: screens::workflows::WorkflowAction) {
        match action {
            screens::workflows::WorkflowAction::Continue => {}
            screens::workflows::WorkflowAction::Back => {}
            screens::workflows::WorkflowAction::Refresh => {}
            screens::workflows::WorkflowAction::CreateWorkflow { name, desc, steps } => {
                self.workflows.status_msg = format!("Workflow '{}' created with {} steps (placeholder)", name, steps.len());
            }
            screens::workflows::WorkflowAction::RunWorkflow { id, inputs } => {
                self.workflows.status_msg = format!("Workflow {} executed with {} inputs (placeholder)", id, inputs.len());
            }
            screens::workflows::WorkflowAction::DeleteWorkflow { id } => {
                self.workflows.status_msg = format!("Workflow {} deleted (placeholder)", id);
            }
        }
    }

    fn handle_triggers_action(&mut self, action: screens::triggers::TriggerAction) {
        match action {
            screens::triggers::TriggerAction::Continue => {}
            screens::triggers::TriggerAction::Back => {}
            screens::triggers::TriggerAction::Refresh => {}
            screens::triggers::TriggerAction::CreateTrigger { name, pattern_type, pattern, action_type, action_target, enabled } => {
                self.triggers.status_msg = format!("Trigger '{}' created (placeholder)", name);
            }
            screens::triggers::TriggerAction::UpdateTrigger { id } => {
                self.triggers.status_msg = format!("Trigger {} updated (placeholder)", id);
            }
            screens::triggers::TriggerAction::DeleteTrigger { id } => {
                self.triggers.status_msg = format!("Trigger {} deleted (placeholder)", id);
            }
            screens::triggers::TriggerAction::ToggleTrigger { id, enabled } => {
                self.triggers.status_msg = format!("Trigger {} toggled to {} (placeholder)", id, if enabled { "enabled" } else { "disabled" });
            }
        }
    }

    fn handle_skills_action(&mut self, action: screens::skills::SkillsAction) {
        match action {
            screens::skills::SkillsAction::Continue => {}
            screens::skills::SkillsAction::Back => {}
            screens::skills::SkillsAction::Refresh => {}
            screens::skills::SkillsAction::Search { query } => {
                self.skills.status_msg = format!("Search: {} (placeholder)", query);
            }
            screens::skills::SkillsAction::InstallSkill { id } => {
                self.skills.status_msg = format!("Skill {} installed (placeholder)", id);
            }
            screens::skills::SkillsAction::UninstallSkill { id } => {
                self.skills.status_msg = format!("Skill {} uninstalled (placeholder)", id);
            }
            screens::skills::SkillsAction::ToggleSkill { id, enabled } => {
                self.skills.status_msg = format!("Skill {} toggled (placeholder)", id);
            }
            screens::skills::SkillsAction::UpdateSkill { id } => {
                self.skills.status_msg = format!("Skill {} updated (placeholder)", id);
            }
        }
    }

    fn handle_hands_action(&mut self, action: screens::hands::HandsAction) {
        match action {
            screens::hands::HandsAction::Continue => {}
            screens::hands::HandsAction::Back => {}
            screens::hands::HandsAction::Refresh => {}
            screens::hands::HandsAction::ActivateHand { id } => {
                self.hands.status_msg = format!("Hand {} activated (placeholder)", id);
            }
            screens::hands::HandsAction::PauseHand { id } => {
                self.hands.status_msg = format!("Hand {} paused (placeholder)", id);
            }
            screens::hands::HandsAction::DeactivateHand { id } => {
                self.hands.status_msg = format!("Hand {} deactivated (placeholder)", id);
            }
            screens::hands::HandsAction::ResumeHand { id } => {
                self.hands.status_msg = format!("Hand {} resumed (placeholder)", id);
            }
        }
    }

    fn handle_extensions_action(&mut self, action: screens::extensions::ExtensionsAction) {
        match action {
            screens::extensions::ExtensionsAction::Continue => {}
            screens::extensions::ExtensionsAction::Back => {}
            screens::extensions::ExtensionsAction::Refresh => {}
            screens::extensions::ExtensionsAction::Search { query } => {
                self.extensions.status_msg = format!("Search: {} (placeholder)", query);
            }
            screens::extensions::ExtensionsAction::InstallExtension { id } => {
                self.extensions.status_msg = format!("Extension {} installed (placeholder)", id);
            }
            screens::extensions::ExtensionsAction::RemoveExtension { id } => {
                self.extensions.status_msg = format!("Extension {} removed (placeholder)", id);
            }
            screens::extensions::ExtensionsAction::ReconnectExtension { id } => {
                self.extensions.status_msg = format!("Extension {} reconnected (placeholder)", id);
            }
        }
    }

    fn handle_templates_action(&mut self, action: screens::templates::TemplatesAction) {
        match action {
            screens::templates::TemplatesAction::Continue => {}
            screens::templates::TemplatesAction::Back => {}
            screens::templates::TemplatesAction::Refresh => {}
            screens::templates::TemplatesAction::Search { query } => {
                self.templates.status_msg = format!("Search: {} (placeholder)", query);
            }
            screens::templates::TemplatesAction::SpawnTemplate { id, name } => {
                self.templates.status_msg = format!("Spawned {} from template {} (placeholder)", name, id);
            }
            screens::templates::TemplatesAction::FilterCategory { category } => {
                self.templates.status_msg = format!("Filter: {} (placeholder)", category);
            }
        }
    }

    fn handle_peers_action(&mut self, action: screens::peers::PeersAction) {
        match action {
            screens::peers::PeersAction::Continue => {}
            screens::peers::PeersAction::Back => {}
            screens::peers::PeersAction::Refresh => {
                self.peers.status_msg = "Peers refreshed (placeholder)".to_string();
            }
        }
    }

    fn handle_comms_action(&mut self, action: screens::comms::CommsAction) {
        match action {
            screens::comms::CommsAction::Continue => {}
            screens::comms::CommsAction::Back => {}
            screens::comms::CommsAction::Refresh => {
                self.comms.status_msg = "Comms refreshed (placeholder)".to_string();
            }
            screens::comms::CommsAction::SendMessage { from, to, msg } => {
                self.comms.status_msg = format!("Message sent from {} to {} (placeholder)", from, to);
            }
            screens::comms::CommsAction::PostTask { title, desc, assign } => {
                self.comms.status_msg = format!("Task '{}' posted (placeholder)", title);
            }
        }
    }

    fn handle_security_action(&mut self, action: screens::security::SecurityAction) {
        match action {
            screens::security::SecurityAction::Continue => {}
            screens::security::SecurityAction::Back => {}
            screens::security::SecurityAction::Refresh => {}
            screens::security::SecurityAction::VerifyChain => {
                self.security.status_msg = "Chain verified (placeholder)".to_string();
            }
        }
    }

    fn handle_audit_action(&mut self, action: screens::audit::AuditAction) {
        match action {
            screens::audit::AuditAction::Continue => {}
            screens::audit::AuditAction::Back => {}
            screens::audit::AuditAction::Refresh => {}
            screens::audit::AuditAction::Search { query } => {
                self.audit.status_msg = format!("Search: {} (placeholder)", query);
            }
            screens::audit::AuditAction::VerifyChain { hash } => {
                self.audit.status_msg = format!("Chain {} verified (placeholder)", hash);
            }
        }
    }

    fn handle_usage_action(&mut self, action: screens::usage::UsageAction) {
        match action {
            screens::usage::UsageAction::Continue => {}
            screens::usage::UsageAction::Back => {}
            screens::usage::UsageAction::Refresh => {
                self.usage.status_msg = "Usage stats refreshed (placeholder)".to_string();
            }
        }
    }

    fn handle_settings_action(&mut self, action: screens::settings::SettingsAction) {
        match action {
            screens::settings::SettingsAction::Continue => {}
            screens::settings::SettingsAction::Back => {}
            screens::settings::SettingsAction::Refresh => {}
            screens::settings::SettingsAction::SetApiKey { provider_id, key } => {
                self.settings.status_msg = format!("API key set for {} (placeholder)", provider_id);
            }
            screens::settings::SettingsAction::TestProvider { provider_id } => {
                self.settings.status_msg = format!("Provider {} tested (placeholder)", provider_id);
            }
            screens::settings::SettingsAction::ToggleModel { model_id, enabled } => {
                self.settings.status_msg = format!("Model {} toggled to {} (placeholder)", model_id, if enabled { "enabled" } else { "disabled" });
            }
            screens::settings::SettingsAction::ToggleTool { tool_id, enabled } => {
                self.settings.status_msg = format!("Tool {} toggled (placeholder)", tool_id);
            }
            screens::settings::SettingsAction::SwitchLanguage { language } => {
                self.switch_language(language);
                self.settings.status_msg = "Language changed".to_string();
            }
        }
    }

    fn handle_logs_action(&mut self, action: screens::logs::LogsAction) {
        match action {
            screens::logs::LogsAction::Continue => {}
            screens::logs::LogsAction::Back => {}
            screens::logs::LogsAction::Refresh => {}
            screens::logs::LogsAction::Search { query } => {
                self.logs.status_msg = format!("Search: {} (placeholder)", query);
            }
            screens::logs::LogsAction::ToggleAutoRefresh => {
                self.logs.status_msg = format!("Auto-refresh: {} (placeholder)", if self.logs.auto_refresh { "ON" } else { "OFF" });
            }
        }
    }

    // Drawing

    pub fn draw(&mut self, frame: &mut Frame) {
        draw::draw(self, frame);
    }
}