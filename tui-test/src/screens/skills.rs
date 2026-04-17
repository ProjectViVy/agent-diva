//! Skills screen: Skill management with Installed, ClawHub, and MCP tabs.
//!
//! Interaction design 100% copied from AgentDiVA.
//! All data is placeholder/fake for demonstration.

use crate::i18n::Translator;
use crate::theme;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, ListState, Padding, Paragraph};
use ratatui::Frame;

// ── Data types (placeholder) ──────────────────────────────────────────────────

#[derive(Clone, Default)]
pub struct SkillInfo {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: String,
    pub enabled: bool,
    pub source: String, // "installed", "clawhub", "mcp"
}

// ── State ───────────────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SkillsSubTab {
    Installed,
    ClawHub,
    Mcp,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SkillsSubScreen {
    List,
    Detail,
    Search,
}

pub struct SkillsState {
    pub installed_skills: Vec<SkillInfo>,
    pub clawhub_skills: Vec<SkillInfo>,
    pub mcp_servers: Vec<SkillInfo>,
    pub list_state: ListState,
    pub sub_tab: SkillsSubTab,
    pub sub: SkillsSubScreen,
    pub loading: bool,
    pub tick: usize,
    // Search mode
    pub search_query: String,
    pub search_results: Vec<SkillInfo>,
    // Detail view
    pub detail_skill: Option<SkillInfo>,
    // Install confirmation
    pub show_install_confirm: bool,
    pub install_skill_name: String,
    pub install_skill_id: String,
    // Uninstall confirmation
    pub show_uninstall_confirm: bool,
    pub uninstall_skill_name: String,
    pub uninstall_skill_id: String,
    // Sort options
    pub sort_mode: usize, // 0: name, 1: author, 2: date
    // Status
    pub status_msg: String,
}

pub enum SkillsAction {
    Continue,
    Back,
    Refresh,
    Search { query: String },
    InstallSkill { id: String },
    UninstallSkill { id: String },
    ToggleSkill { id: String, enabled: bool },
    UpdateSkill { id: String },
}

impl SkillsState {
    pub fn new() -> Self {
        // Populate with fake data
        let installed_skills = vec![
            SkillInfo {
                id: "skill-001".to_string(),
                name: "coding".to_string(),
                version: "1.2.3".to_string(),
                description: "Advanced coding assistance with syntax highlighting".to_string(),
                author: "AgentDiVA".to_string(),
                enabled: true,
                source: "installed".to_string(),
            },
            SkillInfo {
                id: "skill-002".to_string(),
                name: "research".to_string(),
                version: "2.0.1".to_string(),
                description: "Web research and summarization capabilities".to_string(),
                author: "AgentDiVA".to_string(),
                enabled: true,
                source: "installed".to_string(),
            },
            SkillInfo {
                id: "skill-003".to_string(),
                name: "debugging".to_string(),
                version: "0.9.5".to_string(),
                description: "Error analysis and debugging tools".to_string(),
                author: "community".to_string(),
                enabled: false,
                source: "installed".to_string(),
            },
        ];

        let clawhub_skills = vec![
            SkillInfo {
                id: "skill-004".to_string(),
                name: "translation".to_string(),
                version: "3.1.0".to_string(),
                description: "Multi-language translation service".to_string(),
                author: "lang-core".to_string(),
                enabled: false,
                source: "clawhub".to_string(),
            },
            SkillInfo {
                id: "skill-005".to_string(),
                name: "image-gen".to_string(),
                version: "1.0.2".to_string(),
                description: "AI image generation integration".to_string(),
                author: "art-ai".to_string(),
                enabled: false,
                source: "clawhub".to_string(),
            },
            SkillInfo {
                id: "skill-006".to_string(),
                name: "data-analysis".to_string(),
                version: "4.2.0".to_string(),
                description: "Statistical analysis and visualization".to_string(),
                author: "data-lab".to_string(),
                enabled: false,
                source: "clawhub".to_string(),
            },
        ];

        let mcp_servers = vec![
            SkillInfo {
                id: "mcp-001".to_string(),
                name: "filesystem".to_string(),
                version: "1.0.0".to_string(),
                description: "Local filesystem access via MCP protocol".to_string(),
                author: "anthropic".to_string(),
                enabled: true,
                source: "mcp".to_string(),
            },
            SkillInfo {
                id: "mcp-002".to_string(),
                name: "github".to_string(),
                version: "1.1.0".to_string(),
                description: "GitHub API integration for repos and issues".to_string(),
                author: "community".to_string(),
                enabled: true,
                source: "mcp".to_string(),
            },
            SkillInfo {
                id: "mcp-003".to_string(),
                name: "slack".to_string(),
                version: "0.8.0".to_string(),
                description: "Slack messaging integration".to_string(),
                author: "community".to_string(),
                enabled: false,
                source: "mcp".to_string(),
            },
        ];

        Self {
            installed_skills,
            clawhub_skills,
            mcp_servers,
            list_state: ListState::default().with_selected(Some(0)),
            sub_tab: SkillsSubTab::Installed,
            sub: SkillsSubScreen::List,
            loading: false,
            tick: 0,
            search_query: String::new(),
            search_results: Vec::new(),
            detail_skill: None,
            show_install_confirm: false,
            install_skill_name: String::new(),
            install_skill_id: String::new(),
            show_uninstall_confirm: false,
            uninstall_skill_name: String::new(),
            uninstall_skill_id: String::new(),
            sort_mode: 0,
            status_msg: String::new(),
        }
    }

    pub fn tick(&mut self) {
        self.tick = self.tick.wrapping_add(1);
    }

    fn current_skills(&self) -> &Vec<SkillInfo> {
        match self.sub_tab {
            SkillsSubTab::Installed => &self.installed_skills,
            SkillsSubTab::ClawHub => &self.clawhub_skills,
            SkillsSubTab::Mcp => &self.mcp_servers,
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> SkillsAction {
        if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
            return SkillsAction::Continue;
        }

        // Confirmation modals
        if self.show_install_confirm {
            return self.handle_install_confirm_key(key);
        }
        if self.show_uninstall_confirm {
            return self.handle_uninstall_confirm_key(key);
        }

        match self.sub {
            SkillsSubScreen::List => self.handle_list_key(key),
            SkillsSubScreen::Detail => self.handle_detail_key(key),
            SkillsSubScreen::Search => self.handle_search_key(key),
        }
    }

    fn handle_list_key(&mut self, key: KeyEvent) -> SkillsAction {
        let skills = self.current_skills();
        let total = skills.len();

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
            KeyCode::Char('1') => {
                self.sub_tab = SkillsSubTab::Installed;
                self.list_state.select(Some(0));
            }
            KeyCode::Char('2') => {
                self.sub_tab = SkillsSubTab::ClawHub;
                self.list_state.select(Some(0));
            }
            KeyCode::Char('3') => {
                self.sub_tab = SkillsSubTab::Mcp;
                self.list_state.select(Some(0));
            }
            KeyCode::Char('/') => {
                self.sub = SkillsSubScreen::Search;
                self.search_query.clear();
                self.search_results.clear();
            }
            KeyCode::Char('s') => {
                // Sort toggle
                self.sort_mode = (self.sort_mode + 1) % 3;
            }
            KeyCode::Enter => {
                if let Some(idx) = self.list_state.selected() {
                    if let Some(skill) = self.current_skills().get(idx) {
                        self.detail_skill = Some(skill.clone());
                        self.sub = SkillsSubScreen::Detail;
                    }
                }
            }
            KeyCode::Char('i') => {
                if self.sub_tab == SkillsSubTab::ClawHub {
                    if let Some(idx) = self.list_state.selected() {
                        if let Some(skill) = self.clawhub_skills.get(idx) {
                            self.install_skill_id = skill.id.clone();
                            self.install_skill_name = skill.name.clone();
                            self.show_install_confirm = true;
                        }
                    }
                }
            }
            KeyCode::Char('u') => {
                if self.sub_tab == SkillsSubTab::Installed {
                    if let Some(idx) = self.list_state.selected() {
                        if let Some(skill) = self.installed_skills.get(idx) {
                            self.uninstall_skill_id = skill.id.clone();
                            self.uninstall_skill_name = skill.name.clone();
                            self.show_uninstall_confirm = true;
                        }
                    }
                }
            }
            KeyCode::Char('t') => {
                if self.sub_tab == SkillsSubTab::Installed || self.sub_tab == SkillsSubTab::Mcp {
                    if let Some(idx) = self.list_state.selected() {
                        if let Some(skill) = self.current_skills().get(idx) {
                            return SkillsAction::ToggleSkill {
                                id: skill.id.clone(),
                                enabled: !skill.enabled,
                            };
                        }
                    }
                }
            }
            KeyCode::Esc => return SkillsAction::Back,
            _ => {}
        }
        SkillsAction::Continue
    }

    fn handle_detail_key(&mut self, key: KeyEvent) -> SkillsAction {
        match key.code {
            KeyCode::Esc => {
                self.sub = SkillsSubScreen::List;
                self.detail_skill = None;
            }
            KeyCode::Char('u') => {
                if let Some(skill) = &self.detail_skill {
                    if skill.source == "installed" {
                        self.uninstall_skill_id = skill.id.clone();
                        self.uninstall_skill_name = skill.name.clone();
                        self.show_uninstall_confirm = true;
                    }
                }
            }
            KeyCode::Char('t') => {
                if let Some(skill) = &self.detail_skill {
                    return SkillsAction::ToggleSkill {
                        id: skill.id.clone(),
                        enabled: !skill.enabled,
                    };
                }
            }
            _ => {}
        }
        SkillsAction::Continue
    }

    fn handle_search_key(&mut self, key: KeyEvent) -> SkillsAction {
        match key.code {
            KeyCode::Esc => {
                self.sub = SkillsSubScreen::List;
                self.search_query.clear();
            }
            KeyCode::Enter => {
                // Perform search (placeholder - just filter locally)
                let query = self.search_query.to_lowercase();
                self.search_results = self.clawhub_skills.iter()
                    .filter(|s| s.name.to_lowercase().contains(&query) || s.description.to_lowercase().contains(&query))
                    .cloned()
                    .collect();
                if !self.search_results.is_empty() {
                    self.list_state.select(Some(0));
                }
            }
            KeyCode::Char(c) => {
                self.search_query.push(c);
            }
            KeyCode::Backspace => {
                self.search_query.pop();
            }
            _ => {}
        }
        SkillsAction::Continue
    }

    fn handle_install_confirm_key(&mut self, key: KeyEvent) -> SkillsAction {
        match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                self.show_install_confirm = false;
                let id = self.install_skill_id.clone();
                self.sub = SkillsSubScreen::List;
                return SkillsAction::InstallSkill { id };
            }
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                self.show_install_confirm = false;
            }
            _ => {}
        }
        SkillsAction::Continue
    }

    fn handle_uninstall_confirm_key(&mut self, key: KeyEvent) -> SkillsAction {
        match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                self.show_uninstall_confirm = false;
                let id = self.uninstall_skill_id.clone();
                self.sub = SkillsSubScreen::List;
                return SkillsAction::UninstallSkill { id };
            }
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                self.show_uninstall_confirm = false;
            }
            _ => {}
        }
        SkillsAction::Continue
    }
}

// ── Drawing ─────────────────────────────────────────────────────────────────

pub fn draw(f: &mut Frame, area: Rect, state: &mut SkillsState, _i18n: &Translator) {
    let title = match state.sub {
        SkillsSubScreen::List => " Skills ",
        SkillsSubScreen::Detail => " Skill Detail ",
        SkillsSubScreen::Search => " Search Skills ",
    };

    let block = Block::default()
        .title(Line::from(vec![Span::styled(title, theme::title_style())]))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme::ACCENT))
        .padding(Padding::horizontal(1));

    let inner = block.inner(area);
    f.render_widget(block, area);

    match state.sub {
        SkillsSubScreen::List => draw_list(f, inner, state),
        SkillsSubScreen::Detail => draw_detail(f, inner, state),
        SkillsSubScreen::Search => draw_search(f, inner, state),
    }

    // Confirmation modals
    if state.show_install_confirm {
        draw_install_confirm(f, area, state);
    }
    if state.show_uninstall_confirm {
        draw_uninstall_confirm(f, area, state);
    }
}

fn draw_list(f: &mut Frame, area: Rect, state: &mut SkillsState) {
    let chunks = Layout::vertical([
        Constraint::Length(2), // header + sub-tabs
        Constraint::Min(3),    // list
        Constraint::Length(1), // hints
    ]).split(area);

    // Sub-tabs header
    let tab_style = |tab: SkillsSubTab| {
        if state.sub_tab == tab {
            Style::default().fg(theme::ACCENT).add_modifier(Modifier::BOLD)
        } else {
            theme::dim_style()
        }
    };

    let count_installed = state.installed_skills.len();
    let count_clawhub = state.clawhub_skills.len();
    let count_mcp = state.mcp_servers.len();

    f.render_widget(
        Paragraph::new(vec![
            Line::from(vec![
                Span::styled("  ", Style::default()),
                Span::styled(format!("[1] Installed ({})", count_installed), tab_style(SkillsSubTab::Installed)),
                Span::styled("  ", Style::default()),
                Span::styled(format!("[2] ClawHub ({})", count_clawhub), tab_style(SkillsSubTab::ClawHub)),
                Span::styled("  ", Style::default()),
                Span::styled(format!("[3] MCP ({})", count_mcp), tab_style(SkillsSubTab::Mcp)),
            ]),
            Line::from(vec![Span::styled(
                format!("  {:<16} {:<10} {:<8} {:<30} {}",
                    "Name", "Version", "Status", "Description", "Author"),
                theme::table_header(),
            )]),
        ]),
        chunks[0],
    );

    // Skill list
    let skills = if state.search_results.is_empty() {
        state.current_skills()
    } else {
        &state.search_results
    };

    if state.loading && skills.is_empty() {
        let spinner = theme::SPINNER_FRAMES[state.tick % theme::SPINNER_FRAMES.len()];
        f.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled(format!("  {spinner} "), Style::default().fg(theme::CYAN)),
                Span::styled("Loading skills...", theme::dim_style()),
            ])),
            chunks[1],
        );
    } else if skills.is_empty() {
        let empty_msg = match state.sub_tab {
            SkillsSubTab::Installed => "No installed skills. Browse ClawHub to install.",
            SkillsSubTab::ClawHub => "No skills found on ClawHub. Check connection.",
            SkillsSubTab::Mcp => "No MCP servers configured. Add one in Settings.",
        };
        f.render_widget(
            Paragraph::new(Span::styled(format!("  {}", empty_msg), theme::dim_style())),
            chunks[1],
        );
    } else {
        let items: Vec<ListItem> = skills
            .iter()
            .map(|skill| {
                let (status_badge, status_style) = if skill.enabled {
                    ("Active", Style::default().fg(theme::GREEN))
                } else {
                    ("Inactive", Style::default().fg(theme::YELLOW))
                };
                ListItem::new(Line::from(vec![
                    Span::styled(format!("  {:<16}", truncate(&skill.name, 15)), Style::default().fg(theme::CYAN)),
                    Span::styled(format!(" {:<10}", skill.version), theme::dim_style()),
                    Span::styled(format!(" {:<8}", status_badge), status_style),
                    Span::styled(format!(" {:<30}", truncate(&skill.description, 29)), Style::default().fg(theme::TEXT)),
                    Span::styled(format!(" {}", skill.author), Style::default().fg(theme::PURPLE)),
                ]))
            })
            .collect();

        let list = List::new(items)
            .highlight_style(theme::selected_style())
            .highlight_symbol("> ");
        f.render_stateful_widget(list, chunks[1], &mut state.list_state);
    }

    // Hints
    let sort_label = match state.sort_mode {
        0 => "name",
        1 => "author",
        2 => "date",
        _ => "name",
    };
    let hints = match state.sub_tab {
        SkillsSubTab::Installed => format!("  [i]uninstall  [t]oggle  [s]ort:{}  [/]search  [1-3]tab  [Esc]back", sort_label),
        SkillsSubTab::ClawHub => format!("  [i]install  [s]ort:{}  [/]search  [1-3]tab  [Esc]back", sort_label),
        SkillsSubTab::Mcp => format!("  [t]oggle  [s]ort:{}  [/]search  [1-3]tab  [Esc]back", sort_label),
    };
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(hints, theme::hint_style()))),
        chunks[2],
    );
}

fn draw_detail(f: &mut Frame, area: Rect, state: &SkillsState) {
    if let Some(skill) = &state.detail_skill {
        let chunks = Layout::vertical([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(1),
            Constraint::Length(1),
        ]).split(area);

        let (status_badge, status_style) = if skill.enabled {
            ("Active", Style::default().fg(theme::GREEN))
        } else {
            ("Inactive", Style::default().fg(theme::YELLOW))
        };

        f.render_widget(
            Paragraph::new(Span::styled(
                format!("{} ({})", skill.name, skill.version),
                Style::default().fg(theme::ACCENT).add_modifier(Modifier::BOLD),
            )),
            chunks[0],
        );

        f.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled("  Status: ", theme::dim_style()),
                Span::styled(status_badge, status_style),
                Span::styled(format!("  |  Source: {}", skill.source), theme::dim_style()),
            ])),
            chunks[1],
        );

        f.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled("  Author: ", theme::dim_style()),
                Span::styled(&skill.author, Style::default().fg(theme::PURPLE)),
            ])),
            chunks[2],
        );

        f.render_widget(
            Paragraph::new(Span::styled("  Description:", theme::dim_style())),
            chunks[3],
        );

        f.render_widget(
            Paragraph::new(Span::styled(format!("    {}", skill.description), Style::default().fg(theme::TEXT))),
            chunks[4],
        );

        f.render_widget(
            Paragraph::new(Span::styled("  Configuration (placeholder):", theme::dim_style())),
            chunks[5],
        );

        f.render_widget(
            Paragraph::new(Span::styled("    [model: default] [timeout: 30s] [retry: 3]", Style::default().fg(theme::TEXT))),
            chunks[6],
        );

        // Action hints
        let actions = if skill.source == "installed" {
            "[u]uninstall  [t]toggle  [Esc]back"
        } else if skill.source == "clawhub" {
            "[i]install  [Esc]back"
        } else {
            "[t]toggle  [Esc]back"
        };
        f.render_widget(
            Paragraph::new(Span::styled(format!("  {}", actions), theme::hint_style())),
            chunks[7],
        );
    }
}

fn draw_search(f: &mut Frame, area: Rect, state: &mut SkillsState) {
    let chunks = Layout::vertical([
        Constraint::Length(1),
        Constraint::Min(3),
        Constraint::Length(1),
    ]).split(area);

    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("  Search: ", Style::default().fg(theme::CYAN).add_modifier(Modifier::BOLD)),
            Span::styled(format!("{}|", state.search_query), Style::default().fg(theme::TEXT)),
        ])),
        chunks[0],
    );

    // Search results
    if state.search_results.is_empty() && !state.search_query.is_empty() {
        f.render_widget(
            Paragraph::new(Span::styled("  No results found.", theme::dim_style())),
            chunks[1],
        );
    } else if state.search_query.is_empty() {
        f.render_widget(
            Paragraph::new(Span::styled("  Type to search ClawHub skills...", theme::dim_style())),
            chunks[1],
        );
    } else {
        let items: Vec<ListItem> = state.search_results.iter().map(|skill| {
            ListItem::new(Line::from(vec![
                Span::styled(format!("  {:<16}", truncate(&skill.name, 15)), Style::default().fg(theme::CYAN)),
                Span::styled(format!(" {:<10}", skill.version), theme::dim_style()),
                Span::styled(format!(" {:<30}", truncate(&skill.description, 29)), Style::default().fg(theme::TEXT)),
            ]))
        }).collect();

        let list = List::new(items)
            .highlight_style(theme::selected_style())
            .highlight_symbol("> ");
        f.render_stateful_widget(list, chunks[1], &mut state.list_state);
    }

    f.render_widget(
        Paragraph::new(Span::styled("[Enter] search  [i]install  [Esc] cancel", theme::hint_style())),
        chunks[2],
    );
}

fn draw_install_confirm(f: &mut Frame, area: Rect, state: &SkillsState) {
    let modal = centered_rect(50, 6, area);
    f.render_widget(Clear, modal);

    let block = Block::default()
        .title(Span::styled(" Install Skill ", theme::title_style()))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme::GREEN))
        .padding(Padding::uniform(1));
    let inner = block.inner(modal);
    f.render_widget(block, modal);

    let chunks = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
    ]).split(inner);

    f.render_widget(
        Paragraph::new(Span::styled(format!("Install '{}' from ClawHub?", state.install_skill_name), Style::default().fg(theme::TEXT))),
        chunks[0],
    );
    f.render_widget(
        Paragraph::new(Span::styled("This will add the skill to your installed list.", Style::default().fg(theme::CYAN))),
        chunks[1],
    );
    f.render_widget(
        Paragraph::new(Span::styled("[y] Yes  [n/Esc] No", theme::hint_style())),
        chunks[2],
    );
}

fn draw_uninstall_confirm(f: &mut Frame, area: Rect, state: &SkillsState) {
    let modal = centered_rect(50, 6, area);
    f.render_widget(Clear, modal);

    let block = Block::default()
        .title(Span::styled(" Uninstall Skill ", theme::title_style()))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme::RED))
        .padding(Padding::uniform(1));
    let inner = block.inner(modal);
    f.render_widget(block, modal);

    let chunks = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
    ]).split(inner);

    f.render_widget(
        Paragraph::new(Span::styled(format!("Uninstall '{}'?", state.uninstall_skill_name), Style::default().fg(theme::TEXT))),
        chunks[0],
    );
    f.render_widget(
        Paragraph::new(Span::styled("This action cannot be undone.", Style::default().fg(theme::YELLOW))),
        chunks[1],
    );
    f.render_widget(
        Paragraph::new(Span::styled("[y] Yes  [n/Esc] No", theme::hint_style())),
        chunks[2],
    );
}

// ── Helpers ─────────────────────────────────────────────────────────────────

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}...", &s[..max.saturating_sub(3)])
    }
}

fn centered_rect(percent_x: u16, height: u16, area: Rect) -> Rect {
    let w = area.width * percent_x / 100;
    let x = area.x + (area.width.saturating_sub(w)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;
    Rect::new(x, y, w, height.min(area.height))
}