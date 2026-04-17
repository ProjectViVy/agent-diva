//! Memory screen: per-agent KV store browser and editor.
//!
//! Cloned from AgentDiVA TUI screens/memory.rs with placeholder data.

use crate::i18n::Translator;
use crate::theme;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Padding, Paragraph};
use ratatui::Frame;

// ── Data types ──────────────────────────────────────────────────────────────

#[derive(Clone, Default)]
pub struct KvPair {
    pub key: String,
    pub value: String,
}

#[derive(Clone)]
pub struct AgentEntry {
    pub id: String,
    pub name: String,
}

// ── State ───────────────────────────────────────────────────────────────────

#[derive(Clone, PartialEq, Eq)]
pub enum MemorySub {
    AgentSelect,
    KvBrowser,
    EditKey,
    AddKey,
}

#[derive(Clone, PartialEq, Eq)]
pub enum EditField {
    Key,
    Value,
}

pub struct MemoryState {
    pub sub: MemorySub,
    pub agents: Vec<AgentEntry>,
    pub selected_agent: Option<AgentEntry>,
    pub kv_pairs: Vec<KvPair>,
    pub agent_list_state: ListState,
    pub kv_list_state: ListState,
    pub key_buf: String,
    pub value_buf: String,
    pub edit_field: EditField,
    pub loading: bool,
    pub tick: usize,
    pub confirm_delete: bool,
    pub status_msg: String,
}

pub enum MemoryAction {
    Continue,
    Back,
    LoadAgents,
    LoadKv(String),
    SaveKv {
        agent_id: String,
        key: String,
        value: String,
    },
    DeleteKv {
        agent_id: String,
        key: String,
    },
}

impl MemoryState {
    pub fn new() -> Self {
        Self {
            sub: MemorySub::AgentSelect,
            agents: Self::placeholder_agents(),
            selected_agent: None,
            kv_pairs: Vec::new(),
            agent_list_state: ListState::default(),
            kv_list_state: ListState::default(),
            key_buf: String::new(),
            value_buf: String::new(),
            edit_field: EditField::Key,
            loading: false,
            tick: 0,
            confirm_delete: false,
            status_msg: String::new(),
        }
    }

    fn placeholder_agents() -> Vec<AgentEntry> {
        vec![
            AgentEntry { id: "agent-001".to_string(), name: "Coder".to_string() },
            AgentEntry { id: "agent-002".to_string(), name: "Researcher".to_string() },
            AgentEntry { id: "agent-003".to_string(), name: "Writer".to_string() },
        ]
    }

    fn placeholder_kv() -> Vec<KvPair> {
        vec![
            KvPair { key: "project_context".to_string(), value: "Agent Diva TUI project".to_string() },
            KvPair { key: "last_task".to_string(), value: "Implement memory screen".to_string() },
            KvPair { key: "preferences".to_string(), value: "Rust, ratatui, dark mode".to_string() },
        ]
    }

    pub fn tick(&mut self) {
        self.tick = self.tick.wrapping_add(1);
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> MemoryAction {
        if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
            return MemoryAction::Continue;
        }
        match self.sub {
            MemorySub::AgentSelect => self.handle_agent_select(key),
            MemorySub::KvBrowser => self.handle_kv_browser(key),
            MemorySub::EditKey | MemorySub::AddKey => self.handle_edit(key),
        }
    }

    fn handle_agent_select(&mut self, key: KeyEvent) -> MemoryAction {
        let total = self.agents.len();
        if self.agent_list_state.selected().is_none() && total > 0 {
            self.agent_list_state.select(Some(0));
        }
        match key.code {
            KeyCode::Esc => return MemoryAction::Back,
            KeyCode::Up | KeyCode::Char('k') => {
                if total > 0 {
                    let i = self.agent_list_state.selected().unwrap_or(0);
                    let next = if i == 0 { total - 1 } else { i - 1 };
                    self.agent_list_state.select(Some(next));
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if total > 0 {
                    let i = self.agent_list_state.selected().unwrap_or(0);
                    let next = (i + 1) % total;
                    self.agent_list_state.select(Some(next));
                }
            }
            KeyCode::Enter => {
                if let Some(sel) = self.agent_list_state.selected() {
                    if sel < self.agents.len() {
                        let agent = self.agents[sel].clone();
                        self.selected_agent = Some(agent);
                        self.kv_pairs = Self::placeholder_kv();
                        self.kv_list_state.select(Some(0));
                        self.sub = MemorySub::KvBrowser;
                        return MemoryAction::Continue;
                    }
                }
            }
            KeyCode::Char('r') => return MemoryAction::LoadAgents,
            _ => {}
        }
        MemoryAction::Continue
    }

    fn handle_kv_browser(&mut self, key: KeyEvent) -> MemoryAction {
        if self.confirm_delete {
            match key.code {
                KeyCode::Char('y') | KeyCode::Char('Y') => {
                    self.confirm_delete = false;
                    if let (Some(agent), Some(sel)) =
                        (&self.selected_agent, self.kv_list_state.selected())
                    {
                        if sel < self.kv_pairs.len() {
                            let key = self.kv_pairs[sel].key.clone();
                            self.kv_pairs.remove(sel);
                            if self.kv_pairs.is_empty() {
                                self.kv_list_state.select(None);
                            } else {
                                let new_sel = sel.min(self.kv_pairs.len() - 1);
                                self.kv_list_state.select(Some(new_sel));
                            }
                            self.status_msg = format!("Deleted key: {}", key);
                            return MemoryAction::DeleteKv {
                                agent_id: agent.id.clone(),
                                key,
                            };
                        }
                    }
                }
                _ => self.confirm_delete = false,
            }
            return MemoryAction::Continue;
        }

        let total = self.kv_pairs.len();
        match key.code {
            KeyCode::Esc => {
                self.sub = MemorySub::AgentSelect;
                self.kv_pairs.clear();
                self.selected_agent = None;
            }
            KeyCode::Up | KeyCode::Char('k') => {
                if total > 0 {
                    let i = self.kv_list_state.selected().unwrap_or(0);
                    let next = if i == 0 { total - 1 } else { i - 1 };
                    self.kv_list_state.select(Some(next));
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if total > 0 {
                    let i = self.kv_list_state.selected().unwrap_or(0);
                    let next = (i + 1) % total;
                    self.kv_list_state.select(Some(next));
                }
            }
            KeyCode::Char('a') => {
                self.sub = MemorySub::AddKey;
                self.key_buf.clear();
                self.value_buf.clear();
                self.edit_field = EditField::Key;
            }
            KeyCode::Char('e') => {
                if let Some(sel) = self.kv_list_state.selected() {
                    if sel < self.kv_pairs.len() {
                        self.key_buf = self.kv_pairs[sel].key.clone();
                        self.value_buf = self.kv_pairs[sel].value.clone();
                        self.edit_field = EditField::Value;
                        self.sub = MemorySub::EditKey;
                    }
                }
            }
            KeyCode::Char('d') => {
                if self.kv_list_state.selected().is_some() {
                    self.confirm_delete = true;
                }
            }
            _ => {}
        }
        MemoryAction::Continue
    }

    fn handle_edit(&mut self, key: KeyEvent) -> MemoryAction {
        match key.code {
            KeyCode::Esc => {
                self.sub = MemorySub::KvBrowser;
            }
            KeyCode::Tab => {
                self.edit_field = match self.edit_field {
                    EditField::Key => EditField::Value,
                    EditField::Value => EditField::Key,
                };
            }
            KeyCode::Enter => {
                if !self.key_buf.is_empty() {
                    if let Some(agent) = &self.selected_agent {
                        // Update or add KV pair
                        if self.sub == MemorySub::AddKey {
                            self.kv_pairs.push(KvPair {
                                key: self.key_buf.clone(),
                                value: self.value_buf.clone(),
                            });
                        } else if let Some(sel) = self.kv_list_state.selected() {
                            if sel < self.kv_pairs.len() {
                                self.kv_pairs[sel].value = self.value_buf.clone();
                            }
                        }
                        self.kv_list_state.select(Some(self.kv_pairs.len() - 1));
                        self.status_msg = format!("Saved: {}", self.key_buf);
                        self.sub = MemorySub::KvBrowser;
                        return MemoryAction::SaveKv {
                            agent_id: agent.id.clone(),
                            key: self.key_buf.clone(),
                            value: self.value_buf.clone(),
                        };
                    }
                }
                self.sub = MemorySub::KvBrowser;
            }
            KeyCode::Backspace => match self.edit_field {
                EditField::Key if self.sub == MemorySub::AddKey => {
                    self.key_buf.pop();
                }
                EditField::Value => {
                    self.value_buf.pop();
                }
                _ => {}
            },
            KeyCode::Char(c) => match self.edit_field {
                EditField::Key if self.sub == MemorySub::AddKey => self.key_buf.push(c),
                EditField::Value => self.value_buf.push(c),
                _ => {}
            },
            _ => {}
        }
        MemoryAction::Continue
    }
}

// ── Drawing ─────────────────────────────────────────────────────────────────

pub fn draw(f: &mut Frame, area: Rect, state: &mut MemoryState, _i18n: &Translator) {
    let block = Block::default()
        .title(Line::from(vec![Span::styled(
            " Memory ",
            theme::title_style(),
        )]))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme::ACCENT))
        .padding(Padding::horizontal(1));

    let inner = block.inner(area);
    f.render_widget(block, area);

    match state.sub {
        MemorySub::AgentSelect => draw_agent_select(f, inner, state),
        MemorySub::KvBrowser => draw_kv_browser(f, inner, state),
        MemorySub::EditKey | MemorySub::AddKey => draw_edit(f, inner, state),
    }
}

fn draw_agent_select(f: &mut Frame, area: Rect, state: &mut MemoryState) {
    let chunks = Layout::vertical([
        Constraint::Length(2),
        Constraint::Min(3),
        Constraint::Length(1),
    ])
    .split(area);

    f.render_widget(
        Paragraph::new(Line::from(vec![Span::styled(
            "  Select an agent to browse its memory:",
            Style::default()
                .fg(theme::CYAN)
                .add_modifier(Modifier::BOLD),
        )])),
        chunks[0],
    );

    if state.loading {
        let spinner = theme::SPINNER_FRAMES[state.tick % theme::SPINNER_FRAMES.len()];
        f.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled(format!("  {spinner} "), Style::default().fg(theme::CYAN)),
                Span::styled("Loading agents\u{2026}", theme::dim_style()),
            ])),
            chunks[1],
        );
    } else if state.agents.is_empty() {
        f.render_widget(
            Paragraph::new(Span::styled("  No agents available.", theme::dim_style())),
            chunks[1],
        );
    } else {
        let items: Vec<ListItem> = state
            .agents
            .iter()
            .map(|a| {
                let id_short = if a.id.len() > 12 {
                    format!("{}\u{2026}", truncate_str(&a.id, 12))
                } else {
                    a.id.clone()
                };
                ListItem::new(Line::from(vec![
                    Span::styled(
                        format!("  {:<20}", a.name),
                        Style::default().fg(theme::CYAN),
                    ),
                    Span::styled(format!(" ({id_short})"), theme::dim_style()),
                ]))
            })
            .collect();

        let list = List::new(items)
            .highlight_style(theme::selected_style())
            .highlight_symbol("> ");
        f.render_stateful_widget(list, chunks[1], &mut state.agent_list_state);
    }

    f.render_widget(
        Paragraph::new(Line::from(vec![Span::styled(
            "  [\u{2191}\u{2193}] Navigate  [Enter] Browse KV  [r] Refresh",
            theme::hint_style(),
        )])),
        chunks[2],
    );
}

fn draw_kv_browser(f: &mut Frame, area: Rect, state: &mut MemoryState) {
    let chunks = Layout::vertical([
        Constraint::Length(2),
        Constraint::Min(3),
        Constraint::Length(1),
    ])
    .split(area);

    let agent_name = state
        .selected_agent
        .as_ref()
        .map(|a| a.name.as_str())
        .unwrap_or("?");

    f.render_widget(
        Paragraph::new(vec![
            Line::from(vec![
                Span::styled(
                    format!("  Memory: {agent_name}"),
                    Style::default()
                        .fg(theme::CYAN)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!("  ({} pairs)", state.kv_pairs.len()),
                    theme::dim_style(),
                ),
            ]),
            Line::from(vec![Span::styled(
                format!("  {:<24} {}", "Key", "Value"),
                theme::table_header(),
            )]),
        ]),
        chunks[0],
    );

    if state.loading {
        let spinner = theme::SPINNER_FRAMES[state.tick % theme::SPINNER_FRAMES.len()];
        f.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled(format!("  {spinner} "), Style::default().fg(theme::CYAN)),
                Span::styled("Loading\u{2026}", theme::dim_style()),
            ])),
            chunks[1],
        );
    } else if state.kv_pairs.is_empty() {
        f.render_widget(
            Paragraph::new(Span::styled(
                "  No key-value pairs. Press [a] to add one.",
                theme::dim_style(),
            )),
            chunks[1],
        );
    } else {
        let items: Vec<ListItem> = state
            .kv_pairs
            .iter()
            .map(|kv| {
                let val_display = if kv.value.len() > 40 {
                    format!("{}\u{2026}", truncate_str(&kv.value, 39))
                } else {
                    kv.value.clone()
                };
                ListItem::new(Line::from(vec![
                    Span::styled(
                        format!("  {:<24}", truncate(&kv.key, 23)),
                        Style::default().fg(theme::YELLOW),
                    ),
                    Span::styled(format!(" {val_display}"), theme::dim_style()),
                ]))
            })
            .collect();

        let list = List::new(items)
            .highlight_style(theme::selected_style())
            .highlight_symbol("> ");
        f.render_stateful_widget(list, chunks[1], &mut state.kv_list_state);
    }

    if state.confirm_delete {
        f.render_widget(
            Paragraph::new(Line::from(vec![Span::styled(
                "  Delete this key? [y] Yes  [any] Cancel",
                Style::default().fg(theme::YELLOW),
            )])),
            chunks[2],
        );
    } else if !state.status_msg.is_empty() {
        f.render_widget(
            Paragraph::new(Line::from(vec![Span::styled(
                format!("  {}", state.status_msg),
                Style::default().fg(theme::GREEN),
            )])),
            chunks[2],
        );
    } else {
        f.render_widget(
            Paragraph::new(Line::from(vec![Span::styled(
                "  [\u{2191}\u{2193}] Navigate  [a] Add  [e] Edit  [d] Delete  [Esc] Back  [r] Refresh",
                theme::hint_style(),
            )])),
            chunks[2],
        );
    }
}

fn draw_edit(f: &mut Frame, area: Rect, state: &MemoryState) {
    let chunks = Layout::vertical([
        Constraint::Length(2),
        Constraint::Length(1),
        Constraint::Length(2),
        Constraint::Length(1),
        Constraint::Length(2),
        Constraint::Min(1),
        Constraint::Length(1),
    ])
    .split(area);

    let title = if state.sub == MemorySub::AddKey {
        "Add Key-Value Pair"
    } else {
        "Edit Value"
    };

    f.render_widget(
        Paragraph::new(Line::from(vec![Span::styled(
            format!("  {title}"),
            Style::default()
                .fg(theme::CYAN)
                .add_modifier(Modifier::BOLD),
        )])),
        chunks[0],
    );

    // Key field
    let key_active = state.edit_field == EditField::Key && state.sub == MemorySub::AddKey;
    let key_label_style = if key_active {
        Style::default().fg(theme::ACCENT)
    } else {
        theme::dim_style()
    };
    f.render_widget(
        Paragraph::new(Line::from(vec![Span::styled("  Key: ", key_label_style)])),
        chunks[2],
    );
    let key_display = if state.key_buf.is_empty() {
        "enter key..."
    } else {
        &state.key_buf
    };
    let key_style = if state.key_buf.is_empty() {
        theme::dim_style()
    } else {
        theme::input_style()
    };
    let mut key_spans = vec![Span::raw("  > "), Span::styled(key_display, key_style)];
    if key_active {
        key_spans.push(Span::styled(
            "\u{2588}",
            Style::default()
                .fg(theme::GREEN)
                .add_modifier(Modifier::SLOW_BLINK),
        ));
    }
    f.render_widget(Paragraph::new(Line::from(key_spans)), chunks[3]);

    // Value field
    let val_active = state.edit_field == EditField::Value;
    let val_label_style = if val_active {
        Style::default().fg(theme::ACCENT)
    } else {
        theme::dim_style()
    };
    f.render_widget(
        Paragraph::new(Line::from(vec![Span::styled("  Value: ", val_label_style)])),
        chunks[4],
    );
    let val_display = if state.value_buf.is_empty() {
        "enter value..."
    } else {
        &state.value_buf
    };
    let val_style = if state.value_buf.is_empty() {
        theme::dim_style()
    } else {
        theme::input_style()
    };
    let mut val_spans = vec![Span::raw("  > "), Span::styled(val_display, val_style)];
    if val_active {
        val_spans.push(Span::styled(
            "\u{2588}",
            Style::default()
                .fg(theme::GREEN)
                .add_modifier(Modifier::SLOW_BLINK),
        ));
    }
    f.render_widget(Paragraph::new(Line::from(val_spans)), chunks[5]);

    f.render_widget(
        Paragraph::new(Line::from(vec![Span::styled(
            "  [Tab] Switch field  [Enter] Save  [Esc] Cancel",
            theme::hint_style(),
        )])),
        chunks[6],
    );
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}\u{2026}", truncate_str(s, max.saturating_sub(1)))
    }
}

fn truncate_str(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        s[..max].to_string()
    }
}