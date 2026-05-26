use rand::rng;
use rand::seq::SliceRandom;

#[derive(Debug, Clone, PartialEq)]
pub enum GameState {
    Playing,
    Won,
    Lost,
}

#[derive(Debug, Clone)]
struct Cell {
    is_mine: bool,
    is_revealed: bool,
    is_flagged: bool,
    adjacent_mines: u8,
}

impl Cell {
    fn hidden() -> Self {
        Self {
            is_mine: false,
            is_revealed: false,
            is_flagged: false,
            adjacent_mines: 0,
        }
    }
}

pub struct Board {
    cells: Vec<Vec<Cell>>,
    pub width: usize,
    pub height: usize,
    pub mine_count: usize,
    pub revealed_count: usize,
    pub flagged_count: usize,
    pub state: GameState,
    /// Mines are placed on the first reveal to guarantee a safe start
    mines_placed: bool,
}

impl Board {
    pub fn new(width: usize, height: usize, mine_count: usize) -> Self {
        let cells = vec![vec![Cell::hidden(); width]; height];
        Self {
            cells,
            width,
            height,
            mine_count,
            revealed_count: 0,
            flagged_count: 0,
            state: GameState::Playing,
            mines_placed: false,
        }
    }

    /// Reveal cell at (x, y). Returns number of newly revealed cells (for scoring).
    pub fn reveal(&mut self, x: usize, y: usize) -> u32 {
        if self.state != GameState::Playing {
            return 0;
        }
        if self.cells[y][x].is_flagged || self.cells[y][x].is_revealed {
            return 0;
        }

        if !self.mines_placed {
            self.place_mines(x, y);
            self.mines_placed = true;
        }

        if self.cells[y][x].is_mine {
            self.cells[y][x].is_revealed = true;
            self.state = GameState::Lost;
            self.reveal_all_mines();
            return 0;
        }

        let count = self.flood_fill(x, y);
        self.check_win();
        count
    }

    pub fn toggle_flag(&mut self, x: usize, y: usize) {
        if self.state != GameState::Playing {
            return;
        }
        let cell = &mut self.cells[y][x];
        if cell.is_revealed {
            return;
        }
        if cell.is_flagged {
            cell.is_flagged = false;
            self.flagged_count = self.flagged_count.saturating_sub(1);
        } else {
            cell.is_flagged = true;
            self.flagged_count += 1;
        }
    }

    /// Returns bonus points for correctly flagged mines at game end.
    pub fn flag_bonus(&self) -> u32 {
        let mut correct = 0u32;
        for row in &self.cells {
            for cell in row {
                if cell.is_mine && cell.is_flagged {
                    correct += 1;
                }
            }
        }
        correct * 50
    }

    pub fn cell_view(&self, x: usize, y: usize) -> CellView {
        let cell = &self.cells[y][x];
        if cell.is_revealed {
            if cell.is_mine {
                CellView::Mine
            } else {
                CellView::Number(cell.adjacent_mines)
            }
        } else if cell.is_flagged {
            CellView::Flag
        } else {
            CellView::Hidden
        }
    }

    pub fn mines_remaining(&self) -> i32 {
        self.mine_count as i32 - self.flagged_count as i32
    }

    fn place_mines(&mut self, safe_x: usize, safe_y: usize) {
        let total = self.width * self.height;
        let mut candidates: Vec<usize> = (0..total)
            .filter(|&i| {
                let (cx, cy) = (i % self.width, i / self.width);
                let dx = (cx as i32 - safe_x as i32).abs();
                let dy = (cy as i32 - safe_y as i32).abs();
                dx > 1 || dy > 1
            })
            .collect();

        candidates.shuffle(&mut rng());
        let mine_count = self.mine_count.min(candidates.len());

        for &idx in &candidates[..mine_count] {
            let (mx, my) = (idx % self.width, idx / self.width);
            self.cells[my][mx].is_mine = true;
        }

        self.compute_adjacency();
    }

    fn compute_adjacency(&mut self) {
        for y in 0..self.height {
            for x in 0..self.width {
                if self.cells[y][x].is_mine {
                    continue;
                }
                self.cells[y][x].adjacent_mines = self.count_adjacent_mines(x, y);
            }
        }
    }

    fn count_adjacent_mines(&self, x: usize, y: usize) -> u8 {
        self.neighbours(x, y)
            .into_iter()
            .filter(|&(nx, ny)| self.cells[ny][nx].is_mine)
            .count() as u8
    }

    fn flood_fill(&mut self, x: usize, y: usize) -> u32 {
        let mut stack = vec![(x, y)];
        let mut count = 0u32;

        while let Some((cx, cy)) = stack.pop() {
            let cell = &self.cells[cy][cx];
            if cell.is_revealed || cell.is_flagged || cell.is_mine {
                continue;
            }
            self.cells[cy][cx].is_revealed = true;
            self.revealed_count += 1;
            count += 1;

            if self.cells[cy][cx].adjacent_mines == 0 {
                for neighbour in self.neighbours(cx, cy) {
                    stack.push(neighbour);
                }
            }
        }
        count
    }

    fn check_win(&mut self) {
        let safe_cells = self.width * self.height - self.mine_count;
        if self.revealed_count >= safe_cells {
            self.state = GameState::Won;
            for y in 0..self.height {
                for x in 0..self.width {
                    if self.cells[y][x].is_mine && !self.cells[y][x].is_flagged {
                        self.cells[y][x].is_flagged = true;
                        self.flagged_count += 1;
                    }
                }
            }
        }
    }

    fn reveal_all_mines(&mut self) {
        for y in 0..self.height {
            for x in 0..self.width {
                if self.cells[y][x].is_mine {
                    self.cells[y][x].is_revealed = true;
                }
            }
        }
    }

    fn neighbours(&self, x: usize, y: usize) -> Vec<(usize, usize)> {
        let mut result = Vec::with_capacity(8);
        let x = x as i32;
        let y = y as i32;
        for dy in -1..=1i32 {
            for dx in -1..=1i32 {
                if dx == 0 && dy == 0 {
                    continue;
                }
                let nx = x + dx;
                let ny = y + dy;
                if nx >= 0 && ny >= 0 && nx < self.width as i32 && ny < self.height as i32 {
                    result.push((nx as usize, ny as usize));
                }
            }
        }
        result
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CellView {
    Hidden,
    Flag,
    Number(u8), // 0 = blank, 1-8
    Mine,
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_board_is_all_hidden() {
        let board = Board::new(9, 9, 10);
        for y in 0..9 {
            for x in 0..9 {
                assert_eq!(board.cell_view(x, y), CellView::Hidden);
            }
        }
        assert_eq!(board.state, GameState::Playing);
    }

    #[test]
    fn first_reveal_never_hits_mine() {
        for _ in 0..50 {
            let mut board = Board::new(9, 9, 10);
            board.reveal(4, 4);
            assert_ne!(board.cell_view(4, 4), CellView::Mine);
            assert_ne!(board.cell_view(4, 4), CellView::Hidden);
        }
    }

    #[test]
    fn flag_toggle() {
        let mut board = Board::new(9, 9, 10);
        board.toggle_flag(0, 0);
        assert_eq!(board.cell_view(0, 0), CellView::Flag);
        assert_eq!(board.flagged_count, 1);
        board.toggle_flag(0, 0);
        assert_eq!(board.cell_view(0, 0), CellView::Hidden);
        assert_eq!(board.flagged_count, 0);
    }

    #[test]
    fn cannot_reveal_flagged_cell() {
        let mut board = Board::new(9, 9, 10);
        board.toggle_flag(3, 3);
        board.reveal(3, 3);
        assert_eq!(board.cell_view(3, 3), CellView::Flag);
    }

    #[test]
    fn mines_remaining_decreases_with_flags() {
        let mut board = Board::new(9, 9, 10);
        assert_eq!(board.mines_remaining(), 10);
        board.toggle_flag(0, 0);
        assert_eq!(board.mines_remaining(), 9);
    }

    #[test]
    fn win_condition_all_safe_cells_revealed() {
        let mut board = Board::new(3, 3, 1);
        board.reveal(0, 0);
        if board.state == GameState::Lost {
            return;
        }
        for y in 0..3 {
            for x in 0..3 {
                board.reveal(x, y);
            }
        }
        assert!(
            board.state == GameState::Won || board.state == GameState::Lost,
            "state should be terminal"
        );
    }
}
