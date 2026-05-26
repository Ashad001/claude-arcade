use color_eyre::eyre::Result;
use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use std::time::{Duration, Instant};

use crate::games::game2048::App as App2048;
use crate::games::minesweeper::App as MinesweeperApp;
use crate::games::tictactoe::App as TicTacToeApp;
use crate::menu::{Menu, MenuSignal, SelectedGame};
use crate::state::read_state;

// ─── Signal types ─────────────────────────────────────────────────────────────

enum Signal {
    Continue,
    BackToMenu,
    LaunchGame(SelectedGame),
    Quit,
}

// ─── Screen enum ─────────────────────────────────────────────────────────────

enum Screen {
    Menu(Menu),
    Minesweeper(MinesweeperApp),
    TicTacToe(TicTacToeApp),
    Game2048(App2048),
}

impl Screen {
    fn handle_key(&mut self, key: crossterm::event::KeyEvent) -> Signal {
        match self {
            Screen::Menu(menu) => match menu.handle_key(key) {
                MenuSignal::Launch(game) => Signal::LaunchGame(game),
                MenuSignal::Quit => Signal::Quit,
                MenuSignal::None => Signal::Continue,
            },
            Screen::Minesweeper(app) => {
                if let Some(action) = crate::games::minesweeper::input::map_key(key) {
                    app.handle_action(action);
                }
                if app.should_quit {
                    Signal::Quit
                } else if app.back_to_menu {
                    Signal::BackToMenu
                } else {
                    Signal::Continue
                }
            }
            Screen::TicTacToe(app) => {
                app.handle_key(key);
                if app.should_quit {
                    Signal::Quit
                } else if app.back_to_menu {
                    Signal::BackToMenu
                } else {
                    Signal::Continue
                }
            }
            Screen::Game2048(app) => {
                app.handle_key(key);
                if app.should_quit {
                    Signal::Quit
                } else if app.back_to_menu {
                    Signal::BackToMenu
                } else {
                    Signal::Continue
                }
            }
        }
    }

    fn refresh_claude_state(&mut self, state: crate::state::ClaudeState) {
        match self {
            Screen::Menu(m) => m.refresh_claude_state(state),
            Screen::Minesweeper(a) => a.refresh_claude_state(state),
            Screen::TicTacToe(a) => a.refresh_claude_state(state),
            Screen::Game2048(a) => a.refresh_claude_state(state),
        }
    }

    fn tick(&mut self, now: Instant) {
        match self {
            Screen::Menu(m) => m.tick(now),
            Screen::Minesweeper(a) => a.tick(now),
            Screen::TicTacToe(a) => a.tick(now),
            Screen::Game2048(a) => a.tick(now),
        }
    }

    fn render(&mut self, frame: &mut ratatui::Frame) {
        match self {
            Screen::Menu(m) => crate::menu::render(frame, m),
            Screen::Minesweeper(a) => crate::games::minesweeper::render::ui(frame, a),
            Screen::TicTacToe(a) => crate::games::tictactoe::render::ui(frame, a),
            Screen::Game2048(a) => crate::games::game2048::render::ui(frame, a),
        }
    }
}

// ─── Public entry point ───────────────────────────────────────────────────────

pub fn run() -> Result<()> {
    install_panic_hook();
    let mut terminal = ratatui::init();
    let result = run_loop(&mut terminal);
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

fn run_loop(terminal: &mut ratatui::DefaultTerminal) -> Result<()> {
    let mut screen = Screen::Menu(Menu::new());

    loop {
        // 1. Poll keyboard (100ms timeout = tick rate)
        if event::poll(Duration::from_millis(100))?
            && let Event::Key(key) = event::read()?
        {
            // Ctrl-C always quits
            if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
                break;
            }

            let signal = screen.handle_key(key);

            match signal {
                Signal::Quit => break,
                Signal::BackToMenu => {
                    screen = Screen::Menu(Menu::new());
                    // Immediately push current Claude state into the fresh menu
                    screen.refresh_claude_state(read_state());
                }
                Signal::LaunchGame(game) => {
                    screen = launch(game);
                    screen.refresh_claude_state(read_state());
                }
                Signal::Continue => {}
            }
        }

        // 2. Refresh Claude state from disk every tick
        let claude_state = read_state();
        screen.refresh_claude_state(claude_state);

        // 3. Tick timers / animations
        screen.tick(Instant::now());

        // 4. Render
        terminal.draw(|f| screen.render(f))?;
    }

    Ok(())
}

fn launch(game: SelectedGame) -> Screen {
    match game {
        SelectedGame::Minesweeper(diff) => Screen::Minesweeper(MinesweeperApp::new(diff)),
        SelectedGame::TicTacToe => Screen::TicTacToe(TicTacToeApp::new()),
        SelectedGame::Game2048 => Screen::Game2048(App2048::new()),
    }
}
