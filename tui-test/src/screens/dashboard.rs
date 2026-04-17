//! Dashboard screen: system overview with stat cards and scrollable audit trail.
//!
//! Cloned from AgentDiVA TUI screens/dashboard.rs with placeholder data.

use crate::i18n::Translator;
use crate::theme;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Padding, Paragraph};
use ratatui::Frame;

// ── Data types ──────────────────────────────────────────────────────────────

#[derive(Clone, Default)]
pub struct AuditRow {
    pub timestamp: String,
    pub agent: String,
    pub action: String,
    pub detail: String,
}

// ── State ───────────────────────────────────────────────────────────────────

pub struct DashboardState {
    pub agent_count: u64,
    pub uptime_secs: u64,
    pub version: String,
    pub provider: String,
    pub model: String,
    pub recent_audit: Vec<AuditRow>,
    pub loading: bool,
    pub tick: usize,
    pub audit_scroll: u16,
}

pub enum DashboardAction {
    Continue,
    Refresh,
    GoToAgents,
}

impl DashboardState {
    pub fn new() -> Self {
        Self {
            agent_count: 3,
            uptime_secs: 1234,
            version: env!("CARGO_PKG_VERSION").to_string(),
            provider: "placeholder".to_string(),
            model: "model-placeholder".to_string(),
            recent_audit: placeholder_audit(),
            loading: false,
            tick: 0,
            audit_scroll: 0,
        }
    }

    pub fn tick(&mut self) {
        self.tick = self.tick.wrapping_add(1);
        self.uptime_secs += 1;
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> DashboardAction {
        if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
            return DashboardAction::Continue;
        }
        match key.code {
            KeyCode::Char('r') => DashboardAction::Refresh,
            KeyCode::Char('a') => DashboardAction::GoToAgents,
            KeyCode::Up | KeyCode::Char('k') => {
                self.audit_scroll = self.audit_scroll.saturating_add(1);
                DashboardAction::Continue
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.audit_scroll = self.audit_scroll.saturating_sub(1);
                DashboardAction::Continue
            }
            KeyCode::PageUp => {
                self.audit_scroll = self.audit_scroll.saturating_add(10);
                DashboardAction::Continue
            }
            KeyCode::PageDown => {
                self.audit_scroll = self.audit_scroll.saturating_sub(10);
                DashboardAction::Continue
            }
            _ => DashboardAction::Continue,
        }
    }
}

// Placeholder audit data
fn placeholder_audit() -> Vec<AuditRow> {
    vec![
        AuditRow {
            timestamp: "2026-04-16 10:23:45".to_string(),
            agent: "Coder".to_string(),
            action: "tool_call".to_string(),
            detail: "file_read(src/main.rs)".to_string(),
        },
        AuditRow {
            timestamp: "2026-04-16 10:22:30".to_string(),
            agent: "Researcher".to_string(),
            action: "message_sent".to_string(),
            detail: "User query about Rust async".to_string(),
        },
        AuditRow {
            timestamp: "2026-04-16 10:21:15".to_string(),
            agent: "Writer".to_string(),
            action: "tool_call".to_string(),
            detail: "file_write(docs/README.md)".to_string(),
        },
        AuditRow {
            timestamp: "2026-04-16 10:20:00".to_string(),
            agent: "Coder".to_string(),
            action: "spawned".to_string(),
            detail: "New agent instance created".to_string(),
        },
        AuditRow {
            timestamp: "2026-04-16 10:18:45".to_string(),
            agent: "System".to_string(),
            action: "config_reload".to_string(),
            detail: "Provider settings updated".to_string(),
        },
    ]
}

// ── Drawing ─────────────────────────────────────────────────────────────────

pub fn draw(f: &mut Frame, area: Rect, state: &mut DashboardState, _i18n: &Translator) {
    let block = Block::default()
        .title(Line::from(vec![Span::styled(
            " Dashboard ",
            theme::title_style(),
        )]))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme::ACCENT))
        .padding(Padding::horizontal(1));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let chunks = Layout::vertical([
        Constraint::Length(5), // stat cards
        Constraint::Length(1), // separator
        Constraint::Min(4),    // audit trail
        Constraint::Length(1), // hints
    ])
    .split(inner);

    // ── Stat cards ──────────────────────────────────────────────────────────
    draw_stat_cards(f, chunks[0], state);

    // ── Separator ───────────────────────────────────────────────────────────
    let sep = "\u{2500}".repeat(chunks[1].width as usize);
    f.render_widget(
        Paragraph::new(Span::styled(sep, theme::dim_style())),
        chunks[1],
    );

    // ── Audit trail ─────────────────────────────────────────────────────────
    draw_audit_trail(f, chunks[2], state);

    // ── Hints ───────────────────────────────────────────────────────────────
    let hints = Paragraph::new(Line::from(vec![Span::styled(
        "  [r] Refresh  [a] Go to Agents  [\u{2191}\u{2193}] Scroll audit",
        theme::hint_style(),
    )]));
    f.render_widget(hints, chunks[3]);
}

fn draw_stat_cards(f: &mut Frame, area: Rect, state: &DashboardState) {
    let cols = Layout::horizontal([
        Constraint::Percentage(33),
        Constraint::Percentage(34),
        Constraint::Percentage(33),
    ])
    .split(area);

    // Agents card
    let agents_block = Block::default()
        .title(Span::styled(" Agents ", Style::default().fg(theme::CYAN)))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme::DIM));
    let agents_inner = agents_block.inner(cols[0]);
    f.render_widget(agents_block, cols[0]);
    let count_text = format!("{}", state.agent_count);
    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(
                format!(" {count_text}"),
                Style::default()
                    .fg(theme::GREEN)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" active", theme::dim_style()),
        ])),
        agents_inner,
    );

    // Uptime card
    let uptime_block = Block::default()
        .title(Span::styled(" Uptime ", Style::default().fg(theme::CYAN)))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme::DIM));
    let uptime_inner = uptime_block.inner(cols[1]);
    f.render_widget(uptime_block, cols[1]);
    let uptime_str = format_uptime(state.uptime_secs);
    f.render_widget(
        Paragraph::new(Line::from(vec![Span::styled(
            format!(" {uptime_str}"),
            Style::default()
                .fg(theme::YELLOW)
                .add_modifier(Modifier::BOLD),
        )])),
        uptime_inner,
    );

    // Provider card
    let provider_block = Block::default()
        .title(Span::styled(" Provider ", Style::default().fg(theme::CYAN)))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme::DIM));
    let provider_inner = provider_block.inner(cols[2]);
    f.render_widget(provider_block, cols[2]);
    let provider_text = if state.provider.is_empty() {
        "not set".to_string()
    } else {
        format!("{}/{}", state.provider, state.model)
    };
    f.render_widget(
        Paragraph::new(Line::from(vec![Span::styled(
            format!(" {provider_text}"),
            Style::default().fg(theme::CYAN),
        )])),
        provider_inner,
    );
}

fn draw_audit_trail(f: &mut Frame, area: Rect, state: &DashboardState) {
    if state.loading {
        let spinner = theme::SPINNER_FRAMES[state.tick % theme::SPINNER_FRAMES.len()];
        f.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled(format!("  {spinner} "), Style::default().fg(theme::CYAN)),
                Span::styled("Loading audit trail\u{2026}", theme::dim_style()),
            ])),
            area,
        );
        return;
    }

    if state.recent_audit.is_empty() {
        f.render_widget(
            Paragraph::new(Span::styled("  No audit entries yet.", theme::dim_style())),
            area,
        );
        return;
    }

    let mut lines: Vec<Line> = Vec::new();

    // Header
    lines.push(Line::from(vec![Span::styled(
        format!(
            "  {:<20} {:<14} {:<16} {}",
            "Timestamp", "Agent", "Action", "Detail"
        ),
        theme::table_header(),
    )]));

    for row in &state.recent_audit {
        lines.push(Line::from(vec![
            Span::styled(format!("  {:<20}", row.timestamp), theme::dim_style()),
            Span::styled(
                format!(" {:<14}", truncate(&row.agent, 13)),
                Style::default().fg(theme::CYAN),
            ),
            Span::styled(
                format!(" {:<16}", truncate(&row.action, 15)),
                Style::default().fg(theme::YELLOW),
            ),
            Span::styled(
                format!(" {}", truncate(&row.detail, 30)),
                theme::dim_style(),
            ),
        ]));
    }

    let total = lines.len() as u16;
    let visible = area.height;
    let max_scroll = total.saturating_sub(visible);
    let scroll = max_scroll
        .saturating_sub(state.audit_scroll)
        .min(max_scroll);

    f.render_widget(Paragraph::new(lines).scroll((scroll, 0)), area);
}

fn format_uptime(secs: u64) -> String {
    if secs < 60 {
        format!("{secs}s")
    } else if secs < 3600 {
        format!("{}m {}s", secs / 60, secs % 60)
    } else if secs < 86400 {
        format!("{}h {}m", secs / 3600, (secs % 3600) / 60)
    } else {
        format!("{}d {}h", secs / 86400, (secs % 86400) / 3600)
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}\u{2026}", &s[..max.saturating_sub(1)])
    }
}