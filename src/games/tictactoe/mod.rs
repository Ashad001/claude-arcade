pub mod render;

use std::time::Instant;

use crossterm::event::{KeyCode, KeyEvent, KeyEventKind};

use crate::state::{ClaudeState, ClaudeStatus};

// ─── Types ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Mark {
    X, // Human
    O, // AI
}

#[derive(Debug, Clone, PartialEq)]
pub enum GameState {
    Playing,
    WonX,
    WonO,
    Draw,
}

// ─── Win detection ───────────────────────────────────────────────────────────

const WIN_LINES: [[usize; 3]; 8] = [
    [0, 1, 2],
    [3, 4, 5],
    [6, 7, 8], // rows
    [0, 3, 6],
    [1, 4, 7],
    [2, 5, 8], // cols
    [0, 4, 8],
    [2, 4, 6], // diagonals
];

fn check_winner(board: &[Option<Mark>; 9]) -> Option<Mark> {
    for &[a, b, c] in &WIN_LINES {
        if let (Some(m1), Some(m2), Some(m3)) = (board[a], board[b], board[c])
            && m1 == m2
            && m2 == m3
        {
            return Some(m1);
        }
    }
    None
}

fn is_full(board: &[Option<Mark>; 9]) -> bool {
    board.iter().all(|c| c.is_some())
}

// ─── Minimax AI ──────────────────────────────────────────────────────────────

/// Returns a score relative to the AI (O).
/// Positive = AI advantage, negative = human advantage.
/// Depth penalty ensures AI prefers faster wins.
fn minimax(board: &[Option<Mark>; 9], is_ai_turn: bool, depth: i32) -> i32 {
    match check_winner(board) {
        Some(Mark::O) => return 10 - depth, // AI wins — faster is better
        Some(Mark::X) => return depth - 10, // Human wins
        None => {}
    }
    if is_full(board) {
        return 0;
    }

    if is_ai_turn {
        let mut best = i32::MIN;
        for i in 0..9 {
            if board[i].is_none() {
                let mut b = *board;
                b[i] = Some(Mark::O);
                best = best.max(minimax(&b, false, depth + 1));
            }
        }
        best
    } else {
        let mut best = i32::MAX;
        for i in 0..9 {
            if board[i].is_none() {
                let mut b = *board;
                b[i] = Some(Mark::X);
                best = best.min(minimax(&b, true, depth + 1));
            }
        }
        best
    }
}

fn best_ai_move(board: &[Option<Mark>; 9]) -> Option<usize> {
    let mut best_val = i32::MIN;
    let mut best_move = None;
    for i in 0..9 {
        if board[i].is_none() {
            let mut b = *board;
            b[i] = Some(Mark::O);
            let val = minimax(&b, false, 0);
            if val > best_val {
                best_val = val;
                best_move = Some(i);
            }
        }
    }
    best_move
}

// ─── App ─────────────────────────────────────────────────────────────────────

pub struct App {
    pub board: [Option<Mark>; 9],
    /// Cursor position as a cell index 0-8
    pub cursor: usize,
    pub state: GameState,
    /// Which cells form the winning line (for highlight)
    pub winning_line: Option<[usize; 3]>,
    pub score: u32, // accumulated across restarts
    pub games_played: u32,
    pub claude_state: ClaudeState,
    pub should_quit: bool,
    pub back_to_menu: bool,
    pub flash_on: bool,
    flash_tick: u8,
    pub permission_alerted: bool,
    pub done_since: Option<Instant>,
    pub ai_thinking: bool, // brief visual delay before AI plays
    ai_think_tick: u8,
    pub record_saved: bool,
    pub timer_start: Option<Instant>,
    pub elapsed_secs: u64,
}

impl App {
    pub fn new() -> Self {
        Self {
            board: [None; 9],
            cursor: 4, // centre cell
            state: GameState::Playing,
            winning_line: None,
            score: 0,
            games_played: 0,
            claude_state: ClaudeState::default(),
            should_quit: false,
            back_to_menu: false,
            flash_on: true,
            flash_tick: 0,
            permission_alerted: false,
            done_since: None,
            ai_thinking: false,
            ai_think_tick: 0,
            record_saved: false,
            timer_start: None,
            elapsed_secs: 0,
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) {
        if key.kind == KeyEventKind::Release {
            return;
        }

        // Permission lock — only allow menu/quit
        if self.claude_state.status == ClaudeStatus::PermissionNeeded {
            match key.code {
                KeyCode::Char('q') => self.should_quit = true,
                KeyCode::Esc | KeyCode::Char('m') => self.back_to_menu = true,
                _ => {}
            }
            return;
        }

        // Ignore input while AI is "thinking"
        if self.ai_thinking {
            return;
        }

        match key.code {
            KeyCode::Char('q') => self.should_quit = true,
            KeyCode::Esc | KeyCode::Char('m') => self.back_to_menu = true,
            KeyCode::Char('r') if key.kind == KeyEventKind::Press => self.restart(),

            // Movement
            KeyCode::Up | KeyCode::Char('k') => {
                if self.cursor >= 3 {
                    self.cursor -= 3;
                }
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.cursor < 6 {
                    self.cursor += 3;
                }
            }
            KeyCode::Left | KeyCode::Char('h') => {
                if !self.cursor.is_multiple_of(3) {
                    self.cursor -= 1;
                }
            }
            KeyCode::Right | KeyCode::Char('l') => {
                if self.cursor % 3 < 2 {
                    self.cursor += 1;
                }
            }

            // Place X
            KeyCode::Enter | KeyCode::Char(' ') if key.kind == KeyEventKind::Press => {
                self.try_place();
            }

            _ => {}
        }
    }

    fn try_place(&mut self) {
        if self.state != GameState::Playing {
            return;
        }
        if self.board[self.cursor].is_some() {
            return;
        }

        // Start timer on first move
        if self.timer_start.is_none() {
            self.timer_start = Some(Instant::now());
        }

        self.board[self.cursor] = Some(Mark::X);

        match check_winner(&self.board) {
            Some(Mark::X) => {
                self.state = GameState::WonX;
                self.winning_line = self.find_winning_line();
                self.score += 100;
            }
            _ if is_full(&self.board) => {
                self.state = GameState::Draw;
                self.score += 30;
            }
            _ => {
                // AI's turn — set thinking flag for a brief visual delay
                self.ai_thinking = true;
                self.ai_think_tick = 0;
            }
        }
    }

    fn ai_move(&mut self) {
        if let Some(idx) = best_ai_move(&self.board) {
            self.board[idx] = Some(Mark::O);

            match check_winner(&self.board) {
                Some(Mark::O) => {
                    self.state = GameState::WonO;
                    self.winning_line = self.find_winning_line();
                }
                _ if is_full(&self.board) => {
                    self.state = GameState::Draw;
                    self.score += 30;
                }
                _ => {}
            }
        }
    }

    fn find_winning_line(&self) -> Option<[usize; 3]> {
        for &line in &WIN_LINES {
            let [a, b, c] = line;
            if let (Some(m1), Some(m2), Some(m3)) = (self.board[a], self.board[b], self.board[c])
                && m1 == m2
                && m2 == m3
            {
                return Some(line);
            }
        }
        None
    }

    fn restart(&mut self) {
        self.board = [None; 9];
        self.cursor = 4;
        self.state = GameState::Playing;
        self.winning_line = None;
        self.ai_thinking = false;
        self.ai_think_tick = 0;
        self.record_saved = false;
        self.timer_start = None;
        self.elapsed_secs = 0;
        self.games_played += 1;
    }

    pub fn refresh_claude_state(&mut self, new_state: ClaudeState) {
        let prev = self.claude_state.status.clone();
        self.claude_state = new_state;

        if prev != ClaudeStatus::PermissionNeeded
            && self.claude_state.status == ClaudeStatus::PermissionNeeded
        {
            self.permission_alerted = false;
        }
        if prev != ClaudeStatus::Done && self.claude_state.status == ClaudeStatus::Done {
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
            .is_some_and(|s| now.duration_since(s).as_secs() >= 3)
        {
            self.done_since = None;
        }

        // Brief AI "thinking" delay (3 ticks ≈ 300ms) for UX feel
        if self.ai_thinking {
            self.ai_think_tick += 1;
            if self.ai_think_tick >= 3 {
                self.ai_thinking = false;
                self.ai_think_tick = 0;
                self.ai_move();
            }
        }

        // Save record once per game
        if self.state != GameState::Playing && !self.record_saved && self.timer_start.is_some() {
            let elapsed = self
                .timer_start
                .map(|s| now.duration_since(s).as_secs())
                .unwrap_or(0);
            self.elapsed_secs = elapsed;
            let record = crate::stats::GameRecord {
                game: "tictactoe".into(),
                difficulty: "ai".into(),
                score: self.score,
                time_secs: elapsed,
                won: self.state == GameState::WonX,
                timestamp: crate::stats::current_timestamp(),
                board_width: 3,
                board_height: 3,
                mine_count: 0,
            };
            let _ = crate::stats::append_record(record);
            self.record_saved = true;
        }
    }

    pub fn elapsed_display(&self, now: Instant) -> String {
        let s = self
            .timer_start
            .map(|start| {
                if self.state != GameState::Playing {
                    self.elapsed_secs
                } else {
                    now.duration_since(start).as_secs()
                }
            })
            .unwrap_or(0);
        format!("{:02}:{:02}", s / 60, s % 60)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn minimax_blocks_human_win() {
        // Human X in [0,1] — AI should play [2] to block
        let mut board = [None; 9];
        board[0] = Some(Mark::X);
        board[1] = Some(Mark::X);
        let ai = best_ai_move(&board);
        assert_eq!(ai, Some(2), "AI must block [0,1,2] win");
    }

    #[test]
    fn minimax_takes_winning_move() {
        // AI O in [3,4] — AI should complete [3,4,5]
        let mut board = [None; 9];
        board[3] = Some(Mark::O);
        board[4] = Some(Mark::O);
        let ai = best_ai_move(&board);
        assert_eq!(ai, Some(5), "AI must take winning move [3,4,5]");
    }

    #[test]
    fn check_winner_detects_row() {
        let mut board = [None; 9];
        board[0] = Some(Mark::X);
        board[1] = Some(Mark::X);
        board[2] = Some(Mark::X);
        assert_eq!(check_winner(&board), Some(Mark::X));
    }

    #[test]
    fn check_winner_detects_diagonal() {
        let mut board = [None; 9];
        board[2] = Some(Mark::O);
        board[4] = Some(Mark::O);
        board[6] = Some(Mark::O);
        assert_eq!(check_winner(&board), Some(Mark::O));
    }

    #[test]
    fn draw_when_full_no_winner() {
        // X O X / O X O / O X O — no three in a row
        let board = [
            Some(Mark::X),
            Some(Mark::O),
            Some(Mark::X),
            Some(Mark::O),
            Some(Mark::X),
            Some(Mark::O),
            Some(Mark::O),
            Some(Mark::X),
            Some(Mark::O),
        ];
        assert_eq!(check_winner(&board), None);
        assert!(is_full(&board));
    }
}
