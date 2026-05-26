use std::time::Instant;

use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph},
};

use crate::games::game2048::{App, GameState};
use crate::state::ClaudeStatus;

// ─── Tile colours ─────────────────────────────────────────────────────────────

fn tile_colors(value: u32) -> (Color, Color) {
    // (background, foreground)
    match value {
        0 => (Color::Rgb(30, 32, 40), Color::Rgb(50, 55, 65)),
        2 => (Color::Rgb(55, 60, 75), Color::Rgb(230, 235, 255)),
        4 => (Color::Rgb(75, 70, 45), Color::Rgb(255, 240, 160)),
        8 => (Color::Rgb(180, 100, 30), Color::White),
        16 => (Color::Rgb(200, 70, 20), Color::White),
        32 => (Color::Rgb(210, 45, 45), Color::White),
        64 => (Color::Rgb(180, 30, 90), Color::White),
        128 => (Color::Rgb(120, 60, 200), Color::White),
        256 => (Color::Rgb(60, 80, 210), Color::White),
        512 => (Color::Rgb(20, 140, 200), Color::White),
        1024 => (Color::Rgb(20, 180, 150), Color::White),
        2048 => (Color::Rgb(20, 200, 80), Color::Rgb(240, 255, 240)),
        _ => (Color::Rgb(200, 170, 20), Color::Rgb(255, 250, 200)), // 4096+
    }
}

// ─── Entry point ─────────────────────────────────────────────────────────────

pub fn ui(frame: &mut Frame, app: &App) {
    let now = Instant::now();
    let total = frame.area();

    let [header_area, board_area, footer_area] = Layout::vertical([
        Constraint::Length(1),
        Constraint::Fill(1),
        Constraint::Length(1),
    ])
    .areas(total);

    render_header(frame, app, header_area, now);
    render_board(frame, app, board_area);
    render_footer(frame, app, footer_area);
}

// ─── Header ──────────────────────────────────────────────────────────────────

fn render_header(frame: &mut Frame, app: &App, area: Rect, now: Instant) {
    let timer = app.elapsed_display(now);
    let status = match &app.state {
        GameState::Playing | GameState::Continued => {
            let max = app.max_tile();
            Span::styled(
                format!(" Best tile: {max}"),
                Style::new().fg(Color::Cyan),
            )
        }
        GameState::Won => Span::styled(" YOU REACHED 2048! ★", Style::new().fg(Color::Green).bold()),
        GameState::Lost => Span::styled(" GAME OVER ✗", Style::new().fg(Color::Red).bold()),
    };
    let header = Line::from(vec![
        Span::styled(" 2048 ", Style::new().bold()),
        status,
        Span::styled(
            format!("  Score: {}  Best: {}  ⏱ {}", app.score, app.best_score, timer),
            Style::new().fg(Color::DarkGray),
        ),
    ]);
    frame.render_widget(Paragraph::new(header), area);
}

// ─── Board ───────────────────────────────────────────────────────────────────

fn render_board(frame: &mut Frame, app: &App, area: Rect) {
    let (border_style, border_type, title) = border_config(app);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_style)
        .border_type(border_type)
        .title(title)
        .title_alignment(Alignment::Center);

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Each tile: 8 chars wide, 3 lines tall. 4 tiles per dimension.
    // Board: 32 wide (no gaps between tiles), 12 tall.
    let tile_w: u16 = 8;
    let tile_h: u16 = 3;
    let board_w = tile_w * 4;
    let board_h = tile_h * 4;

    let x0 = inner.left() + inner.width.saturating_sub(board_w) / 2;
    let y0 = inner.top() + inner.height.saturating_sub(board_h) / 2;

    for row in 0..4usize {
        for col in 0..4usize {
            let value = app.board[row][col];
            let (bg, fg) = tile_colors(value);

            let cx = x0 + col as u16 * tile_w;
            let cy = y0 + row as u16 * tile_h;

            for line in 0..tile_h {
                let y = cy + line;
                if y >= inner.bottom() {
                    break;
                }
                let text = if line == tile_h / 2 && value > 0 {
                    format!("{:^width$}", value, width = tile_w as usize)
                } else {
                    " ".repeat(tile_w as usize)
                };
                frame.render_widget(
                    Paragraph::new(text).style(Style::new().fg(fg).bg(bg)),
                    Rect::new(cx, y, tile_w, 1),
                );
            }
        }
    }

    // Overlay banners for terminal states
    match &app.state {
        GameState::Won => render_overlay(
            frame,
            inner,
            " YOU WIN!  c=continue  r=restart ",
            Color::Green,
        ),
        GameState::Lost => render_overlay(
            frame,
            inner,
            " GAME OVER!  r=restart  Esc=menu ",
            Color::Red,
        ),
        _ => {}
    }
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
            Span::styled(" hjkl/arrows", Style::new().fg(Color::Cyan)),
            Span::styled(" slide  ", Style::new().dim()),
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
    let title = " 2048 ".into();
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

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn key_hints() -> Span<'static> {
    Span::styled(
        "  hjkl/arrows slide  r restart  Esc menu  q quit",
        Style::new().dim(),
    )
}

fn render_overlay(frame: &mut Frame, area: Rect, text: &str, color: Color) {
    let width = (text.len() as u16 + 4).min(area.width);
    let height = 3u16.min(area.height);
    let x = area.left() + area.width.saturating_sub(width) / 2;
    let y = area.top() + area.height.saturating_sub(height) / 2;
    let overlay_area = Rect::new(x, y, width, height);

    frame.render_widget(Clear, overlay_area);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::new().fg(color).bold())
        .style(Style::new().bg(Color::Black));
    let inner = block.inner(overlay_area);
    frame.render_widget(block, overlay_area);
    frame.render_widget(
        Paragraph::new(text)
            .centered()
            .style(Style::new().fg(color).bold()),
        inner,
    );
}
