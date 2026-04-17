//! Hands screen: Agent hand management with Marketplace and Active tabs.
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
pub struct HandInfo {
    pub id: String,
    pub name: String,
    pub description: String,
    pub provider: String,
    pub status: String, // "active", "paused", "inactive"
    pub tasks_completed: u64,
    pub last_activity: Option<String>,
    pub capabilities: Vec<String>,
}

// ── State ───────────────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum HandsSubTab {
    Marketplace,
    Active,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum HandsSubScreen {
    List,
    Detail,
}

pub struct HandsState {
    pub marketplace_hands: Vec<HandInfo>,
    pub active_hands: Vec<HandInfo>,
    pub list_state: ListState,
    pub sub_tab: HandsSubTab,
    pub sub: HandsSubScreen,
    pub loading: bool,
    pub tick: usize,
    // Detail view
    pub detail_hand: Option<HandInfo>,
    // Activate confirmation
    pub show_activate_confirm: bool,
    pub activate_hand_name: String,
    pub activate_hand_id: String,
    // Pause confirmation
    pub show_pause_confirm: bool,
    pub pause_hand_name: String,
    pub pause_hand_id: String,
    // Deactivate confirmation
    pub show_deactivate_confirm: bool,
    pub deactivate_hand_name: String,
    pub deactivate_hand_id: String,
    // Status
    pub status_msg: String,
}

pub enum HandsAction {
    Continue,
    Back,
    Refresh,
    ActivateHand { id: String },
    PauseHand { id: String },
    DeactivateHand { id: String },
    ResumeHand { id: String },
}

impl HandsState {
    pub fn new() -> Self {
        // Populate with fake data
        let marketplace_hands = vec![
            HandInfo {
                id: "hand-001".to_string(),
                name: "code-assist".to_string(),
                description: "AI-powered code completion and refactoring".to_string(),
                provider: "anthropic".to_string(),
                status: "inactive".to_string(),
                tasks_completed: 0,
                last_activity: None,
                capabilities: vec!["completion".to_string(), "refactor".to_string()],
            },
            HandInfo {
                id: "hand-002".to_string(),
                name: "test-gen".to_string(),
                description: "Automatic test generation and coverage analysis".to_string(),
                provider: "openai".to_string(),
                status: "inactive".to_string(),
                tasks_completed: 0,
                last_activity: None,
                capabilities: vec!["test-gen".to_string(), "coverage".to_string()],
            },
            HandInfo {
                id: "hand-003".to_string(),
                name: "doc-writer".to_string(),
                description: "Documentation generation and formatting".to_string(),
                provider: "deepseek".to_string(),
                status: "inactive".to_string(),
                tasks_completed: 0,
                last_activity: None,
                capabilities: vec!["docs".to_string(), "formatting".to_string()],
            },
            HandInfo {
                id: "hand-004".to_string(),
                name: "review-bot".to_string(),
                description: "Code review and quality assessment".to_string(),
                provider: "anthropic".to_string(),
                status: "inactive".to_string(),
                tasks_completed: 0,
                last_activity: None,
                capabilities: vec!["review".to_string(), "quality".to_string()],
            },
        ];

        let active_hands = vec![
            HandInfo {
                id: "hand-001".to_string(),
                name: "code-assist".to_string(),
                description: "AI-powered code completion and refactoring".to_string(),
                provider: "anthropic".to_string(),
                status: "active".to_string(),
                tasks_completed: 156,
                last_activity: Some("2025-04-16T08:30:00Z".to_string()),
                capabilities: vec!["completion".to_string(), "refactor".to_string()],
            },
            HandInfo {
                id: "hand-005".to_string(),
                name: "search-agent".to_string(),
                description: "Web search and research assistance".to_string(),
                provider: "google".to_string(),
                status: "paused".to_string(),
                tasks_completed: 45,
                last_activity: Some("2025-04-15T14:00:00Z".to_string()),
                capabilities: vec!["search".to_string(), "research".to_string()],
            },
        ];

        Self {
            marketplace_hands,
            active_hands,
            list_state: ListState::default().with_selected(Some(0)),
            sub_tab: HandsSubTab::Marketplace,
            sub: HandsSubScreen::List,
            loading: false,
            tick: 0,
            detail_hand: None,
            show_activate_confirm: false,
            activate_hand_name: String::new(),
            activate_hand_id: String::new(),
            show_pause_confirm: false,
            pause_hand_name: String::new(),
            pause_hand_id: String::new(),
            show_deactivate_confirm: false,
            deactivate_hand_name: String::new(),
            deactivate_hand_id: String::new(),
            status_msg: String::new(),
        }
    }

    pub fn tick(&mut self) {
        self.tick = self.tick.wrapping_add(1);
    }

    fn current_hands(&self) -> &Vec<HandInfo> {
        match self.sub_tab {
            HandsSubTab::Marketplace => &self.marketplace_hands,
            HandsSubTab::Active => &self.active_hands,
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> HandsAction {
        if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
            return HandsAction::Continue;
        }

        // Confirmation modals
        if self.show_activate_confirm {
            return self.handle_activate_confirm_key(key);
        }
        if self.show_pause_confirm {
            return self.handle_pause_confirm_key(key);
        }
        if self.show_deactivate_confirm {
            return self.handle_deactivate_confirm_key(key);
        }

        match self.sub {
            HandsSubScreen::List => self.handle_list_key(key),
            HandsSubScreen::Detail => self.handle_detail_key(key),
        }
    }

    fn handle_list_key(&mut self, key: KeyEvent) -> HandsAction {
        let hands = self.current_hands();
        let total = hands.len();

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
                self.sub_tab = HandsSubTab::Marketplace;
                self.list_state.select(Some(0));
            }
            KeyCode::Char('2') => {
                self.sub_tab = HandsSubTab::Active;
                self.list_state.select(Some(0));
            }
            KeyCode::Enter => {
                if let Some(idx) = self.list_state.selected() {
                    if let Some(hand) = self.current_hands().get(idx) {
                        self.detail_hand = Some(hand.clone());
                        self.sub = HandsSubScreen::Detail;
                    }
                }
            }
            KeyCode::Char('a') => {
                if self.sub_tab == HandsSubTab::Marketplace {
                    if let Some(idx) = self.list_state.selected() {
                        if let Some(hand) = self.marketplace_hands.get(idx) {
                            self.activate_hand_id = hand.id.clone();
                            self.activate_hand_name = hand.name.clone();
                            self.show_activate_confirm = true;
                        }
                    }
                }
            }
            KeyCode::Char('p') => {
                if self.sub_tab == HandsSubTab::Active {
                    if let Some(idx) = self.list_state.selected() {
                        if let Some(hand) = self.active_hands.get(idx) {
                            if hand.status == "active" {
                                self.pause_hand_id = hand.id.clone();
                                self.pause_hand_name = hand.name.clone();
                                self.show_pause_confirm = true;
                            }
                        }
                    }
                }
            }
            KeyCode::Char('r') => {
                if self.sub_tab == HandsSubTab::Active {
                    if let Some(idx) = self.list_state.selected() {
                        if let Some(hand) = self.active_hands.get(idx) {
                            if hand.status == "paused" {
                                return HandsAction::ResumeHand { id: hand.id.clone() };
                            }
                        }
                    }
                }
            }
            KeyCode::Char('d') => {
                if self.sub_tab == HandsSubTab::Active {
                    if let Some(idx) = self.list_state.selected() {
                        if let Some(hand) = self.active_hands.get(idx) {
                            self.deactivate_hand_id = hand.id.clone();
                            self.deactivate_hand_name = hand.name.clone();
                            self.show_deactivate_confirm = true;
                        }
                    }
                }
            }
            KeyCode::Esc => return HandsAction::Back,
            _ => {}
        }
        HandsAction::Continue
    }

    fn handle_detail_key(&mut self, key: KeyEvent) -> HandsAction {
        match key.code {
            KeyCode::Esc => {
                self.sub = HandsSubScreen::List;
                self.detail_hand = None;
            }
            KeyCode::Char('a') => {
                if let Some(hand) = &self.detail_hand {
                    if hand.status == "inactive" {
                        self.activate_hand_id = hand.id.clone();
                        self.activate_hand_name = hand.name.clone();
                        self.show_activate_confirm = true;
                    }
                }
            }
            KeyCode::Char('p') => {
                if let Some(hand) = &self.detail_hand {
                    if hand.status == "active" {
                        self.pause_hand_id = hand.id.clone();
                        self.pause_hand_name = hand.name.clone();
                        self.show_pause_confirm = true;
                    }
                }
            }
            KeyCode::Char('r') => {
                if let Some(hand) = &self.detail_hand {
                    if hand.status == "paused" {
                        return HandsAction::ResumeHand { id: hand.id.clone() };
                    }
                }
            }
            KeyCode::Char('d') => {
                if let Some(hand) = &self.detail_hand {
                    if hand.status != "inactive" {
                        self.deactivate_hand_id = hand.id.clone();
                        self.deactivate_hand_name = hand.name.clone();
                        self.show_deactivate_confirm = true;
                    }
                }
            }
            _ => {}
        }
        HandsAction::Continue
    }

    fn handle_activate_confirm_key(&mut self, key: KeyEvent) -> HandsAction {
        match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                self.show_activate_confirm = false;
                let id = self.activate_hand_id.clone();
                self.sub = HandsSubScreen::List;
                return HandsAction::ActivateHand { id };
            }
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                self.show_activate_confirm = false;
            }
            _ => {}
        }
        HandsAction::Continue
    }

    fn handle_pause_confirm_key(&mut self, key: KeyEvent) -> HandsAction {
        match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                self.show_pause_confirm = false;
                let id = self.pause_hand_id.clone();
                return HandsAction::PauseHand { id };
            }
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                self.show_pause_confirm = false;
            }
            _ => {}
        }
        HandsAction::Continue
    }

    fn handle_deactivate_confirm_key(&mut self, key: KeyEvent) -> HandsAction {
        match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                self.show_deactivate_confirm = false;
                let id = self.deactivate_hand_id.clone();
                self.sub = HandsSubScreen::List;
                return HandsAction::DeactivateHand { id };
            }
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                self.show_deactivate_confirm = false;
            }
            _ => {}
        }
        HandsAction::Continue
    }
}

// ── Drawing ─────────────────────────────────────────────────────────────────

pub fn draw(f: &mut Frame, area: Rect, state: &mut HandsState, _i18n: &Translator) {
    let title = match state.sub {
        HandsSubScreen::List => " Hands ",
        HandsSubScreen::Detail => " Hand Detail ",
    };

    let block = Block::default()
        .title(Line::from(vec![Span::styled(title, theme::title_style())]))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme::ACCENT))
        .padding(Padding::horizontal(1));

    let inner = block.inner(area);
    f.render_widget(block, area);

    match state.sub {
        HandsSubScreen::List => draw_list(f, inner, state),
        HandsSubScreen::Detail => draw_detail(f, inner, state),
    }

    // Confirmation modals
    if state.show_activate_confirm {
        draw_activate_confirm(f, area, state);
    }
    if state.show_pause_confirm {
        draw_pause_confirm(f, area, state);
    }
    if state.show_deactivate_confirm {
        draw_deactivate_confirm(f, area, state);
    }
}

fn draw_list(f: &mut Frame, area: Rect, state: &mut HandsState) {
    let chunks = Layout::vertical([
        Constraint::Length(2), // header + sub-tabs
        Constraint::Min(3),    // list
        Constraint::Length(1), // hints
    ]).split(area);

    // Sub-tabs header
    let tab_style = |tab: HandsSubTab| {
        if state.sub_tab == tab {
            Style::default().fg(theme::ACCENT).add_modifier(Modifier::BOLD)
        } else {
            theme::dim_style()
        }
    };

    let count_marketplace = state.marketplace_hands.len();
    let count_active = state.active_hands.len();

    f.render_widget(
        Paragraph::new(vec![
            Line::from(vec![
                Span::styled("  ", Style::default()),
                Span::styled(format!("[1] Marketplace ({})", count_marketplace), tab_style(HandsSubTab::Marketplace)),
                Span::styled("  ", Style::default()),
                Span::styled(format!("[2] Active ({})", count_active), tab_style(HandsSubTab::Active)),
            ]),
            Line::from(vec![Span::styled(
                format!("  {:<16} {:<12} {:<8} {:<30} {}",
                    "Name", "Provider", "Status", "Description", "Tasks"),
                theme::table_header(),
            )]),
        ]),
        chunks[0],
    );

    // Hand list
    let hands = state.current_hands();

    if state.loading && hands.is_empty() {
        let spinner = theme::SPINNER_FRAMES[state.tick % theme::SPINNER_FRAMES.len()];
        f.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled(format!("  {spinner} "), Style::default().fg(theme::CYAN)),
                Span::styled("Loading hands...", theme::dim_style()),
            ])),
            chunks[1],
        );
    } else if hands.is_empty() {
        let empty_msg = match state.sub_tab {
            HandsSubTab::Marketplace => "No hands in marketplace. Check connection.",
            HandsSubTab::Active => "No active hands. Activate one from Marketplace.",
        };
        f.render_widget(
            Paragraph::new(Span::styled(format!("  {}", empty_msg), theme::dim_style())),
            chunks[1],
        );
    } else {
        let items: Vec<ListItem> = hands
            .iter()
            .map(|hand| {
                let (status_badge, status_style) = match hand.status.as_str() {
                    "active" => ("Active", Style::default().fg(theme::GREEN)),
                    "paused" => ("Paused", Style::default().fg(theme::YELLOW)),
                    "inactive" => ("Inactive", theme::dim_style()),
                    _ => (hand.status.as_str(), theme::dim_style()),
                };
                let tasks = if hand.tasks_completed > 0 {
                    hand.tasks_completed.to_string()
                } else {
                    "N/A".to_string()
                };
                ListItem::new(Line::from(vec![
                    Span::styled(format!("  {:<16}", truncate(&hand.name, 15)), Style::default().fg(theme::CYAN)),
                    Span::styled(format!(" {:<12}", hand.provider), Style::default().fg(theme::PURPLE)),
                    Span::styled(format!(" {:<8}", status_badge), status_style),
                    Span::styled(format!(" {:<30}", truncate(&hand.description, 29)), Style::default().fg(theme::TEXT)),
                    Span::styled(format!(" {}", tasks), Style::default().fg(theme::ACCENT)),
                ]))
            })
            .collect();

        let list = List::new(items)
            .highlight_style(theme::selected_style())
            .highlight_symbol("> ");
        f.render_stateful_widget(list, chunks[1], &mut state.list_state);
    }

    // Hints
    let hints = match state.sub_tab {
        HandsSubTab::Marketplace => "[a]ctivate  [Enter]detail  [1-2]tab  [Esc]back",
        HandsSubTab::Active => "[p]ause  [r]esume  [d]eactivate  [Enter]detail  [1-2]tab  [Esc]back",
    };
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(format!("  {}", hints), theme::hint_style()))),
        chunks[2],
    );
}

fn draw_detail(f: &mut Frame, area: Rect, state: &HandsState) {
    if let Some(hand) = &state.detail_hand {
        let chunks = Layout::vertical([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(1),
            Constraint::Length(1),
        ]).split(area);

        let (status_badge, status_style) = match hand.status.as_str() {
            "active" => ("Active", Style::default().fg(theme::GREEN)),
            "paused" => ("Paused", Style::default().fg(theme::YELLOW)),
            "inactive" => ("Inactive", theme::dim_style()),
            _ => (hand.status.as_str(), theme::dim_style()),
        };

        f.render_widget(
            Paragraph::new(Span::styled(
                format!("{} ({})", hand.name, hand.id),
                Style::default().fg(theme::ACCENT).add_modifier(Modifier::BOLD),
            )),
            chunks[0],
        );

        f.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled("  Status: ", theme::dim_style()),
                Span::styled(status_badge, status_style),
                Span::styled(format!("  |  Provider: {}", hand.provider), theme::dim_style()),
            ])),
            chunks[1],
        );

        f.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled("  Tasks Completed: ", theme::dim_style()),
                Span::styled(hand.tasks_completed.to_string(), Style::default().fg(theme::ACCENT)),
            ])),
            chunks[2],
        );

        f.render_widget(
            Paragraph::new(Span::styled("  Description:", theme::dim_style())),
            chunks[3],
        );

        f.render_widget(
            Paragraph::new(Span::styled(format!("    {}", hand.description), Style::default().fg(theme::TEXT))),
            chunks[4],
        );

        f.render_widget(
            Paragraph::new(Span::styled("  Capabilities:", theme::dim_style())),
            chunks[5],
        );

        let caps_text = hand.capabilities.join(", ");
        f.render_widget(
            Paragraph::new(Span::styled(format!("    {}", caps_text), Style::default().fg(theme::CYAN))),
            chunks[6],
        );

        // Action hints
        let actions = match hand.status.as_str() {
            "inactive" => "[a]ctivate  [Esc]back",
            "active" => "[p]ause  [d]eactivate  [Esc]back",
            "paused" => "[r]esume  [d]eactivate  [Esc]back",
            _ => "[Esc]back",
        };
        f.render_widget(
            Paragraph::new(Span::styled(format!("  {}", actions), theme::hint_style())),
            chunks[7],
        );
    }
}

fn draw_activate_confirm(f: &mut Frame, area: Rect, state: &HandsState) {
    let modal = centered_rect(50, 6, area);
    f.render_widget(Clear, modal);

    let block = Block::default()
        .title(Span::styled(" Activate Hand ", theme::title_style()))
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
        Paragraph::new(Span::styled(format!("Activate '{}'?", state.activate_hand_name), Style::default().fg(theme::TEXT))),
        chunks[0],
    );
    f.render_widget(
        Paragraph::new(Span::styled("This hand will start processing tasks.", Style::default().fg(theme::CYAN))),
        chunks[1],
    );
    f.render_widget(
        Paragraph::new(Span::styled("[y] Yes  [n/Esc] No", theme::hint_style())),
        chunks[2],
    );
}

fn draw_pause_confirm(f: &mut Frame, area: Rect, state: &HandsState) {
    let modal = centered_rect(50, 6, area);
    f.render_widget(Clear, modal);

    let block = Block::default()
        .title(Span::styled(" Pause Hand ", theme::title_style()))
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

    f.render_widget(
        Paragraph::new(Span::styled(format!("Pause '{}'?", state.pause_hand_name), Style::default().fg(theme::TEXT))),
        chunks[0],
    );
    f.render_widget(
        Paragraph::new(Span::styled("The hand will stop processing but remain active.", Style::default().fg(theme::CYAN))),
        chunks[1],
    );
    f.render_widget(
        Paragraph::new(Span::styled("[y] Yes  [n/Esc] No", theme::hint_style())),
        chunks[2],
    );
}

fn draw_deactivate_confirm(f: &mut Frame, area: Rect, state: &HandsState) {
    let modal = centered_rect(50, 6, area);
    f.render_widget(Clear, modal);

    let block = Block::default()
        .title(Span::styled(" Deactivate Hand ", theme::title_style()))
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
        Paragraph::new(Span::styled(format!("Deactivate '{}'?", state.deactivate_hand_name), Style::default().fg(theme::TEXT))),
        chunks[0],
    );
    f.render_widget(
        Paragraph::new(Span::styled("The hand will return to Marketplace.", Style::default().fg(theme::YELLOW))),
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