//! Agent selection + creation: list running agents, template picker, custom builder.
//!
//! Cloned from AgentDiVA TUI screens/agents.rs with placeholder data.

use crate::i18n::Translator;
use crate::theme;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::{Alignment, Constraint, Flex, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Padding, Paragraph};
use ratatui::Frame;

/// Available built-in tools for the custom agent builder.
const TOOL_OPTIONS: &[(&str, &str)] = &[
    ("file_read", "Read files"),
    ("file_write", "Write files"),
    ("file_list", "List directory contents"),
    ("memory_store", "Store data in agent memory"),
    ("memory_recall", "Recall data from memory"),
    ("web_fetch", "Fetch web pages"),
    ("shell_exec", "Execute shell commands"),
    ("agent_send", "Send messages to other agents"),
    ("agent_list", "List running agents"),
];

const DEFAULT_TOOLS: &[bool] = &[true, false, true, true, true, true, false, false, false];

/// Placeholder templates
const PLACEHOLDER_TEMPLATES: &[(&str, &str, &str)] = &[
    ("Coder", "Code assistant", "coder"),
    ("Researcher", "Web research agent", "researcher"),
    ("Writer", "Content generation", "writer"),
];

#[derive(Clone, PartialEq, Eq)]
pub enum AgentSubScreen {
    AgentList,
    AgentDetail,
    CreateMethod,
    TemplatePicker,
    CustomName,
    CustomDesc,
    CustomPrompt,
    CustomTools,
    CustomSkills,
    CustomMcpServers,
    EditSkills,
    EditMcpServers,
    Spawning,
}

pub struct AgentSelectState {
    pub sub: AgentSubScreen,
    pub list: ListState,
    pub daemon_agents: Vec<DaemonAgent>,
    pub search_active: bool,
    pub search_query: String,
    filtered_indices: Vec<usize>,
    pub detail: Option<AgentDetail>,
    pub create_method_list: ListState,
    pub templates: Vec<AgentTemplate>,
    pub template_list: ListState,
    pub custom_name: String,
    pub custom_desc: String,
    pub custom_prompt: String,
    pub tool_checks: Vec<bool>,
    pub tool_cursor: usize,
    pub available_skills: Vec<(String, bool)>,
    pub skill_cursor: usize,
    pub available_mcp: Vec<(String, bool)>,
    pub mcp_cursor: usize,
    pub status_msg: String,
    pub spinner_frame: usize,
}

#[derive(Clone)]
pub struct DaemonAgent {
    pub id: String,
    pub name: String,
    pub state: String,
    pub provider: String,
    pub model: String,
}

#[derive(Clone, Default)]
pub struct AgentDetail {
    pub id: String,
    pub name: String,
    pub state: String,
    pub model: String,
    pub provider: String,
    pub skills: Vec<String>,
    pub mcp_servers: Vec<String>,
}

#[derive(Clone)]
pub struct AgentTemplate {
    pub name: String,
    pub description: String,
    pub content: String,
}

pub enum AgentAction {
    Continue,
    CreatedManifest(String),
    Back,
    ChatWithAgent { id: String, name: String },
    KillAgent(String),
    UpdateSkills { id: String, skills: Vec<String> },
    UpdateMcpServers { id: String, servers: Vec<String> },
    FetchAgentSkills(String),
    FetchAgentMcpServers(String),
}

impl AgentSelectState {
    pub fn new() -> Self {
        Self {
            sub: AgentSubScreen::AgentList,
            list: ListState::default(),
            daemon_agents: placeholder_agents(),
            search_active: false,
            search_query: String::new(),
            filtered_indices: Vec::new(),
            detail: None,
            create_method_list: ListState::default(),
            templates: placeholder_templates(),
            template_list: ListState::default(),
            custom_name: String::new(),
            custom_desc: String::new(),
            custom_prompt: String::new(),
            tool_checks: DEFAULT_TOOLS.to_vec(),
            tool_cursor: 0,
            available_skills: Vec::new(),
            skill_cursor: 0,
            available_mcp: Vec::new(),
            mcp_cursor: 0,
            status_msg: String::new(),
            spinner_frame: 0,
        }
    }

    pub fn reset(&mut self) {
        self.sub = AgentSubScreen::AgentList;
        self.list.select(Some(0));
        self.create_method_list.select(Some(0));
        self.template_list.select(Some(0));
        self.custom_name.clear();
        self.custom_desc.clear();
        self.custom_prompt.clear();
        self.tool_checks = DEFAULT_TOOLS.to_vec();
        self.tool_cursor = 0;
        self.available_skills.clear();
        self.skill_cursor = 0;
        self.available_mcp.clear();
        self.mcp_cursor = 0;
        self.status_msg.clear();
        self.search_active = false;
        self.search_query.clear();
        self.filtered_indices.clear();
        self.detail = None;
    }

    pub fn tick(&mut self) {
        self.spinner_frame = (self.spinner_frame + 1) % theme::SPINNER_FRAMES.len();
    }

    fn visible_count(&self) -> usize {
        if self.search_query.is_empty() {
            self.daemon_agents.len() + 1
        } else {
            self.filtered_indices.len() + 1
        }
    }

    fn rebuild_filter(&mut self) {
        self.filtered_indices.clear();
        if self.search_query.is_empty() {
            return;
        }
        let q = self.search_query.to_lowercase();
        for i in 0..self.daemon_agents.len() {
            let a = &self.daemon_agents[i];
            if a.name.to_lowercase().contains(&q)
                || a.model.to_lowercase().contains(&q)
            {
                self.filtered_indices.push(i);
            }
        }
    }

    fn visible_to_combined(&self, visible_idx: usize) -> Option<usize> {
        if self.search_query.is_empty() {
            if visible_idx < self.daemon_agents.len() {
                Some(visible_idx)
            } else {
                None // "Create new"
            }
        } else if visible_idx < self.filtered_indices.len() {
            Some(self.filtered_indices[visible_idx])
        } else {
            None
        }
    }

    fn build_detail(&self, idx: usize) -> AgentDetail {
        let a = &self.daemon_agents[idx];
        AgentDetail {
            id: a.id.clone(),
            name: a.name.clone(),
            state: a.state.clone(),
            model: a.model.clone(),
            provider: a.provider.clone(),
            skills: vec!["coding".to_string()],
            mcp_servers: vec!["filesystem".to_string()],
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> AgentAction {
        if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
            return AgentAction::Back;
        }

        match self.sub {
            AgentSubScreen::AgentList => self.handle_agent_list(key),
            AgentSubScreen::AgentDetail => self.handle_detail(key),
            AgentSubScreen::CreateMethod => self.handle_create_method(key),
            AgentSubScreen::TemplatePicker => self.handle_template_picker(key),
            AgentSubScreen::CustomName => self.handle_custom_name(key),
            AgentSubScreen::CustomDesc => self.handle_custom_desc(key),
            AgentSubScreen::CustomPrompt => self.handle_custom_prompt(key),
            AgentSubScreen::CustomTools => self.handle_custom_tools(key),
            AgentSubScreen::CustomSkills => self.handle_custom_skills(key),
            AgentSubScreen::CustomMcpServers => self.handle_custom_mcp_servers(key),
            AgentSubScreen::EditSkills => self.handle_edit_skills(key),
            AgentSubScreen::EditMcpServers => self.handle_edit_mcp_servers(key),
            AgentSubScreen::Spawning => AgentAction::Continue,
        }
    }

    fn handle_agent_list(&mut self, key: KeyEvent) -> AgentAction {
        if self.search_active {
            match key.code {
                KeyCode::Esc => {
                    self.search_active = false;
                    self.search_query.clear();
                    self.rebuild_filter();
                    self.list.select(Some(0));
                    return AgentAction::Continue;
                }
                KeyCode::Enter => {
                    self.search_active = false;
                    return AgentAction::Continue;
                }
                KeyCode::Char(c) => {
                    self.search_query.push(c);
                    self.rebuild_filter();
                    self.list.select(Some(0));
                    return AgentAction::Continue;
                }
                KeyCode::Backspace => {
                    self.search_query.pop();
                    self.rebuild_filter();
                    self.list.select(Some(0));
                    return AgentAction::Continue;
                }
                _ => return AgentAction::Continue,
            }
        }

        let total = self.visible_count();
        if total == 0 {
            return AgentAction::Continue;
        }

        match key.code {
            KeyCode::Esc => return AgentAction::Back,
            KeyCode::Char('/') => {
                self.search_active = true;
                self.search_query.clear();
                return AgentAction::Continue;
            }
            KeyCode::Up | KeyCode::Char('k') => {
                let i = self.list.selected().unwrap_or(0);
                let next = if i == 0 { total - 1 } else { i - 1 };
                self.list.select(Some(next));
            }
            KeyCode::Down | KeyCode::Char('j') => {
                let i = self.list.selected().unwrap_or(0);
                let next = (i + 1) % total;
                self.list.select(Some(next));
            }
            KeyCode::Enter => {
                if let Some(vis_idx) = self.list.selected() {
                    match self.visible_to_combined(vis_idx) {
                        Some(combined) => {
                            self.detail = Some(self.build_detail(combined));
                            self.sub = AgentSubScreen::AgentDetail;
                        }
                        None => {
                            self.create_method_list.select(Some(0));
                            self.sub = AgentSubScreen::CreateMethod;
                        }
                    }
                }
            }
            _ => {}
        }
        AgentAction::Continue
    }

    fn handle_detail(&mut self, key: KeyEvent) -> AgentAction {
        match key.code {
            KeyCode::Esc => {
                self.sub = AgentSubScreen::AgentList;
            }
            KeyCode::Char('c') => {
                if let Some(ref detail) = self.detail {
                    return AgentAction::ChatWithAgent {
                        id: detail.id.clone(),
                        name: detail.name.clone(),
                    };
                }
            }
            KeyCode::Char('k') => {
                if let Some(ref detail) = self.detail {
                    return AgentAction::KillAgent(detail.id.clone());
                }
            }
            KeyCode::Char('s') => {
                if let Some(ref detail) = self.detail {
                    let id = detail.id.clone();
                    self.sub = AgentSubScreen::EditSkills;
                    return AgentAction::FetchAgentSkills(id);
                }
            }
            KeyCode::Char('m') => {
                if let Some(ref detail) = self.detail {
                    let id = detail.id.clone();
                    self.sub = AgentSubScreen::EditMcpServers;
                    return AgentAction::FetchAgentMcpServers(id);
                }
            }
            _ => {}
        }
        AgentAction::Continue
    }

    fn handle_create_method(&mut self, key: KeyEvent) -> AgentAction {
        match key.code {
            KeyCode::Esc => {
                self.sub = AgentSubScreen::AgentList;
            }
            KeyCode::Up | KeyCode::Char('k') => {
                let i = self.create_method_list.selected().unwrap_or(0);
                self.create_method_list.select(Some(if i == 0 { 1 } else { 0 }));
            }
            KeyCode::Down | KeyCode::Char('j') => {
                let i = self.create_method_list.selected().unwrap_or(0);
                self.create_method_list.select(Some(if i == 0 { 1 } else { 0 }));
            }
            KeyCode::Enter => {
                match self.create_method_list.selected() {
                    Some(0) => {
                        self.template_list.select(Some(0));
                        self.sub = AgentSubScreen::TemplatePicker;
                    }
                    Some(1) => {
                        self.custom_name.clear();
                        self.custom_desc.clear();
                        self.custom_prompt.clear();
                        self.tool_checks = DEFAULT_TOOLS.to_vec();
                        self.tool_cursor = 0;
                        self.sub = AgentSubScreen::CustomName;
                    }
                    _ => {}
                }
            }
            _ => {}
        }
        AgentAction::Continue
    }

    fn handle_template_picker(&mut self, key: KeyEvent) -> AgentAction {
        match key.code {
            KeyCode::Esc => {
                self.sub = AgentSubScreen::CreateMethod;
            }
            KeyCode::Up | KeyCode::Char('k') => {
                let i = self.template_list.selected().unwrap_or(0);
                let total = self.templates.len();
                let next = if i == 0 { total.saturating_sub(1) } else { i - 1 };
                self.template_list.select(Some(next));
            }
            KeyCode::Down | KeyCode::Char('j') => {
                let i = self.template_list.selected().unwrap_or(0);
                let next = (i + 1) % self.templates.len().max(1);
                self.template_list.select(Some(next));
            }
            KeyCode::Enter => {
                if let Some(idx) = self.template_list.selected() {
                    if idx < self.templates.len() {
                        let toml = self.templates[idx].content.clone();
                        return AgentAction::CreatedManifest(toml);
                    }
                }
            }
            _ => {}
        }
        AgentAction::Continue
    }

    fn handle_custom_name(&mut self, key: KeyEvent) -> AgentAction {
        match key.code {
            KeyCode::Esc => {
                self.sub = AgentSubScreen::CreateMethod;
            }
            KeyCode::Enter => {
                if !self.custom_name.is_empty() {
                    if self.custom_desc.is_empty() {
                        self.custom_desc = format!("A custom {} agent", self.custom_name);
                    }
                    self.sub = AgentSubScreen::CustomDesc;
                }
            }
            KeyCode::Char(c) => {
                self.custom_name.push(c);
            }
            KeyCode::Backspace => {
                self.custom_name.pop();
            }
            _ => {}
        }
        AgentAction::Continue
    }

    fn handle_custom_desc(&mut self, key: KeyEvent) -> AgentAction {
        match key.code {
            KeyCode::Esc => {
                self.sub = AgentSubScreen::CustomName;
            }
            KeyCode::Enter => {
                if self.custom_prompt.is_empty() {
                    self.custom_prompt = format!("You are {}, a helpful agent.", self.custom_name);
                }
                self.sub = AgentSubScreen::CustomPrompt;
            }
            KeyCode::Char(c) => {
                self.custom_desc.push(c);
            }
            KeyCode::Backspace => {
                self.custom_desc.pop();
            }
            _ => {}
        }
        AgentAction::Continue
    }

    fn handle_custom_prompt(&mut self, key: KeyEvent) -> AgentAction {
        match key.code {
            KeyCode::Esc => {
                self.sub = AgentSubScreen::CustomDesc;
            }
            KeyCode::Enter => {
                self.sub = AgentSubScreen::CustomTools;
            }
            KeyCode::Char(c) => {
                self.custom_prompt.push(c);
            }
            KeyCode::Backspace => {
                self.custom_prompt.pop();
            }
            _ => {}
        }
        AgentAction::Continue
    }

    fn handle_custom_tools(&mut self, key: KeyEvent) -> AgentAction {
        match key.code {
            KeyCode::Esc => {
                self.sub = AgentSubScreen::CustomPrompt;
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if self.tool_cursor > 0 {
                    self.tool_cursor -= 1;
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.tool_cursor < TOOL_OPTIONS.len() - 1 {
                    self.tool_cursor += 1;
                }
            }
            KeyCode::Char(' ') => {
                self.tool_checks[self.tool_cursor] = !self.tool_checks[self.tool_cursor];
            }
            KeyCode::Enter => {
                self.skill_cursor = 0;
                self.sub = AgentSubScreen::CustomSkills;
            }
            _ => {}
        }
        AgentAction::Continue
    }

    fn handle_custom_skills(&mut self, key: KeyEvent) -> AgentAction {
        let len = self.available_skills.len();
        match key.code {
            KeyCode::Esc => {
                self.sub = AgentSubScreen::CustomTools;
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if self.skill_cursor > 0 {
                    self.skill_cursor -= 1;
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if len > 0 && self.skill_cursor < len - 1 {
                    self.skill_cursor += 1;
                }
            }
            KeyCode::Char(' ') => {
                if len > 0 {
                    self.available_skills[self.skill_cursor].1 = !self.available_skills[self.skill_cursor].1;
                }
            }
            KeyCode::Enter => {
                self.mcp_cursor = 0;
                self.sub = AgentSubScreen::CustomMcpServers;
            }
            _ => {}
        }
        AgentAction::Continue
    }

    fn handle_custom_mcp_servers(&mut self, key: KeyEvent) -> AgentAction {
        let len = self.available_mcp.len();
        match key.code {
            KeyCode::Esc => {
                self.sub = AgentSubScreen::CustomSkills;
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if self.mcp_cursor > 0 {
                    self.mcp_cursor -= 1;
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if len > 0 && self.mcp_cursor < len - 1 {
                    self.mcp_cursor += 1;
                }
            }
            KeyCode::Char(' ') => {
                if len > 0 {
                    self.available_mcp[self.mcp_cursor].1 = !self.available_mcp[self.mcp_cursor].1;
                }
            }
            KeyCode::Enter => {
                let toml = self.build_custom_toml();
                return AgentAction::CreatedManifest(toml);
            }
            _ => {}
        }
        AgentAction::Continue
    }

    fn handle_edit_skills(&mut self, key: KeyEvent) -> AgentAction {
        let len = self.available_skills.len();
        match key.code {
            KeyCode::Esc => {
                self.sub = AgentSubScreen::AgentDetail;
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if self.skill_cursor > 0 {
                    self.skill_cursor -= 1;
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if len > 0 && self.skill_cursor < len - 1 {
                    self.skill_cursor += 1;
                }
            }
            KeyCode::Char(' ') => {
                if len > 0 {
                    self.available_skills[self.skill_cursor].1 = !self.available_skills[self.skill_cursor].1;
                }
            }
            KeyCode::Enter => {
                if let Some(ref detail) = self.detail {
                    let skills: Vec<String> = self
                        .available_skills
                        .iter()
                        .filter(|(_, checked)| *checked)
                        .map(|(name, _)| name.clone())
                        .collect();
                    return AgentAction::UpdateSkills {
                        id: detail.id.clone(),
                        skills,
                    };
                }
                self.sub = AgentSubScreen::AgentDetail;
            }
            _ => {}
        }
        AgentAction::Continue
    }

    fn handle_edit_mcp_servers(&mut self, key: KeyEvent) -> AgentAction {
        let len = self.available_mcp.len();
        match key.code {
            KeyCode::Esc => {
                self.sub = AgentSubScreen::AgentDetail;
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if self.mcp_cursor > 0 {
                    self.mcp_cursor -= 1;
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if len > 0 && self.mcp_cursor < len - 1 {
                    self.mcp_cursor += 1;
                }
            }
            KeyCode::Char(' ') => {
                if len > 0 {
                    self.available_mcp[self.mcp_cursor].1 = !self.available_mcp[self.mcp_cursor].1;
                }
            }
            KeyCode::Enter => {
                if let Some(ref detail) = self.detail {
                    let servers: Vec<String> = self
                        .available_mcp
                        .iter()
                        .filter(|(_, checked)| *checked)
                        .map(|(name, _)| name.clone())
                        .collect();
                    return AgentAction::UpdateMcpServers {
                        id: detail.id.clone(),
                        servers,
                    };
                }
                self.sub = AgentSubScreen::AgentDetail;
            }
            _ => {}
        }
        AgentAction::Continue
    }

    fn build_custom_toml(&self) -> String {
        let tools_str: String = TOOL_OPTIONS
            .iter()
            .zip(self.tool_checks.iter())
            .filter(|(_, &checked)| checked)
            .map(|((name, _), _)| format!("\"{}\"", name))
            .collect::<Vec<_>>()
            .join(", ");

        let skills_str: String = self
            .available_skills
            .iter()
            .filter(|(_, checked)| *checked)
            .map(|(name, _)| format!("\"{}\"", name))
            .collect::<Vec<_>>()
            .join(", ");

        let mcp_str: String = self
            .available_mcp
            .iter()
            .filter(|(_, checked)| *checked)
            .map(|(name, _)| format!("\"{}\"", name))
            .collect::<Vec<_>>()
            .join(", ");

        format!(
            r#"name = "{name}"
description = "{desc}"

[model]
provider = "placeholder"
model = "placeholder"

[capabilities]
tools = [{tools_str}]
skills = [{skills_str}]
mcp_servers = [{mcp_str}]
"#,
            name = self.custom_name,
            desc = self.custom_desc,
        )
    }
}

// Placeholder data

fn placeholder_agents() -> Vec<DaemonAgent> {
    vec![
        DaemonAgent {
            id: "agent-001".into(),
            name: "Coder".into(),
            state: "running".into(),
            provider: "anthropic".into(),
            model: "claude-3-5-sonnet".into(),
        },
        DaemonAgent {
            id: "agent-002".into(),
            name: "Researcher".into(),
            state: "idle".into(),
            provider: "openai".into(),
            model: "gpt-4o".into(),
        },
        DaemonAgent {
            id: "agent-003".into(),
            name: "Writer".into(),
            state: "suspended".into(),
            provider: "deepseek".into(),
            model: "deepseek-chat".into(),
        },
    ]
}

fn placeholder_templates() -> Vec<AgentTemplate> {
    PLACEHOLDER_TEMPLATES
        .iter()
        .map(|(name, desc, _)| AgentTemplate {
            name: name.to_string(),
            description: desc.to_string(),
            content: format!("name = \"{}\"\n[model]\nprovider = \"placeholder\"", name),
        })
        .collect()
}

// Drawing

pub fn draw(f: &mut Frame, area: Rect, state: &mut AgentSelectState, _i18n: &Translator) {
    f.render_widget(Block::default(), area);

    match state.sub {
        AgentSubScreen::AgentDetail => {
            draw_detail(f, area, state, _i18n);
            return;
        }
        AgentSubScreen::AgentList => {
            draw_agent_list_full(f, area, state, _i18n);
            return;
        }
        AgentSubScreen::EditSkills | AgentSubScreen::EditMcpServers => {
            draw_edit_allowlist(f, area, state);
            return;
        }
        _ => {}
    }

    let sub_title = match state.sub {
        AgentSubScreen::AgentList | AgentSubScreen::AgentDetail | AgentSubScreen::EditSkills | AgentSubScreen::EditMcpServers => unreachable!(),
        AgentSubScreen::CreateMethod => "Create Agent",
        AgentSubScreen::TemplatePicker => "Templates",
        AgentSubScreen::CustomName => "Custom \u{2014} Name",
        AgentSubScreen::CustomDesc => "Custom \u{2014} Description",
        AgentSubScreen::CustomPrompt => "Custom \u{2014} System Prompt",
        AgentSubScreen::CustomTools => "Custom \u{2014} Tools",
        AgentSubScreen::CustomSkills => "Custom \u{2014} Skills",
        AgentSubScreen::CustomMcpServers => "Custom \u{2014} MCP Servers",
        AgentSubScreen::Spawning => "Spawning...",
    };

    let card_h = 18u16.min(area.height);
    let card_w = 64u16.min(area.width.saturating_sub(2));
    let [card_area] = Layout::horizontal([Constraint::Length(card_w)])
        .flex(Flex::Center)
        .areas(area);
    let [card_area] = Layout::vertical([Constraint::Length(card_h)])
        .flex(Flex::Center)
        .areas(card_area);

    let block = Block::default()
        .title(Line::from(vec![Span::styled(
            format!(" {sub_title} "),
            theme::title_style(),
        )]))
        .title_alignment(Alignment::Left)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme::ACCENT))
        .padding(Padding::horizontal(1));

    let inner = block.inner(card_area);
    f.render_widget(block, card_area);

    match state.sub {
        AgentSubScreen::CreateMethod => draw_create_method(f, inner, state),
        AgentSubScreen::TemplatePicker => draw_template_picker(f, inner, state),
        AgentSubScreen::CustomName => draw_text_input(f, inner, "Agent name:", &state.custom_name, "my-agent"),
        AgentSubScreen::CustomDesc => draw_text_input(f, inner, "Description:", &state.custom_desc, "A custom agent"),
        AgentSubScreen::CustomPrompt => draw_text_input(f, inner, "System prompt:", &state.custom_prompt, "You are a helpful agent."),
        AgentSubScreen::CustomTools => draw_tool_select(f, inner, state),
        AgentSubScreen::CustomSkills => draw_skill_select(f, inner, state),
        AgentSubScreen::CustomMcpServers => draw_mcp_select(f, inner, state),
        AgentSubScreen::Spawning => {
            let spinner = theme::SPINNER_FRAMES[state.spinner_frame % theme::SPINNER_FRAMES.len()];
            let msg = Paragraph::new(Line::from(vec![Span::styled(
                format!("  {spinner} Spawning agent..."),
                theme::dim_style(),
            )]));
            f.render_widget(msg, inner);
        }
        _ => {}
    }
}

fn draw_agent_list_full(f: &mut Frame, area: Rect, state: &mut AgentSelectState, i18n: &Translator) {
    let block = Block::default()
        .title(Line::from(vec![Span::styled(" Agents ", theme::title_style())]))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme::ACCENT))
        .padding(Padding::horizontal(1));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let has_search = state.search_active || !state.search_query.is_empty();
    let search_height = if has_search { 1 } else { 0 };

    let chunks = Layout::vertical([
        Constraint::Length(search_height),
        Constraint::Length(2),
        Constraint::Min(3),
        Constraint::Length(1),
    ])
    .split(inner);

    // Search bar
    if has_search {
        let cursor = if state.search_active { "\u{2588}" } else { "" };
        f.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled("  / ", Style::default().fg(theme::YELLOW)),
                Span::styled(&state.search_query, theme::input_style()),
                Span::styled(cursor, Style::default().fg(theme::GREEN).add_modifier(Modifier::SLOW_BLINK)),
            ])),
            chunks[0],
        );
    }

    // Table header
    f.render_widget(
        Paragraph::new(Line::from(vec![Span::styled(
            format!("  {:<5} {:<18} {:<24} {}", "State", "Name", "Model", "ID"),
            theme::table_header(),
        )])),
        chunks[1],
    );

    // Agent list
    let agent_indices: Vec<usize> = if state.search_query.is_empty() {
        (0..state.daemon_agents.len()).collect()
    } else {
        state.filtered_indices.clone()
    };

    let mut items: Vec<ListItem> = agent_indices
        .iter()
        .map(|&combined| {
            let a = &state.daemon_agents[combined];
            let (badge, badge_style) = theme::state_badge(&a.state, i18n);
            ListItem::new(Line::from(vec![
                Span::styled(format!("  {:<5}", badge), badge_style),
                Span::styled(format!(" {:<18}", truncate(&a.name, 17)), Style::default().fg(theme::CYAN)),
                Span::styled(format!(" {:<24}", truncate(&format!("{}/{}", a.provider, a.model), 23)), Style::default().fg(theme::YELLOW)),
                Span::styled(format!(" {}", truncate(&a.id, 12)), theme::dim_style()),
            ]))
        })
        .collect();

    items.push(ListItem::new(Line::from(vec![Span::styled(
        "  + Create new agent",
        Style::default().fg(theme::GREEN).add_modifier(Modifier::BOLD),
    )])));

    let list = List::new(items)
        .highlight_style(theme::selected_style())
        .highlight_symbol("> ");

    f.render_stateful_widget(list, chunks[2], &mut state.list);

    // Hints
    let hints = if state.search_active {
        "  [Type] Filter  [Enter] Accept  [Esc] Cancel search"
    } else {
        "  [\u{2191}\u{2193}] Navigate  [Enter] Detail  [/] Search  [Esc] Back"
    };
    f.render_widget(Paragraph::new(Line::from(vec![Span::styled(hints, theme::hint_style())])), chunks[3]);
}

fn draw_detail(f: &mut Frame, area: Rect, state: &AgentSelectState, i18n: &Translator) {
    let block = Block::default()
        .title(Line::from(vec![Span::styled(" Agent Detail ", theme::title_style())]))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme::ACCENT))
        .padding(Padding::horizontal(1));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let chunks = Layout::vertical([Constraint::Min(10), Constraint::Length(1)]).split(inner);

    match &state.detail {
        Some(detail) => {
            let (badge, badge_style) = theme::state_badge(&detail.state, i18n);
            let lines = vec![
                Line::from(""),
                Line::from(vec![
                    Span::raw("  ID:       "),
                    Span::styled(&detail.id, theme::dim_style()),
                ]),
                Line::from(vec![
                    Span::raw("  Name:     "),
                    Span::styled(&detail.name, Style::default().fg(theme::CYAN).add_modifier(Modifier::BOLD)),
                ]),
                Line::from(vec![
                    Span::raw("  State:    "),
                    Span::styled(badge, badge_style),
                ]),
                Line::from(vec![
                    Span::raw("  Provider: "),
                    Span::styled(&detail.provider, Style::default().fg(theme::YELLOW)),
                ]),
                Line::from(vec![
                    Span::raw("  Model:    "),
                    Span::styled(&detail.model, Style::default().fg(theme::YELLOW)),
                ]),
                Line::from(vec![
                    Span::raw("  Skills:   "),
                    Span::styled(detail.skills.join(", "), Style::default().fg(theme::CYAN)),
                ]),
                Line::from(vec![
                    Span::raw("  MCP:      "),
                    Span::styled(detail.mcp_servers.join(", "), Style::default().fg(theme::CYAN)),
                ]),
            ];
            f.render_widget(Paragraph::new(lines), chunks[0]);
        }
        None => {
            f.render_widget(Paragraph::new(Span::styled("  No agent selected.", theme::dim_style())), chunks[0]);
        }
    }

    f.render_widget(
        Paragraph::new(Line::from(vec![Span::styled(
            "  [s] Edit skills  [m] Edit MCP  [c] Chat  [k] Kill  [Esc] Back",
            theme::hint_style(),
        )])),
        chunks[1],
    );
}

fn draw_create_method(f: &mut Frame, area: Rect, state: &mut AgentSelectState) {
    let chunks = Layout::vertical([Constraint::Length(2), Constraint::Min(3), Constraint::Length(1)]).split(area);

    let prompt = Paragraph::new("  How would you like to create your agent?");
    f.render_widget(prompt, chunks[0]);

    let items = vec![
        ListItem::new(Line::from(vec![
            Span::raw("  Choose from templates"),
            Span::styled("  (pre-built agents)", theme::dim_style()),
        ])),
        ListItem::new(Line::from(vec![
            Span::raw("  Build custom agent"),
            Span::styled("  (pick name, tools, prompt)", theme::dim_style()),
        ])),
    ];

    let list = List::new(items)
        .highlight_style(theme::selected_style())
        .highlight_symbol("> ");

    f.render_stateful_widget(list, chunks[1], &mut state.create_method_list);

    let hints = Paragraph::new(Line::from(vec![Span::styled(
        "    [\u{2191}\u{2193}] Navigate  [Enter] Select  [Esc] Back",
        theme::hint_style(),
    )]));
    f.render_widget(hints, chunks[2]);
}

fn draw_template_picker(f: &mut Frame, area: Rect, state: &mut AgentSelectState) {
    let chunks = Layout::vertical([Constraint::Min(3), Constraint::Length(1)]).split(area);

    let items: Vec<ListItem> = state
        .templates
        .iter()
        .map(|t| {
            ListItem::new(Line::from(vec![
                Span::styled(format!("  {:<20}", t.name), Style::default().fg(theme::CYAN)),
                Span::styled(&t.description, theme::dim_style()),
            ]))
        })
        .collect();

    let list = List::new(items)
        .highlight_style(theme::selected_style())
        .highlight_symbol("> ");

    f.render_stateful_widget(list, chunks[0], &mut state.template_list);

    let hints = Paragraph::new(Line::from(vec![Span::styled(
        "    [\u{2191}\u{2193}] Navigate  [Enter] Select  [Esc] Back",
        theme::hint_style(),
    )]));
    f.render_widget(hints, chunks[1]);
}

fn draw_text_input(f: &mut Frame, area: Rect, label: &str, value: &str, placeholder: &str) {
    let chunks = Layout::vertical([
        Constraint::Length(2),
        Constraint::Length(1),
        Constraint::Length(2),
        Constraint::Min(0),
        Constraint::Length(1),
    ])
    .split(area);

    let prompt = Paragraph::new(format!("  {label}"));
    f.render_widget(prompt, chunks[0]);

    let display = if value.is_empty() { placeholder } else { value };
    let style = if value.is_empty() { theme::dim_style() } else { theme::input_style() };

    let input = Paragraph::new(Line::from(vec![
        Span::raw("  > "),
        Span::styled(display, style),
        Span::styled("\u{2588}", Style::default().fg(theme::GREEN).add_modifier(Modifier::SLOW_BLINK)),
    ]));
    f.render_widget(input, chunks[1]);

    if value.is_empty() {
        let hint = Paragraph::new(Line::from(vec![Span::styled(
            format!("    placeholder: {placeholder}"),
            theme::dim_style(),
        )]));
        f.render_widget(hint, chunks[2]);
    }

    let hints = Paragraph::new(Line::from(vec![Span::styled(
        "    [Enter] Next  [Esc] Back",
        theme::hint_style(),
    )]));
    f.render_widget(hints, chunks[4]);
}

fn draw_tool_select(f: &mut Frame, area: Rect, state: &AgentSelectState) {
    let chunks = Layout::vertical([Constraint::Length(2), Constraint::Min(3), Constraint::Length(1)]).split(area);

    let prompt = Paragraph::new("  Select tools (Space to toggle):");
    f.render_widget(prompt, chunks[0]);

    let items: Vec<ListItem> = TOOL_OPTIONS
        .iter()
        .zip(state.tool_checks.iter())
        .enumerate()
        .map(|(i, ((name, desc), &checked))| {
            let check = if checked { "\u{25c9}" } else { "\u{25cb}" };
            let highlight = if i == state.tool_cursor {
                Style::default().fg(theme::CYAN).add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            ListItem::new(Line::from(vec![
                Span::styled(format!("  {check} {name:<16}"), highlight),
                Span::styled(*desc, theme::dim_style()),
            ]))
        })
        .collect();

    f.render_widget(List::new(items), chunks[1]);

    let hints = Paragraph::new(Line::from(vec![Span::styled(
        "    [\u{2191}\u{2193}] Navigate  [Space] Toggle  [Enter] Next  [Esc] Back",
        theme::hint_style(),
    )]));
    f.render_widget(hints, chunks[2]);
}

fn draw_skill_select(f: &mut Frame, area: Rect, state: &AgentSelectState) {
    draw_checkbox_list(
        f,
        area,
        "Select skills (none checked = all skills):",
        &state.available_skills,
        state.skill_cursor,
        "    [\u{2191}\u{2193}] Navigate  [Space] Toggle  [Enter] Next  [Esc] Back",
    );
}

fn draw_mcp_select(f: &mut Frame, area: Rect, state: &AgentSelectState) {
    draw_checkbox_list(
        f,
        area,
        "Select MCP servers (none checked = all servers):",
        &state.available_mcp,
        state.mcp_cursor,
        "    [\u{2191}\u{2193}] Navigate  [Space] Toggle  [Enter] Create  [Esc] Back",
    );
}

fn draw_edit_allowlist(f: &mut Frame, area: Rect, state: &AgentSelectState) {
    let (title, items, cursor) = match state.sub {
        AgentSubScreen::EditSkills => (" Edit Skills ", &state.available_skills, state.skill_cursor),
        AgentSubScreen::EditMcpServers => (" Edit MCP Servers ", &state.available_mcp, state.mcp_cursor),
        _ => return,
    };

    let block = Block::default()
        .title(Line::from(vec![Span::styled(title, theme::title_style())]))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme::ACCENT))
        .padding(Padding::horizontal(1));

    let inner = block.inner(area);
    f.render_widget(block, area);

    draw_checkbox_list(
        f,
        inner,
        "Space to toggle, Enter to save (none checked = all):",
        items,
        cursor,
        "    [\u{2191}\u{2193}] Navigate  [Space] Toggle  [Enter] Save  [Esc] Cancel",
    );
}

fn draw_checkbox_list(
    f: &mut Frame,
    area: Rect,
    prompt_text: &str,
    items: &[(String, bool)],
    cursor: usize,
    hints_text: &str,
) {
    let chunks = Layout::vertical([Constraint::Length(2), Constraint::Min(3), Constraint::Length(1)]).split(area);

    let prompt = Paragraph::new(format!("  {prompt_text}"));
    f.render_widget(prompt, chunks[0]);

    if items.is_empty() {
        let msg = Paragraph::new(Span::styled("  (none available)", theme::dim_style()));
        f.render_widget(msg, chunks[1]);
    } else {
        let list_items: Vec<ListItem> = items
            .iter()
            .enumerate()
            .map(|(i, (name, checked))| {
                let check = if *checked { "\u{25c9}" } else { "\u{25cb}" };
                let highlight = if i == cursor {
                    Style::default().fg(theme::CYAN).add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };
                ListItem::new(Line::from(vec![Span::styled(format!("  {check} {name}"), highlight)]))
            })
            .collect();

        f.render_widget(List::new(list_items), chunks[1]);
    }

    let hints = Paragraph::new(Line::from(vec![Span::styled(hints_text, theme::hint_style())]));
    f.render_widget(hints, chunks[2]);
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}\u{2026}", &s[..max.saturating_sub(1)])
    }
}