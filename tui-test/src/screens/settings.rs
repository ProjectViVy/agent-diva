//! Settings screen: Provider/model/tool/language configuration with Providers, Models, Tools, Language tabs.
//!
//! Interaction design 100% copied from AgentDiVA.
//! All data is placeholder/fake for demonstration.

use crate::i18n::{Language, TranslationKey, Translator};
use crate::theme;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, ListState, Padding, Paragraph};
use ratatui::Frame;

// ── Data types (placeholder) ──────────────────────────────────────────────────

#[derive(Clone, Default)]
pub struct ProviderConfig {
    pub id: String,
    pub name: String,
    pub api_base: String,
    pub api_key_set: bool,
    pub models_available: u32,
    pub status: String,
}

#[derive(Clone, Default)]
pub struct ModelConfig {
    pub id: String,
    pub provider_id: String,
    pub display_name: String,
    pub context_window: u32,
    pub cost_per_1k: f64,
    pub enabled: bool,
}

#[derive(Clone, Default)]
pub struct ToolConfig {
    pub id: String,
    pub name: String,
    pub enabled: bool,
    pub requires_approval: bool,
    pub description: String,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SettingsSubTab {
    Providers,
    Models,
    Tools,
    Language,
}

// ── State ───────────────────────────────────────────────────────────────────

pub struct SettingsState {
    pub providers: Vec<ProviderConfig>,
    pub models: Vec<ModelConfig>,
    pub tools: Vec<ToolConfig>,
    pub list_state: ListState,
    pub sub_tab: SettingsSubTab,
    pub loading: bool,
    pub tick: usize,
    // Key input mode
    pub show_key_input: bool,
    pub key_input_provider_id: String,
    pub key_input_value: String,
    // Test provider modal
    pub show_test_modal: bool,
    pub test_provider_id: String,
    pub test_result: String,
    // Model toggle mode
    pub show_model_toggle: bool,
    pub toggle_model_id: String,
    pub toggle_model_name: String,
    // Language selection
    pub language_list_state: ListState,
    // Status
    pub status_msg: String,
}

pub enum SettingsAction {
    Continue,
    Back,
    Refresh,
    SetApiKey { provider_id: String, key: String },
    TestProvider { provider_id: String },
    ToggleModel { model_id: String, enabled: bool },
    ToggleTool { tool_id: String, enabled: bool },
    SwitchLanguage { language: Language },
}

impl SettingsState {
    pub fn new() -> Self {
        let providers = vec![
            ProviderConfig {
                id: "anthropic".to_string(),
                name: "Anthropic".to_string(),
                api_base: "https://api.anthropic.com".to_string(),
                api_key_set: true,
                models_available: 4,
                status: "Active".to_string(),
            },
            ProviderConfig {
                id: "openai".to_string(),
                name: "OpenAI".to_string(),
                api_base: "https://api.openai.com/v1".to_string(),
                api_key_set: true,
                models_available: 6,
                status: "Active".to_string(),
            },
            ProviderConfig {
                id: "deepseek".to_string(),
                name: "DeepSeek".to_string(),
                api_base: "https://api.deepseek.com/v1".to_string(),
                api_key_set: false,
                models_available: 2,
                status: "Missing Key".to_string(),
            },
            ProviderConfig {
                id: "google".to_string(),
                name: "Google AI".to_string(),
                api_base: "https://generativelanguage.googleapis.com".to_string(),
                api_key_set: true,
                models_available: 3,
                status: "Active".to_string(),
            },
        ];

        let models = vec![
            ModelConfig { id: "claude-3-opus".to_string(), provider_id: "anthropic".to_string(), display_name: "Claude 3 Opus".to_string(), context_window: 200000, cost_per_1k: 0.015, enabled: true },
            ModelConfig { id: "claude-3-sonnet".to_string(), provider_id: "anthropic".to_string(), display_name: "Claude 3 Sonnet".to_string(), context_window: 200000, cost_per_1k: 0.003, enabled: true },
            ModelConfig { id: "claude-3-haiku".to_string(), provider_id: "anthropic".to_string(), display_name: "Claude 3 Haiku".to_string(), context_window: 200000, cost_per_1k: 0.00025, enabled: true },
            ModelConfig { id: "gpt-4o".to_string(), provider_id: "openai".to_string(), display_name: "GPT-4o".to_string(), context_window: 128000, cost_per_1k: 0.005, enabled: true },
            ModelConfig { id: "gpt-4o-mini".to_string(), provider_id: "openai".to_string(), display_name: "GPT-4o Mini".to_string(), context_window: 128000, cost_per_1k: 0.00015, enabled: false },
            ModelConfig { id: "deepseek-chat".to_string(), provider_id: "deepseek".to_string(), display_name: "DeepSeek Chat".to_string(), context_window: 64000, cost_per_1k: 0.00014, enabled: false },
        ];

        let tools = vec![
            ToolConfig { id: "file_read".to_string(), name: "File Read".to_string(), enabled: true, requires_approval: false, description: "Read file contents".to_string() },
            ToolConfig { id: "file_write".to_string(), name: "File Write".to_string(), enabled: true, requires_approval: true, description: "Write/modify files".to_string() },
            ToolConfig { id: "shell_exec".to_string(), name: "Shell Exec".to_string(), enabled: true, requires_approval: true, description: "Execute shell commands".to_string() },
            ToolConfig { id: "web_fetch".to_string(), name: "Web Fetch".to_string(), enabled: true, requires_approval: false, description: "Fetch web content".to_string() },
            ToolConfig { id: "spawn_agent".to_string(), name: "Spawn Agent".to_string(), enabled: false, requires_approval: true, description: "Create sub-agents".to_string() },
        ];

        Self {
            providers,
            models,
            tools,
            list_state: ListState::default().with_selected(Some(0)),
            sub_tab: SettingsSubTab::Providers,
            loading: false,
            tick: 0,
            show_key_input: false,
            key_input_provider_id: String::new(),
            key_input_value: String::new(),
            show_test_modal: false,
            test_provider_id: String::new(),
            test_result: String::new(),
            show_model_toggle: false,
            toggle_model_id: String::new(),
            toggle_model_name: String::new(),
            language_list_state: ListState::default().with_selected(Some(0)),
            status_msg: String::new(),
        }
    }

    pub fn tick(&mut self) {
        self.tick = self.tick.wrapping_add(1);
    }

    pub fn handle_key(&mut self, key: KeyEvent, i18n: &Translator) -> SettingsAction {
        if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
            return SettingsAction::Continue;
        }

        if self.show_key_input {
            return self.handle_key_input_key(key);
        }
        if self.show_test_modal {
            match key.code {
                KeyCode::Esc | KeyCode::Enter => self.show_test_modal = false,
                _ => {}
            }
            return SettingsAction::Continue;
        }
        if self.show_model_toggle {
            return self.handle_toggle_key(key);
        }

        match self.sub_tab {
            SettingsSubTab::Providers => self.handle_providers_key(key, i18n),
            SettingsSubTab::Models => self.handle_models_key(key, i18n),
            SettingsSubTab::Tools => self.handle_tools_key(key, i18n),
            SettingsSubTab::Language => self.handle_language_key(key, i18n),
        }
    }

    fn handle_providers_key(&mut self, key: KeyEvent, _i18n: &Translator) -> SettingsAction {
        let total = self.providers.len();
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                if total > 0 {
                    let i = self.list_state.selected().unwrap_or(0);
                    let next = if i == 0 { total - 1 } else { i - 1 };
                    self.list_state.select(Some(next));
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if total > 0 {
                    let i = self.list_state.selected().unwrap_or(0);
                    let next = (i + 1) % total;
                    self.list_state.select(Some(next));
                }
            }
            KeyCode::Char('1') => self.sub_tab = SettingsSubTab::Providers,
            KeyCode::Char('2') => self.sub_tab = SettingsSubTab::Models,
            KeyCode::Char('3') => self.sub_tab = SettingsSubTab::Tools,
            KeyCode::Char('4') => self.sub_tab = SettingsSubTab::Language,
            KeyCode::Char('K') => {
                if let Some(idx) = self.list_state.selected() {
                    if let Some(p) = self.providers.get(idx) {
                        self.key_input_provider_id = p.id.clone();
                        self.key_input_value.clear();
                        self.show_key_input = true;
                    }
                }
            }
            KeyCode::Char('t') => {
                if let Some(idx) = self.list_state.selected() {
                    if let Some(p) = self.providers.get(idx) {
                        self.test_provider_id = p.id.clone();
                        self.test_result = if p.api_key_set { "Connection successful! (placeholder)".to_string() } else { "No API key set. Please set key first.".to_string() };
                        self.show_test_modal = true;
                        return SettingsAction::TestProvider { provider_id: p.id.clone() };
                    }
                }
            }
            KeyCode::Esc => return SettingsAction::Back,
            _ => {}
        }
        SettingsAction::Continue
    }

    fn handle_models_key(&mut self, key: KeyEvent, _i18n: &Translator) -> SettingsAction {
        let total = self.models.len();
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                if total > 0 {
                    let i = self.list_state.selected().unwrap_or(0);
                    let next = if i == 0 { total - 1 } else { i - 1 };
                    self.list_state.select(Some(next));
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if total > 0 {
                    let i = self.list_state.selected().unwrap_or(0);
                    let next = (i + 1) % total;
                    self.list_state.select(Some(next));
                }
            }
            KeyCode::Char('1') => self.sub_tab = SettingsSubTab::Providers,
            KeyCode::Char('2') => self.sub_tab = SettingsSubTab::Models,
            KeyCode::Char('3') => self.sub_tab = SettingsSubTab::Tools,
            KeyCode::Char('4') => self.sub_tab = SettingsSubTab::Language,
            KeyCode::Enter | KeyCode::Char('t') => {
                if let Some(idx) = self.list_state.selected() {
                    if let Some(m) = self.models.get(idx) {
                        return SettingsAction::ToggleModel { model_id: m.id.clone(), enabled: !m.enabled };
                    }
                }
            }
            KeyCode::Esc => return SettingsAction::Back,
            _ => {}
        }
        SettingsAction::Continue
    }

    fn handle_tools_key(&mut self, key: KeyEvent, _i18n: &Translator) -> SettingsAction {
        let total = self.tools.len();
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                if total > 0 {
                    let i = self.list_state.selected().unwrap_or(0);
                    let next = if i == 0 { total - 1 } else { i - 1 };
                    self.list_state.select(Some(next));
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if total > 0 {
                    let i = self.list_state.selected().unwrap_or(0);
                    let next = (i + 1) % total;
                    self.list_state.select(Some(next));
                }
            }
            KeyCode::Char('1') => self.sub_tab = SettingsSubTab::Providers,
            KeyCode::Char('2') => self.sub_tab = SettingsSubTab::Models,
            KeyCode::Char('3') => self.sub_tab = SettingsSubTab::Tools,
            KeyCode::Char('4') => self.sub_tab = SettingsSubTab::Language,
            KeyCode::Enter | KeyCode::Char('t') => {
                if let Some(idx) = self.list_state.selected() {
                    if let Some(t) = self.tools.get(idx) {
                        return SettingsAction::ToggleTool { tool_id: t.id.clone(), enabled: !t.enabled };
                    }
                }
            }
            KeyCode::Char('a') => {
                if let Some(idx) = self.list_state.selected() {
                    if let Some(t) = self.tools.get(idx) {
                        // Toggle approval requirement (placeholder)
                        self.status_msg = format!("Approval toggled for {}", t.name);
                    }
                }
            }
            KeyCode::Esc => return SettingsAction::Back,
            _ => {}
        }
        SettingsAction::Continue
    }

    fn handle_language_key(&mut self, key: KeyEvent, _i18n: &Translator) -> SettingsAction {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                self.language_list_state.select(Some(0));
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.language_list_state.select(Some(1));
            }
            KeyCode::Char('1') => self.sub_tab = SettingsSubTab::Providers,
            KeyCode::Char('2') => self.sub_tab = SettingsSubTab::Models,
            KeyCode::Char('3') => self.sub_tab = SettingsSubTab::Tools,
            KeyCode::Char('4') => self.sub_tab = SettingsSubTab::Language,
            KeyCode::Enter => {
                if let Some(idx) = self.language_list_state.selected() {
                    let lang = match idx {
                        0 => Language::English,
                        1 => Language::Chinese,
                        _ => Language::English,
                    };
                    return SettingsAction::SwitchLanguage { language: lang };
                }
            }
            KeyCode::Esc => return SettingsAction::Back,
            _ => {}
        }
        SettingsAction::Continue
    }

    fn handle_key_input_key(&mut self, key: KeyEvent) -> SettingsAction {
        match key.code {
            KeyCode::Esc => self.show_key_input = false,
            KeyCode::Enter => {
                if !self.key_input_value.is_empty() {
                    self.show_key_input = false;
                    return SettingsAction::SetApiKey {
                        provider_id: self.key_input_provider_id.clone(),
                        key: self.key_input_value.clone(),
                    };
                }
            }
            KeyCode::Char(c) => self.key_input_value.push(c),
            KeyCode::Backspace => { self.key_input_value.pop(); }
            _ => {}
        }
        SettingsAction::Continue
    }

    fn handle_toggle_key(&mut self, key: KeyEvent) -> SettingsAction {
        match key.code {
            KeyCode::Esc => self.show_model_toggle = false,
            KeyCode::Char('y') | KeyCode::Enter => {
                self.show_model_toggle = false;
                return SettingsAction::ToggleModel { model_id: self.toggle_model_id.clone(), enabled: true };
            }
            KeyCode::Char('n') => {
                self.show_model_toggle = false;
                return SettingsAction::ToggleModel { model_id: self.toggle_model_id.clone(), enabled: false };
            }
            _ => {}
        }
        SettingsAction::Continue
    }
}

// ── Drawing ─────────────────────────────────────────────────────────────────

pub fn draw(f: &mut Frame, area: Rect, state: &mut SettingsState, i18n: &Translator) {
    let title = format!(" {} ", i18n.t(TranslationKey::SettingsTitle));
    let block = Block::default()
        .title(Line::from(vec![Span::styled(title, theme::title_style())]))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme::ACCENT))
        .padding(Padding::horizontal(1));

    let inner = block.inner(area);
    f.render_widget(block, area);

    match state.sub_tab {
        SettingsSubTab::Providers => draw_providers(f, inner, state, i18n),
        SettingsSubTab::Models => draw_models(f, inner, state, i18n),
        SettingsSubTab::Tools => draw_tools(f, inner, state, i18n),
        SettingsSubTab::Language => draw_language(f, inner, state, i18n),
    }

    if state.show_key_input { draw_key_input(f, area, state, i18n); }
    if state.show_test_modal { draw_test_modal(f, area, state, i18n); }
}

fn draw_providers(f: &mut Frame, area: Rect, state: &mut SettingsState, i18n: &Translator) {
    let chunks = Layout::vertical([
        Constraint::Length(2),
        Constraint::Min(3),
        Constraint::Length(1),
    ]).split(area);

    draw_tabs(f, chunks[0], state, i18n);
    f.render_widget(
        Paragraph::new(Line::from(vec![Span::styled(
            format!("  {:<12} {:<8} {:<6} {:<30} {}",
                i18n.t(TranslationKey::SettingsHeaderName),
                i18n.t(TranslationKey::SettingsHeaderKey),
                i18n.t(TranslationKey::SettingsHeaderModels),
                i18n.t(TranslationKey::SettingsHeaderApiBase),
                i18n.t(TranslationKey::SettingsHeaderStatus)),
            theme::table_header(),
        )])),
        chunks[0],
    );

    let items: Vec<ListItem> = state.providers.iter().map(|p| {
        let (key_icon, key_style) = if p.api_key_set {
            (i18n.t(TranslationKey::SettingsKeySet), Style::default().fg(theme::GREEN))
        } else {
            (i18n.t(TranslationKey::SettingsKeyMissing), Style::default().fg(theme::RED))
        };
        let (status_badge, status_style) = if p.api_key_set {
            (i18n.t(TranslationKey::SettingsStatusActive), Style::default().fg(theme::GREEN))
        } else {
            (i18n.t(TranslationKey::SettingsStatusMissingKey), Style::default().fg(theme::RED))
        };
        ListItem::new(Line::from(vec![
            Span::styled(format!("  {:<12}", p.name), Style::default().fg(theme::CYAN)),
            Span::styled(format!(" {:<8}", key_icon), key_style),
            Span::styled(format!(" {:<6}", p.models_available), Style::default().fg(theme::TEXT)),
            Span::styled(format!(" {:<30}", truncate(&p.api_base, 29)), theme::dim_style()),
            Span::styled(format!(" {}", status_badge), status_style),
        ]))
    }).collect();

    let list = List::new(items).highlight_style(theme::selected_style()).highlight_symbol("> ");
    f.render_stateful_widget(list, chunks[1], &mut state.list_state);

    f.render_widget(
        Paragraph::new(Line::from(Span::styled(
            format!("  {}  {}  {}  {}",
                i18n.t(TranslationKey::SettingsHintSetKey),
                i18n.t(TranslationKey::SettingsHintTest),
                i18n.t(TranslationKey::SettingsHintTab),
                i18n.t(TranslationKey::SettingsHintBack)),
            theme::hint_style(),
        ))),
        chunks[2],
    );
}

fn draw_models(f: &mut Frame, area: Rect, state: &mut SettingsState, i18n: &Translator) {
    let chunks = Layout::vertical([
        Constraint::Length(2),
        Constraint::Min(3),
        Constraint::Length(1),
    ]).split(area);

    draw_tabs(f, chunks[0], state, i18n);
    f.render_widget(
        Paragraph::new(Line::from(vec![Span::styled(
            format!("  {:<20} {:<12} {:<12} {:<10} {}",
                i18n.t(TranslationKey::SettingsHeaderName),
                i18n.t(TranslationKey::SettingsTabProviders),
                i18n.t(TranslationKey::SettingsHeaderContext),
                i18n.t(TranslationKey::SettingsHeaderCost),
                i18n.t(TranslationKey::SettingsHeaderEnabled)),
            theme::table_header(),
        )])),
        chunks[0],
    );

    let items: Vec<ListItem> = state.models.iter().map(|m| {
        let (enabled_icon, enabled_style) = if m.enabled {
            (i18n.t(TranslationKey::SettingsEnabledOn), Style::default().fg(theme::GREEN))
        } else {
            (i18n.t(TranslationKey::SettingsEnabledOff), Style::default().fg(theme::YELLOW))
        };
        ListItem::new(Line::from(vec![
            Span::styled(format!("  {:<20}", truncate(&m.display_name, 19)), Style::default().fg(theme::CYAN)),
            Span::styled(format!(" {:<12}", m.provider_id), Style::default().fg(theme::PURPLE)),
            Span::styled(format!(" {:<12}", format_ctx(m.context_window)), Style::default().fg(theme::TEXT)),
            Span::styled(format!(" {:<10}", format!("${:.5}", m.cost_per_1k)), Style::default().fg(theme::TEXT)),
            Span::styled(format!(" {}", enabled_icon), enabled_style),
        ]))
    }).collect();

    let list = List::new(items).highlight_style(theme::selected_style()).highlight_symbol("> ");
    f.render_stateful_widget(list, chunks[1], &mut state.list_state);

    f.render_widget(
        Paragraph::new(Line::from(Span::styled(
            format!("  {}  {}  {}",
                i18n.t(TranslationKey::SettingsHintToggle),
                i18n.t(TranslationKey::SettingsHintTab),
                i18n.t(TranslationKey::SettingsHintBack)),
            theme::hint_style(),
        ))),
        chunks[2],
    );
}

fn draw_tools(f: &mut Frame, area: Rect, state: &mut SettingsState, i18n: &Translator) {
    let chunks = Layout::vertical([
        Constraint::Length(2),
        Constraint::Min(3),
        Constraint::Length(1),
    ]).split(area);

    draw_tabs(f, chunks[0], state, i18n);
    f.render_widget(
        Paragraph::new(Line::from(vec![Span::styled(
            format!("  {:<16} {:<8} {:<10} {}",
                i18n.t(TranslationKey::SettingsHeaderName),
                i18n.t(TranslationKey::SettingsHeaderEnabled),
                i18n.t(TranslationKey::SettingsHeaderApproval),
                i18n.t(TranslationKey::SettingsHeaderDescription)),
            theme::table_header(),
        )])),
        chunks[0],
    );

    let items: Vec<ListItem> = state.tools.iter().map(|t| {
        let (enabled_icon, enabled_style) = if t.enabled {
            (i18n.t(TranslationKey::SettingsEnabledOn), Style::default().fg(theme::GREEN))
        } else {
            (i18n.t(TranslationKey::SettingsEnabledOff), Style::default().fg(theme::YELLOW))
        };
        let (approval_icon, approval_style) = if t.requires_approval {
            (i18n.t(TranslationKey::SettingsApprovalReq), Style::default().fg(theme::YELLOW))
        } else {
            (i18n.t(TranslationKey::SettingsApprovalAuto), Style::default().fg(theme::GREEN))
        };
        ListItem::new(Line::from(vec![
            Span::styled(format!("  {:<16}", truncate(&t.name, 15)), Style::default().fg(theme::CYAN)),
            Span::styled(format!(" {:<8}", enabled_icon), enabled_style),
            Span::styled(format!(" {:<10}", approval_icon), approval_style),
            Span::styled(format!(" {}", t.description), theme::dim_style()),
        ]))
    }).collect();

    let list = List::new(items).highlight_style(theme::selected_style()).highlight_symbol("> ");
    f.render_stateful_widget(list, chunks[1], &mut state.list_state);

    f.render_widget(
        Paragraph::new(Line::from(Span::styled(
            format!("  {}  {}  {}  {}",
                i18n.t(TranslationKey::SettingsHintToggle),
                i18n.t(TranslationKey::SettingsHintApproval),
                i18n.t(TranslationKey::SettingsHintTab),
                i18n.t(TranslationKey::SettingsHintBack)),
            theme::hint_style(),
        ))),
        chunks[2],
    );
}

fn draw_tabs(f: &mut Frame, area: Rect, state: &SettingsState, i18n: &Translator) {
    let tab_style = |tab: SettingsSubTab| {
        if state.sub_tab == tab { Style::default().fg(theme::ACCENT).add_modifier(Modifier::BOLD) }
        else { theme::dim_style() }
    };

    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("  ", Style::default()),
            Span::styled(format!("[1] {}", i18n.t(TranslationKey::SettingsTabProviders)), tab_style(SettingsSubTab::Providers)),
            Span::styled("  ", Style::default()),
            Span::styled(format!("[2] {}", i18n.t(TranslationKey::SettingsTabModels)), tab_style(SettingsSubTab::Models)),
            Span::styled("  ", Style::default()),
            Span::styled(format!("[3] {}", i18n.t(TranslationKey::SettingsTabTools)), tab_style(SettingsSubTab::Tools)),
            Span::styled("  ", Style::default()),
            Span::styled(format!("[4] {}", i18n.t(TranslationKey::SettingsTabLanguage)), tab_style(SettingsSubTab::Language)),
        ])),
        area,
    );
}

fn draw_language(f: &mut Frame, area: Rect, state: &mut SettingsState, i18n: &Translator) {
    let chunks = Layout::vertical([
        Constraint::Length(2),
        Constraint::Min(3),
        Constraint::Length(1),
    ]).split(area);

    draw_tabs(f, chunks[0], state, i18n);

    let current_lang = i18n.language();
    let lang_options = [
        (i18n.t(TranslationKey::SettingsLangEn), Language::English),
        (i18n.t(TranslationKey::SettingsLangZh), Language::Chinese),
    ];

    let items: Vec<ListItem> = lang_options.iter().map(|(name, lang)| {
        let is_current = *lang == current_lang;
        let indicator = if is_current {
            format!(" [{}]", i18n.t(TranslationKey::SettingsLangCurrent))
        } else {
            String::new()
        };
        ListItem::new(Line::from(vec![
            Span::styled(format!("  {:<20}", name), Style::default().fg(theme::CYAN)),
            Span::styled(indicator, Style::default().fg(theme::GREEN)),
        ]))
    }).collect();

    let list = List::new(items).highlight_style(theme::selected_style()).highlight_symbol("> ");
    f.render_stateful_widget(list, chunks[1], &mut state.language_list_state);

    f.render_widget(
        Paragraph::new(Line::from(Span::styled(
            format!("  {}  {}",
                i18n.t(TranslationKey::SettingsHintTab),
                i18n.t(TranslationKey::SettingsHintBack)),
            theme::hint_style(),
        ))),
        chunks[2],
    );
}

fn draw_key_input(f: &mut Frame, area: Rect, state: &SettingsState, i18n: &Translator) {
    let modal = centered_rect(50, 6, area);
    f.render_widget(Clear, modal);

    let title = format!(" {} ", i18n.t(TranslationKey::SettingsModalSetKey));
    let block = Block::default()
        .title(Span::styled(title, theme::title_style()))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme::ACCENT))
        .padding(Padding::uniform(1));
    let inner = block.inner(modal);
    f.render_widget(block, modal);

    let chunks = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
    ]).split(inner);

    f.render_widget(Paragraph::new(Span::styled(format!("{} {}", i18n.t(TranslationKey::SettingsHintProvider), state.key_input_provider_id), Style::default().fg(theme::CYAN))), chunks[0]);
    f.render_widget(Paragraph::new(Span::styled(format!("{} {}|", i18n.t(TranslationKey::SettingsHintKey), state.key_input_value), Style::default().fg(theme::TEXT))), chunks[1]);
    f.render_widget(Paragraph::new(Span::styled(format!("[Enter] {}  [Esc] {}", i18n.t(TranslationKey::CommonSave), i18n.t(TranslationKey::CommonCancel)), theme::hint_style())), chunks[2]);
}

fn draw_test_modal(f: &mut Frame, area: Rect, state: &SettingsState, i18n: &Translator) {
    let modal = centered_rect(50, 6, area);
    f.render_widget(Clear, modal);

    let title = format!(" {} ", i18n.t(TranslationKey::SettingsModalTestProvider));
    let block = Block::default()
        .title(Span::styled(title, theme::title_style()))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme::ACCENT))
        .padding(Padding::uniform(1));
    let inner = block.inner(modal);
    f.render_widget(block, modal);

    let chunks = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(2),
        Constraint::Length(1),
    ]).split(inner);

    f.render_widget(Paragraph::new(Span::styled(format!("{} {}", i18n.t(TranslationKey::SettingsTesting), state.test_provider_id), Style::default().fg(theme::CYAN))), chunks[0]);
    f.render_widget(Paragraph::new(Span::styled(state.test_result.clone(), Style::default().fg(theme::TEXT))), chunks[1]);
    f.render_widget(Paragraph::new(Span::styled("[Enter/Esc] Close", theme::hint_style())), chunks[2]);
}

fn format_ctx(ctx: u32) -> String {
    if ctx >= 1000 { format!("{:.0}K", ctx as f64 / 1000.0) } else { ctx.to_string() }
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max { s.to_string() } else { format!("{}...", &s[..max.saturating_sub(3)]) }
}

fn centered_rect(percent_x: u16, height: u16, area: Rect) -> Rect {
    let w = area.width * percent_x / 100;
    let x = area.x + (area.width.saturating_sub(w)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;
    Rect::new(x, y, w, height.min(area.height))
}