use color_eyre::eyre::Result;
use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use std::time::{Duration, Instant};

use crate::Difficulty;
use crate::game::{App, render};
use crate::state::read_state;

pub fn run(difficulty: Difficulty) -> Result<()> {
    install_panic_hook();
    let mut terminal = ratatui::init();
    let result = run_loop(&mut terminal, difficulty);
    ratatui::restore();
    result
}

fn install_panic_hook() {
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        ratatui::restore();
        original_hook(info);
    }));
}

fn run_loop(terminal: &mut ratatui::DefaultTerminal, difficulty: Difficulty) -> Result<()> {
    let mut app = App::new(difficulty);

    loop {
        // 1. Poll keyboard (100ms timeout = tick rate)
        if event::poll(Duration::from_millis(100))?
            && let Event::Key(key) = event::read()?
        {
            // Ctrl-C always quits
            if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
                break;
            }
            if let Some(action) = crate::game::input::map_key(key) {
                app.handle_action(action);
            }
        }

        // 2. Refresh Claude state from disk
        app.refresh_claude_state(read_state());

        // 3. Tick timers
        app.tick(Instant::now());

        // 4. Render  (pass &mut app so render can update viewport)
        terminal.draw(|f| render::ui(f, &mut app))?;

        if app.should_quit {
            break;
        }
    }

    Ok(())
}
