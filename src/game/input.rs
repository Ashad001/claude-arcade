use crossterm::event::{KeyCode, KeyEvent, KeyEventKind};

#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    MoveUp,
    MoveDown,
    MoveLeft,
    MoveRight,
    Reveal,
    Flag,
    Restart,
    Quit,
}

/// Map a key event to an Action.
/// Only processes Press events — ignores Repeat and Release.
/// This prevents toggle actions (flag, restart) from firing twice when a key
/// is held down on Windows, which sends both Press and Repeat events.
pub fn map_key(key: KeyEvent) -> Option<Action> {
    // Allow movement and reveal to repeat (comfortable to hold)
    // Block repeat for toggles: flag, restart, quit
    let is_toggle = matches!(
        key.code,
        KeyCode::Char('f') | KeyCode::Char('r') | KeyCode::Char('q') | KeyCode::Esc
    );

    if is_toggle && key.kind != KeyEventKind::Press {
        return None;
    }

    // Ignore release events for everything
    if key.kind == KeyEventKind::Release {
        return None;
    }

    match key.code {
        KeyCode::Up | KeyCode::Char('k') => Some(Action::MoveUp),
        KeyCode::Down | KeyCode::Char('j') => Some(Action::MoveDown),
        KeyCode::Left | KeyCode::Char('h') => Some(Action::MoveLeft),
        KeyCode::Right | KeyCode::Char('l') => Some(Action::MoveRight),
        KeyCode::Char(' ') | KeyCode::Enter => Some(Action::Reveal),
        KeyCode::Char('f') => Some(Action::Flag),
        KeyCode::Char('r') => Some(Action::Restart),
        KeyCode::Char('q') | KeyCode::Esc => Some(Action::Quit),
        _ => None,
    }
}
