pub mod board;
pub mod input;
pub mod render;

use std::time::Instant;

use crate::Difficulty;
use crate::state::{ClaudeState, ClaudeStatus};
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

    // Timer tracking
    pub timer_start: Option<Instant>, // set on first actual reveal
    paused_at: Option<Instant>,       // Some while PermissionNeeded is active
    paused_secs: u64,                 // accumulated pause time
    pub elapsed_secs: u64,            // frozen final value when game ends

    // Leaderboard overlay
    pub show_leaderboard: bool,
    pub leaderboard_cache: Vec<crate::stats::GameRecord>,
    pub record_saved: bool, // guard: save only once per game
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

            timer_start: None,
            paused_at: None,
            paused_secs: 0,
            elapsed_secs: 0,

            show_leaderboard: false,
            leaderboard_cache: vec![],
            record_saved: false,
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
                if newly_revealed > 0 {
                    // Start timer on the first successful reveal
                    if self.timer_start.is_none() {
                        self.timer_start = Some(Instant::now());
                    }
                    if self.claude_state.status != ClaudeStatus::PermissionNeeded {
                        self.score += newly_revealed * self.difficulty.score_multiplier();
                    }
                }
            }
            input::Action::Flag => self.board.toggle_flag(self.cursor.0, self.cursor.1),
            input::Action::Restart => self.restart(),
            input::Action::ToggleLeaderboard => {
                self.show_leaderboard = !self.show_leaderboard;
                if self.show_leaderboard {
                    self.leaderboard_cache = crate::stats::leaderboard_top(10);
                }
            }
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

        if prev_status != ClaudeStatus::PermissionNeeded
            && self.claude_state.status == ClaudeStatus::PermissionNeeded
        {
            // Transitioning INTO PermissionNeeded — start pausing
            if self.timer_start.is_some() {
                self.paused_at = Some(Instant::now());
            }
        }

        if prev_status == ClaudeStatus::PermissionNeeded
            && self.claude_state.status != ClaudeStatus::PermissionNeeded
        {
            // Transitioning OUT of PermissionNeeded — accumulate pause duration
            if let Some(pa) = self.paused_at.take() {
                self.paused_secs += pa.elapsed().as_secs();
            }
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

        if self
            .done_since
            .is_some_and(|since| now.duration_since(since).as_secs() >= 3)
        {
            self.done_since = None;
        }

        if self.board.state == GameState::Won && self.claude_state.status == ClaudeStatus::Working {
            self.score += self.board.flag_bonus() * self.difficulty.score_multiplier();
        }

        // Save record once when the game ends
        if self.board.state != GameState::Playing
            && !self.record_saved
            && self.timer_start.is_some()
        {
            self.elapsed_secs = self.current_elapsed_secs(now);
            let (_, _, mine_count) = self.difficulty.board_params();
            let record = crate::stats::GameRecord {
                difficulty: format!("{:?}", self.difficulty).to_lowercase(),
                score: self.score,
                time_secs: self.elapsed_secs,
                won: self.board.state == GameState::Won,
                timestamp: crate::stats::current_timestamp(),
                board_width: self.board.width,
                board_height: self.board.height,
                mine_count,
            };
            let _ = crate::stats::append_record(record);
            self.record_saved = true;
            if self.show_leaderboard {
                self.leaderboard_cache = crate::stats::leaderboard_top(10);
            }
        }
    }

    /// Elapsed game time in seconds, accounting for pauses.
    /// Returns 0 if the timer has not started.
    /// Returns the frozen value if the game is over.
    fn current_elapsed_secs(&self, now: Instant) -> u64 {
        let Some(start) = self.timer_start else {
            return 0;
        };
        if self.board.state != GameState::Playing {
            return self.elapsed_secs;
        }
        let raw = now.duration_since(start).as_secs();
        let currently_pausing = self
            .paused_at
            .map(|pa| now.duration_since(pa).as_secs())
            .unwrap_or(0);
        raw.saturating_sub(self.paused_secs + currently_pausing)
    }

    pub fn elapsed_display(&self, now: Instant) -> String {
        let s = self.current_elapsed_secs(now);
        format!("{:02}:{:02}", s / 60, s % 60)
    }

    fn restart(&mut self) {
        let (w, h, m) = self.difficulty.board_params();
        self.board = Board::new(w, h, m);
        self.cursor = (w / 2, h / 2);
        self.viewport = (0, 0);
        self.score = 0;
        self.permission_alerted = false;

        // Reset timer
        self.timer_start = None;
        self.paused_at = None;
        self.paused_secs = 0;
        self.elapsed_secs = 0;
        self.record_saved = false;
    }
}
