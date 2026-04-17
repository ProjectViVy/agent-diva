//! Extensions screen: Extension management with Browse, Installed, and Health tabs.
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
pub struct ExtensionInfo {
    pub id: String,
    pub name: String,
    pub version: String,
    pub description: String,
    pub author: String,
    pub status: String, // "installed", "available", "error", "healthy"
    pub health: Option<String>,
    pub last_checked: Option<String>,
}

// ── State ───────────────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ExtensionsSubTab {
    Browse,
    Installed,
    Health,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ExtensionsSubScreen {
    List,
    Detail,
    Search,
}

pub struct ExtensionsState {
    pub browse_extensions: Vec<ExtensionInfo>,
    pub installed_extensions: Vec<ExtensionInfo>,
    pub health_status: Vec<ExtensionInfo>,
    pub list_state: ListState,
    pub sub_tab: ExtensionsSubTab,
    pub sub: ExtensionsSubScreen,
    pub loading: bool,
    pub tick: usize,
    // Search mode
    pub search_query: String,
    pub search_results: Vec<ExtensionInfo>,
    // Detail view
    pub detail_extension: Option<ExtensionInfo>,
    // Install confirmation
    pub show_install_confirm: bool,
    pub install_ext_name: String,
    pub install_ext_id: String,
    // Remove confirmation
    pub show_remove_confirm: bool,
    pub remove_ext_name: String,
    pub remove_ext_id: String,
    // Reconnect confirmation
    pub show_reconnect_confirm: bool,
    pub reconnect_ext_name: String,
    pub reconnect_ext_id: String,
    // Status
    pub status_msg: String,
}

pub enum ExtensionsAction {
    Continue,
    Back,
    Refresh,
    Search { query: String },
    InstallExtension { id: String },
    RemoveExtension { id: String },
    ReconnectExtension { id: String },
}

impl ExtensionsState {
    pub fn new() -> Self {
        let browse_extensions = vec![
            ExtensionInfo {
                id: "ext-001".to_string(),
                name: "git-integration".to_string(),
                version: "2.1.0".to_string(),
                description: "Git operations and repository management".to_string(),
                author: "dev-tools".to_string(),
                status: "available".to_string(),
                health: None,
                last_checked: None,
            },
            ExtensionInfo {
                id: "ext-002".to_string(),
                name: "docker-manager".to_string(),
                version: "1.5.2".to_string(),
                description: "Docker container management and orchestration".to_string(),
                author: "cloud-native".to_string(),
                status: "available".to_string(),
                health: None,
                last_checked: None,
            },
            ExtensionInfo {
                id: "ext-003".to_string(),
                name: "k8s-operator".to_string(),
                version: "0.9.1".to_string(),
                description: "Kubernetes cluster management".to_string(),
                author: "k8s-team".to_string(),
                status: "available".to_string(),
                health: None,
                last_checked: None,
            },
        ];

        let installed_extensions = vec![
            ExtensionInfo {
                id: "ext-004".to_string(),
                name: "terminal".to_string(),
                version: "3.0.0".to_string(),
                description: "Integrated terminal and shell execution".to_string(),
                author: "AgentDiVA".to_string(),
                status: "installed".to_string(),
                health: Some("healthy".to_string()),
                last_checked: Some("2025-04-16T08:00:00Z".to_string()),
            },
            ExtensionInfo {
                id: "ext-005".to_string(),
                name: "http-client".to_string(),
                version: "1.2.3".to_string(),
                description: "HTTP request builder and API testing".to_string(),
                author: "web-tools".to_string(),
                status: "installed".to_string(),
                health: Some("healthy".to_string()),
                last_checked: Some("2025-04-16T08:00:00Z".to_string()),
            },
            ExtensionInfo {
                id: "ext-006".to_string(),
                name: "database".to_string(),
                version: "0.8.5".to_string(),
                description: "Database browser and query runner".to_string(),
                author: "data-tools".to_string(),
                status: "installed".to_string(),
                health: Some("error".to_string()),
                last_checked: Some("2025-04-16T08:00:00Z".to_string()),
            },
        ];

        let health_status = installed_extensions.clone();

        Self {
            browse_extensions,
            installed_extensions,
            health_status,
            list_state: ListState::default().with_selected(Some(0)),
            sub_tab: ExtensionsSubTab::Browse,
            sub: ExtensionsSubScreen::List,
            loading: false,
            tick: 0,
            search_query: String::new(),
            search_results: Vec::new(),
            detail_extension: None,
            show_install_confirm: false,
            install_ext_name: String::new(),
            install_ext_id: String::new(),
            show_remove_confirm: false,
            remove_ext_name: String::new(),
            remove_ext_id: String::new(),
            show_reconnect_confirm: false,
            reconnect_ext_name: String::new(),
            reconnect_ext_id: String::new(),
            status_msg: String::new(),
        }
    }

    pub fn tick(&mut self) {
        self.tick = self.tick.wrapping_add(1);
    }

    fn current_extensions(&self) -> &Vec<ExtensionInfo> {
        match self.sub_tab {
            ExtensionsSubTab::Browse => &self.browse_extensions,
            ExtensionsSubTab::Installed => &self.installed_extensions,
            ExtensionsSubTab::Health => &self.health_status,
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> ExtensionsAction {
        if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
            return ExtensionsAction::Continue;
        }

        if self.show_install_confirm {
            return self.handle_install_confirm_key(key);
        }
        if self.show_remove_confirm {
            return self.handle_remove_confirm_key(key);
        }
        if self.show_reconnect_confirm {
            return self.handle_reconnect_confirm_key(key);
        }

        match self.sub {
            ExtensionsSubScreen::List => self.handle_list_key(key),
            ExtensionsSubScreen::Detail => self.handle_detail_key(key),
            ExtensionsSubScreen::Search => self.handle_search_key(key),
        }
    }

    fn handle_list_key(&mut self, key: KeyEvent) -> ExtensionsAction {
        let extensions = self.current_extensions();
        let total = extensions.len();

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
                self.sub_tab = ExtensionsSubTab::Browse;
                self.list_state.select(Some(0));
            }
            KeyCode::Char('2') => {
                self.sub_tab = ExtensionsSubTab::Installed;
                self.list_state.select(Some(0));
            }
            KeyCode::Char('3') => {
                self.sub_tab = ExtensionsSubTab::Health;
                self.list_state.select(Some(0));
            }
            KeyCode::Char('/') => {
                self.sub = ExtensionsSubScreen::Search;
                self.search_query.clear();
                self.search_results.clear();
            }
            KeyCode::Char('r') => {
                return ExtensionsAction::Refresh;
            }
            KeyCode::Enter => {
                if let Some(idx) = self.list_state.selected() {
                    if let Some(ext) = self.current_extensions().get(idx) {
                        self.detail_extension = Some(ext.clone());
                        self.sub = ExtensionsSubScreen::Detail;
                    }
                }
            }
            KeyCode::Char('i') => {
                if self.sub_tab == ExtensionsSubTab::Browse {
                    if let Some(idx) = self.list_state.selected() {
                        if let Some(ext) = self.browse_extensions.get(idx) {
                            self.install_ext_id = ext.id.clone();
                            self.install_ext_name = ext.name.clone();
                            self.show_install_confirm = true;
                        }
                    }
                }
            }
            KeyCode::Char('d') => {
                if self.sub_tab == ExtensionsSubTab::Installed {
                    if let Some(idx) = self.list_state.selected() {
                        if let Some(ext) = self.installed_extensions.get(idx) {
                            self.remove_ext_id = ext.id.clone();
                            self.remove_ext_name = ext.name.clone();
                            self.show_remove_confirm = true;
                        }
                    }
                }
            }
            KeyCode::Char('c') => {
                if self.sub_tab == ExtensionsSubTab::Health {
                    if let Some(idx) = self.list_state.selected() {
                        if let Some(ext) = self.health_status.get(idx) {
                            if ext.health.as_deref() == Some("error") {
                                self.reconnect_ext_id = ext.id.clone();
                                self.reconnect_ext_name = ext.name.clone();
                                self.show_reconnect_confirm = true;
                            }
                        }
                    }
                }
            }
            KeyCode::Esc => return ExtensionsAction::Back,
            _ => {}
        }
        ExtensionsAction::Continue
    }

    fn handle_detail_key(&mut self, key: KeyEvent) -> ExtensionsAction {
        match key.code {
            KeyCode::Esc => {
                self.sub = ExtensionsSubScreen::List;
                self.detail_extension = None;
            }
            KeyCode::Char('i') => {
                if let Some(ext) = &self.detail_extension {
                    if ext.status == "available" {
                        self.install_ext_id = ext.id.clone();
                        self.install_ext_name = ext.name.clone();
                        self.show_install_confirm = true;
                    }
                }
            }
            KeyCode::Char('d') => {
                if let Some(ext) = &self.detail_extension {
                    if ext.status == "installed" {
                        self.remove_ext_id = ext.id.clone();
                        self.remove_ext_name = ext.name.clone();
                        self.show_remove_confirm = true;
                    }
                }
            }
            KeyCode::Char('c') => {
                if let Some(ext) = &self.detail_extension {
                    if ext.health.as_deref() == Some("error") {
                        self.reconnect_ext_id = ext.id.clone();
                        self.reconnect_ext_name = ext.name.clone();
                        self.show_reconnect_confirm = true;
                    }
                }
            }
            _ => {}
        }
        ExtensionsAction::Continue
    }

    fn handle_search_key(&mut self, key: KeyEvent) -> ExtensionsAction {
        match key.code {
            KeyCode::Esc => {
                self.sub = ExtensionsSubScreen::List;
                self.search_query.clear();
            }
            KeyCode::Enter => {
                let query = self.search_query.to_lowercase();
                self.search_results = self.browse_extensions.iter()
                    .filter(|e| e.name.to_lowercase().contains(&query) || e.description.to_lowercase().contains(&query))
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
        ExtensionsAction::Continue
    }

    fn handle_install_confirm_key(&mut self, key: KeyEvent) -> ExtensionsAction {
        match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                self.show_install_confirm = false;
                let id = self.install_ext_id.clone();
                self.sub = ExtensionsSubScreen::List;
                return ExtensionsAction::InstallExtension { id };
            }
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                self.show_install_confirm = false;
            }
            _ => {}
        }
        ExtensionsAction::Continue
    }

    fn handle_remove_confirm_key(&mut self, key: KeyEvent) -> ExtensionsAction {
        match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                self.show_remove_confirm = false;
                let id = self.remove_ext_id.clone();
                self.sub = ExtensionsSubScreen::List;
                return ExtensionsAction::RemoveExtension { id };
            }
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                self.show_remove_confirm = false;
            }
            _ => {}
        }
        ExtensionsAction::Continue
    }

    fn handle_reconnect_confirm_key(&mut self, key: KeyEvent) -> ExtensionsAction {
        match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                self.show_reconnect_confirm = false;
                let id = self.reconnect_ext_id.clone();
                return ExtensionsAction::ReconnectExtension { id };
            }
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                self.show_reconnect_confirm = false;
            }
            _ => {}
        }
        ExtensionsAction::Continue
    }
}

// ── Drawing ─────────────────────────────────────────────────────────────────

pub fn draw(f: &mut Frame, area: Rect, state: &mut ExtensionsState, _i18n: &Translator) {
    let title = match state.sub {
        ExtensionsSubScreen::List => " Extensions ",
        ExtensionsSubScreen::Detail => " Extension Detail ",
        ExtensionsSubScreen::Search => " Search Extensions ",
    };

    let block = Block::default()
        .title(Line::from(vec![Span::styled(title, theme::title_style())]))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme::ACCENT))
        .padding(Padding::horizontal(1));

    let inner = block.inner(area);
    f.render_widget(block, area);

    match state.sub {
        ExtensionsSubScreen::List => draw_list(f, inner, state),
        ExtensionsSubScreen::Detail => draw_detail(f, inner, state),
        ExtensionsSubScreen::Search => draw_search(f, inner, state),
    }

    if state.show_install_confirm {
        draw_install_confirm(f, area, state);
    }
    if state.show_remove_confirm {
        draw_remove_confirm(f, area, state);
    }
    if state.show_reconnect_confirm {
        draw_reconnect_confirm(f, area, state);
    }
}

fn draw_list(f: &mut Frame, area: Rect, state: &mut ExtensionsState) {
    let chunks = Layout::vertical([
        Constraint::Length(2),
        Constraint::Min(3),
        Constraint::Length(1),
    ]).split(area);

    let tab_style = |tab: ExtensionsSubTab| {
        if state.sub_tab == tab {
            Style::default().fg(theme::ACCENT).add_modifier(Modifier::BOLD)
        } else {
            theme::dim_style()
        }
    };

    let count_browse = state.browse_extensions.len();
    let count_installed = state.installed_extensions.len();
    let healthy_count = state.health_status.iter().filter(|e| e.health.as_deref() == Some("healthy")).count();

    f.render_widget(
        Paragraph::new(vec![
            Line::from(vec![
                Span::styled("  ", Style::default()),
                Span::styled(format!("[1] Browse ({})", count_browse), tab_style(ExtensionsSubTab::Browse)),
                Span::styled("  ", Style::default()),
                Span::styled(format!("[2] Installed ({})", count_installed), tab_style(ExtensionsSubTab::Installed)),
                Span::styled("  ", Style::default()),
                Span::styled(format!("[3] Health ({}/{})", healthy_count, count_installed), tab_style(ExtensionsSubTab::Health)),
            ]),
            Line::from(vec![Span::styled(
                format!("  {:<16} {:<10} {:<8} {:<30} {}",
                    "Name", "Version", "Status", "Description", "Author"),
                theme::table_header(),
            )]),
        ]),
        chunks[0],
    );

    let extensions = state.current_extensions();

    if state.loading && extensions.is_empty() {
        let spinner = theme::SPINNER_FRAMES[state.tick % theme::SPINNER_FRAMES.len()];
        f.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled(format!("  {spinner} "), Style::default().fg(theme::CYAN)),
                Span::styled("Loading extensions...", theme::dim_style()),
            ])),
            chunks[1],
        );
    } else if extensions.is_empty() {
        let empty_msg = match state.sub_tab {
            ExtensionsSubTab::Browse => "No extensions available.",
            ExtensionsSubTab::Installed => "No extensions installed. Browse to install.",
            ExtensionsSubTab::Health => "No health data. Check installed extensions.",
        };
        f.render_widget(
            Paragraph::new(Span::styled(format!("  {}", empty_msg), theme::dim_style())),
            chunks[1],
        );
    } else {
        let items: Vec<ListItem> = extensions
            .iter()
            .map(|ext| {
                let (status_badge, status_style) = if state.sub_tab == ExtensionsSubTab::Health {
                    match ext.health.as_deref() {
                        Some("healthy") => ("Healthy", Style::default().fg(theme::GREEN)),
                        Some("error") => ("Error", Style::default().fg(theme::RED)),
                        Some("warning") => ("Warning", Style::default().fg(theme::YELLOW)),
                        _ => ("Unknown", theme::dim_style()),
                    }
                } else {
                    match ext.status.as_str() {
                        "installed" => ("Installed", Style::default().fg(theme::GREEN)),
                        "available" => ("Available", Style::default().fg(theme::CYAN)),
                        "error" => ("Error", Style::default().fg(theme::RED)),
                        _ => (ext.status.as_str(), theme::dim_style()),
                    }
                };
                ListItem::new(Line::from(vec![
                    Span::styled(format!("  {:<16}", truncate(&ext.name, 15)), Style::default().fg(theme::CYAN)),
                    Span::styled(format!(" {:<10}", ext.version), theme::dim_style()),
                    Span::styled(format!(" {:<8}", status_badge), status_style),
                    Span::styled(format!(" {:<30}", truncate(&ext.description, 29)), Style::default().fg(theme::TEXT)),
                    Span::styled(format!(" {}", ext.author), Style::default().fg(theme::PURPLE)),
                ]))
            })
            .collect();

        let list = List::new(items)
            .highlight_style(theme::selected_style())
            .highlight_symbol("> ");
        f.render_stateful_widget(list, chunks[1], &mut state.list_state);
    }

    let hints = match state.sub_tab {
        ExtensionsSubTab::Browse => "[i]install  [r]efresh  [/]search  [1-3]tab  [Esc]back",
        ExtensionsSubTab::Installed => "[d]remove  [r]efresh  [/]search  [1-3]tab  [Esc]back",
        ExtensionsSubTab::Health => "[c]reconnect error  [r]efresh  [1-3]tab  [Esc]back",
    };
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(format!("  {}", hints), theme::hint_style()))),
        chunks[2],
    );
}

fn draw_detail(f: &mut Frame, area: Rect, state: &ExtensionsState) {
    if let Some(ext) = &state.detail_extension {
        let chunks = Layout::vertical([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(1),
            Constraint::Length(1),
        ]).split(area);

        let (status_badge, status_style) = match ext.status.as_str() {
            "installed" => ("Installed", Style::default().fg(theme::GREEN)),
            "available" => ("Available", Style::default().fg(theme::CYAN)),
            "error" => ("Error", Style::default().fg(theme::RED)),
            _ => (ext.status.as_str(), theme::dim_style()),
        };

        f.render_widget(
            Paragraph::new(Span::styled(
                format!("{} v{}", ext.name, ext.version),
                Style::default().fg(theme::ACCENT).add_modifier(Modifier::BOLD),
            )),
            chunks[0],
        );

        f.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled("  Status: ", theme::dim_style()),
                Span::styled(status_badge, status_style),
                Span::styled(format!("  |  ID: {}", ext.id), theme::dim_style()),
            ])),
            chunks[1],
        );

        f.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled("  Author: ", theme::dim_style()),
                Span::styled(&ext.author, Style::default().fg(theme::PURPLE)),
            ])),
            chunks[2],
        );

        f.render_widget(
            Paragraph::new(Span::styled("  Description:", theme::dim_style())),
            chunks[3],
        );

        f.render_widget(
            Paragraph::new(Span::styled(format!("    {}", ext.description), Style::default().fg(theme::TEXT))),
            chunks[4],
        );

        if let Some(health) = &ext.health {
            let (health_badge, health_style) = match health.as_str() {
                "healthy" => ("Healthy", Style::default().fg(theme::GREEN)),
                "error" => ("Error", Style::default().fg(theme::RED)),
                _ => (health.as_str(), theme::dim_style()),
            };
            f.render_widget(
                Paragraph::new(Line::from(vec![
                    Span::styled("  Health: ", theme::dim_style()),
                    Span::styled(health_badge, health_style),
                ])),
                chunks[5],
            );
        }

        let actions = if ext.status == "available" {
            "[i]install  [Esc]back"
        } else if ext.status == "installed" {
            if ext.health.as_deref() == Some("error") {
                "[c]reconnect  [d]remove  [Esc]back"
            } else {
                "[d]remove  [Esc]back"
            }
        } else {
            "[Esc]back"
        };
        f.render_widget(
            Paragraph::new(Span::styled(format!("  {}", actions), theme::hint_style())),
            chunks[6],
        );
    }
}

fn draw_search(f: &mut Frame, area: Rect, state: &mut ExtensionsState) {
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

    if state.search_query.is_empty() {
        f.render_widget(
            Paragraph::new(Span::styled("  Type to search extensions...", theme::dim_style())),
            chunks[1],
        );
    } else if state.search_results.is_empty() {
        f.render_widget(
            Paragraph::new(Span::styled("  No results found.", theme::dim_style())),
            chunks[1],
        );
    } else {
        let items: Vec<ListItem> = state.search_results.iter().map(|ext| {
            ListItem::new(Line::from(vec![
                Span::styled(format!("  {:<16}", truncate(&ext.name, 15)), Style::default().fg(theme::CYAN)),
                Span::styled(format!(" {:<30}", truncate(&ext.description, 29)), Style::default().fg(theme::TEXT)),
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

fn draw_install_confirm(f: &mut Frame, area: Rect, state: &ExtensionsState) {
    let modal = centered_rect(50, 6, area);
    f.render_widget(Clear, modal);

    let block = Block::default()
        .title(Span::styled(" Install Extension ", theme::title_style()))
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

    f.render_widget(Paragraph::new(Span::styled(format!("Install '{}'?", state.install_ext_name), Style::default().fg(theme::TEXT))), chunks[0]);
    f.render_widget(Paragraph::new(Span::styled("This will add the extension to your installed list.", Style::default().fg(theme::CYAN))), chunks[1]);
    f.render_widget(Paragraph::new(Span::styled("[y] Yes  [n/Esc] No", theme::hint_style())), chunks[2]);
}

fn draw_remove_confirm(f: &mut Frame, area: Rect, state: &ExtensionsState) {
    let modal = centered_rect(50, 6, area);
    f.render_widget(Clear, modal);

    let block = Block::default()
        .title(Span::styled(" Remove Extension ", theme::title_style()))
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

    f.render_widget(Paragraph::new(Span::styled(format!("Remove '{}'?", state.remove_ext_name), Style::default().fg(theme::TEXT))), chunks[0]);
    f.render_widget(Paragraph::new(Span::styled("This action cannot be undone.", Style::default().fg(theme::YELLOW))), chunks[1]);
    f.render_widget(Paragraph::new(Span::styled("[y] Yes  [n/Esc] No", theme::hint_style())), chunks[2]);
}

fn draw_reconnect_confirm(f: &mut Frame, area: Rect, state: &ExtensionsState) {
    let modal = centered_rect(50, 6, area);
    f.render_widget(Clear, modal);

    let block = Block::default()
        .title(Span::styled(" Reconnect Extension ", theme::title_style()))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme::YELLOW))
        .padding(Padding::uniform(1));
    let inner = block.inner(modal);
    f.render_widget(block, modal);

    let chunks = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
    ]).split(inner);

    f.render_widget(Paragraph::new(Span::styled(format!("Reconnect '{}'?", state.reconnect_ext_name), Style::default().fg(theme::TEXT))), chunks[0]);
    f.render_widget(Paragraph::new(Span::styled("This will attempt to restore the extension connection.", Style::default().fg(theme::CYAN))), chunks[1]);
    f.render_widget(Paragraph::new(Span::styled("[y] Yes  [n/Esc] No", theme::hint_style())), chunks[2]);
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