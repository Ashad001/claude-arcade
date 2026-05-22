use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
};

use crate::game::board::{CellView, GameState};
use crate::game::App;
use crate::state::ClaudeStatus;

// ─── Colour palette ──────────────────────────────────────────────────────────

/// Hidden tile: light enough to pop against any dark terminal background
const HIDDEN_BG: Color = Color::Rgb(90, 100, 120);
const HIDDEN_FG: Color = Color::Rgb(130, 145, 170);

/// Revealed (pressed-in) tile — clearly darker than hidden
const REVEALED_BG: Color = Color::Rgb(28, 30, 38);

/// Cursor highlight
const CURSOR_BG: Color = Color::Rgb(230, 190, 0);
const CURSOR_FG: Color = Color::Black;

// ─── Entry point ─────────────────────────────────────────────────────────────

pub fn ui(frame: &mut Frame, app: &mut App) {
    let [header_area, board_area, footer_area] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Fill(1),
        Constraint::Length(1),
    ])
    .areas(frame.area());

    render_header(frame, app, header_area);
    render_board(frame, app, board_area);
    render_footer(frame, app, footer_area);
}

// ─── Header ──────────────────────────────────────────────────────────────────

fn render_header(frame: &mut Frame, app: &mut App, area: Rect) {
    let mines_left = app.board.mines_remaining();
    let (status_char, status_style) = match app.board.state {
        GameState::Playing => ("▶", Style::new().fg(Color::Cyan)),
        GameState::Won => ("★", Style::new().fg(Color::Green).bold()),
        GameState::Lost => ("✗", Style::new().fg(Color::Red).bold()),
    };
    let mine_style = if mines_left <= 3 {
        Style::new().fg(Color::Red).bold()
    } else {
        Style::new().fg(Color::Yellow)
    };
    let (cx, cy) = app.cursor;
    let header = Line::from(vec![
        Span::styled(" MINESWEEPER ", Style::new().bold()),
        Span::styled(status_char, status_style),
        Span::raw("  "),
        Span::styled("⚑ ", Style::new().fg(Color::Red)),
        Span::styled(format!("{:<3}", mines_left), mine_style),
        Span::raw("  "),
        Span::styled(format!("Score: {}", app.score), Style::new().fg(Color::Cyan)),
        Span::styled(
            format!("  [{},{}]", cx + 1, cy + 1),
            Style::new().fg(Color::DarkGray),
        ),
    ]);
    frame.render_widget(Paragraph::new(header), area);
}

// ─── Board ───────────────────────────────────────────────────────────────────

fn render_board(frame: &mut Frame, app: &mut App, area: Rect) {
    let (border_style, border_type, title) = border_config(app);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_style)
        .border_type(border_type)
        .title(title)
        .title_alignment(Alignment::Center);

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Each cell = 2 chars wide. How many columns fit?
    let visible_cols = ((inner.width as usize) / 2).min(app.board.width);
    let visible_rows = (inner.height as usize).min(app.board.height);

    // Update viewport so cursor stays on screen (no inserts into spans — kept clean)
    app.scroll_to_cursor(visible_cols, visible_rows);
    let (vx, vy) = app.viewport;

    // Vertical centering when board fits fully
    let y_pad = if app.board.height < visible_rows {
        (inner.height as usize).saturating_sub(app.board.height) / 2
    } else {
        0
    };

    for row in 0..visible_rows {
        let board_row = vy + row;
        if board_row >= app.board.height {
            break;
        }
        let y = inner.top() + (y_pad + row) as u16;
        if y >= inner.bottom() {
            break;
        }

        let mut spans: Vec<Span> = Vec::with_capacity(visible_cols);
        for col in 0..visible_cols {
            let board_col = vx + col;
            if board_col >= app.board.width {
                break;
            }
            spans.push(cell_span(
                app.board.cell_view(board_col, board_row),
                app.cursor == (board_col, board_row),
            ));
        }

        frame.render_widget(
            Paragraph::new(Line::from(spans)),
            Rect::new(inner.left(), y, inner.width, 1),
        );
    }

    // Scroll arrows rendered separately — never mixed into cell spans
    render_scroll_arrows(frame, inner, app, visible_cols, visible_rows);

    if app.board.state == GameState::Won {
        render_banner(frame, inner, " YOU WIN!  Press r to restart ", Color::Green);
    } else if app.board.state == GameState::Lost {
        render_banner(frame, inner, " BOOM!  Press r to restart ", Color::Red);
    }
}

/// Draw directional arrows at the border — never inserted into cell rows.
fn render_scroll_arrows(
    frame: &mut Frame,
    inner: Rect,
    app: &App,
    visible_cols: usize,
    visible_rows: usize,
) {
    let (vx, vy) = app.viewport;
    let arrow = Style::new().fg(Color::DarkGray);
    let mid_x = inner.left() + inner.width / 2;
    let mid_y = inner.top() + inner.height / 2;

    if vy > 0 {
        frame.render_widget(
            Paragraph::new("▲").style(arrow),
            Rect::new(mid_x, inner.top(), 1, 1),
        );
    }
    if vy + visible_rows < app.board.height {
        frame.render_widget(
            Paragraph::new("▼").style(arrow),
            Rect::new(mid_x, inner.bottom().saturating_sub(1), 1, 1),
        );
    }
    if vx > 0 {
        frame.render_widget(
            Paragraph::new("◀").style(arrow),
            Rect::new(inner.left(), mid_y, 1, 1),
        );
    }
    if vx + visible_cols < app.board.width {
        frame.render_widget(
            Paragraph::new("▶").style(arrow),
            Rect::new(inner.right().saturating_sub(1), mid_y, 1, 1),
        );
    }
}

// ─── Footer ──────────────────────────────────────────────────────────────────

fn render_footer(frame: &mut Frame, app: &mut App, area: Rect) {
    let line = match &app.claude_state.status {
        ClaudeStatus::Working => {
            let tool = app.claude_state.tool.as_deref().unwrap_or("unknown");
            Line::from(vec![
                Span::styled(" ⏺ ", Style::new().fg(Color::Cyan)),
                Span::styled(format!("Claude is working: {tool}"), Style::new().dim()),
                key_hints(),
            ])
        }
        ClaudeStatus::PermissionNeeded => {
            let style = if app.flash_on {
                Style::new().fg(Color::White).bg(Color::Red).bold()
            } else {
                Style::new().fg(Color::Red).bold()
            };
            Line::from(Span::styled(
                " ⚠  CLAUDE NEEDS PERMISSION — SWITCH PANES  ",
                style,
            ))
        }
        ClaudeStatus::Idle => Line::from(vec![
            Span::styled(" ⏸ ", Style::new().fg(Color::Yellow)),
            Span::styled(
                "Claude is waiting for your input",
                Style::new().fg(Color::Yellow).dim(),
            ),
            key_hints(),
        ]),
        ClaudeStatus::Done => Line::from(vec![
            Span::styled(" ✓ Claude finished", Style::new().fg(Color::Green).bold()),
            key_hints(),
        ]),
        ClaudeStatus::Unknown => Line::from(vec![
            Span::styled(" hjkl", Style::new().fg(Color::Cyan)),
            Span::styled("/arrows ", Style::new().dim()),
            Span::styled("Space", Style::new().fg(Color::Cyan)),
            Span::styled(" reveal  ", Style::new().dim()),
            Span::styled("f", Style::new().fg(Color::Cyan)),
            Span::styled(" flag  ", Style::new().dim()),
            Span::styled("r", Style::new().fg(Color::Cyan)),
            Span::styled(" restart  ", Style::new().dim()),
            Span::styled("q", Style::new().fg(Color::Cyan)),
            Span::styled(" quit", Style::new().dim()),
        ]),
    };
    frame.render_widget(Paragraph::new(line), area);
}

// ─── Cell rendering ───────────────────────────────────────────────────────────

fn cell_span(view: CellView, is_cursor: bool) -> Span<'static> {
    match view {
        CellView::Hidden => {
            if is_cursor {
                // Solid yellow block — unmistakable cursor on unrevealed tile
                Span::styled("██", Style::new().fg(CURSOR_BG).bg(CURSOR_BG))
            } else {
                // Light raised tile — clearly distinct from revealed (dark) cells
                Span::styled("▒▒", Style::new().fg(HIDDEN_FG).bg(HIDDEN_BG))
            }
        }

        CellView::Flag => {
            // Flag character on saturated red — Color::Red is ANSI muted red,
            // Rgb gives the vivid red that actually reads as "flag"
            let bg = if is_cursor {
                CURSOR_BG
            } else {
                Color::Rgb(210, 35, 35)
            };
            Span::styled("⚑ ", Style::new().fg(Color::White).bg(bg).bold())
        }

        CellView::Mine => {
            // Exploded mine
            Span::styled("* ", Style::new().fg(Color::White).bg(Color::Red).bold())
        }

        CellView::Number(0) => {
            // Revealed empty — dark "pressed in" look, clearly different from hidden tiles
            let bg = if is_cursor { CURSOR_BG } else { REVEALED_BG };
            let fg = if is_cursor { CURSOR_FG } else { Color::DarkGray };
            Span::styled("· ", Style::new().fg(fg).bg(bg))
        }

        CellView::Number(n) => {
            let fg = if is_cursor { CURSOR_FG } else { number_color(n) };
            let bg = if is_cursor { CURSOR_BG } else { REVEALED_BG };
            Span::styled(format!("{n} "), Style::new().fg(fg).bg(bg).bold())
        }
    }
}

fn number_color(n: u8) -> Color {
    match n {
        1 => Color::Cyan,
        2 => Color::Green,
        3 => Color::LightRed,
        4 => Color::Blue,
        5 => Color::Red,
        6 => Color::LightCyan,
        7 => Color::Magenta,
        _ => Color::White,
    }
}

// ─── Border config ────────────────────────────────────────────────────────────

fn border_config(app: &App) -> (Style, BorderType, String) {
    let diff = format!(
        " {} ",
        format!("{:?}", app.difficulty).to_lowercase()
    );
    match &app.claude_state.status {
        ClaudeStatus::Working => (Style::new().fg(Color::Blue), BorderType::Rounded, diff),
        ClaudeStatus::PermissionNeeded => {
            if app.flash_on {
                (
                    Style::new().fg(Color::Red).bold(),
                    BorderType::Double,
                    " ⚠ PERMISSION NEEDED ".into(),
                )
            } else {
                (Style::new().fg(Color::DarkGray), BorderType::Rounded, diff)
            }
        }
        ClaudeStatus::Idle => (Style::new().fg(Color::Yellow), BorderType::Rounded, diff),
        ClaudeStatus::Done => {
            if app.done_since.is_some() {
                (
                    Style::new().fg(Color::Green),
                    BorderType::Double,
                    " ✓ done ".into(),
                )
            } else {
                (Style::new(), BorderType::Rounded, diff)
            }
        }
        ClaudeStatus::Unknown => (Style::new(), BorderType::Rounded, diff),
    }
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn key_hints() -> Span<'static> {
    Span::styled(
        "  hjkl move  Space reveal  f flag  r restart  q quit",
        Style::new().dim(),
    )
}

fn render_banner(frame: &mut Frame, area: Rect, text: &str, color: Color) {
    let width = (text.len() as u16 + 2).min(area.width);
    let height = 3u16.min(area.height);
    let x = area.left() + area.width.saturating_sub(width) / 2;
    let y = area.top() + area.height.saturating_sub(height) / 2;
    let banner_area = Rect::new(x, y, width, height);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::new().fg(color).bold())
        .style(Style::new().bg(Color::Black));
    let inner = block.inner(banner_area);
    frame.render_widget(block, banner_area);
    frame.render_widget(
        Paragraph::new(text)
            .centered()
            .style(Style::new().fg(color).bold()),
        inner,
    );
}
