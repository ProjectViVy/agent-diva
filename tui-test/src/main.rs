//! TUI Test - AgentDiVA TUI interaction clone
//!
//! This is a test project that clones the interaction patterns from AgentDiVA
//! with placeholder functionality.

mod app;
mod config;
mod draw;
mod event;
mod i18n;
mod screens;
mod theme;

use std::time::Duration;

use app::App;
use event::{spawn_event_thread, AppEvent};

/// Entry point for the TUI interactive mode.
pub fn run() {
    // Panic hook: always restore terminal
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        ratatui::restore();
        original_hook(info);
    }));

    let mut terminal = ratatui::init();

    // 50ms tick → 20fps spinner animation, snappy key response
    let (tx, rx) = spawn_event_thread(Duration::from_millis(50));
    let mut app = App::new(tx);

    // Initial screen - Welcome
    app.start_welcome();

    // Main loop
    // Draw first, then block on events. This ensures the first frame appears
    // immediately, before any event processing.
    while !app.should_quit {
        terminal
            .draw(|frame| app.draw(frame))
            .expect("Failed to draw");

        // Block until at least one event arrives (or 33ms timeout for ~30fps)
        match rx.recv_timeout(Duration::from_millis(33)) {
            Ok(ev) => app.handle_event(ev),
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
        }
        // Drain all queued events immediately (batch processing)
        while let Ok(ev) = rx.try_recv() {
            app.handle_event(ev);
        }
    }

    ratatui::restore();
}

fn main() {
    run();
}