//! Security screen: Security features and chain verification.
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
pub struct SecurityFeature {
    pub name: String,
    pub status: String,
    pub description: String,
    pub enabled: bool,
}

#[derive(Clone, Default)]
pub struct ChainVerification {
    pub name: String,
    pub hash: String,
    pub verified: bool,
    pub timestamp: String,
}

// ── State ───────────────────────────────────────────────────────────────────

pub struct SecurityState {
    pub features: Vec<SecurityFeature>,
    pub chains: Vec<ChainVerification>,
    pub list_state: ListState,
    pub loading: bool,
    pub tick: usize,
    // Verification mode
    pub show_verification: bool,
    pub verification_result: String,
    // Status
    pub status_msg: String,
}

pub enum SecurityAction {
    Continue,
    Back,
    Refresh,
    VerifyChain,
}

impl SecurityState {
    pub fn new() -> Self {
        // Populate with fake data
        let features = vec![
            SecurityFeature {
                name: "Input Validation".to_string(),
                status: "Active".to_string(),
                description: "Validates all input data for malicious content".to_string(),
                enabled: true,
            },
            SecurityFeature {
                name: "Rate Limiting".to_string(),
                status: "Active".to_string(),
                description: "Limits API calls per agent to prevent abuse".to_string(),
                enabled: true,
            },
            SecurityFeature {
                name: "Tool Sandbox".to_string(),
                status: "Active".to_string(),
                description: "Executes tools in isolated sandbox environment".to_string(),
                enabled: true,
            },
            SecurityFeature {
                name: "Audit Logging".to_string(),
                status: "Active".to_string(),
                description: "Logs all agent actions for audit trail".to_string(),
                enabled: true,
            },
            SecurityFeature {
                name: "Message Signing".to_string(),
                status: "Inactive".to_string(),
                description: "Signs messages with cryptographic keys".to_string(),
                enabled: false,
            },
            SecurityFeature {
                name: "Encrypted Storage".to_string(),
                status: "Active".to_string(),
                description: "Encrypts session and memory data at rest".to_string(),
                enabled: true,
            },
        ];

        let chains = vec![
            ChainVerification {
                name: "config.toml".to_string(),
                hash: "a1b2c3d4e5f6...".to_string(),
                verified: true,
                timestamp: "2025-04-16T08:00:00Z".to_string(),
            },
            ChainVerification {
                name: "agents/agent-a.md".to_string(),
                hash: "f6e5d4c3b2a1...".to_string(),
                verified: true,
                timestamp: "2025-04-16T08:00:00Z".to_string(),
            },
            ChainVerification {
                name: "sessions/session-1.jsonl".to_string(),
                hash: "1234567890ab...".to_string(),
                verified: true,
                timestamp: "2025-04-16T08:00:00Z".to_string(),
            },
            ChainVerification {
                name: "memory/agent-a/memory.md".to_string(),
                hash: "abcdef123456...".to_string(),
                verified: true,
                timestamp: "2025-04-16T08:00:00Z".to_string(),
            },
        ];

        Self {
            features,
            chains,
            list_state: ListState::default().with_selected(Some(0)),
            loading: false,
            tick: 0,
            show_verification: false,
            verification_result: String::new(),
            status_msg: String::new(),
        }
    }

    pub fn tick(&mut self) {
        self.tick = self.tick.wrapping_add(1);
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> SecurityAction {
        if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
            return SecurityAction::Continue;
        }

        if self.show_verification {
            match key.code {
                KeyCode::Esc | KeyCode::Enter => {
                    self.show_verification = false;
                }
                _ => {}
            }
            return SecurityAction::Continue;
        }

        let total = self.features.len();
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
            KeyCode::Char('v') => {
                self.show_verification = true;
                self.verification_result = "All 4 chain links verified. Hash chain integrity confirmed.\n\nTimestamp: 2025-04-16T08:00:00Z\nRoot hash: a1b2c3d4e5f67890...".to_string();
                return SecurityAction::VerifyChain;
            }
            KeyCode::Char('r') => return SecurityAction::Refresh,
            KeyCode::Esc => return SecurityAction::Back,
            _ => {}
        }
        SecurityAction::Continue
    }
}

// ── Drawing ─────────────────────────────────────────────────────────────────

pub fn draw(f: &mut Frame, area: Rect, state: &mut SecurityState, _i18n: &Translator) {
    let block = Block::default()
        .title(Line::from(vec![Span::styled(" Security ", theme::title_style())]))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme::ACCENT))
        .padding(Padding::horizontal(1));

    let inner = block.inner(area);
    f.render_widget(block, area);

    if state.show_verification {
        draw_verification(f, inner, state);
    } else {
        draw_main(f, inner, state);
    }
}

fn draw_main(f: &mut Frame, area: Rect, state: &mut SecurityState) {
    let chunks = Layout::vertical([
        Constraint::Length(2), // header
        Constraint::Length(1), // separator
        Constraint::Min(3),    // features
        Constraint::Length(1), // separator
        Constraint::Length(4), // chain verification
        Constraint::Length(1), // hints
    ]).split(area);

    // Header
    f.render_widget(
        Paragraph::new(vec![
            Line::from(vec![Span::styled(
                "  Security Features",
                Style::default().fg(theme::CYAN).add_modifier(Modifier::BOLD),
            )]),
            Line::from(vec![Span::styled(
                format!("  {:<20} {:<10} {}", "Feature", "Status", "Description"),
                theme::table_header(),
            )]),
        ]),
        chunks[0],
    );

    // Features list
    let items: Vec<ListItem> = state
        .features
        .iter()
        .map(|feat| {
            let (status_badge, status_style) = if feat.enabled {
                ("Active", Style::default().fg(theme::GREEN))
            } else {
                ("Inactive", Style::default().fg(theme::YELLOW))
            };
            ListItem::new(Line::from(vec![
                Span::styled(format!("  {:<20}", feat.name), Style::default().fg(theme::CYAN)),
                Span::styled(format!(" {:<10}", status_badge), status_style),
                Span::styled(format!(" {}", feat.description), theme::dim_style()),
            ]))
        })
        .collect();

    let list = List::new(items)
        .highlight_style(theme::selected_style())
        .highlight_symbol("> ");
    f.render_stateful_widget(list, chunks[2], &mut state.list_state);

    // Chain verification section
    f.render_widget(
        Paragraph::new(Line::from(Span::styled("-".repeat(area.width as usize), theme::dim_style()))),
        chunks[3],
    );

    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("  Chain Verification", Style::default().fg(theme::CYAN).add_modifier(Modifier::BOLD)),
            Span::styled(format!("  ({} links)", state.chains.len()), theme::dim_style()),
        ])),
        chunks[4],
    );

    let chain_items: Vec<Line> = state.chains.iter().map(|chain| {
        let (status_icon, status_style) = if chain.verified {
            ("[OK]", Style::default().fg(theme::GREEN))
        } else {
            ("[ERR]", Style::default().fg(theme::RED))
        };
        Line::from(vec![
            Span::styled(format!("    {:<24}", truncate(&chain.name, 23)), Style::default().fg(theme::TEXT)),
            Span::styled(format!(" {:<8}", status_icon), status_style),
            Span::styled(format!(" hash:{}", truncate(&chain.hash, 12)), theme::dim_style()),
        ])
    }).collect();

    f.render_widget(Paragraph::new(chain_items), chunks[4]);

    // Hints
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(
            "  [v] Verify chain  [r] Refresh  [j/k] Navigate  [Esc] Back",
            theme::hint_style(),
        ))),
        chunks[5],
    );
}

fn draw_verification(f: &mut Frame, area: Rect, state: &SecurityState) {
    let chunks = Layout::vertical([
        Constraint::Length(1),
        Constraint::Min(3),
        Constraint::Length(1),
    ]).split(area);

    f.render_widget(
        Paragraph::new(Span::styled(
            " Chain Verification Result ",
            Style::default().fg(theme::ACCENT).add_modifier(Modifier::BOLD),
        )),
        chunks[0],
    );

    // Verification result
    let result_lines: Vec<Line> = state.verification_result.lines().map(|l| {
        Line::from(Span::styled(format!("  {}", l), Style::default().fg(theme::TEXT)))
    }).collect();

    f.render_widget(Paragraph::new(result_lines), chunks[1]);

    f.render_widget(
        Paragraph::new(Span::styled("[Enter/Esc] Close", theme::hint_style())),
        chunks[2],
    );
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max { s.to_string() } else { format!("{}...", &s[..max.saturating_sub(3)]) }
}