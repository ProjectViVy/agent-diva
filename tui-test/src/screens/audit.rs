//! Audit screen: Audit log viewing and chain verification.
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
pub struct AuditEntry {
    pub id: String,
    pub timestamp: String,
    pub action: String,
    pub agent_id: String,
    pub agent_name: String,
    pub target: String,
    pub hash: String,
    pub chain_valid: bool,
}

const ACTION_FILTERS: &[&str] = &[
    "All",
    "Tool",
    "Agent",
    "Message",
    "Config",
    "System",
];

// ── State ───────────────────────────────────────────────────────────────────

pub struct AuditState {
    pub entries: Vec<AuditEntry>,
    pub list_state: ListState,
    pub action_filter: usize,
    pub loading: bool,
    pub tick: usize,
    // Search mode
    pub search_query: String,
    pub search_active: bool,
    // Detail view
    pub show_detail: bool,
    pub detail_entry: Option<AuditEntry>,
    // Chain verification modal
    pub show_chain_verify: bool,
    pub chain_verify_hash: String,
    pub chain_verify_valid: bool,
    // Status
    pub status_msg: String,
}

pub enum AuditAction {
    Continue,
    Back,
    Refresh,
    Search { query: String },
    VerifyChain { hash: String },
}

impl AuditState {
    pub fn new() -> Self {
        // Populate with fake data
        let entries = vec![
            AuditEntry {
                id: "audit-001".to_string(),
                timestamp: "2025-04-16T08:00:00Z".to_string(),
                action: "Tool".to_string(),
                agent_id: "agent-001".to_string(),
                agent_name: "research-agent".to_string(),
                target: "file_read: src/main.rs".to_string(),
                hash: "a1b2c3d4e5f6...".to_string(),
                chain_valid: true,
            },
            AuditEntry {
                id: "audit-002".to_string(),
                timestamp: "2025-04-16T08:01:00Z".to_string(),
                action: "Message".to_string(),
                agent_id: "agent-001".to_string(),
                agent_name: "research-agent".to_string(),
                target: "root-agent".to_string(),
                hash: "b2c3d4e5f6a1...".to_string(),
                chain_valid: true,
            },
            AuditEntry {
                id: "audit-003".to_string(),
                timestamp: "2025-04-16T08:02:00Z".to_string(),
                action: "Agent".to_string(),
                agent_id: "root-agent".to_string(),
                agent_name: "root-agent".to_string(),
                target: "spawn: code-agent".to_string(),
                hash: "c3d4e5f6a1b2...".to_string(),
                chain_valid: true,
            },
            AuditEntry {
                id: "audit-004".to_string(),
                timestamp: "2025-04-16T08:03:00Z".to_string(),
                action: "Tool".to_string(),
                agent_id: "agent-002".to_string(),
                agent_name: "code-agent".to_string(),
                target: "shell_exec: cargo build".to_string(),
                hash: "d4e5f6a1b2c3...".to_string(),
                chain_valid: true,
            },
            AuditEntry {
                id: "audit-005".to_string(),
                timestamp: "2025-04-16T08:04:00Z".to_string(),
                action: "Config".to_string(),
                agent_id: "system".to_string(),
                agent_name: "system".to_string(),
                target: "update: provider config".to_string(),
                hash: "e5f6a1b2c3d4...".to_string(),
                chain_valid: true,
            },
            AuditEntry {
                id: "audit-006".to_string(),
                timestamp: "2025-04-16T08:05:00Z".to_string(),
                action: "System".to_string(),
                agent_id: "system".to_string(),
                agent_name: "system".to_string(),
                target: "heartbeat: check".to_string(),
                hash: "f6a1b2c3d4e5...".to_string(),
                chain_valid: true,
            },
        ];

        Self {
            entries,
            list_state: ListState::default().with_selected(Some(0)),
            action_filter: 0,
            loading: false,
            tick: 0,
            search_query: String::new(),
            search_active: false,
            show_detail: false,
            detail_entry: None,
            show_chain_verify: false,
            chain_verify_hash: String::new(),
            chain_verify_valid: true,
            status_msg: String::new(),
        }
    }

    pub fn tick(&mut self) {
        self.tick = self.tick.wrapping_add(1);
    }

    fn filtered_entries(&self) -> Vec<AuditEntry> {
        let filter = ACTION_FILTERS[self.action_filter];
        if filter == "All" {
            self.entries.clone()
        } else {
            self.entries.iter()
                .filter(|e| e.action == filter)
                .cloned()
                .collect()
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> AuditAction {
        if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
            return AuditAction::Continue;
        }

        if self.show_chain_verify {
            match key.code {
                KeyCode::Esc | KeyCode::Enter => {
                    self.show_chain_verify = false;
                }
                _ => {}
            }
            return AuditAction::Continue;
        }

        if self.show_detail {
            match key.code {
                KeyCode::Esc => {
                    self.show_detail = false;
                    self.detail_entry = None;
                }
                KeyCode::Char('v') => {
                    if let Some(entry) = &self.detail_entry {
                        self.chain_verify_hash = entry.hash.clone();
                        self.chain_verify_valid = entry.chain_valid;
                        self.show_chain_verify = true;
                    }
                }
                _ => {}
            }
            return AuditAction::Continue;
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
                    return AuditAction::Search { query };
                }
                KeyCode::Char(c) => {
                    self.search_query.push(c);
                }
                KeyCode::Backspace => {
                    self.search_query.pop();
                }
                _ => {}
            }
            return AuditAction::Continue;
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
            KeyCode::Char('a') | KeyCode::Char('A') => {
                self.action_filter = (self.action_filter + 1) % ACTION_FILTERS.len();
                self.list_state.select(Some(0));
            }
            KeyCode::Char('/') => {
                self.search_active = true;
                self.search_query.clear();
            }
            KeyCode::Enter => {
                let entries = self.filtered_entries();
                if let Some(idx) = self.list_state.selected() {
                    if let Some(entry) = entries.get(idx) {
                        self.detail_entry = Some(entry.clone());
                        self.show_detail = true;
                    }
                }
            }
            KeyCode::Char('v') => {
                let entries = self.filtered_entries();
                if let Some(idx) = self.list_state.selected() {
                    if let Some(entry) = entries.get(idx) {
                        self.chain_verify_hash = entry.hash.clone();
                        self.chain_verify_valid = entry.chain_valid;
                        self.show_chain_verify = true;
                        return AuditAction::VerifyChain { hash: entry.hash.clone() };
                    }
                }
            }
            KeyCode::Char('r') => return AuditAction::Refresh,
            KeyCode::Esc => return AuditAction::Back,
            _ => {}
        }
        AuditAction::Continue
    }
}

// ── Drawing ─────────────────────────────────────────────────────────────────

pub fn draw(f: &mut Frame, area: Rect, state: &mut AuditState, _i18n: &Translator) {
    let block = Block::default()
        .title(Line::from(vec![Span::styled(" Audit ", theme::title_style())]))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme::ACCENT))
        .padding(Padding::horizontal(1));

    let inner = block.inner(area);
    f.render_widget(block, area);

    if state.show_chain_verify {
        draw_chain_verify(f, inner, state);
    } else if state.show_detail {
        draw_detail(f, inner, state);
    } else if state.search_active {
        draw_search(f, inner, state);
    } else {
        draw_list(f, inner, state);
    }
}

fn draw_list(f: &mut Frame, area: Rect, state: &mut AuditState) {
    let chunks = Layout::vertical([
        Constraint::Length(2), // header
        Constraint::Min(3),    // list
        Constraint::Length(1), // hints
    ]).split(area);

    // Filter header
    let filter_spans: Vec<Span> = ACTION_FILTERS.iter().enumerate().map(|(i, f)| {
        if i == state.action_filter {
            Span::styled(format!("[{}]", f), Style::default().fg(theme::ACCENT).add_modifier(Modifier::BOLD))
        } else {
            Span::styled(*f, theme::dim_style())
        }
    }).collect();

    let entries = state.filtered_entries();

    f.render_widget(
        Paragraph::new(vec![
            Line::from(filter_spans),
            Line::from(vec![Span::styled(
                format!("  {:<10} {:<16} {:<14} {:<24} {:<8} {}",
                    "Time", "Agent", "Action", "Target", "Chain", "Hash"),
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
                Span::styled("Loading audit log...", theme::dim_style()),
            ])),
            chunks[1],
        );
    } else if entries.is_empty() {
        f.render_widget(
            Paragraph::new(Span::styled("  No audit entries for this filter.", theme::dim_style())),
            chunks[1],
        );
    } else {
        let items: Vec<ListItem> = entries
            .iter()
            .map(|entry| {
                let (chain_icon, chain_style) = if entry.chain_valid {
                    ("[OK]", Style::default().fg(theme::GREEN))
                } else {
                    ("[ERR]", Style::default().fg(theme::RED))
                };
                ListItem::new(Line::from(vec![
                    Span::styled(format!("  {:<10}", short_time(&entry.timestamp)), theme::dim_style()),
                    Span::styled(format!(" {:<16}", truncate(&entry.agent_name, 15)), Style::default().fg(theme::CYAN)),
                    Span::styled(format!(" {:<14}", entry.action), Style::default().fg(theme::PURPLE)),
                    Span::styled(format!(" {:<24}", truncate(&entry.target, 23)), Style::default().fg(theme::TEXT)),
                    Span::styled(format!(" {:<8}", chain_icon), chain_style),
                    Span::styled(format!(" {}", truncate(&entry.hash, 12)), theme::dim_style()),
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
            "  [a]filter  [v]verify  [Enter]detail  [/]search  [r]refresh  [Esc]back",
            theme::hint_style(),
        ))),
        chunks[2],
    );
}

fn draw_search(f: &mut Frame, area: Rect, state: &mut AuditState) {
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
        Paragraph::new(Span::styled("  Type to search audit entries by agent, action, or target...", theme::dim_style())),
        chunks[1],
    );

    f.render_widget(
        Paragraph::new(Span::styled("[Enter] search  [Esc] cancel", theme::hint_style())),
        chunks[2],
    );
}

fn draw_detail(f: &mut Frame, area: Rect, state: &AuditState) {
    if let Some(entry) = &state.detail_entry {
        let chunks = Layout::vertical([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
        ]).split(area);

        f.render_widget(
            Paragraph::new(Span::styled(
                format!("Audit Entry: {}", entry.id),
                Style::default().fg(theme::ACCENT).add_modifier(Modifier::BOLD),
            )),
            chunks[0],
        );

        f.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled("  Timestamp: ", theme::dim_style()),
                Span::styled(&entry.timestamp, Style::default().fg(theme::TEXT)),
            ])),
            chunks[1],
        );

        f.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled("  Agent: ", theme::dim_style()),
                Span::styled(format!("{} ({})", entry.agent_name, entry.agent_id), Style::default().fg(theme::CYAN)),
            ])),
            chunks[2],
        );

        f.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled("  Action: ", theme::dim_style()),
                Span::styled(&entry.action, Style::default().fg(theme::PURPLE)),
            ])),
            chunks[3],
        );

        f.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled("  Target: ", theme::dim_style()),
                Span::styled(&entry.target, Style::default().fg(theme::TEXT)),
            ])),
            chunks[4],
        );

        f.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled("  Hash: ", theme::dim_style()),
                Span::styled(&entry.hash, Style::default().fg(theme::TEXT)),
            ])),
            chunks[5],
        );

        let (chain_status, chain_style) = if entry.chain_valid {
            ("Chain Valid", Style::default().fg(theme::GREEN))
        } else {
            ("Chain Invalid", Style::default().fg(theme::RED))
        };
        f.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled("  Chain: ", theme::dim_style()),
                Span::styled(chain_status, chain_style),
            ])),
            chunks[6],
        );

        f.render_widget(
            Paragraph::new(Span::styled("[v] verify chain  [Esc] close", theme::hint_style())),
            chunks[7],
        );
    }
}

fn draw_chain_verify(f: &mut Frame, area: Rect, state: &AuditState) {
    let chunks = Layout::vertical([
        Constraint::Length(1),
        Constraint::Min(3),
        Constraint::Length(1),
    ]).split(area);

    f.render_widget(
        Paragraph::new(Span::styled(
            " Chain Verification ",
            Style::default().fg(theme::ACCENT).add_modifier(Modifier::BOLD),
        )),
        chunks[0],
    );

    let (status, style) = if state.chain_verify_valid {
        ("Hash verified. Chain integrity confirmed.\n\nHash: ".to_string(), Style::default().fg(theme::GREEN))
    } else {
        ("Hash verification failed. Chain may be corrupted.\n\nHash: ".to_string(), Style::default().fg(theme::RED))
    };

    f.render_widget(
        Paragraph::new(vec![
            Line::from(Span::styled(status, style)),
            Line::from(Span::styled(state.chain_verify_hash.clone(), Style::default().fg(theme::TEXT))),
        ]),
        chunks[1],
    );

    f.render_widget(
        Paragraph::new(Span::styled("[Enter/Esc] Close", theme::hint_style())),
        chunks[2],
    );
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