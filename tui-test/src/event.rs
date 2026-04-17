//! Event system: crossterm polling + tick timer.
//!
//! Cloned from AgentDiVA TUI with minimal changes.

use crossterm::event::{self, Event as CtEvent, KeyEvent, KeyEventKind};
use std::sync::mpsc;
use std::time::Duration;

/// Unified application event.
pub enum AppEvent {
    /// A crossterm key press event (filtered to Press only).
    Key(KeyEvent),
    /// Periodic tick for animations (spinners, etc.).
    Tick,
}

/// Spawn the crossterm polling + tick thread. Returns sender + receiver.
pub fn spawn_event_thread(tick_rate: Duration) -> (mpsc::Sender<AppEvent>, mpsc::Receiver<AppEvent>) {
    let (tx, rx) = mpsc::channel();
    let poll_tx = tx.clone();

    std::thread::spawn(move || {
        loop {
            if event::poll(tick_rate).unwrap_or(false) {
                if let Ok(ev) = event::read() {
                    let sent = match ev {
                        // CRITICAL: only forward Press events — Windows sends
                        // Release and Repeat too, which causes double/triple input
                        CtEvent::Key(key) if key.kind == KeyEventKind::Press => {
                            poll_tx.send(AppEvent::Key(key))
                        }
                        _ => Ok(()),
                    };
                    if sent.is_err() {
                        break;
                    }
                }
            } else {
                // No event within tick_rate → send tick for spinner animations
                if poll_tx.send(AppEvent::Tick).is_err() {
                    break;
                }
            }
        }
    });

    (tx, rx)
}