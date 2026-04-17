//! Triggers screen: Trigger management with list and creation wizard.
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
pub struct TriggerInfo {
    pub id: String,
    pub name: String,
    pub pattern_type: String,
    pub pattern: String,
    pub action_type: String,
    pub action_target: String,
    pub enabled: bool,
    pub last_triggered: Option<String>,
    pub trigger_count: u64,
}

// Pattern types from AgentDiVA
const PATTERN_TYPES: &[&str] = &[
    "cron_schedule",
    "file_watch",
    "webhook",
    "message_pattern",
    "agent_event",
    "system_event",
];

const ACTION_TYPES: &[&str] = &[
    "run_workflow",
    "spawn_agent",
    "send_message",
    "log_event",
];

// ── State ───────────────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum TriggerSubScreen {
    List,
    Create,
    Edit,
}

pub struct TriggersState {
    pub triggers: Vec<TriggerInfo>,
    pub list_state: ListState,
    pub sub: TriggerSubScreen,
    pub loading: bool,
    pub tick: usize,
    // Create wizard (6 steps)
    pub create_step: usize,
    pub create_name: String,
    pub create_pattern_type: usize,
    pub create_pattern: String,
    pub create_action_type: usize,
    pub create_action_target: String,
    pub create_enabled: bool,
    // Edit
    pub edit_trigger_id: String,
    pub edit_field: usize,
    pub edit_value: String,
    // Delete confirmation
    pub show_delete_confirm: bool,
    pub delete_trigger_id: String,
    pub delete_trigger_name: String,
    // Status
    pub status_msg: String,
}

pub enum TriggerAction {
    Continue,
    Back,
    Refresh,
    CreateTrigger { name: String, pattern_type: String, pattern: String, action_type: String, action_target: String, enabled: bool },
    UpdateTrigger { id: String },
    DeleteTrigger { id: String },
    ToggleTrigger { id: String, enabled: bool },
}

impl TriggersState {
    pub fn new() -> Self {
        // Populate with fake data
        let triggers = vec![
            TriggerInfo {
                id: "tr-001".to_string(),
                name: "daily-review".to_string(),
                pattern_type: "cron_schedule".to_string(),
                pattern: "0 9 * * *".to_string(),
                action_type: "run_workflow".to_string(),
                action_target: "wf-001".to_string(),
                enabled: true,
                last_triggered: Some("2025-04-15T09:00:00Z".to_string()),
                trigger_count: 45,
            },
            TriggerInfo {
                id: "tr-002".to_string(),
                name: "pr-webhook".to_string(),
                pattern_type: "webhook".to_string(),
                pattern: "/hooks/pr".to_string(),
                action_type: "run_workflow".to_string(),
                action_target: "wf-001".to_string(),
                enabled: true,
                last_triggered: Some("2025-04-14T16:30:00Z".to_string()),
                trigger_count: 12,
            },
            TriggerInfo {
                id: "tr-003".to_string(),
                name: "error-monitor".to_string(),
                pattern_type: "agent_event".to_string(),
                pattern: "error.*".to_string(),
                action_type: "spawn_agent".to_string(),
                action_target: "error-handler".to_string(),
                enabled: false,
                last_triggered: None,
                trigger_count: 0,
            },
            TriggerInfo {
                id: "tr-004".to_string(),
                name: "file-watcher".to_string(),
                pattern_type: "file_watch".to_string(),
                pattern: "src/**/*.rs".to_string(),
                action_type: "run_workflow".to_string(),
                action_target: "wf-003".to_string(),
                enabled: true,
                last_triggered: Some("2025-04-13T11:15:00Z".to_string()),
                trigger_count: 78,
            },
        ];

        Self {
            triggers,
            list_state: ListState::default().with_selected(Some(0)),
            sub: TriggerSubScreen::List,
            loading: false,
            tick: 0,
            create_step: 0,
            create_name: String::new(),
            create_pattern_type: 0,
            create_pattern: String::new(),
            create_action_type: 0,
            create_action_target: String::new(),
            create_enabled: true,
            edit_trigger_id: String::new(),
            edit_field: 0,
            edit_value: String::new(),
            show_delete_confirm: false,
            delete_trigger_id: String::new(),
            delete_trigger_name: String::new(),
            status_msg: String::new(),
        }
    }

    pub fn tick(&mut self) {
        self.tick = self.tick.wrapping_add(1);
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> TriggerAction {
        if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
            return TriggerAction::Continue;
        }

        // Delete confirmation modal
        if self.show_delete_confirm {
            return self.handle_delete_confirm_key(key);
        }

        match self.sub {
            TriggerSubScreen::List => self.handle_list_key(key),
            TriggerSubScreen::Create => self.handle_create_key(key),
            TriggerSubScreen::Edit => self.handle_edit_key(key),
        }
    }

    fn handle_list_key(&mut self, key: KeyEvent) -> TriggerAction {
        let total = self.triggers.len();
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
            KeyCode::Char('n') => {
                self.sub = TriggerSubScreen::Create;
                self.create_step = 0;
                self.create_name.clear();
                self.create_pattern_type = 0;
                self.create_pattern.clear();
                self.create_action_type = 0;
                self.create_action_target.clear();
                self.create_enabled = true;
            }
            KeyCode::Char('e') => {
                if let Some(idx) = self.list_state.selected() {
                    if let Some(tr) = self.triggers.get(idx) {
                        self.edit_trigger_id = tr.id.clone();
                        self.edit_field = 0;
                        self.edit_value = tr.pattern.clone();
                        self.sub = TriggerSubScreen::Edit;
                    }
                }
            }
            KeyCode::Char('t') => {
                if let Some(idx) = self.list_state.selected() {
                    if let Some(tr) = self.triggers.get(idx) {
                        return TriggerAction::ToggleTrigger {
                            id: tr.id.clone(),
                            enabled: !tr.enabled,
                        };
                    }
                }
            }
            KeyCode::Char('d') => {
                if let Some(idx) = self.list_state.selected() {
                    if let Some(tr) = self.triggers.get(idx) {
                        self.delete_trigger_id = tr.id.clone();
                        self.delete_trigger_name = tr.name.clone();
                        self.show_delete_confirm = true;
                    }
                }
            }
            KeyCode::Esc => return TriggerAction::Back,
            _ => {}
        }
        TriggerAction::Continue
    }

    fn handle_create_key(&mut self, key: KeyEvent) -> TriggerAction {
        match key.code {
            KeyCode::Esc => {
                self.sub = TriggerSubScreen::List;
            }
            KeyCode::Up | KeyCode::Char('k') => {
                // Navigate within current step options
                match self.create_step {
                    1 => { // Pattern type picker
                        self.create_pattern_type = if self.create_pattern_type == 0 {
                            PATTERN_TYPES.len() - 1
                        } else {
                            self.create_pattern_type - 1
                        };
                    }
                    3 => { // Action type picker
                        self.create_action_type = if self.create_action_type == 0 {
                            ACTION_TYPES.len() - 1
                        } else {
                            self.create_action_type - 1
                        };
                    }
                    _ => {}
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                match self.create_step {
                    1 => {
                        self.create_pattern_type = (self.create_pattern_type + 1) % PATTERN_TYPES.len();
                    }
                    3 => {
                        self.create_action_type = (self.create_action_type + 1) % ACTION_TYPES.len();
                    }
                    _ => {}
                }
            }
            KeyCode::Tab => {
                self.create_step = (self.create_step + 1) % 6;
            }
            KeyCode::BackTab => {
                self.create_step = if self.create_step == 0 { 5 } else { self.create_step - 1 };
            }
            KeyCode::Enter => {
                // On final step, create trigger
                if self.create_step == 5 {
                    if !self.create_name.is_empty() && !self.create_pattern.is_empty() {
                        self.sub = TriggerSubScreen::List;
                        return TriggerAction::CreateTrigger {
                            name: self.create_name.clone(),
                            pattern_type: PATTERN_TYPES[self.create_pattern_type].to_string(),
                            pattern: self.create_pattern.clone(),
                            action_type: ACTION_TYPES[self.create_action_type].to_string(),
                            action_target: self.create_action_target.clone(),
                            enabled: self.create_enabled,
                        };
                    }
                } else {
                    // Move to next step
                    self.create_step += 1;
                }
            }
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                if self.create_step == 4 {
                    self.create_enabled = true;
                }
            }
            KeyCode::Char('n') | KeyCode::Char('N') => {
                if self.create_step == 4 {
                    self.create_enabled = false;
                }
            }
            KeyCode::Char(c) => match self.create_step {
                0 => self.create_name.push(c),
                2 => self.create_pattern.push(c),
                4 => self.create_action_target.push(c),
                _ => {}
            },
            KeyCode::Backspace => match self.create_step {
                0 => { self.create_name.pop(); }
                2 => { self.create_pattern.pop(); }
                4 => { self.create_action_target.pop(); }
                _ => {}
            },
            _ => {}
        }
        TriggerAction::Continue
    }

    fn handle_edit_key(&mut self, key: KeyEvent) -> TriggerAction {
        match key.code {
            KeyCode::Esc => {
                self.sub = TriggerSubScreen::List;
            }
            KeyCode::Tab => {
                self.edit_field = (self.edit_field + 1) % 3;
            }
            KeyCode::Enter => {
                self.sub = TriggerSubScreen::List;
                return TriggerAction::UpdateTrigger {
                    id: self.edit_trigger_id.clone(),
                };
            }
            KeyCode::Char(c) => {
                self.edit_value.push(c);
            }
            KeyCode::Backspace => {
                self.edit_value.pop();
            }
            _ => {}
        }
        TriggerAction::Continue
    }

    fn handle_delete_confirm_key(&mut self, key: KeyEvent) -> TriggerAction {
        match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                self.show_delete_confirm = false;
                let id = self.delete_trigger_id.clone();
                self.sub = TriggerSubScreen::List;
                return TriggerAction::DeleteTrigger { id };
            }
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                self.show_delete_confirm = false;
            }
            _ => {}
        }
        TriggerAction::Continue
    }
}

// ── Drawing ─────────────────────────────────────────────────────────────────

pub fn draw(f: &mut Frame, area: Rect, state: &mut TriggersState, _i18n: &Translator) {
    let title = match state.sub {
        TriggerSubScreen::List => " Triggers ",
        TriggerSubScreen::Create => " New Trigger ",
        TriggerSubScreen::Edit => " Edit Trigger ",
    };

    let block = Block::default()
        .title(Line::from(vec![Span::styled(title, theme::title_style())]))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme::ACCENT))
        .padding(Padding::horizontal(1));

    let inner = block.inner(area);
    f.render_widget(block, area);

    match state.sub {
        TriggerSubScreen::List => draw_list(f, inner, state),
        TriggerSubScreen::Create => draw_create(f, inner, state),
        TriggerSubScreen::Edit => draw_edit(f, inner, state),
    }

    // Delete confirmation modal overlay
    if state.show_delete_confirm {
        draw_delete_confirm(f, area, state);
    }
}

fn draw_list(f: &mut Frame, area: Rect, state: &mut TriggersState) {
    let chunks = Layout::vertical([
        Constraint::Length(2), // header
        Constraint::Min(3),    // list
        Constraint::Length(1), // hints
    ]).split(area);

    // Header
    f.render_widget(
        Paragraph::new(vec![
            Line::from(vec![Span::styled(
                format!("  Triggers ({})", state.triggers.len()),
                Style::default().fg(theme::CYAN).add_modifier(Modifier::BOLD),
            )]),
            Line::from(vec![Span::styled(
                format!("  {:<16} {:<14} {:<20} {:<14} {:<8} {}",
                    "Name", "Pattern Type", "Pattern", "Action", "Enabled", "Count"),
                theme::table_header(),
            )]),
        ]),
        chunks[0],
    );

    // Trigger list
    if state.loading && state.triggers.is_empty() {
        let spinner = theme::SPINNER_FRAMES[state.tick % theme::SPINNER_FRAMES.len()];
        f.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled(format!("  {spinner} "), Style::default().fg(theme::CYAN)),
                Span::styled("Loading triggers...", theme::dim_style()),
            ])),
            chunks[1],
        );
    } else if state.triggers.is_empty() {
        f.render_widget(
            Paragraph::new(Span::styled(
                "  No triggers. Press [n] to create one.",
                theme::dim_style(),
            )),
            chunks[1],
        );
    } else {
        let items: Vec<ListItem> = state
            .triggers
            .iter()
            .map(|tr| {
                let (enabled_badge, enabled_style) = if tr.enabled {
                    ("ON", Style::default().fg(theme::GREEN))
                } else {
                    ("OFF", Style::default().fg(theme::RED))
                };
                let last = tr.last_triggered.as_ref().map(|t| short_time(t)).unwrap_or_else(|| "Never".to_string());
                ListItem::new(Line::from(vec![
                    Span::styled(format!("  {:<16}", truncate(&tr.name, 15)), Style::default().fg(theme::CYAN)),
                    Span::styled(format!(" {:<14}", tr.pattern_type), theme::dim_style()),
                    Span::styled(format!(" {:<20}", truncate(&tr.pattern, 19)), Style::default().fg(theme::TEXT)),
                    Span::styled(format!(" {:<14}", tr.action_type), Style::default().fg(theme::PURPLE)),
                    Span::styled(format!(" {:<8}", enabled_badge), enabled_style),
                    Span::styled(format!(" {}", tr.trigger_count), Style::default().fg(theme::ACCENT)),
                ]))
            })
            .collect();

        let list = List::new(items)
            .highlight_style(theme::selected_style())
            .highlight_symbol("> ");
        f.render_stateful_widget(list, chunks[1], &mut state.list_state);
    }

    // Hints
    let hints = if !state.status_msg.is_empty() {
        format!("  {} | [n]ew  [e]dit  [t]oggle  [d]elete  [Esc] back", state.status_msg)
    } else {
        "  [n]ew  [e]dit  [t]oggle  [d]elete  [Esc] back".to_string()
    };
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(hints, theme::hint_style()))),
        chunks[2],
    );
}

fn draw_create(f: &mut Frame, area: Rect, state: &TriggersState) {
    let chunks = Layout::vertical([
        Constraint::Length(1), // progress
        Constraint::Length(1), // step title
        Constraint::Min(3),    // content
        Constraint::Length(1), // hints
    ]).split(area);

    // Progress indicator
    let progress_steps: Vec<Span> = (0..6).map(|i| {
        let label = match i {
            0 => "Name",
            1 => "Pattern Type",
            2 => "Pattern",
            3 => "Action Type",
            4 => "Action Target",
            5 => "Enabled",
            _ => "",
        };
        if i == state.create_step {
            Span::styled(format!("[{}]", label), Style::default().fg(theme::ACCENT).add_modifier(Modifier::BOLD))
        } else if i < state.create_step {
            Span::styled(format!("[{}]", label), Style::default().fg(theme::GREEN))
        } else {
            Span::styled(format!("[{}]", label), theme::dim_style())
        }
    }).collect();
    f.render_widget(Paragraph::new(Line::from(progress_steps)), chunks[0]);

    // Current step content
    match state.create_step {
        0 => {
            f.render_widget(
                Paragraph::new(vec![
                    Line::from(Span::styled("Trigger Name:", Style::default().fg(theme::CYAN).add_modifier(Modifier::BOLD))),
                    Line::from(Span::styled(format!("  {}|", state.create_name), Style::default().fg(theme::TEXT))),
                ]),
                chunks[1],
            );
        }
        1 => {
            f.render_widget(
                Paragraph::new(Span::styled("Select Pattern Type:", Style::default().fg(theme::CYAN).add_modifier(Modifier::BOLD))),
                chunks[1],
            );
            let type_items: Vec<ListItem> = PATTERN_TYPES.iter().enumerate().map(|(i, t)| {
                let style = if i == state.create_pattern_type {
                    Style::default().fg(theme::ACCENT).add_modifier(Modifier::BOLD)
                } else {
                    theme::dim_style()
                };
                ListItem::new(Line::from(Span::styled(format!("  {}", t), style)))
            }).collect();
            f.render_widget(List::new(type_items), chunks[2]);
        }
        2 => {
            f.render_widget(
                Paragraph::new(vec![
                    Line::from(Span::styled(format!("Pattern for {}:", PATTERN_TYPES[state.create_pattern_type]), Style::default().fg(theme::CYAN).add_modifier(Modifier::BOLD))),
                    Line::from(Span::styled(format!("  {}|", state.create_pattern), Style::default().fg(theme::TEXT))),
                ]),
                chunks[1],
            );
        }
        3 => {
            f.render_widget(
                Paragraph::new(Span::styled("Select Action Type:", Style::default().fg(theme::CYAN).add_modifier(Modifier::BOLD))),
                chunks[1],
            );
            let action_items: Vec<ListItem> = ACTION_TYPES.iter().enumerate().map(|(i, t)| {
                let style = if i == state.create_action_type {
                    Style::default().fg(theme::ACCENT).add_modifier(Modifier::BOLD)
                } else {
                    theme::dim_style()
                };
                ListItem::new(Line::from(Span::styled(format!("  {}", t), style)))
            }).collect();
            f.render_widget(List::new(action_items), chunks[2]);
        }
        4 => {
            f.render_widget(
                Paragraph::new(vec![
                    Line::from(Span::styled(format!("Target for {}:", ACTION_TYPES[state.create_action_type]), Style::default().fg(theme::CYAN).add_modifier(Modifier::BOLD))),
                    Line::from(Span::styled(format!("  {}|", state.create_action_target), Style::default().fg(theme::TEXT))),
                    Line::from(Span::styled(format!("Enabled: {}", if state.create_enabled { "[y] Yes" } else { "[n] No" }), Style::default().fg(theme::YELLOW))),
                ]),
                chunks[1],
            );
        }
        5 => {
            f.render_widget(
                Paragraph::new(vec![
                    Line::from(Span::styled("Review & Create:", Style::default().fg(theme::CYAN).add_modifier(Modifier::BOLD))),
                    Line::from(vec![
                        Span::styled("  Name: ", theme::dim_style()),
                        Span::styled(&state.create_name, Style::default().fg(theme::TEXT)),
                    ]),
                    Line::from(vec![
                        Span::styled("  Pattern: ", theme::dim_style()),
                        Span::styled(format!("{}:{}", PATTERN_TYPES[state.create_pattern_type], state.create_pattern), Style::default().fg(theme::TEXT)),
                    ]),
                    Line::from(vec![
                        Span::styled("  Action: ", theme::dim_style()),
                        Span::styled(format!("{} -> {}", ACTION_TYPES[state.create_action_type], state.create_action_target), Style::default().fg(theme::TEXT)),
                    ]),
                    Line::from(vec![
                        Span::styled("  Enabled: ", theme::dim_style()),
                        Span::styled(if state.create_enabled { "Yes" } else { "No" }, Style::default().fg(if state.create_enabled { theme::GREEN } else { theme::RED })),
                    ]),
                ]),
                chunks[1],
            );
        }
        _ => {}
    }

    // Hints
    let hints = if state.create_step == 1 || state.create_step == 3 {
        "[j/k] select  [Enter] next  [Tab] skip  [Esc] cancel"
    } else if state.create_step == 5 {
        "[Enter] create  [Tab] back  [Esc] cancel"
    } else {
        "[Enter] next  [Tab] skip  [Esc] cancel"
    };
    f.render_widget(
        Paragraph::new(Span::styled(format!("  {}", hints), theme::hint_style())),
        chunks[3],
    );
}

fn draw_edit(f: &mut Frame, area: Rect, state: &TriggersState) {
    let chunks = Layout::vertical([
        Constraint::Length(1),
        Constraint::Min(3),
        Constraint::Length(1),
    ]).split(area);

    f.render_widget(
        Paragraph::new(Span::styled(
            format!("Edit Trigger: {}", state.edit_trigger_id),
            Style::default().fg(theme::ACCENT).add_modifier(Modifier::BOLD),
        )),
        chunks[0],
    );

    // Edit fields
    let fields = ["Pattern", "Action Target", "Enabled"];
    let field_values = [&state.edit_value, &state.edit_value, &state.edit_value];
    let items: Vec<Line> = fields.iter().enumerate().map(|(i, label)| {
        let style = if state.edit_field == i {
            Style::default().fg(theme::CYAN).add_modifier(Modifier::BOLD)
        } else {
            theme::dim_style()
        };
        Line::from(vec![
            Span::styled(format!("  {}: ", label), style),
            Span::styled(field_values[i].clone(), Style::default().fg(theme::TEXT)),
        ])
    }).collect();

    f.render_widget(Paragraph::new(items), chunks[1]);

    f.render_widget(
        Paragraph::new(Span::styled("[Tab] field  [Enter] save  [Esc] cancel", theme::hint_style())),
        chunks[2],
    );
}

fn draw_delete_confirm(f: &mut Frame, area: Rect, state: &TriggersState) {
    let modal = centered_rect(50, 6, area);
    f.render_widget(Clear, modal);

    let block = Block::default()
        .title(Span::styled(" Confirm Delete ", theme::title_style()))
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
        Paragraph::new(Span::styled(
            format!("Delete trigger '{}'?", state.delete_trigger_name),
            Style::default().fg(theme::TEXT),
        )),
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

fn short_time(ts: &str) -> String {
    if let Some(t_pos) = ts.find('T') {
        let time_part = &ts[t_pos + 1..];
        if time_part.len() >= 8 {
            return time_part[..8].to_string();
        }
    }
    ts.chars().take(8).collect()
}

fn centered_rect(percent_x: u16, height: u16, area: Rect) -> Rect {
    let w = area.width * percent_x / 100;
    let x = area.x + (area.width.saturating_sub(w)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;
    Rect::new(x, y, w, height.min(area.height))
}