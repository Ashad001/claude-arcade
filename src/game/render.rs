use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
};

use crate::game::board::{CellView, GameState};
use crate::game::App;
use crate::state::ClaudeStatus;

pub fn ui(frame: &mut Frame, app: &App) {
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

fn render_header(frame: &mut Frame, app: &App, area: Rect) {
    let mines_left = app.board.mines_remaining();
    let state_indicator = match app.board.state {
        GameState::Playing => "●",
        GameState::Won => "✓",
        GameState::Lost => "✗",
    };
    let header = Line::from(vec![
        Span::styled(" MINESWEEPER ", Style::new().bold()),
        Span::raw(state_indicator),
        Span::raw(format!("  Mines: {}  ", mines_left)),
        Span::styled(format!("Score: {}", app.score), Style::new().cyan()),
    ]);
    frame.render_widget(Paragraph::new(header), area);
}

fn render_board(frame: &mut Frame, app: &App, area: Rect) {
    let (border_style, border_type) = border_for_status(app);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_style)
        .border_type(border_type);

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Each cell is 2 chars wide for readability
    let cell_width = 2usize;
    let board_width = app.board.width * cell_width;
    let board_height = app.board.height;

    // Horizontally and vertically centre the board within inner
    let x_offset = (inner.width as usize).saturating_sub(board_width) / 2;
    let y_offset = (inner.height as usize).saturating_sub(board_height) / 2;

    for row in 0..app.board.height {
        let y = inner.top() + (y_offset + row) as u16;
        if y >= inner.bottom() {
            break;
        }

        let mut spans: Vec<Span> = Vec::with_capacity(app.board.width);
        // Add left padding
        if x_offset > 0 {
            spans.push(Span::raw(" ".repeat(x_offset)));
        }

        for col in 0..app.board.width {
            let is_cursor = app.cursor == (col, row);
            let view = app.board.cell_view(col, row);
            spans.push(cell_span(view, is_cursor));
        }

        let line = Line::from(spans);
        let row_rect = Rect::new(inner.left(), y, inner.width, 1);
        frame.render_widget(Paragraph::new(line), row_rect);
    }

    // Overlay banners for win/loss
    if app.board.state == GameState::Won {
        render_banner(frame, inner, "  YOU WIN! Press 'r' to restart  ", Color::Green);
    } else if app.board.state == GameState::Lost {
        render_banner(frame, inner, "  BOOM! Press 'r' to restart  ", Color::Red);
    }
}

fn render_footer(frame: &mut Frame, app: &App, area: Rect) {
    let line = match &app.claude_state.status {
        ClaudeStatus::Working => {
            let tool = app
                .claude_state
                .tool
                .as_deref()
                .unwrap_or("unknown");
            Line::from(vec![
                Span::styled(" ⏺ ", Style::new().cyan()),
                Span::styled(
                    format!("Claude is working: {tool}"),
                    Style::new().dim(),
                ),
                key_hints_right(),
            ])
        }
        ClaudeStatus::PermissionNeeded => Line::from(vec![
            if app.flash_on {
                Span::styled(" ⚠  CLAUDE NEEDS PERMISSION — SWITCH PANES ", Style::new().red().bold())
            } else {
                Span::styled(" ⚠  CLAUDE NEEDS PERMISSION — SWITCH PANES ", Style::new().dim())
            },
        ]),
        ClaudeStatus::Idle => Line::from(vec![
            Span::styled(" ⏸  Claude is waiting for your input", Style::new().yellow()),
            key_hints_right(),
        ]),
        ClaudeStatus::Done => Line::from(vec![
            Span::styled(" ✓ Claude finished", Style::new().green().bold()),
            key_hints_right(),
        ]),
        ClaudeStatus::Unknown => Line::from(vec![
            Span::raw(" ↑↓←→/hjkl "),
            Span::styled("move  ", Style::new().dim()),
            Span::raw("Space "),
            Span::styled("reveal  ", Style::new().dim()),
            Span::raw("f "),
            Span::styled("flag  ", Style::new().dim()),
            Span::raw("r "),
            Span::styled("restart  ", Style::new().dim()),
            Span::raw("q "),
            Span::styled("quit", Style::new().dim()),
        ]),
    };
    frame.render_widget(Paragraph::new(line), area);
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

fn border_for_status(app: &App) -> (Style, BorderType) {
    match &app.claude_state.status {
        ClaudeStatus::Working => (Style::new().blue(), BorderType::Rounded),
        ClaudeStatus::PermissionNeeded => {
            if app.flash_on {
                (Style::new().red().bold(), BorderType::Double)
            } else {
                (Style::new().dark_gray(), BorderType::Rounded)
            }
        }
        ClaudeStatus::Idle => (Style::new().yellow(), BorderType::Rounded),
        ClaudeStatus::Done => {
            if app.done_since.is_some() {
                (Style::new().green(), BorderType::Double)
            } else {
                (Style::new(), BorderType::Rounded)
            }
        }
        ClaudeStatus::Unknown => (Style::new(), BorderType::Rounded),
    }
}

fn cell_span(view: CellView, is_cursor: bool) -> Span<'static> {
    let (text, style) = match view {
        CellView::Hidden => ("██", Style::new().dark_gray()),
        CellView::Flag => ("⚑ ", Style::new().red().bold()),
        CellView::Mine => ("✸ ", Style::new().red().bold()),
        CellView::Number(0) => ("  ", Style::new()),
        CellView::Number(n) => {
            let style = match n {
                1 => Style::new().blue(),
                2 => Style::new().green(),
                3 => Style::new().red(),
                4 => Style::new().cyan(),
                5 => Style::new().magenta(),
                6 => Style::new().cyan().bold(),
                7 => Style::new().red().bold(),
                _ => Style::new().white().bold(),
            };
            return Span::styled(format!("{n} "), style.bg(if is_cursor { Color::DarkGray } else { Color::Reset }));
        }
    };
    if is_cursor {
        Span::styled(text, style.bg(Color::DarkGray))
    } else {
        Span::styled(text, style)
    }
}

fn key_hints_right() -> Span<'static> {
    Span::styled("  hjkl/arrows:move  Space:reveal  f:flag  r:restart  q:quit", Style::new().dim())
}

fn render_banner(frame: &mut Frame, area: Rect, text: &str, color: Color) {
    let width = text.len() as u16 + 2;
    let height = 3u16;
    let x = area.left() + area.width.saturating_sub(width) / 2;
    let y = area.top() + area.height.saturating_sub(height) / 2;
    let banner_area = Rect::new(x, y, width.min(area.width), height.min(area.height));

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::new().fg(color).bold());
    let inner = block.inner(banner_area);
    frame.render_widget(block, banner_area);
    frame.render_widget(
        Paragraph::new(text).centered().style(Style::new().fg(color).bold()),
        inner,
    );
}
