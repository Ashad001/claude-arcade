use crossterm::event::{KeyCode, KeyEvent};

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

/// Map a key event to an Action. Returns None for unrecognised keys.
pub fn map_key(key: KeyEvent) -> Option<Action> {
    match key.code {
        // Movement: arrows + vi keys
        KeyCode::Up | KeyCode::Char('k') => Some(Action::MoveUp),
        KeyCode::Down | KeyCode::Char('j') => Some(Action::MoveDown),
        KeyCode::Left | KeyCode::Char('h') => Some(Action::MoveLeft),
        KeyCode::Right | KeyCode::Char('l') => Some(Action::MoveRight),
        // Reveal: Space or Enter
        KeyCode::Char(' ') | KeyCode::Enter => Some(Action::Reveal),
        // Flag
        KeyCode::Char('f') => Some(Action::Flag),
        // Restart
        KeyCode::Char('r') => Some(Action::Restart),
        // Quit
        KeyCode::Char('q') | KeyCode::Esc => Some(Action::Quit),
        _ => None,
    }
}
