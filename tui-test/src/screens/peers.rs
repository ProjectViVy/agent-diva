//! Peers screen: OFP peer network status with auto-refresh.
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
pub struct PeerInfo {
    pub node_id: String,
    pub node_name: String,
    pub address: String,
    pub state: String,
    pub agent_count: u64,
    pub protocol_version: String,
}

// ── State ───────────────────────────────────────────────────────────────────

pub struct PeersState {
    pub peers: Vec<PeerInfo>,
    pub list_state: ListState,
    pub loading: bool,
    pub tick: usize,
    pub poll_tick: usize,
    pub status_msg: String,
}

pub enum PeersAction {
    Continue,
    Back,
    Refresh,
}

impl PeersState {
    pub fn new() -> Self {
        // Populate with fake data
        let peers = vec![
            PeerInfo {
                node_id: "node-alpha-001".to_string(),
                node_name: "alpha-node".to_string(),
                address: "192.168.1.100:8443".to_string(),
                state: "Connected".to_string(),
                agent_count: 5,
                protocol_version: "OFP/1.2".to_string(),
            },
            PeerInfo {
                node_id: "node-beta-002".to_string(),
                node_name: "beta-node".to_string(),
                address: "192.168.1.101:8443".to_string(),
                state: "Connected".to_string(),
                agent_count: 3,
                protocol_version: "OFP/1.2".to_string(),
            },
            PeerInfo {
                node_id: "node-gamma-003".to_string(),
                node_name: "gamma-node".to_string(),
                address: "192.168.1.102:8443".to_string(),
                state: "Connecting".to_string(),
                agent_count: 0,
                protocol_version: "OFP/1.2".to_string(),
            },
            PeerInfo {
                node_id: "node-delta-004".to_string(),
                node_name: "delta-node".to_string(),
                address: "192.168.1.103:8443".to_string(),
                state: "Disconnected".to_string(),
                agent_count: 2,
                protocol_version: "OFP/1.1".to_string(),
            },
            PeerInfo {
                node_id: "node-epsilon-005".to_string(),
                node_name: "epsilon-node".to_string(),
                address: "10.0.0.50:8443".to_string(),
                state: "Connected".to_string(),
                agent_count: 8,
                protocol_version: "OFP/1.2".to_string(),
            },
        ];

        Self {
            peers,
            list_state: ListState::default().with_selected(Some(0)),
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

    /// Returns true if it's time to auto-refresh (every ~15s at 20fps tick rate).
    pub fn should_poll(&self) -> bool {
        self.poll_tick > 0 && self.poll_tick % 300 == 0
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> PeersAction {
        if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
            return PeersAction::Continue;
        }
        let total = self.peers.len();
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
            KeyCode::Char('r') => return PeersAction::Refresh,
            KeyCode::Esc => return PeersAction::Back,
            _ => {}
        }
        PeersAction::Continue
    }
}

// ── Drawing ─────────────────────────────────────────────────────────────────

pub fn draw(f: &mut Frame, area: Rect, state: &mut PeersState, _i18n: &Translator) {
    let block = Block::default()
        .title(Line::from(vec![Span::styled(" Peers ", theme::title_style())]))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme::ACCENT))
        .padding(Padding::horizontal(1));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let chunks = Layout::vertical([
        Constraint::Length(2), // header
        Constraint::Min(3),    // list
        Constraint::Length(1), // hints
    ]).split(inner);

    // Header
    f.render_widget(
        Paragraph::new(vec![
            Line::from(vec![Span::styled(
                format!("  OFP Peer Network  ({} peers)", state.peers.len()),
                Style::default().fg(theme::CYAN).add_modifier(Modifier::BOLD),
            )]),
            Line::from(vec![Span::styled(
                format!(
                    "  {:<14} {:<16} {:<20} {:<14} {:<8} {}",
                    "Node ID", "Name", "Address", "State", "Agents", "Protocol"
                ),
                theme::table_header(),
            )]),
        ]),
        chunks[0],
    );

    // List
    if state.loading && state.peers.is_empty() {
        let spinner = theme::SPINNER_FRAMES[state.tick % theme::SPINNER_FRAMES.len()];
        f.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled(format!("  {spinner} "), Style::default().fg(theme::CYAN)),
                Span::styled("Discovering peers...", theme::dim_style()),
            ])),
            chunks[1],
        );
    } else if state.peers.is_empty() {
        f.render_widget(
            Paragraph::new(Span::styled(
                "  No peers connected. Configure [network] in config.toml to enable OFP.",
                theme::dim_style(),
            )),
            chunks[1],
        );
    } else {
        let items: Vec<ListItem> = state
            .peers
            .iter()
            .map(|p| {
                let id_short = if p.node_id.len() > 12 {
                    format!("{}...", &p.node_id[..12])
                } else {
                    p.node_id.clone()
                };
                let (state_badge, state_style) = match p.state.to_lowercase().as_str() {
                    "connected" | "active" => (" Connected", Style::default().fg(theme::GREEN)),
                    "disconnected" | "inactive" => (" Disconnected", Style::default().fg(theme::RED)),
                    "connecting" | "pending" => (" Connecting", Style::default().fg(theme::YELLOW)),
                    _ => (&*p.state, theme::dim_style()),
                };
                ListItem::new(Line::from(vec![
                    Span::styled(format!("  {:<14}", id_short), Style::default().fg(theme::PURPLE)),
                    Span::styled(format!(" {:<16}", truncate(&p.node_name, 15)), Style::default().fg(theme::CYAN)),
                    Span::styled(format!(" {:<20}", truncate(&p.address, 19)), theme::dim_style()),
                    Span::styled(format!(" {:<14}", state_badge), state_style),
                    Span::styled(format!(" {:<8}", p.agent_count), Style::default().fg(theme::GREEN)),
                    Span::styled(format!(" {}", &p.protocol_version), theme::dim_style()),
                ]))
            })
            .collect();

        let list = List::new(items)
            .highlight_style(theme::selected_style())
            .highlight_symbol("> ");
        f.render_stateful_widget(list, chunks[1], &mut state.list_state);
    }

    // Hints
    let hint_text = if !state.status_msg.is_empty() {
        format!("  {} | [r] Refresh  (auto-refreshes every 15s)", state.status_msg)
    } else {
        "  [j/k] Navigate  [r] Refresh  (auto-refreshes every 15s)  [Esc] Back".to_string()
    };
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(hint_text, theme::hint_style()))),
        chunks[2],
    );
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}...", &s[..max.saturating_sub(1)])
    }
}