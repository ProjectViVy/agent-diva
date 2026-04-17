//! Main rendering logic and Tab Bar.
//!
//! Cloned from AgentDiVA TUI mod.rs draw() and draw_tab_bar().

use crate::app::{App, BootScreen, Phase, Tab, TABS};
use crate::i18n::TranslationKey;
use crate::screens::{
    agents, audit, channels, chat, comms, dashboard, extensions, hands, logs, memory, peers,
    security, sessions, settings, skills, templates, triggers, usage, workflows, welcome,
    wizard,
};
use crate::theme;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

/// Main draw function - dispatches to appropriate screen.
pub fn draw(app: &mut App, frame: &mut Frame) {
    let area = frame.area();

    match app.phase {
        Phase::Boot(BootScreen::Welcome) => {
            welcome::draw(frame, area, &mut app.welcome, &app.i18n);

            // Overlay boot status toast
            if app.ctrl_c_pending {
                let msg = app.i18n.t(TranslationKey::TabBarCtrlCQuit);
                render_toast(frame, area, msg, theme::YELLOW);
            }
        }
        Phase::Boot(BootScreen::Wizard) => {
            wizard::draw(frame, area, &mut app.wizard, &app.i18n);
        }
        Phase::Main => {
            // Split: tab bar (1 line) + content
            let chunks = Layout::vertical([
                Constraint::Length(1), // tab bar
                Constraint::Min(1),    // content
            ])
            .split(area);

            draw_tab_bar(frame, chunks[0], app);

            match app.active_tab {
                Tab::Dashboard => dashboard::draw(frame, chunks[1], &mut app.dashboard, &app.i18n),
                Tab::Agents => agents::draw(frame, chunks[1], &mut app.agents, &app.i18n),
                Tab::Chat => chat::draw(frame, chunks[1], &mut app.chat, &app.i18n),
                Tab::Sessions => sessions::draw(frame, chunks[1], &mut app.sessions, &app.i18n),
                Tab::Workflows => workflows::draw(frame, chunks[1], &mut app.workflows, &app.i18n),
                Tab::Triggers => triggers::draw(frame, chunks[1], &mut app.triggers, &app.i18n),
                Tab::Memory => memory::draw(frame, chunks[1], &mut app.memory, &app.i18n),
                Tab::Channels => channels::draw(frame, chunks[1], &mut app.channels, &app.i18n),
                Tab::Skills => skills::draw(frame, chunks[1], &mut app.skills, &app.i18n),
                Tab::Hands => hands::draw(frame, chunks[1], &mut app.hands, &app.i18n),
                Tab::Extensions => extensions::draw(frame, chunks[1], &mut app.extensions, &app.i18n),
                Tab::Templates => templates::draw(frame, chunks[1], &mut app.templates, &app.i18n),
                Tab::Peers => peers::draw(frame, chunks[1], &mut app.peers, &app.i18n),
                Tab::Comms => comms::draw(frame, chunks[1], &mut app.comms, &app.i18n),
                Tab::Security => security::draw(frame, chunks[1], &mut app.security, &app.i18n),
                Tab::Audit => audit::draw(frame, chunks[1], &mut app.audit, &app.i18n),
                Tab::Usage => usage::draw(frame, chunks[1], &mut app.usage, &app.i18n),
                Tab::Settings => settings::draw(frame, chunks[1], &mut app.settings, &app.i18n),
                Tab::Logs => logs::draw(frame, chunks[1], &mut app.logs, &app.i18n),
            }
        }
    }
}

/// Draw the tab bar with scrolling support and overflow indicators.
fn draw_tab_bar(frame: &mut Frame, area: Rect, app: &App) {
    let width = area.width as usize;

    // Compute all tab labels with their widths
    let tab_labels: Vec<(usize, String)> = TABS
        .iter()
        .map(|tab| {
            let label = format!(" {} ", tab.label(&app.i18n));
            let w = label.len() + 1; // +1 for spacing
            (w, label)
        })
        .collect();

    // Reserve space for overflow indicators (2 chars each) and hint
    let indicator_width = 2; // "< " or " >"
    let hint = if app.ctrl_c_pending {
        app.i18n.t(TranslationKey::TabBarCtrlCQuit)
    } else {
        app.i18n.t(TranslationKey::TabBarCtrlCHint)
    };
    let hint_width = hint.len() + 2;
    let available = width.saturating_sub(hint_width + 2);

    // Ensure active tab is visible by adjusting scroll offset
    let active_idx = app.active_tab.index();

    // Scroll so active tab fits in the visible window
    let mut tab_scroll_offset = app.tab_scroll_offset;
    if active_idx < tab_scroll_offset {
        tab_scroll_offset = active_idx;
    }

    // Find how many tabs fit starting from scroll offset
    loop {
        let mut used = if tab_scroll_offset > 0 {
            indicator_width
        } else {
            1
        };
        let mut last_visible = tab_scroll_offset;
        for (i, (tab_w, _)) in tab_labels.iter().enumerate().skip(tab_scroll_offset) {
            if used + tab_w > available {
                break;
            }
            used += tab_w;
            last_visible = i;
        }
        if active_idx <= last_visible || tab_scroll_offset >= TABS.len() - 1 {
            break;
        }
        tab_scroll_offset += 1;
    }

    let mut spans: Vec<Span> = Vec::new();

    // Left overflow indicator
    if tab_scroll_offset > 0 {
        spans.push(Span::styled("< ", Style::default().fg(theme::TEXT_TERTIARY)));
    } else {
        spans.push(Span::raw(" "));
    }

    // Render visible tabs
    let mut used = if tab_scroll_offset > 0 { indicator_width } else { 1 };
    let mut last_rendered = tab_scroll_offset;
    for (i, ((tab_w, label), &tab)) in tab_labels
        .iter()
        .zip(TABS.iter())
        .enumerate()
        .skip(tab_scroll_offset)
    {
        if used + tab_w > available {
            break;
        }
        if tab == app.active_tab {
            spans.push(Span::styled(label.clone(), theme::tab_active()));
        } else {
            spans.push(Span::styled(label.clone(), theme::tab_inactive()));
        }
        spans.push(Span::raw(" "));
        used += tab_w;
        last_rendered = i;
    }

    // Right overflow indicator
    if last_rendered < TABS.len() - 1 {
        spans.push(Span::styled(" >", Style::default().fg(theme::TEXT_TERTIARY)));
    }

    // Right-aligned hint (yellow warning when Ctrl+C pending)
    let hint_style = if app.ctrl_c_pending {
        Style::default().fg(theme::YELLOW).add_modifier(Modifier::BOLD)
    } else {
        theme::hint_style()
    };
    let spans_width: usize = spans.iter().map(|s| s.content.len()).sum();
    let padding = width.saturating_sub(spans_width + hint.len());
    if padding > 0 {
        spans.push(Span::raw(" ".repeat(padding)));
        spans.push(Span::styled(hint, hint_style));
    }

    let bar = Paragraph::new(Line::from(spans)).style(Style::default().bg(theme::BG_CARD));
    frame.render_widget(bar, area);
}

/// Draw a one-line toast at the bottom of the screen.
fn render_toast(frame: &mut Frame, area: Rect, msg: &str, color: ratatui::style::Color) {
    let w = (msg.len() as u16 + 4).min(area.width);
    let x = area.width.saturating_sub(w) / 2;
    let y = area.height.saturating_sub(2);
    let toast_area = Rect::new(x, y, w, 1);
    let para = Paragraph::new(Line::from(vec![Span::styled(msg, Style::default().fg(color))]));
    frame.render_widget(para, toast_area);
}