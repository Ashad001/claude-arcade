pub mod render;

use std::time::Instant;

use crossterm::event::{KeyCode, KeyEvent, KeyEventKind};
use rand::prelude::IndexedRandom;
use rand::rng;

use crate::state::{ClaudeState, ClaudeStatus};

// ─── Game state ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum GameState {
    Playing,
    Won,      // reached 2048
    Continued, // kept playing after 2048
    Lost,
}

// ─── Board helpers ───────────────────────────────────────────────────────────

fn slide_row_left(row: &mut [u32; 4]) -> (bool, u32) {
    let mut changed = false;
    let mut score_gain = 0u32;

    // 1. Compact non-zeros to the left
    let mut write = 0usize;
    for read in 0..4 {
        if row[read] != 0 {
            if write != read {
                row[write] = row[read];
                row[read] = 0;
                changed = true;
            }
            write += 1;
        }
    }

    // 2. Merge adjacent equals
    for i in 0..3 {
        if row[i] != 0 && row[i] == row[i + 1] {
            row[i] *= 2;
            score_gain += row[i];
            row[i + 1] = 0;
            changed = true;
        }
    }

    // 3. Compact again after merges
    let mut write = 0usize;
    for read in 0..4 {
        if row[read] != 0 {
            if write != read {
                row[write] = row[read];
                row[read] = 0;
            }
            write += 1;
        }
    }

    (changed, score_gain)
}

fn slide_left(board: &mut [[u32; 4]; 4]) -> (bool, u32) {
    let mut any = false;
    let mut score = 0;
    for row in board.iter_mut() {
        let (c, s) = slide_row_left(row);
        any |= c;
        score += s;
    }
    (any, score)
}

fn slide_right(board: &mut [[u32; 4]; 4]) -> (bool, u32) {
    for row in board.iter_mut() {
        row.reverse();
    }
    let r = slide_left(board);
    for row in board.iter_mut() {
        row.reverse();
    }
    r
}

#[allow(clippy::needless_range_loop)]
fn transpose(board: &mut [[u32; 4]; 4]) {
    // We need both i and j to index board[i][j] and board[j][i] simultaneously.
    for i in 0..4 {
        for j in i + 1..4 {
            let tmp = board[i][j];
            board[i][j] = board[j][i];
            board[j][i] = tmp;
        }
    }
}

fn slide_up(board: &mut [[u32; 4]; 4]) -> (bool, u32) {
    transpose(board);
    let r = slide_left(board);
    transpose(board);
    r
}

fn slide_down(board: &mut [[u32; 4]; 4]) -> (bool, u32) {
    transpose(board);
    let r = slide_right(board);
    transpose(board);
    r
}

fn has_moves(board: &[[u32; 4]; 4]) -> bool {
    // Any empty cell?
    for row in board {
        for &cell in row {
            if cell == 0 {
                return true;
            }
        }
    }
    // Any adjacent equal pair?
    for r in 0..4 {
        for c in 0..4 {
            if c + 1 < 4 && board[r][c] == board[r][c + 1] {
                return true;
            }
            if r + 1 < 4 && board[r][c] == board[r + 1][c] {
                return true;
            }
        }
    }
    false
}

fn has_2048(board: &[[u32; 4]; 4]) -> bool {
    board.iter().any(|row| row.iter().any(|&c| c >= 2048))
}

fn spawn_tile(board: &mut [[u32; 4]; 4]) {
    let empties: Vec<(usize, usize)> = (0..4)
        .flat_map(|r| (0..4).map(move |c| (r, c)))
        .filter(|&(r, c)| board[r][c] == 0)
        .collect();

    let mut rng = rng();
    if let Some(&(r, c)) = empties.choose(&mut rng) {
        // 90% chance of 2, 10% chance of 4
        board[r][c] = if rand::random::<f32>() < 0.9 { 2 } else { 4 };
    }
}

// ─── App ─────────────────────────────────────────────────────────────────────

pub struct App {
    pub board: [[u32; 4]; 4],
    pub score: u32,
    pub best_score: u32,
    pub state: GameState,
    pub claude_state: ClaudeState,
    pub should_quit: bool,
    pub back_to_menu: bool,
    pub flash_on: bool,
    flash_tick: u8,
    pub permission_alerted: bool,
    pub done_since: Option<Instant>,
    pub record_saved: bool,
    pub timer_start: Option<Instant>,
    pub elapsed_secs: u64,
}

impl App {
    pub fn new() -> Self {
        let mut board = [[0u32; 4]; 4];
        spawn_tile(&mut board);
        spawn_tile(&mut board);
        Self {
            board,
            score: 0,
            best_score: 0,
            state: GameState::Playing,
            claude_state: ClaudeState::default(),
            should_quit: false,
            back_to_menu: false,
            flash_on: true,
            flash_tick: 0,
            permission_alerted: false,
            done_since: None,
            record_saved: false,
            timer_start: None,
            elapsed_secs: 0,
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) {
        if key.kind == KeyEventKind::Release {
            return;
        }

        // Permission lock — only menu/quit
        if self.claude_state.status == ClaudeStatus::PermissionNeeded {
            match key.code {
                KeyCode::Char('q') => self.should_quit = true,
                KeyCode::Esc | KeyCode::Char('m') => self.back_to_menu = true,
                _ => {}
            }
            return;
        }

        match key.code {
            KeyCode::Char('q') => self.should_quit = true,
            KeyCode::Esc | KeyCode::Char('m') => self.back_to_menu = true,
            KeyCode::Char('r') if key.kind == KeyEventKind::Press => self.restart(),

            // Continue after winning
            KeyCode::Char('c') if self.state == GameState::Won => {
                self.state = GameState::Continued;
            }

            // Slides — only when still playing
            _ if matches!(
                self.state,
                GameState::Playing | GameState::Continued
            ) =>
            {
                let moved = match key.code {
                    KeyCode::Up | KeyCode::Char('k') => Some(slide_up(&mut self.board)),
                    KeyCode::Down | KeyCode::Char('j') => Some(slide_down(&mut self.board)),
                    KeyCode::Left | KeyCode::Char('h') => Some(slide_left(&mut self.board)),
                    KeyCode::Right | KeyCode::Char('l') => Some(slide_right(&mut self.board)),
                    _ => None,
                };

                if let Some((changed, gain)) = moved
                    && changed
                {
                    if self.timer_start.is_none() {
                        self.timer_start = Some(Instant::now());
                    }
                    self.score += gain;
                    self.best_score = self.best_score.max(self.score);
                    spawn_tile(&mut self.board);

                    // Check win/loss
                    if self.state == GameState::Playing && has_2048(&self.board) {
                        self.state = GameState::Won;
                    } else if !has_moves(&self.board) {
                        self.state = GameState::Lost;
                    }
                }
            }
            _ => {}
        }
    }

    fn restart(&mut self) {
        self.board = [[0; 4]; 4];
        spawn_tile(&mut self.board);
        spawn_tile(&mut self.board);
        self.score = 0;
        self.state = GameState::Playing;
        self.record_saved = false;
        self.timer_start = None;
        self.elapsed_secs = 0;
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

        // Save record once when game ends
        if matches!(self.state, GameState::Won | GameState::Lost)
            && !self.record_saved
            && self.timer_start.is_some()
        {
            let elapsed = self
                .timer_start
                .map(|s| now.duration_since(s).as_secs())
                .unwrap_or(0);
            self.elapsed_secs = elapsed;
            let record = crate::stats::GameRecord {
                game: "2048".into(),
                difficulty: "classic".into(),
                score: self.score,
                time_secs: elapsed,
                won: self.state == GameState::Won,
                timestamp: crate::stats::current_timestamp(),
                board_width: 4,
                board_height: 4,
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
                if matches!(self.state, GameState::Won | GameState::Lost) {
                    self.elapsed_secs
                } else {
                    now.duration_since(start).as_secs()
                }
            })
            .unwrap_or(0);
        format!("{:02}:{:02}", s / 60, s % 60)
    }

    /// Highest tile on the board (for display in header)
    pub fn max_tile(&self) -> u32 {
        self.board.iter().flat_map(|r| r.iter()).copied().max().unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slide_merges_and_scores() {
        let mut row = [2, 2, 0, 0];
        let (changed, score) = slide_row_left(&mut row);
        assert!(changed);
        assert_eq!(row, [4, 0, 0, 0]);
        assert_eq!(score, 4);
    }

    #[test]
    fn slide_no_double_merge() {
        // [2,2,2,2] → [4,4,0,0], not [8,0,0,0]
        let mut row = [2, 2, 2, 2];
        let (changed, score) = slide_row_left(&mut row);
        assert!(changed);
        assert_eq!(row, [4, 4, 0, 0]);
        assert_eq!(score, 8);
    }

    #[test]
    fn slide_right_works() {
        let mut board = [[2, 2, 0, 0]; 4];
        let (changed, _) = slide_right(&mut board);
        assert!(changed);
        for row in &board {
            assert_eq!(row[3], 4, "merged tile should be on the right");
            assert_eq!(row[0], 0);
        }
    }

    #[test]
    fn slide_up_works() {
        let mut board = [[0u32; 4]; 4];
        board[0][0] = 2;
        board[1][0] = 2;
        let (changed, score) = slide_up(&mut board);
        assert!(changed);
        assert_eq!(board[0][0], 4);
        assert_eq!(board[1][0], 0);
        assert_eq!(score, 4);
    }

    #[test]
    fn has_moves_detects_empty() {
        let board = [[0u32; 4]; 4];
        assert!(has_moves(&board));
    }

    #[test]
    fn has_moves_false_when_locked() {
        // Fill with alternating 2/4 so no moves are possible
        let board = [
            [2, 4, 2, 4],
            [4, 2, 4, 2],
            [2, 4, 2, 4],
            [4, 2, 4, 2],
        ];
        assert!(!has_moves(&board));
    }
}
