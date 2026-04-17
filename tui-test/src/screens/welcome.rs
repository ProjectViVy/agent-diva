//! Welcome screen: branded logo, mode selection menu.
//!
//! Cloned from AgentDiVA TUI screens/welcome.rs with i18n support.

use crate::i18n::{Language, TranslationKey, Translator};
use crate::theme;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{List, ListItem, ListState, Paragraph};
use ratatui::Frame;

// ASCII Logo

const LOGO: &str = r#"  ▄▄▄▄    ▄▄▄▄▄▄▄   ▄▄▄▄▄▄▄ ▄▄▄    ▄▄▄ ▄▄▄▄▄▄▄▄▄    ▄▄▄▄▄▄   ▄▄▄▄▄ ▄▄▄▄  ▄▄▄▄   ▄▄▄▄               
▄██▀▀██▄ ███▀▀▀▀▀  ███▀▀▀▀▀ ████▄  ███ ▀▀▀███▀▀▀    ███▀▀██▄  ███  ▀███  ███▀ ▄██▀▀██▄              
███  ███ ███       ███▄▄    ███▀██▄███    ███       ███  ███  ███   ███  ███  ███  ███              
███▀▀███ ███  ███▀ ███      ███  ▀████    ███ ▀▀▀▀▀ ███  ███  ███   ███▄▄███  ███▀▀███              
███  ███ ▀██████▀  ▀███████ ███    ███    ███       ██████▀  ▄███▄   ▀████▀   ███  ███              "#;

const LOGO_HEIGHT: u16 = 5;
const LOGO_MIN_WIDTH: u16 = 90;

// State

pub struct WelcomeState {
    pub menu: ListState,
    pub tick: usize,
    pub ctrl_c_pending: bool,
    ctrl_c_tick: usize,
    /// Language selection sub-menu mode
    pub language_menu_active: bool,
    pub language_menu: ListState,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum WelcomeAction {
    ConnectDaemon,
    InProcess,
    Wizard,
    SwitchLanguage(Language),
    Exit,
}

impl WelcomeState {
    const CTRL_C_TIMEOUT: usize = 40;

    pub fn new() -> Self {
        Self {
            menu: ListState::default().with_selected(Some(0)),
            tick: 0,
            ctrl_c_pending: false,
            ctrl_c_tick: 0,
            language_menu_active: false,
            language_menu: ListState::default().with_selected(Some(0)),
        }
    }

    pub fn tick(&mut self) {
        self.tick = self.tick.wrapping_add(1);
        // Auto-reset Ctrl+C pending after timeout
        if self.ctrl_c_pending && self.tick.wrapping_sub(self.ctrl_c_tick) > Self::CTRL_C_TIMEOUT {
            self.ctrl_c_pending = false;
        }
    }

    /// Get menu items based on current language
    fn get_menu_items(i18n: &Translator) -> Vec<(TranslationKey, TranslationKey, WelcomeAction)> {
        vec![
            (TranslationKey::WelcomeMenuConnect, TranslationKey::WelcomeHintConnect, WelcomeAction::ConnectDaemon),
            (TranslationKey::WelcomeMenuInProcess, TranslationKey::WelcomeHintInProcess, WelcomeAction::InProcess),
            (TranslationKey::WelcomeMenuWizard, TranslationKey::WelcomeHintWizard, WelcomeAction::Wizard),
            (TranslationKey::WelcomeMenuLanguage, TranslationKey::WelcomeHintLanguage, WelcomeAction::SwitchLanguage(Language::English)), // placeholder, actual language determined by selection
            (TranslationKey::WelcomeMenuExit, TranslationKey::WelcomeHintExit, WelcomeAction::Exit),
        ]
    }

    /// Get language options
    fn get_language_options(i18n: &Translator) -> Vec<(String, Language)> {
        vec![
            (i18n.t(TranslationKey::SettingsLangEn).to_string(), Language::English),
            (i18n.t(TranslationKey::SettingsLangZh).to_string(), Language::Chinese),
        ]
    }

    /// Handle a key event. Returns Some(action) if one was selected.
    pub fn handle_key(&mut self, key: KeyEvent, i18n: &Translator) -> Option<WelcomeAction> {
        let is_ctrl_c =
            key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL);

        // Double Ctrl+C to exit
        if is_ctrl_c {
            if self.ctrl_c_pending {
                return Some(WelcomeAction::Exit);
            }
            self.ctrl_c_pending = true;
            self.ctrl_c_tick = self.tick;
            return None;
        }

        // Any other key clears the Ctrl+C pending state
        self.ctrl_c_pending = false;

        // Language selection sub-menu
        if self.language_menu_active {
            match key.code {
                KeyCode::Esc => {
                    self.language_menu_active = false;
                    return None;
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    self.language_menu.select(Some(0));
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    self.language_menu.select(Some(1));
                }
                KeyCode::Enter => {
                    let lang_options = Self::get_language_options(i18n);
                    if let Some(idx) = self.language_menu.selected() {
                        if idx < lang_options.len() {
                            let lang = lang_options[idx].1;
                            self.language_menu_active = false;
                            return Some(WelcomeAction::SwitchLanguage(lang));
                        }
                    }
                }
                _ => {}
            }
            return None;
        }

        // Main menu navigation
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => return Some(WelcomeAction::Exit),
            KeyCode::Up | KeyCode::Char('k') => {
                let i = self.menu.selected().unwrap_or(0);
                let menu_len = Self::get_menu_items(i18n).len();
                let next = if i == 0 { menu_len - 1 } else { i - 1 };
                self.menu.select(Some(next));
            }
            KeyCode::Down | KeyCode::Char('j') => {
                let i = self.menu.selected().unwrap_or(0);
                let menu_len = Self::get_menu_items(i18n).len();
                let next = (i + 1) % menu_len;
                self.menu.select(Some(next));
            }
            KeyCode::Enter => {
                if let Some(i) = self.menu.selected() {
                    let menu_items = Self::get_menu_items(i18n);
                    if i < menu_items.len() {
                        let (_, _, action) = menu_items[i];
                        // Special handling for language menu
                        if matches!(action, WelcomeAction::SwitchLanguage(_)) {
                            self.language_menu_active = true;
                            self.language_menu.select(Some(0));
                            return None;
                        }
                        return Some(action);
                    }
                }
            }
            _ => {}
        }
        None
    }
}

// Drawing

/// Render the welcome screen.
pub fn draw(f: &mut Frame, area: Rect, state: &mut WelcomeState, i18n: &Translator) {
    // Fill background
    f.render_widget(
        ratatui::widgets::Block::default().style(Style::default().bg(theme::BG_PRIMARY)),
        area,
    );

    let version = env!("CARGO_PKG_VERSION");
    let compact = area.width < LOGO_MIN_WIDTH;

    // Logo height: full (6 lines) or compact (1 line)
    let logo_h: u16 = if compact { 1 } else { LOGO_HEIGHT };

    // Left-aligned content area
    let content = if area.width < 10 || area.height < 5 {
        area
    } else {
        let margin = 3u16.min(area.width.saturating_sub(10));
        let w = 100u16.min(area.width.saturating_sub(margin));
        Rect {
            x: area.x.saturating_add(margin),
            y: area.y,
            width: w,
            height: area.height,
        }
    };

    // Vertical layout with upper-third positioning
    let total_needed = 1 + logo_h + 1 + 1 + 2 + 1 + 5 + 1;
    let top_pad = if area.height > total_needed + 2 {
        ((area.height - total_needed) / 3).max(1)
    } else {
        1
    };

    let chunks = Layout::vertical([
        Constraint::Length(top_pad), // top space
        Constraint::Length(logo_h),  // logo
        Constraint::Length(1),       // tagline + version
        Constraint::Length(1),       // separator
        Constraint::Length(2),       // status block
        Constraint::Length(1),       // separator
        Constraint::Min(1),          // menu
        Constraint::Length(1),       // key hints
        Constraint::Min(0),          // remaining
    ])
    .split(content);

    // Logo
    if compact {
        let line = Line::from(vec![Span::styled(
            i18n.t(TranslationKey::WelcomeLogoCompact),
            Style::default()
                .fg(theme::ACCENT)
                .add_modifier(Modifier::BOLD),
        )]);
        f.render_widget(Paragraph::new(line), chunks[1]);
    } else {
        let logo_lines: Vec<Line> = LOGO
            .lines()
            .map(|l| Line::from(vec![Span::styled(l, Style::default().fg(theme::ACCENT))]))
            .collect();
        f.render_widget(
            Paragraph::new(logo_lines).alignment(Alignment::Left),
            chunks[1],
        );
    }

    // Tagline + version
    let tagline = Line::from(vec![
        Span::styled(
            i18n.t(TranslationKey::WelcomeTitle),
            Style::default()
                .fg(theme::TEXT_PRIMARY)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(format!("  v{version}"), theme::dim_style()),
    ]);
    f.render_widget(Paragraph::new(tagline), chunks[2]);

    // Separator
    let sep_w = content.width.min(60) as usize;
    let sep_line = Line::from(vec![Span::styled(
        "\u{2500}".repeat(sep_w),
        Style::default().fg(theme::BORDER),
    )]);
    f.render_widget(Paragraph::new(sep_line.clone()), chunks[3]);

    // Status block (placeholder)
    let status_lines: Vec<Line> = vec![
        Line::from(vec![
            Span::styled("\u{25cf} ", Style::default().fg(theme::GREEN).add_modifier(Modifier::BOLD)),
            Span::styled(i18n.t(TranslationKey::WelcomeStatusDaemon), Style::default().fg(theme::TEXT_PRIMARY)),
        ]),
        Line::from(vec![
            Span::styled("\u{2714} ", Style::default().fg(theme::GREEN).add_modifier(Modifier::BOLD)),
            Span::styled(i18n.t(TranslationKey::WelcomeStatusProvider), Style::default().fg(theme::TEXT_PRIMARY)),
            Span::styled(i18n.t(TranslationKey::WelcomeStatusMock), theme::dim_style()),
        ]),
    ];
    f.render_widget(Paragraph::new(status_lines), chunks[4]);

    // Separator 2
    f.render_widget(Paragraph::new(sep_line), chunks[5]);

    // Menu or language sub-menu
    if state.language_menu_active {
        draw_language_menu(f, chunks[6], state, i18n);
    } else {
        draw_main_menu(f, chunks[6], state, i18n);
    }

    // Hints
    let hints = if state.ctrl_c_pending {
        Line::from(vec![Span::styled(
            i18n.t(TranslationKey::WelcomeCtrlCQuit),
            Style::default().fg(theme::YELLOW),
        )])
    } else if state.language_menu_active {
        Line::from(vec![Span::styled(
            "\u{2191}\u{2193} select  Enter confirm  Esc back",
            theme::hint_style(),
        )])
    } else {
        Line::from(vec![Span::styled(
            i18n.t(TranslationKey::WelcomeHintNavigate),
            theme::hint_style(),
        )])
    };
    f.render_widget(Paragraph::new(hints), chunks[7]);
}

fn draw_main_menu(f: &mut Frame, area: Rect, state: &mut WelcomeState, i18n: &Translator) {
    let menu_items = WelcomeState::get_menu_items(i18n);
    let current_lang = i18n.language();

    let items: Vec<ListItem> = menu_items
        .iter()
        .enumerate()
        .map(|(i, (label_key, hint_key, action))| {
            let label = i18n.t(*label_key);
            let hint = i18n.t(*hint_key);

            // Add current language indicator for language menu item
            let label_text = if matches!(action, WelcomeAction::SwitchLanguage(_)) {
                format!("{:<26}", label)
            } else {
                format!("{:<26}", label)
            };

            // Show current language in hint for language menu item
            let hint_text = if matches!(action, WelcomeAction::SwitchLanguage(_)) {
                let lang_display = match current_lang {
                    Language::English => "English",
                    Language::Chinese => "中文",
                };
                format!("({})", lang_display)
            } else {
                hint.to_string()
            };

            ListItem::new(Line::from(vec![
                Span::raw(label_text),
                Span::styled(hint_text, theme::dim_style()),
            ]))
        })
        .collect();

    let list = List::new(items)
        .highlight_style(
            Style::default()
                .fg(theme::ACCENT)
                .bg(theme::BG_HOVER)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("\u{25b8} ");

    f.render_stateful_widget(list, area, &mut state.menu);
}

fn draw_language_menu(f: &mut Frame, area: Rect, state: &mut WelcomeState, i18n: &Translator) {
    let lang_options = WelcomeState::get_language_options(i18n);
    let current_lang = i18n.language();

    let title = Paragraph::new(Line::from(vec![Span::styled(
        format!(" {} ", i18n.t(TranslationKey::SettingsLangSelect)),
        theme::title_style(),
    )]));
    f.render_widget(title, area);

    let items: Vec<ListItem> = lang_options
        .iter()
        .map(|(name, lang)| {
            let is_current = *lang == current_lang;
            let indicator = if is_current {
                format!(" [{}]", i18n.t(TranslationKey::SettingsLangCurrent))
            } else {
                String::new()
            };
            ListItem::new(Line::from(vec![
                Span::raw(format!("  {:<20}", name)),
                Span::styled(indicator, Style::default().fg(theme::GREEN)),
            ]))
        })
        .collect();

    let list_area = Rect {
        x: area.x + 2,
        y: area.y + 2,
        width: area.width.saturating_sub(4),
        height: area.height.saturating_sub(2).min(4),
    };

    let list = List::new(items)
        .highlight_style(
            Style::default()
                .fg(theme::ACCENT)
                .bg(theme::BG_HOVER)
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("\u{25b8} ");

    f.render_stateful_widget(list, list_area, &mut state.language_menu);
}