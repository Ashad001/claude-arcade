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
    ToggleLeaderboard,
    BackToMenu,
    Quit,
}

/// Map a key event to an Action.
/// Toggle actions only fire on Press — never on Repeat.
/// Movement and reveal are allowed to repeat (comfortable to hold).
/// Release events are always ignored.
pub fn map_key(key: KeyEvent) -> Option<Action> {
    let is_toggle = matches!(
        key.code,
        KeyCode::Char('f')
            | KeyCode::Char('r')
            | KeyCode::Char('q')
            | KeyCode::Char('m')
            | KeyCode::Esc
            | KeyCode::Tab
    );

    if is_toggle && key.kind != KeyEventKind::Press {
        return None;
    }
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
        KeyCode::Tab => Some(Action::ToggleLeaderboard),
        KeyCode::Esc | KeyCode::Char('m') => Some(Action::BackToMenu),
        KeyCode::Char('q') => Some(Action::Quit),
        _ => None,
    }
}
