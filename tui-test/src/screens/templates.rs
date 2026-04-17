//! Templates screen: Template library with category filter and spawn capability.
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
pub struct TemplateInfo {
    pub id: String,
    pub name: String,
    pub category: String,
    pub description: String,
    pub author: String,
    pub tags: Vec<String>,
    pub popularity: u32,
}

const CATEGORIES: &[&str] = &[
    "All",
    "Agent",
    "Workflow",
    "Skill",
    "Integration",
    "Utility",
];

// ── State ───────────────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum TemplatesSubScreen {
    List,
    Detail,
    Search,
}

pub struct TemplatesState {
    pub templates: Vec<TemplateInfo>,
    pub list_state: ListState,
    pub category_filter: usize,
    pub sub: TemplatesSubScreen,
    pub loading: bool,
    pub tick: usize,
    // Search mode
    pub search_query: String,
    pub search_results: Vec<TemplateInfo>,
    // Detail view
    pub detail_template: Option<TemplateInfo>,
    // Spawn confirmation
    pub show_spawn_confirm: bool,
    pub spawn_template_name: String,
    pub spawn_template_id: String,
    pub spawn_name_override: String,
    pub spawn_field: usize,
    // Status
    pub status_msg: String,
}

pub enum TemplatesAction {
    Continue,
    Back,
    Refresh,
    Search { query: String },
    SpawnTemplate { id: String, name: String },
    FilterCategory { category: String },
}

impl TemplatesState {
    pub fn new() -> Self {
        let templates = vec![
            TemplateInfo {
                id: "tpl-001".to_string(),
                name: "basic-agent".to_string(),
                category: "Agent".to_string(),
                description: "Simple agent with default configuration".to_string(),
                author: "AgentDiVA".to_string(),
                tags: vec!["agent".to_string(), "basic".to_string()],
                popularity: 156,
            },
            TemplateInfo {
                id: "tpl-002".to_string(),
                name: "research-agent".to_string(),
                category: "Agent".to_string(),
                description: "Agent optimized for research tasks".to_string(),
                author: "AgentDiVA".to_string(),
                tags: vec!["agent".to_string(), "research".to_string()],
                popularity: 89,
            },
            TemplateInfo {
                id: "tpl-003".to_string(),
                name: "code-review-workflow".to_string(),
                category: "Workflow".to_string(),
                description: "Automated code review pipeline".to_string(),
                author: "community".to_string(),
                tags: vec!["workflow".to_string(), "review".to_string()],
                popularity: 124,
            },
            TemplateInfo {
                id: "tpl-004".to_string(),
                name: "daily-report".to_string(),
                category: "Workflow".to_string(),
                description: "Generate daily summary reports".to_string(),
                author: "community".to_string(),
                tags: vec!["workflow".to_string(), "report".to_string()],
                popularity: 67,
            },
            TemplateInfo {
                id: "tpl-005".to_string(),
                name: "slack-integration".to_string(),
                category: "Integration".to_string(),
                description: "Slack channel integration template".to_string(),
                author: "AgentDiVA".to_string(),
                tags: vec!["integration".to_string(), "slack".to_string()],
                popularity: 203,
            },
            TemplateInfo {
                id: "tpl-006".to_string(),
                name: "github-skill".to_string(),
                category: "Skill".to_string(),
                description: "GitHub API interaction skill".to_string(),
                author: "community".to_string(),
                tags: vec!["skill".to_string(), "github".to_string()],
                popularity: 145,
            },
            TemplateInfo {
                id: "tpl-007".to_string(),
                name: "cleanup-util".to_string(),
                category: "Utility".to_string(),
                description: "Cleanup and maintenance utility".to_string(),
                author: "AgentDiVA".to_string(),
                tags: vec!["utility".to_string(), "cleanup".to_string()],
                popularity: 34,
            },
        ];

        Self {
            templates,
            list_state: ListState::default().with_selected(Some(0)),
            category_filter: 0,
            sub: TemplatesSubScreen::List,
            loading: false,
            tick: 0,
            search_query: String::new(),
            search_results: Vec::new(),
            detail_template: None,
            show_spawn_confirm: false,
            spawn_template_name: String::new(),
            spawn_template_id: String::new(),
            spawn_name_override: String::new(),
            spawn_field: 0,
            status_msg: String::new(),
        }
    }

    pub fn tick(&mut self) {
        self.tick = self.tick.wrapping_add(1);
    }

    fn filtered_templates(&self) -> Vec<TemplateInfo> {
        let category = CATEGORIES[self.category_filter];
        if category == "All" {
            self.templates.clone()
        } else {
            self.templates.iter()
                .filter(|t| t.category == category)
                .cloned()
                .collect()
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> TemplatesAction {
        if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
            return TemplatesAction::Continue;
        }

        if self.show_spawn_confirm {
            return self.handle_spawn_confirm_key(key);
        }

        match self.sub {
            TemplatesSubScreen::List => self.handle_list_key(key),
            TemplatesSubScreen::Detail => self.handle_detail_key(key),
            TemplatesSubScreen::Search => self.handle_search_key(key),
        }
    }

    fn handle_list_key(&mut self, key: KeyEvent) -> TemplatesAction {
        let templates = self.filtered_templates();
        let total = templates.len();

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
            KeyCode::Char('c') | KeyCode::Char('C') => {
                // Cycle category filter
                self.category_filter = (self.category_filter + 1) % CATEGORIES.len();
                self.list_state.select(Some(0));
            }
            KeyCode::Char('/') => {
                self.sub = TemplatesSubScreen::Search;
                self.search_query.clear();
                self.search_results.clear();
            }
            KeyCode::Char('r') => {
                return TemplatesAction::Refresh;
            }
            KeyCode::Enter => {
                let templates = self.filtered_templates();
                if let Some(idx) = self.list_state.selected() {
                    if let Some(tpl) = templates.get(idx) {
                        self.detail_template = Some(tpl.clone());
                        self.sub = TemplatesSubScreen::Detail;
                    }
                }
            }
            KeyCode::Char('s') => {
                let templates = self.filtered_templates();
                if let Some(idx) = self.list_state.selected() {
                    if let Some(tpl) = templates.get(idx) {
                        self.spawn_template_id = tpl.id.clone();
                        self.spawn_template_name = tpl.name.clone();
                        self.spawn_name_override = tpl.name.clone();
                        self.spawn_field = 0;
                        self.show_spawn_confirm = true;
                    }
                }
            }
            KeyCode::Esc => return TemplatesAction::Back,
            _ => {}
        }
        TemplatesAction::Continue
    }

    fn handle_detail_key(&mut self, key: KeyEvent) -> TemplatesAction {
        match key.code {
            KeyCode::Esc => {
                self.sub = TemplatesSubScreen::List;
                self.detail_template = None;
            }
            KeyCode::Char('s') => {
                if let Some(tpl) = &self.detail_template {
                    self.spawn_template_id = tpl.id.clone();
                    self.spawn_template_name = tpl.name.clone();
                    self.spawn_name_override = tpl.name.clone();
                    self.spawn_field = 0;
                    self.show_spawn_confirm = true;
                }
            }
            _ => {}
        }
        TemplatesAction::Continue
    }

    fn handle_search_key(&mut self, key: KeyEvent) -> TemplatesAction {
        match key.code {
            KeyCode::Esc => {
                self.sub = TemplatesSubScreen::List;
                self.search_query.clear();
            }
            KeyCode::Enter => {
                let query = self.search_query.to_lowercase();
                self.search_results = self.templates.iter()
                    .filter(|t| t.name.to_lowercase().contains(&query) || t.description.to_lowercase().contains(&query) || t.tags.iter().any(|tag| tag.to_lowercase().contains(&query)))
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
        TemplatesAction::Continue
    }

    fn handle_spawn_confirm_key(&mut self, key: KeyEvent) -> TemplatesAction {
        match key.code {
            KeyCode::Esc => {
                self.show_spawn_confirm = false;
            }
            KeyCode::Tab => {
                self.spawn_field = (self.spawn_field + 1) % 2;
            }
            KeyCode::Enter => {
                self.show_spawn_confirm = false;
                self.sub = TemplatesSubScreen::List;
                return TemplatesAction::SpawnTemplate {
                    id: self.spawn_template_id.clone(),
                    name: self.spawn_name_override.clone(),
                };
            }
            KeyCode::Char(c) => {
                if self.spawn_field == 1 {
                    self.spawn_name_override.push(c);
                }
            }
            KeyCode::Backspace => {
                if self.spawn_field == 1 {
                    self.spawn_name_override.pop();
                }
            }
            _ => {}
        }
        TemplatesAction::Continue
    }
}

// ── Drawing ─────────────────────────────────────────────────────────────────

pub fn draw(f: &mut Frame, area: Rect, state: &mut TemplatesState, _i18n: &Translator) {
    let title = match state.sub {
        TemplatesSubScreen::List => " Templates ",
        TemplatesSubScreen::Detail => " Template Detail ",
        TemplatesSubScreen::Search => " Search Templates ",
    };

    let block = Block::default()
        .title(Line::from(vec![Span::styled(title, theme::title_style())]))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme::ACCENT))
        .padding(Padding::horizontal(1));

    let inner = block.inner(area);
    f.render_widget(block, area);

    match state.sub {
        TemplatesSubScreen::List => draw_list(f, inner, state),
        TemplatesSubScreen::Detail => draw_detail(f, inner, state),
        TemplatesSubScreen::Search => draw_search(f, inner, state),
    }

    if state.show_spawn_confirm {
        draw_spawn_confirm(f, area, state);
    }
}

fn draw_list(f: &mut Frame, area: Rect, state: &mut TemplatesState) {
    let chunks = Layout::vertical([
        Constraint::Length(2),
        Constraint::Min(3),
        Constraint::Length(1),
    ]).split(area);

    // Category filter display
    let category_spans: Vec<Span> = CATEGORIES.iter().enumerate().map(|(i, cat)| {
        if i == state.category_filter {
            Span::styled(format!("[{}]", cat), Style::default().fg(theme::ACCENT).add_modifier(Modifier::BOLD))
        } else {
            Span::styled(*cat, theme::dim_style())
        }
    }).collect();

    let templates = state.filtered_templates();
    let count = templates.len();

    f.render_widget(
        Paragraph::new(vec![
            Line::from(category_spans),
            Line::from(vec![Span::styled(
                format!("  {:<16} {:<12} {:<30} {:<8} {}",
                    "Name", "Category", "Description", "Popular", "Author"),
                theme::table_header(),
            )]),
        ]),
        chunks[0],
    );

    if state.loading && templates.is_empty() {
        let spinner = theme::SPINNER_FRAMES[state.tick % theme::SPINNER_FRAMES.len()];
        f.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled(format!("  {spinner} "), Style::default().fg(theme::CYAN)),
                Span::styled("Loading templates...", theme::dim_style()),
            ])),
            chunks[1],
        );
    } else if templates.is_empty() {
        f.render_widget(
            Paragraph::new(Span::styled("  No templates in this category.", theme::dim_style())),
            chunks[1],
        );
    } else {
        let items: Vec<ListItem> = templates
            .iter()
            .map(|tpl| {
                ListItem::new(Line::from(vec![
                    Span::styled(format!("  {:<16}", truncate(&tpl.name, 15)), Style::default().fg(theme::CYAN)),
                    Span::styled(format!(" {:<12}", tpl.category), Style::default().fg(theme::PURPLE)),
                    Span::styled(format!(" {:<30}", truncate(&tpl.description, 29)), Style::default().fg(theme::TEXT)),
                    Span::styled(format!(" {:<8}", tpl.popularity), Style::default().fg(theme::ACCENT)),
                    Span::styled(format!(" {}", tpl.author), theme::dim_style()),
                ]))
            })
            .collect();

        let list = List::new(items)
            .highlight_style(theme::selected_style())
            .highlight_symbol("> ");
        f.render_stateful_widget(list, chunks[1], &mut state.list_state);
    }

    f.render_widget(
        Paragraph::new(Line::from(Span::styled(
            "  [c]ategory  [s]pawn  [Enter]detail  [/]search  [r]efresh  [Esc]back",
            theme::hint_style(),
        ))),
        chunks[2],
    );
}

fn draw_detail(f: &mut Frame, area: Rect, state: &TemplatesState) {
    if let Some(tpl) = &state.detail_template {
        let chunks = Layout::vertical([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(1),
            Constraint::Length(1),
        ]).split(area);

        f.render_widget(
            Paragraph::new(Span::styled(
                format!("{} ({})", tpl.name, tpl.id),
                Style::default().fg(theme::ACCENT).add_modifier(Modifier::BOLD),
            )),
            chunks[0],
        );

        f.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled("  Category: ", theme::dim_style()),
                Span::styled(&tpl.category, Style::default().fg(theme::PURPLE)),
                Span::styled(format!("  |  Author: {}", tpl.author), theme::dim_style()),
            ])),
            chunks[1],
        );

        f.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled("  Popularity: ", theme::dim_style()),
                Span::styled(tpl.popularity.to_string(), Style::default().fg(theme::ACCENT)),
            ])),
            chunks[2],
        );

        f.render_widget(
            Paragraph::new(Span::styled("  Description:", theme::dim_style())),
            chunks[3],
        );

        f.render_widget(
            Paragraph::new(Span::styled(format!("    {}", tpl.description), Style::default().fg(theme::TEXT))),
            chunks[4],
        );

        f.render_widget(
            Paragraph::new(Span::styled("  Tags:", theme::dim_style())),
            chunks[5],
        );

        let tags_text = tpl.tags.join(", ");
        f.render_widget(
            Paragraph::new(Span::styled(format!("    {}", tags_text), Style::default().fg(theme::CYAN))),
            chunks[6],
        );

        f.render_widget(
            Paragraph::new(Span::styled("  [s]pawn  [Esc]back", theme::hint_style())),
            chunks[7],
        );
    }
}

fn draw_search(f: &mut Frame, area: Rect, state: &mut TemplatesState) {
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
            Paragraph::new(Span::styled("  Type to search templates by name, description, or tags...", theme::dim_style())),
            chunks[1],
        );
    } else if state.search_results.is_empty() {
        f.render_widget(
            Paragraph::new(Span::styled("  No templates found.", theme::dim_style())),
            chunks[1],
        );
    } else {
        let items: Vec<ListItem> = state.search_results.iter().map(|tpl| {
            ListItem::new(Line::from(vec![
                Span::styled(format!("  {:<16}", truncate(&tpl.name, 15)), Style::default().fg(theme::CYAN)),
                Span::styled(format!(" {:<12}", tpl.category), Style::default().fg(theme::PURPLE)),
                Span::styled(format!(" {:<30}", truncate(&tpl.description, 29)), Style::default().fg(theme::TEXT)),
            ]))
        }).collect();

        let list = List::new(items)
            .highlight_style(theme::selected_style())
            .highlight_symbol("> ");
        f.render_stateful_widget(list, chunks[1], &mut state.list_state);
    }

    f.render_widget(
        Paragraph::new(Span::styled("[Enter] search  [s]spawn  [Esc] cancel", theme::hint_style())),
        chunks[2],
    );
}

fn draw_spawn_confirm(f: &mut Frame, area: Rect, state: &TemplatesState) {
    let modal = centered_rect(50, 8, area);
    f.render_widget(Clear, modal);

    let block = Block::default()
        .title(Span::styled(" Spawn Template ", theme::title_style()))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme::GREEN))
        .padding(Padding::uniform(1));
    let inner = block.inner(modal);
    f.render_widget(block, modal);

    let chunks = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
    ]).split(inner);

    f.render_widget(
        Paragraph::new(Span::styled(format!("Spawn from '{}'", state.spawn_template_name), Style::default().fg(theme::ACCENT).add_modifier(Modifier::BOLD))),
        chunks[0],
    );

    let name_style = if state.spawn_field == 1 {
        Style::default().fg(theme::CYAN).add_modifier(Modifier::BOLD)
    } else {
        theme::dim_style()
    };

    f.render_widget(Paragraph::new(Span::styled("Instance Name:", name_style)), chunks[1]);
    f.render_widget(Paragraph::new(Span::styled(format!("  {}|", state.spawn_name_override), Style::default().fg(theme::TEXT))), chunks[2]);

    f.render_widget(
        Paragraph::new(Span::styled("This will create a new instance from the template.", Style::default().fg(theme::CYAN))),
        chunks[3],
    );

    f.render_widget(
        Paragraph::new(Span::styled("[Tab] edit name  [Enter] spawn  [Esc] cancel", theme::hint_style())),
        chunks[4],
    );
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