//! Logs screen: Log viewing with level filter and auto-refresh.
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
pub struct LogEntry {
    pub timestamp: String,
    pub level: String,
    pub source: String,
    pub message: String,
}

const LEVEL_FILTERS: &[&str] = &[
    "All",
    "ERROR",
    "WARN",
    "INFO",
    "DEBUG",
    "TRACE",
];

// ── State ───────────────────────────────────────────────────────────────────

pub struct LogsState {
    pub entries: Vec<LogEntry>,
    pub list_state: ListState,
    pub level_filter: usize,
    pub loading: bool,
    pub tick: usize,
    pub poll_tick: usize,
    pub auto_refresh: bool,
    // Search mode
    pub search_query: String,
    pub search_active: bool,
    // Status
    pub status_msg: String,
}

pub enum LogsAction {
    Continue,
    Back,
    Refresh,
    Search { query: String },
    ToggleAutoRefresh,
}

impl LogsState {
    pub fn new() -> Self {
        // Populate with fake data
        let entries = vec![
            LogEntry { timestamp: "2025-04-16T08:00:00Z".to_string(), level: "INFO".to_string(), source: "agent-diva".to_string(), message: "Application started successfully".to_string() },
            LogEntry { timestamp: "2025-04-16T08:00:01Z".to_string(), level: "INFO".to_string(), source: "gateway".to_string(), message: "Gateway listening on port 8080".to_string() },
            LogEntry { timestamp: "2025-04-16T08:00:02Z".to_string(), level: "DEBUG".to_string(), source: "agent-loop".to_string(), message: "Initializing agent loop for root-agent".to_string() },
            LogEntry { timestamp: "2025-04-16T08:00:05Z".to_string(), level: "INFO".to_string(), source: "provider".to_string(), message: "Provider anthropic connected".to_string() },
            LogEntry { timestamp: "2025-04-16T08:00:10Z".to_string(), level: "WARN".to_string(), source: "channel".to_string(), message: "Channel slack retrying connection (attempt 2)".to_string() },
            LogEntry { timestamp: "2025-04-16T08:00:15Z".to_string(), level: "INFO".to_string(), source: "agent-loop".to_string(), message: "Processing message from user: 'Hello'".to_string() },
            LogEntry { timestamp: "2025-04-16T08:00:20Z".to_string(), level: "DEBUG".to_string(), source: "tool".to_string(), message: "Tool file_read executed: src/main.rs".to_string() },
            LogEntry { timestamp: "2025-04-16T08:00:25Z".to_string(), level: "ERROR".to_string(), source: "provider".to_string(), message: "Provider deepseek: API key not configured".to_string() },
            LogEntry { timestamp: "2025-04-16T08:00:30Z".to_string(), level: "INFO".to_string(), source: "channel".to_string(), message: "Channel slack connected successfully".to_string() },
            LogEntry { timestamp: "2025-04-16T08:00:35Z".to_string(), level: "TRACE".to_string(), source: "agent-loop".to_string(), message: "Token usage: 150 input, 45 output".to_string() },
            LogEntry { timestamp: "2025-04-16T08:00:40Z".to_string(), level: "WARN".to_string(), source: "memory".to_string(), message: "Memory file large, consider cleanup".to_string() },
            LogEntry { timestamp: "2025-04-16T08:00:45Z".to_string(), level: "INFO".to_string(), source: "session".to_string(), message: "Session saved: session-001.jsonl".to_string() },
        ];

        Self {
            entries,
            list_state: ListState::default().with_selected(Some(0)),
            level_filter: 0,
            loading: false,
            tick: 0,
            poll_tick: 0,
            auto_refresh: true,
            search_query: String::new(),
            search_active: false,
            status_msg: String::new(),
        }
    }

    pub fn tick(&mut self) {
        self.tick = self.tick.wrapping_add(1);
        self.poll_tick = self.poll_tick.wrapping_add(1);
    }

    pub fn should_poll(&self) -> bool {
        self.auto_refresh && self.poll_tick > 0 && self.poll_tick % 100 == 0
    }

    fn filtered_entries(&self) -> Vec<LogEntry> {
        let filter = LEVEL_FILTERS[self.level_filter];
        if filter == "All" {
            self.entries.clone()
        } else {
            self.entries.iter().filter(|e| e.level == filter).cloned().collect()
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> LogsAction {
        if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
            return LogsAction::Continue;
        }

        if self.search_active {
            match key.code {
                KeyCode::Esc => {
                    self.search_active = false;
                    self.search_query.clear();
                }
                KeyCode::Enter => {
                    let query = self.search_query.clone();
                    self.search_active = false;
                    return LogsAction::Search { query };
                }
                KeyCode::Char(c) => self.search_query.push(c),
                KeyCode::Backspace => { self.search_query.pop(); }
                _ => {}
            }
            return LogsAction::Continue;
        }

        let entries = self.filtered_entries();
        let total = entries.len();

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
            KeyCode::Char('l') | KeyCode::Char('L') => {
                self.level_filter = (self.level_filter + 1) % LEVEL_FILTERS.len();
                self.list_state.select(Some(0));
            }
            KeyCode::Char('a') | KeyCode::Char('A') => {
                self.auto_refresh = !self.auto_refresh;
                return LogsAction::ToggleAutoRefresh;
            }
            KeyCode::Char('/') => {
                self.search_active = true;
                self.search_query.clear();
            }
            KeyCode::Char('r') => return LogsAction::Refresh,
            KeyCode::Esc => return LogsAction::Back,
            _ => {}
        }
        LogsAction::Continue
    }
}

// ── Drawing ─────────────────────────────────────────────────────────────────

pub fn draw(f: &mut Frame, area: Rect, state: &mut LogsState, _i18n: &Translator) {
    let block = Block::default()
        .title(Line::from(vec![Span::styled(" Logs ", theme::title_style())]))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme::ACCENT))
        .padding(Padding::horizontal(1));

    let inner = block.inner(area);
    f.render_widget(block, area);

    if state.search_active {
        draw_search(f, inner, state);
    } else {
        draw_list(f, inner, state);
    }
}

fn draw_list(f: &mut Frame, area: Rect, state: &mut LogsState) {
    let chunks = Layout::vertical([
        Constraint::Length(2), // header
        Constraint::Min(3),    // list
        Constraint::Length(1), // hints
    ]).split(area);

    // Filter header
    let filter_spans: Vec<Span> = LEVEL_FILTERS.iter().enumerate().map(|(i, f)| {
        if i == state.level_filter {
            Span::styled(format!("[{}]", f), Style::default().fg(theme::ACCENT).add_modifier(Modifier::BOLD))
        } else {
            Span::styled(*f, theme::dim_style())
        }
    }).collect();

    let entries = state.filtered_entries();
    let auto_icon = if state.auto_refresh { "[AUTO]" } else { "[MANUAL]" };

    f.render_widget(
        Paragraph::new(vec![
            Line::from(vec![
                Span::styled("  ", Style::default()),
                Span::styled("Level:", theme::dim_style()),
                Span::styled(" ", Style::default()),
            ].into_iter().chain(filter_spans.into_iter()).chain(vec![
                Span::styled(format!("  {}  {} entries", auto_icon, entries.len()), theme::dim_style()),
            ]).collect::<Vec<Span>>()),
            Line::from(vec![Span::styled(
                format!("  {:<10} {:<8} {:<12} {}", "Time", "Level", "Source", "Message"),
                theme::table_header(),
            )]),
        ]),
        chunks[0],
    );

    if state.loading && entries.is_empty() {
        let spinner = theme::SPINNER_FRAMES[state.tick % theme::SPINNER_FRAMES.len()];
        f.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled(format!("  {spinner} "), Style::default().fg(theme::CYAN)),
                Span::styled("Loading logs...", theme::dim_style()),
            ])),
            chunks[1],
        );
    } else if entries.is_empty() {
        f.render_widget(
            Paragraph::new(Span::styled("  No logs for this filter level.", theme::dim_style())),
            chunks[1],
        );
    } else {
        let items: Vec<ListItem> = entries
            .iter()
            .map(|entry| {
                let level_style = level_color(&entry.level);
                ListItem::new(Line::from(vec![
                    Span::styled(format!("  {:<10}", short_time(&entry.timestamp)), theme::dim_style()),
                    Span::styled(format!(" {:<8}", entry.level), level_style),
                    Span::styled(format!(" {:<12}", truncate(&entry.source, 11)), Style::default().fg(theme::PURPLE)),
                    Span::styled(format!(" {}", truncate(&entry.message, 60)), Style::default().fg(theme::TEXT)),
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
            "  [l] Filter  [a] Auto-refresh  [/] Search  [r] Refresh  [j/k] Scroll  [Esc] Back",
            theme::hint_style(),
        ))),
        chunks[2],
    );
}

fn draw_search(f: &mut Frame, area: Rect, state: &mut LogsState) {
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

    f.render_widget(
        Paragraph::new(Span::styled("  Type to search logs by source or message...", theme::dim_style())),
        chunks[1],
    );

    f.render_widget(
        Paragraph::new(Span::styled("[Enter] search  [Esc] cancel", theme::hint_style())),
        chunks[2],
    );
}

fn level_color(level: &str) -> Style {
    match level {
        "ERROR" => Style::default().fg(theme::RED),
        "WARN" => Style::default().fg(theme::YELLOW),
        "INFO" => Style::default().fg(theme::GREEN),
        "DEBUG" => Style::default().fg(theme::CYAN),
        "TRACE" => theme::dim_style(),
        _ => theme::dim_style(),
    }
}

fn short_time(ts: &str) -> String {
    if let Some(t_pos) = ts.find('T') {
        let time_part = &ts[t_pos + 1..];
        if time_part.len() >= 8 { return time_part[..8].to_string(); }
    }
    ts.chars().take(8).collect()
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max { s.to_string() } else { format!("{}...", &s[..max.saturating_sub(3)]) }
}