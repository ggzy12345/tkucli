use crossterm::event::{self, Event as CrossEvent, KeyCode, KeyEvent, KeyModifiers};
use std::time::Duration;
use tokio::sync::mpsc;

/// Normalised application events.
#[derive(Debug, Clone)]
pub enum AppEvent {
    Key(KeyEvent),
    Tick,
    Resize(u16, u16),
    Quit,
}

/// Spawns a background task that polls crossterm and forwards events
/// onto an async channel. The caller drives the event loop by receiving
/// from the returned `Receiver`.
pub fn spawn_event_loop(tick_ms: u64) -> mpsc::UnboundedReceiver<AppEvent> {
    let (tx, rx) = mpsc::unbounded_channel();

    tokio::spawn(async move {
        loop {
            let timeout = Duration::from_millis(tick_ms);
            if event::poll(timeout).unwrap_or(false) {
                match event::read() {
                    Ok(CrossEvent::Key(k)) => {
                        // Ctrl-C / Ctrl-Q always quit.
                        if (k.code == KeyCode::Char('c')
                            && k.modifiers.contains(KeyModifiers::CONTROL))
                            || (k.code == KeyCode::Char('q')
                                && k.modifiers.contains(KeyModifiers::CONTROL))
                        {
                            let _ = tx.send(AppEvent::Quit);
                            break;
                        }
                        let _ = tx.send(AppEvent::Key(k));
                    }
                    Ok(CrossEvent::Resize(w, h)) => {
                        let _ = tx.send(AppEvent::Resize(w, h));
                    }
                    _ => {}
                }
            } else {
                let _ = tx.send(AppEvent::Tick);
            }
        }
    });

    rx
}

/// Simple keybinding helper.
pub fn is_key(ev: &AppEvent, code: KeyCode) -> bool {
    matches!(ev, AppEvent::Key(k) if k.code == code)
}

pub fn is_char(ev: &AppEvent, c: char) -> bool {
    matches!(ev, AppEvent::Key(k) if k.code == KeyCode::Char(c))
}
