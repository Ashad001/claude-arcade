use std::time::Instant;

use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
};

use crate::games::tictactoe::{App, GameState, Mark};
use crate::state::ClaudeStatus;

// ─── Colours ─────────────────────────────────────────────────────────────────

const X_COLOR: Color = Color::Rgb(100, 200, 255); // electric blue for human
const O_COLOR: Color = Color::Rgb(255, 120, 80);  // warm red for AI
const WIN_HL: Color = Color::Rgb(240, 220, 60);   // gold for winning cells
const CURSOR_BG: Color = Color::Rgb(60, 60, 80);  // subtle cursor highlight
const EMPTY_FG: Color = Color::Rgb(50, 55, 65);   // dim empty cells

// ─── Entry point ─────────────────────────────────────────────────────────────

pub fn ui(frame: &mut Frame, app: &App) {
    let now = Instant::now();
    let total = frame.area();

    let [header_area, game_area, footer_area] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Fill(1),
        Constraint::Length(1),
    ])
    .areas(total);

    render_header(frame, app, header_area, now);
    render_game(frame, app, game_area);
    render_footer(frame, app, footer_area);
}

// ─── Header ──────────────────────────────────────────────────────────────────

fn render_header(frame: &mut Frame, app: &App, area: Rect, now: Instant) {
    let timer = app.elapsed_display(now);
    let status = match &app.state {
        GameState::Playing => {
            if app.ai_thinking {
                Span::styled(" AI thinking…", Style::new().fg(O_COLOR).italic())
            } else {
                Span::styled(" Your turn (X)", Style::new().fg(X_COLOR).bold())
            }
        }
        GameState::WonX => Span::styled(" You win! ★", Style::new().fg(Color::Green).bold()),
        GameState::WonO => Span::styled(" AI wins! ✗", Style::new().fg(Color::Red).bold()),
        GameState::Draw => Span::styled(" Draw!", Style::new().fg(Color::Yellow).bold()),
    };
    let header = Line::from(vec![
        Span::styled(" TIC TAC TOE ", Style::new().bold()),
        status,
        Span::styled(
            format!("  Score: {}  ⏱ {}", app.score, timer),
            Style::new().fg(Color::DarkGray),
        ),
    ]);
    frame.render_widget(Paragraph::new(header), area);
}

// ─── Game area ───────────────────────────────────────────────────────────────

fn render_game(frame: &mut Frame, app: &App, area: Rect) {
    let (border_style, border_type, title) = border_config(app);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_style)
        .border_type(border_type)
        .title(title)
        .title_alignment(Alignment::Center);

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Board dimensions: 3 cells × (9 chars wide + 2 separators) = 29 wide
    //                  3 cells × (3 lines tall + 2 separators) = 11 tall
    let cell_w: u16 = 9;
    let cell_h: u16 = 3;
    let sep_w: u16 = 1;
    let sep_h: u16 = 1;
    let board_w = 3 * cell_w + 2 * sep_w;
    let board_h = 3 * cell_h + 2 * sep_h;

    // Centre the board in the inner area
    let x0 = inner.left() + inner.width.saturating_sub(board_w) / 2;
    let y0 = inner.top() + inner.height.saturating_sub(board_h) / 2;

    // Render cells
    for row in 0..3usize {
        for col in 0..3usize {
            let idx = row * 3 + col;
            let cx = x0 + col as u16 * (cell_w + sep_w);
            let cy = y0 + row as u16 * (cell_h + sep_h);
            render_cell(frame, app, idx, Rect::new(cx, cy, cell_w, cell_h));
        }
    }

    // Render vertical separators
    for col in 1..3usize {
        let sx = x0 + col as u16 * (cell_w + sep_w) - sep_w;
        for dy in 0..board_h {
            if y0 + dy < inner.bottom() {
                frame.render_widget(
                    Paragraph::new("│").style(Style::new().fg(Color::DarkGray)),
                    Rect::new(sx, y0 + dy, 1, 1),
                );
            }
        }
    }

    // Render horizontal separators
    for row in 1..3usize {
        let sy = y0 + row as u16 * (cell_h + sep_h) - sep_h;
        let horiz = "─".repeat(board_w as usize);
        if sy < inner.bottom() {
            frame.render_widget(
                Paragraph::new(horiz.as_str()).style(Style::new().fg(Color::DarkGray)),
                Rect::new(x0, sy, board_w, 1),
            );
        }
    }

    // Render intersection dots
    for row in 1..3usize {
        for col in 1..3usize {
            let ix = x0 + col as u16 * (cell_w + sep_w) - sep_w;
            let iy = y0 + row as u16 * (cell_h + sep_h) - sep_h;
            if ix < inner.right() && iy < inner.bottom() {
                frame.render_widget(
                    Paragraph::new("┼").style(Style::new().fg(Color::DarkGray)),
                    Rect::new(ix, iy, 1, 1),
                );
            }
        }
    }
}

fn render_cell(frame: &mut Frame, app: &App, idx: usize, area: Rect) {
    let is_cursor = idx == app.cursor && app.state == GameState::Playing;
    let is_winning = app
        .winning_line
        .is_some_and(|line| line.contains(&idx));

    let (symbol, style) = match app.board[idx] {
        Some(Mark::X) => {
            let color = if is_winning { WIN_HL } else { X_COLOR };
            let style = Style::new().fg(color).bold();
            (" X ", style)
        }
        Some(Mark::O) => {
            let color = if is_winning { WIN_HL } else { O_COLOR };
            let style = Style::new().fg(color).bold();
            (" O ", style)
        }
        None => {
            let style = if is_cursor {
                Style::new().fg(Color::White).bg(CURSOR_BG)
            } else {
                Style::new().fg(EMPTY_FG)
            };
            ("   ", style)
        }
    };

    let bg_style = if is_winning {
        Style::new().bg(Color::Rgb(50, 45, 0))
    } else if is_cursor {
        Style::new().bg(CURSOR_BG)
    } else {
        Style::new()
    };

    // Fill all cell lines with background
    for dy in 0..area.height {
        frame.render_widget(
            Paragraph::new(" ".repeat(area.width as usize)).style(bg_style),
            Rect::new(area.left(), area.top() + dy, area.width, 1),
        );
    }

    // Render symbol on the middle line
    let mid_y = area.top() + area.height / 2;
    let sym_padded = format!("{:^width$}", symbol, width = area.width as usize);
    frame.render_widget(
        Paragraph::new(sym_padded).style(style),
        Rect::new(area.left(), mid_y, area.width, 1),
    );
}

// ─── Footer ──────────────────────────────────────────────────────────────────

fn render_footer(frame: &mut Frame, app: &App, area: Rect) {
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
            Span::styled("Claude is waiting", Style::new().fg(Color::Yellow).dim()),
            key_hints(),
        ]),
        ClaudeStatus::Done => Line::from(vec![
            Span::styled(" ✓ Claude finished ", Style::new().fg(Color::Green).bold()),
            key_hints(),
        ]),
        ClaudeStatus::Unknown => Line::from(vec![
            Span::styled(
                " hjkl/arrows",
                Style::new().fg(Color::Cyan),
            ),
            Span::styled(" move  ", Style::new().dim()),
            Span::styled("Space/Enter", Style::new().fg(Color::Cyan)),
            Span::styled(" place X  ", Style::new().dim()),
            Span::styled("r", Style::new().fg(Color::Cyan)),
            Span::styled(" restart  ", Style::new().dim()),
            Span::styled("Esc", Style::new().fg(Color::Cyan)),
            Span::styled(" menu  ", Style::new().dim()),
            Span::styled("q", Style::new().fg(Color::Cyan)),
            Span::styled(" quit", Style::new().dim()),
        ]),
    };
    frame.render_widget(Paragraph::new(line), area);
}

// ─── Border config ────────────────────────────────────────────────────────────

fn border_config(app: &App) -> (Style, BorderType, String) {
    let title = " TIC TAC TOE — vs AI ".into();
    match &app.claude_state.status {
        ClaudeStatus::Working => (Style::new().fg(Color::Blue), BorderType::Rounded, title),
        ClaudeStatus::PermissionNeeded => {
            if app.flash_on {
                (
                    Style::new().fg(Color::Red).bold(),
                    BorderType::Double,
                    " ⚠ PERMISSION NEEDED ".into(),
                )
            } else {
                (Style::new().fg(Color::DarkGray), BorderType::Rounded, title)
            }
        }
        ClaudeStatus::Idle => (Style::new().fg(Color::Yellow), BorderType::Rounded, title),
        ClaudeStatus::Done => {
            if app.done_since.is_some() {
                (
                    Style::new().fg(Color::Green),
                    BorderType::Double,
                    " ✓ done ".into(),
                )
            } else {
                (Style::new(), BorderType::Rounded, title)
            }
        }
        ClaudeStatus::Unknown => (Style::new(), BorderType::Rounded, title),
    }
}

fn key_hints() -> Span<'static> {
    Span::styled(
        "  hjkl move  Space place  r restart  Esc menu  q quit",
        Style::new().dim(),
    )
}
