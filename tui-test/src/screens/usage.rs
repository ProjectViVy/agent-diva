//! Usage screen: Usage statistics with Summary, ByModel, ByAgent tabs.
//!
//! Interaction design 100% copied from AgentDiVA.
//! All data is placeholder/fake for demonstration.

use crate::i18n::Translator;
use crate::theme;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Padding, Paragraph};
use ratatui::Frame;

// ── Data types (placeholder) ──────────────────────────────────────────────────

#[derive(Clone, Default)]
pub struct UsageStats {
    pub total_requests: u64,
    pub total_tokens: u64,
    pub total_cost_usd: f64,
    pub avg_latency_ms: f64,
}

#[derive(Clone, Default)]
pub struct ModelUsage {
    pub model_id: String,
    pub provider: String,
    pub requests: u64,
    pub tokens_in: u64,
    pub tokens_out: u64,
    pub cost_usd: f64,
}

#[derive(Clone, Default)]
pub struct AgentUsage {
    pub agent_id: String,
    pub agent_name: String,
    pub requests: u64,
    pub tokens: u64,
    pub cost_usd: f64,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum UsageSubTab {
    Summary,
    ByModel,
    ByAgent,
}

// ── State ───────────────────────────────────────────────────────────────────

pub struct UsageState {
    pub stats: UsageStats,
    pub model_usage: Vec<ModelUsage>,
    pub agent_usage: Vec<AgentUsage>,
    pub list_state: ListState,
    pub sub_tab: UsageSubTab,
    pub loading: bool,
    pub tick: usize,
    pub poll_tick: usize,
    pub status_msg: String,
}

pub enum UsageAction {
    Continue,
    Back,
    Refresh,
}

impl UsageState {
    pub fn new() -> Self {
        let stats = UsageStats {
            total_requests: 1234,
            total_tokens: 567890,
            total_cost_usd: 12.45,
            avg_latency_ms: 850.5,
        };

        let model_usage = vec![
            ModelUsage {
                model_id: "claude-3-opus".to_string(),
                provider: "anthropic".to_string(),
                requests: 450,
                tokens_in: 120000,
                tokens_out: 45000,
                cost_usd: 8.50,
            },
            ModelUsage {
                model_id: "claude-3-sonnet".to_string(),
                provider: "anthropic".to_string(),
                requests: 600,
                tokens_in: 180000,
                tokens_out: 90000,
                cost_usd: 3.20,
            },
            ModelUsage {
                model_id: "gpt-4o".to_string(),
                provider: "openai".to_string(),
                requests: 150,
                tokens_in: 50000,
                tokens_out: 25000,
                cost_usd: 0.75,
            },
            ModelUsage {
                model_id: "deepseek-chat".to_string(),
                provider: "deepseek".to_string(),
                requests: 34,
                tokens_in: 15000,
                tokens_out: 8000,
                cost_usd: 0.0010,
            },
        ];

        let agent_usage = vec![
            AgentUsage {
                agent_id: "root-agent".to_string(),
                agent_name: "root-agent".to_string(),
                requests: 500,
                tokens: 200000,
                cost_usd: 6.25,
            },
            AgentUsage {
                agent_id: "research-agent".to_string(),
                agent_name: "research-agent".to_string(),
                requests: 400,
                tokens: 150000,
                cost_usd: 4.10,
            },
            AgentUsage {
                agent_id: "code-agent".to_string(),
                agent_name: "code-agent".to_string(),
                requests: 250,
                tokens: 120000,
                cost_usd: 1.85,
            },
            AgentUsage {
                agent_id: "test-agent".to_string(),
                agent_name: "test-agent".to_string(),
                requests: 84,
                tokens: 50000,
                cost_usd: 0.25,
            },
        ];

        Self {
            stats,
            model_usage,
            agent_usage,
            list_state: ListState::default().with_selected(Some(0)),
            sub_tab: UsageSubTab::Summary,
            loading: false,
            tick: 0,
            poll_tick: 0,
            status_msg: String::new(),
        }
    }

    pub fn tick(&mut self) {
        self.tick = self.tick.wrapping_add(1);
        self.poll_tick = self.poll_tick.wrapping_add(1);
    }

    pub fn should_poll(&self) -> bool {
        self.poll_tick > 0 && self.poll_tick % 200 == 0
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> UsageAction {
        if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
            return UsageAction::Continue;
        }

        match self.sub_tab {
            UsageSubTab::Summary => self.handle_summary_key(key),
            UsageSubTab::ByModel => self.handle_model_key(key),
            UsageSubTab::ByAgent => self.handle_agent_key(key),
        }
    }

    fn handle_summary_key(&mut self, key: KeyEvent) -> UsageAction {
        match key.code {
            KeyCode::Char('1') => self.sub_tab = UsageSubTab::Summary,
            KeyCode::Char('2') => self.sub_tab = UsageSubTab::ByModel,
            KeyCode::Char('3') => self.sub_tab = UsageSubTab::ByAgent,
            KeyCode::Char('r') => return UsageAction::Refresh,
            KeyCode::Esc => return UsageAction::Back,
            _ => {}
        }
        UsageAction::Continue
    }

    fn handle_model_key(&mut self, key: KeyEvent) -> UsageAction {
        let total = self.model_usage.len();
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
            KeyCode::Char('1') => self.sub_tab = UsageSubTab::Summary,
            KeyCode::Char('2') => self.sub_tab = UsageSubTab::ByModel,
            KeyCode::Char('3') => self.sub_tab = UsageSubTab::ByAgent,
            KeyCode::Char('r') => return UsageAction::Refresh,
            KeyCode::Esc => return UsageAction::Back,
            _ => {}
        }
        UsageAction::Continue
    }

    fn handle_agent_key(&mut self, key: KeyEvent) -> UsageAction {
        let total = self.agent_usage.len();
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
            KeyCode::Char('1') => self.sub_tab = UsageSubTab::Summary,
            KeyCode::Char('2') => self.sub_tab = UsageSubTab::ByModel,
            KeyCode::Char('3') => self.sub_tab = UsageSubTab::ByAgent,
            KeyCode::Char('r') => return UsageAction::Refresh,
            KeyCode::Esc => return UsageAction::Back,
            _ => {}
        }
        UsageAction::Continue
    }
}

// ── Drawing ─────────────────────────────────────────────────────────────────

pub fn draw(f: &mut Frame, area: Rect, state: &mut UsageState, _i18n: &Translator) {
    let block = Block::default()
        .title(Line::from(vec![Span::styled(" Usage ", theme::title_style())]))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme::ACCENT))
        .padding(Padding::horizontal(1));

    let inner = block.inner(area);
    f.render_widget(block, area);

    match state.sub_tab {
        UsageSubTab::Summary => draw_summary(f, inner, state),
        UsageSubTab::ByModel => draw_by_model(f, inner, state),
        UsageSubTab::ByAgent => draw_by_agent(f, inner, state),
    }
}

fn draw_summary(f: &mut Frame, area: Rect, state: &UsageState) {
    let chunks = Layout::vertical([
        Constraint::Length(1), // tabs
        Constraint::Length(4), // stat cards
        Constraint::Length(1), // hints
    ]).split(area);

    // Tabs
    draw_tabs(f, chunks[0], state);

    // Stat cards
    let cards = Layout::horizontal([
        Constraint::Percentage(25),
        Constraint::Percentage(25),
        Constraint::Percentage(25),
        Constraint::Percentage(25),
    ]).split(chunks[1]);

    draw_stat_card(f, cards[0], "Requests", state.stats.total_requests.to_string(), theme::CYAN);
    draw_stat_card(f, cards[1], "Tokens", format!("{:.1}K", state.stats.total_tokens as f64 / 1000.0), theme::PURPLE);
    draw_stat_card(f, cards[2], "Cost", format!("${:.2}", state.stats.total_cost_usd), theme::GREEN);
    draw_stat_card(f, cards[3], "Latency", format!("{:.0}ms", state.stats.avg_latency_ms), theme::YELLOW);

    // Hints
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(
            "  [r] Refresh  (auto-refresh every 10s)  [Esc] Back",
            theme::hint_style(),
        ))),
        chunks[2],
    );
}

fn draw_by_model(f: &mut Frame, area: Rect, state: &mut UsageState) {
    let chunks = Layout::vertical([
        Constraint::Length(2), // tabs + header
        Constraint::Min(3),    // list
        Constraint::Length(1), // hints
    ]).split(area);

    draw_tabs(f, chunks[0].clone(), state);
    f.render_widget(
        Paragraph::new(Line::from(vec![Span::styled(
            format!("  {:<20} {:<12} {:<10} {:<12} {:<12} {}",
                "Model", "Provider", "Requests", "Tokens In", "Tokens Out", "Cost"),
            theme::table_header(),
        )])),
        chunks[0],
    );

    let items: Vec<ListItem> = state.model_usage.iter().map(|m| {
        ListItem::new(Line::from(vec![
            Span::styled(format!("  {:<20}", truncate(&m.model_id, 19)), Style::default().fg(theme::CYAN)),
            Span::styled(format!(" {:<12}", m.provider), Style::default().fg(theme::PURPLE)),
            Span::styled(format!(" {:<10}", m.requests), Style::default().fg(theme::TEXT)),
            Span::styled(format!(" {:<12}", format_tokens(m.tokens_in)), Style::default().fg(theme::TEXT)),
            Span::styled(format!(" {:<12}", format_tokens(m.tokens_out)), Style::default().fg(theme::TEXT)),
            Span::styled(format!(" ${:.4}", m.cost_usd), Style::default().fg(theme::GREEN)),
        ]))
    }).collect();

    let list = List::new(items)
        .highlight_style(theme::selected_style())
        .highlight_symbol("> ");
    f.render_stateful_widget(list, chunks[1], &mut state.list_state);

    f.render_widget(
        Paragraph::new(Line::from(Span::styled(
            "  [j/k] Navigate  [r] Refresh  [1-3] Tab  [Esc] Back",
            theme::hint_style(),
        ))),
        chunks[2],
    );
}

fn draw_by_agent(f: &mut Frame, area: Rect, state: &mut UsageState) {
    let chunks = Layout::vertical([
        Constraint::Length(2), // tabs + header
        Constraint::Min(3),    // list
        Constraint::Length(1), // hints
    ]).split(area);

    draw_tabs(f, chunks[0].clone(), state);
    f.render_widget(
        Paragraph::new(Line::from(vec![Span::styled(
            format!("  {:<20} {:<10} {:<12} {}",
                "Agent", "Requests", "Tokens", "Cost"),
            theme::table_header(),
        )])),
        chunks[0],
    );

    let items: Vec<ListItem> = state.agent_usage.iter().map(|a| {
        ListItem::new(Line::from(vec![
            Span::styled(format!("  {:<20}", truncate(&a.agent_name, 19)), Style::default().fg(theme::CYAN)),
            Span::styled(format!(" {:<10}", a.requests), Style::default().fg(theme::TEXT)),
            Span::styled(format!(" {:<12}", format_tokens(a.tokens)), Style::default().fg(theme::PURPLE)),
            Span::styled(format!(" ${:.2}", a.cost_usd), Style::default().fg(theme::GREEN)),
        ]))
    }).collect();

    let list = List::new(items)
        .highlight_style(theme::selected_style())
        .highlight_symbol("> ");
    f.render_stateful_widget(list, chunks[1], &mut state.list_state);

    f.render_widget(
        Paragraph::new(Line::from(Span::styled(
            "  [j/k] Navigate  [r] Refresh  [1-3] Tab  [Esc] Back",
            theme::hint_style(),
        ))),
        chunks[2],
    );
}

fn draw_tabs(f: &mut Frame, area: Rect, state: &UsageState) {
    let tab_style = |tab: UsageSubTab| {
        if state.sub_tab == tab {
            Style::default().fg(theme::ACCENT).add_modifier(Modifier::BOLD)
        } else {
            theme::dim_style()
        }
    };

    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("  ", Style::default()),
            Span::styled("[1] Summary", tab_style(UsageSubTab::Summary)),
            Span::styled("  ", Style::default()),
            Span::styled("[2] By Model", tab_style(UsageSubTab::ByModel)),
            Span::styled("  ", Style::default()),
            Span::styled("[3] By Agent", tab_style(UsageSubTab::ByAgent)),
        ])),
        area,
    );
}

fn draw_stat_card(f: &mut Frame, area: Rect, label: &str, value: String, color: ratatui::style::Color) {
    let inner = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
    ]).split(area);

    f.render_widget(
        Paragraph::new(Span::styled(label, theme::dim_style())),
        inner[0],
    );
    f.render_widget(
        Paragraph::new(Span::styled(value, Style::default().fg(color).add_modifier(Modifier::BOLD))),
        inner[1],
    );
}

fn format_tokens(tokens: u64) -> String {
    if tokens >= 1000 { format!("{:.1}K", tokens as f64 / 1000.0) }
    else { tokens.to_string() }
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max { s.to_string() } else { format!("{}...", &s[..max.saturating_sub(3)]) }
}