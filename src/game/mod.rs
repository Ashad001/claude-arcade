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
    pub score: u32,
    pub difficulty: Difficulty,
    pub claude_state: ClaudeState,
    pub should_quit: bool,
    /// Tracks when permission_needed was first seen (for bell)
    pub permission_alerted: bool,
    /// When `done` state started (for 3-second green border)
    pub done_since: Option<Instant>,
    /// Flash phase for red border (true = red, false = dim)
    pub flash_on: bool,
    /// Counts 100ms ticks for flash toggle
    flash_tick: u8,
}

impl App {
    pub fn new(difficulty: Difficulty) -> Self {
        let (w, h, m) = difficulty.board_params();
        Self {
            board: Board::new(w, h, m),
            cursor: (w / 2, h / 2),
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
        // Freeze input during permission_needed (game pauses)
        if self.claude_state.status == ClaudeStatus::PermissionNeeded {
            if action == input::Action::Quit {
                self.should_quit = true;
            }
            return;
        }

        let (w, h, _) = self.difficulty.board_params();

        match action {
            input::Action::MoveUp => self.cursor.1 = self.cursor.1.saturating_sub(1),
            input::Action::MoveDown => {
                self.cursor.1 = (self.cursor.1 + 1).min(h.saturating_sub(1))
            }
            input::Action::MoveLeft => self.cursor.0 = self.cursor.0.saturating_sub(1),
            input::Action::MoveRight => {
                self.cursor.0 = (self.cursor.0 + 1).min(w.saturating_sub(1))
            }
            input::Action::Reveal => {
                let newly_revealed = self.board.reveal(self.cursor.0, self.cursor.1);
                if self.claude_state.status != ClaudeStatus::PermissionNeeded {
                    self.score += newly_revealed * self.difficulty.score_multiplier();
                }
            }
            input::Action::Flag => {
                self.board.toggle_flag(self.cursor.0, self.cursor.1);
            }
            input::Action::Restart => self.restart(),
            input::Action::Quit => self.should_quit = true,
        }
    }

    pub fn refresh_claude_state(&mut self, new_state: ClaudeState) {
        let prev_status = self.claude_state.status.clone();
        self.claude_state = new_state;

        // Reset alert flag when leaving permission_needed
        if prev_status == ClaudeStatus::PermissionNeeded
            && self.claude_state.status != ClaudeStatus::PermissionNeeded
        {
            self.permission_alerted = false;
        }

        // Track when done state starts
        if prev_status != ClaudeStatus::Done
            && self.claude_state.status == ClaudeStatus::Done
        {
            self.done_since = Some(Instant::now());
        }
    }

    pub fn tick(&mut self, now: Instant) {
        // Flash the border every 5 ticks (~500ms)
        self.flash_tick = self.flash_tick.wrapping_add(1);
        if self.flash_tick >= 5 {
            self.flash_tick = 0;
            self.flash_on = !self.flash_on;
        }

        // Ring bell once on first permission_needed tick
        if self.claude_state.status == ClaudeStatus::PermissionNeeded
            && !self.permission_alerted
        {
            self.permission_alerted = true;
            print!("\x07"); // BEL character
        }

        // Clear done state after 3 seconds
        if let Some(since) = self.done_since {
            if now.duration_since(since).as_secs() >= 3 {
                self.done_since = None;
            }
        }

        // Auto-restart after win/loss (optional: let user press 'r')
        if self.board.state == GameState::Won && self.claude_state.status == ClaudeStatus::Working
        {
            // Award win bonus then restart
            self.score += self.board.flag_bonus() * self.difficulty.score_multiplier();
        }
    }

    fn restart(&mut self) {
        let (w, h, m) = self.difficulty.board_params();
        self.board = Board::new(w, h, m);
        self.cursor = (w / 2, h / 2);
        self.permission_alerted = false;
    }
}
