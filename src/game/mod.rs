pub mod board;
pub mod input;
pub mod render;

use std::time::Instant;

use crate::state::{ClaudeState, ClaudeStatus};
use crate::Difficulty;
use board::{Board, GameState};

pub struct App {
    pub board: Board,
    pub cursor: (usize, usize),
    /// Top-left cell of the visible viewport (follows cursor)
    pub viewport: (usize, usize),
    pub score: u32,
    pub difficulty: Difficulty,
    pub claude_state: ClaudeState,
    pub should_quit: bool,
    pub permission_alerted: bool,
    pub done_since: Option<Instant>,
    pub flash_on: bool,
    flash_tick: u8,
}

impl App {
    pub fn new(difficulty: Difficulty) -> Self {
        let (w, h, m) = difficulty.board_params();
        Self {
            board: Board::new(w, h, m),
            cursor: (w / 2, h / 2),
            viewport: (0, 0),
            score: 0,
            difficulty,
            claude_state: ClaudeState::default(),
            should_quit: false,
            permission_alerted: false,
            done_since: None,
            flash_on: true,
            flash_tick: 0,
        }
    }

    pub fn handle_action(&mut self, action: input::Action) {
        if self.claude_state.status == ClaudeStatus::PermissionNeeded {
            if action == input::Action::Quit {
                self.should_quit = true;
            }
            return;
        }

        let (w, h, _) = self.difficulty.board_params();

        match action {
            input::Action::MoveUp => self.cursor.1 = self.cursor.1.saturating_sub(1),
            input::Action::MoveDown => self.cursor.1 = (self.cursor.1 + 1).min(h - 1),
            input::Action::MoveLeft => self.cursor.0 = self.cursor.0.saturating_sub(1),
            input::Action::MoveRight => self.cursor.0 = (self.cursor.0 + 1).min(w - 1),
            input::Action::Reveal => {
                let newly_revealed = self.board.reveal(self.cursor.0, self.cursor.1);
                if self.claude_state.status != ClaudeStatus::PermissionNeeded {
                    self.score += newly_revealed * self.difficulty.score_multiplier();
                }
            }
            input::Action::Flag => self.board.toggle_flag(self.cursor.0, self.cursor.1),
            input::Action::Restart => self.restart(),
            input::Action::Quit => self.should_quit = true,
        }
    }

    /// Called by the renderer with how many cells are visible.
    /// Uses edge-only (lazy) scrolling: the viewport only moves when the cursor
    /// actually exits the visible area. This ensures every keypress moves the
    /// cursor by exactly 1 visual cell — no phantom "skip" frames.
    pub fn scroll_to_cursor(&mut self, visible_cols: usize, visible_rows: usize) {
        let (cx, cy) = self.cursor;

        // Horizontal — only scroll if board is wider than the pane
        if self.board.width <= visible_cols {
            self.viewport.0 = 0;
        } else {
            let vx = &mut self.viewport.0;
            if cx < *vx {
                *vx = cx;
            } else if cx >= *vx + visible_cols {
                *vx = cx + 1 - visible_cols;
            }
            *vx = (*vx).min(self.board.width.saturating_sub(visible_cols));
        }

        // Vertical — only scroll if board is taller than the pane
        if self.board.height <= visible_rows {
            self.viewport.1 = 0;
        } else {
            let vy = &mut self.viewport.1;
            if cy < *vy {
                *vy = cy;
            } else if cy >= *vy + visible_rows {
                *vy = cy + 1 - visible_rows;
            }
            *vy = (*vy).min(self.board.height.saturating_sub(visible_rows));
        }
    }

    pub fn refresh_claude_state(&mut self, new_state: ClaudeState) {
        let prev_status = self.claude_state.status.clone();
        self.claude_state = new_state;

        if prev_status == ClaudeStatus::PermissionNeeded
            && self.claude_state.status != ClaudeStatus::PermissionNeeded
        {
            self.permission_alerted = false;
        }

        if prev_status != ClaudeStatus::Done && self.claude_state.status == ClaudeStatus::Done {
            self.done_since = Some(Instant::now());
        }
    }

    pub fn tick(&mut self, now: Instant) {
        self.flash_tick = self.flash_tick.wrapping_add(1);
        if self.flash_tick >= 5 {
            self.flash_tick = 0;
            self.flash_on = !self.flash_on;
        }

        if self.claude_state.status == ClaudeStatus::PermissionNeeded && !self.permission_alerted {
            self.permission_alerted = true;
            print!("\x07");
        }

        if let Some(since) = self.done_since {
            if now.duration_since(since).as_secs() >= 3 {
                self.done_since = None;
            }
        }

        if self.board.state == GameState::Won
            && self.claude_state.status == ClaudeStatus::Working
        {
            self.score += self.board.flag_bonus() * self.difficulty.score_multiplier();
        }
    }

    fn restart(&mut self) {
        let (w, h, m) = self.difficulty.board_params();
        self.board = Board::new(w, h, m);
        self.cursor = (w / 2, h / 2);
        self.viewport = (0, 0);
        self.score = 0;
        self.permission_alerted = false;
    }
}
