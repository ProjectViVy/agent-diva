//! Placeholder screen - generic "Coming Soon" screen for tabs without implementation.
//!
//! Used for all tabs except Welcome, Agents, Dashboard, and Chat.

use crate::i18n::Translator;
use crate::theme;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Padding, Paragraph};
use ratatui::Frame;

pub struct PlaceholderState {
    pub name: String,
    pub list: ListState,
    pub items: Vec<String>,
}

pub enum PlaceholderAction {
    Continue,
    Back,
}

impl PlaceholderState {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            list: ListState::default(),
            items: vec!["Coming Soon".to_string(), "Feature not implemented".to_string()],
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> PlaceholderAction {
        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                let i = self.list.selected().unwrap_or(0);
                let next = if i == 0 { self.items.len() - 1 } else { i - 1 };
                self.list.select(Some(next));
            }
            KeyCode::Down | KeyCode::Char('j') => {
                let i = self.list.selected().unwrap_or(0);
                let next = (i + 1) % self.items.len();
                self.list.select(Some(next));
            }
            KeyCode::Esc => return PlaceholderAction::Back,
            _ => {}
        }
        PlaceholderAction::Continue
    }
}

pub fn draw(f: &mut Frame, area: Rect, state: &mut PlaceholderState, _i18n: &Translator) {
    let block = Block::default()
        .title(Line::from(vec![Span::styled(
            format!(" {} ", state.name),
            theme::title_style(),
        )]))
        .title_alignment(Alignment::Left)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme::ACCENT))
        .padding(Padding::horizontal(1));

    let inner = block.inner(area);
    f.render_widget(block, area);

    let chunks = Layout::vertical([Constraint::Min(3), Constraint::Length(1)]).split(inner);

    // Placeholder message
    let items: Vec<ListItem> = state
        .items
        .iter()
        .map(|s| ListItem::new(Line::from(vec![Span::styled(format!("  {}", s), theme::dim_style())])))
        .collect();

    let list = List::new(items)
        .highlight_style(theme::selected_style())
        .highlight_symbol("> ");

    f.render_stateful_widget(list, chunks[0], &mut state.list);

    // Hints
    let hints = Paragraph::new(Line::from(vec![Span::styled(
        "  [\u{2191}\u{2193}] Navigate  [Esc] Back  [Tab] Switch tab",
        theme::hint_style(),
    )]));
    f.render_widget(hints, chunks[1]);
}