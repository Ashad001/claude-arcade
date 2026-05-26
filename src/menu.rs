use crossterm::event::{KeyCode, KeyEvent, KeyEventKind};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
};
use std::time::Instant;

use crate::games::minesweeper::Difficulty;
use crate::state::{ClaudeState, ClaudeStatus};

// ─── Public types ────────────────────────────────────────────────────────────

pub enum SelectedGame {
    Minesweeper(Difficulty),
    TicTacToe,
    Game2048,
}

pub enum MenuSignal {
    None,
    Launch(SelectedGame),
    Quit,
}

// ─── Menu entries ─────────────────────────────────────────────────────────────

struct Entry {
    icon: &'static str,
    label: &'static str,
    desc: &'static str,
    color: Color,
}

const ENTRIES: &[Entry] = &[
    Entry {
        icon: "💣",
        label: "Minesweeper  Easy",
        desc: "9×9 · 10 mines",
        color: Color::Green,
    },
    Entry {
        icon: "💣",
        label: "Minesweeper  Medium",
        desc: "16×16 · 40 mines  ← default",
        color: Color::Yellow,
    },
    Entry {
        icon: "💣",
        label: "Minesweeper  Hard",
        desc: "30×16 · 99 mines",
        color: Color::Red,
    },
    Entry {
        icon: "✕○",
        label: "Tic Tac Toe",
        desc: "vs. unbeatable AI  (minimax)",
        color: Color::Cyan,
    },
    Entry {
        icon: "▦",
        label: "2048",
        desc: "slide tiles · reach 2048",
        color: Color::Magenta,
    },
];

// ─── Menu state ───────────────────────────────────────────────────────────────

pub struct Menu {
    pub selected: usize,
    pub claude_state: ClaudeState,
    pub flash_on: bool,
    flash_tick: u8,
    pub permission_alerted: bool,
    pub done_since: Option<Instant>,
}

impl Menu {
    pub fn new() -> Self {
        Self {
            selected: 1, // default: Minesweeper Medium
            claude_state: ClaudeState::default(),
            flash_on: true,
            flash_tick: 0,
            permission_alerted: false,
            done_since: None,
        }
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> MenuSignal {
        if key.kind != KeyEventKind::Press {
            return MenuSignal::None;
        }

        match key.code {
            KeyCode::Up | KeyCode::Char('k') => {
                if self.selected > 0 {
                    self.selected -= 1;
                }
                MenuSignal::None
            }
            KeyCode::Down | KeyCode::Char('j') => {
                if self.selected < ENTRIES.len() - 1 {
                    self.selected += 1;
                }
                MenuSignal::None
            }
            KeyCode::Enter | KeyCode::Char(' ') => {
                let game = match self.selected {
                    0 => SelectedGame::Minesweeper(Difficulty::Easy),
                    1 => SelectedGame::Minesweeper(Difficulty::Medium),
                    2 => SelectedGame::Minesweeper(Difficulty::Hard),
                    3 => SelectedGame::TicTacToe,
                    4 => SelectedGame::Game2048,
                    _ => return MenuSignal::None,
                };
                MenuSignal::Launch(game)
            }
            KeyCode::Char('q') | KeyCode::Esc => MenuSignal::Quit,
            // Number shortcuts
            KeyCode::Char('1') => {
                self.selected = 0;
                MenuSignal::None
            }
            KeyCode::Char('2') => {
                self.selected = 1;
                MenuSignal::None
            }
            KeyCode::Char('3') => {
                self.selected = 2;
                MenuSignal::None
            }
            KeyCode::Char('4') => {
                self.selected = 3;
                MenuSignal::None
            }
            KeyCode::Char('5') => {
                self.selected = 4;
                MenuSignal::None
            }
            _ => MenuSignal::None,
        }
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
    }
}

// ─── Render ───────────────────────────────────────────────────────────────────

pub fn render(frame: &mut Frame, menu: &Menu) {
    let total = frame.area();

    let [title_area, list_area, footer_area] = Layout::vertical([
        Constraint::Length(7),
        Constraint::Fill(1),
        Constraint::Length(1),
    ])
    .areas(total);

    render_title(frame, menu, title_area);
    render_list(frame, menu, list_area);
    render_footer(frame, menu, footer_area);
}

// ─── Title block ─────────────────────────────────────────────────────────────

const LOGO: &[&str] = &[
    r" ██████╗██╗      █████╗ ██╗   ██╗██████╗ ███████╗",
    r" ██╔════╝██║     ██╔══██╗██║   ██║██╔══██╗██╔════╝",
    r" ██║     ██║     ███████║██║   ██║██║  ██║█████╗  ",
    r" ╚██████╗███████╗██║  ██║╚██████╔╝██████╔╝███████╗",
    r"  ╚═════╝╚══════╝╚═╝  ╚═╝ ╚═════╝ ╚═════╝╚══════╝",
];

fn render_title(frame: &mut Frame, menu: &Menu, area: Rect) {
    // Subtitle line
    let subtitle = Line::from(vec![
        Span::styled(
            "  Terminal retro games for Claude Code  ",
            Style::new().fg(Color::DarkGray),
        ),
    ]);

    // Status indicator in the title block corner
    let status_line = match &menu.claude_state.status {
        ClaudeStatus::Working => {
            let tool = menu.claude_state.tool.as_deref().unwrap_or("tool");
            Line::from(vec![
                Span::styled("  ⏺ ", Style::new().fg(Color::Cyan)),
                Span::styled(format!("Claude is working: {tool}"), Style::new().dim()),
            ])
        }
        ClaudeStatus::PermissionNeeded => {
            let style = if menu.flash_on {
                Style::new().fg(Color::White).bg(Color::Red).bold()
            } else {
                Style::new().fg(Color::Red).bold()
            };
            Line::from(Span::styled(
                "  ⚠  CLAUDE NEEDS PERMISSION — SWITCH PANES  ",
                style,
            ))
        }
        ClaudeStatus::Idle => Line::from(vec![
            Span::styled("  ⏸ ", Style::new().fg(Color::Yellow)),
            Span::styled("Claude is waiting for your input", Style::new().fg(Color::Yellow).dim()),
        ]),
        ClaudeStatus::Done => {
            if menu.done_since.is_some() {
                Line::from(Span::styled(
                    "  ✓ Claude finished",
                    Style::new().fg(Color::Green).bold(),
                ))
            } else {
                Line::from("")
            }
        }
        ClaudeStatus::Unknown => Line::from(""),
    };

    // Try to render the block logo lines if there's enough height & width
    let logo_width = LOGO.iter().map(|l| l.len()).max().unwrap_or(0) as u16;

    if area.width >= logo_width + 2 && area.height >= 7 {
        // Draw logo + subtitle
        let x0 = area.left() + area.width.saturating_sub(logo_width) / 2;
        for (i, &line) in LOGO.iter().enumerate() {
            let y = area.top() + i as u16;
            if y >= area.bottom() {
                break;
            }
            frame.render_widget(
                Paragraph::new(line).style(Style::new().fg(Color::Cyan)),
                Rect::new(x0, y, logo_width, 1),
            );
        }
        let subtitle_y = area.top() + LOGO.len() as u16;
        if subtitle_y < area.bottom() {
            frame.render_widget(
                Paragraph::new(subtitle).alignment(Alignment::Center),
                Rect::new(area.left(), subtitle_y, area.width, 1),
            );
        }
        let status_y = subtitle_y + 1;
        if status_y < area.bottom() {
            frame.render_widget(
                Paragraph::new(status_line),
                Rect::new(area.left(), status_y, area.width, 1),
            );
        }
    } else {
        // Compact fallback
        let title = Line::from(vec![
            Span::styled("CLAUDE", Style::new().fg(Color::Cyan).bold()),
            Span::styled("-", Style::new().fg(Color::DarkGray)),
            Span::styled("ARCADE", Style::new().fg(Color::Magenta).bold()),
            Span::raw("  🎮  Terminal retro games for Claude Code"),
        ]);
        frame.render_widget(Paragraph::new(title), area);
        if area.height >= 2 {
            frame.render_widget(
                Paragraph::new(status_line),
                Rect::new(area.left(), area.top() + 1, area.width, 1),
            );
        }
    }
}

// ─── Game list ────────────────────────────────────────────────────────────────

fn render_list(frame: &mut Frame, menu: &Menu, area: Rect) {
    let (border_style, border_type) = match &menu.claude_state.status {
        ClaudeStatus::Working => (Style::new().fg(Color::Blue), BorderType::Rounded),
        ClaudeStatus::PermissionNeeded => {
            if menu.flash_on {
                (Style::new().fg(Color::Red).bold(), BorderType::Double)
            } else {
                (Style::new().fg(Color::DarkGray), BorderType::Rounded)
            }
        }
        ClaudeStatus::Idle => (Style::new().fg(Color::Yellow), BorderType::Rounded),
        ClaudeStatus::Done if menu.done_since.is_some() => {
            (Style::new().fg(Color::Green), BorderType::Double)
        }
        _ => (Style::new().fg(Color::DarkGray), BorderType::Rounded),
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_style)
        .border_type(border_type)
        .title(" SELECT A GAME ")
        .title_alignment(Alignment::Center);

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let n = ENTRIES.len() as u16;
    // Vertically centre the list
    let start_y = inner.top() + inner.height.saturating_sub(n * 2 + 1) / 2;

    for (i, entry) in ENTRIES.iter().enumerate() {
        let y = start_y + i as u16 * 2;
        if y >= inner.bottom() {
            break;
        }

        let is_sel = i == menu.selected;

        if is_sel {
            // Selected item: arrow + icon + coloured label + dim description
            let line = Line::from(vec![
                Span::styled("  ▶ ", Style::new().fg(Color::Yellow).bold()),
                Span::styled(
                    format!("{}  ", entry.icon),
                    Style::new().fg(entry.color).bold(),
                ),
                Span::styled(entry.label, Style::new().fg(Color::White).bold()),
                Span::styled("   ", Style::new()),
                Span::styled(entry.desc, Style::new().fg(Color::DarkGray)),
            ]);
            frame.render_widget(Paragraph::new(line), Rect::new(inner.left(), y, inner.width, 1));
        } else {
            // Unselected item: dim
            let line = Line::from(vec![
                Span::raw("     "),
                Span::styled(
                    format!("{}  ", entry.icon),
                    Style::new().fg(Color::DarkGray),
                ),
                Span::styled(entry.label, Style::new().fg(Color::DarkGray)),
            ]);
            frame.render_widget(Paragraph::new(line), Rect::new(inner.left(), y, inner.width, 1));
        }
    }
}

// ─── Footer ──────────────────────────────────────────────────────────────────

fn render_footer(frame: &mut Frame, _menu: &Menu, area: Rect) {
    let line = Line::from(vec![
        Span::styled(" ↑↓ / jk", Style::new().fg(Color::Cyan)),
        Span::styled(" navigate  ", Style::new().dim()),
        Span::styled("Enter", Style::new().fg(Color::Cyan)),
        Span::styled(" select  ", Style::new().dim()),
        Span::styled("1-5", Style::new().fg(Color::Cyan)),
        Span::styled(" quick pick  ", Style::new().dim()),
        Span::styled("q", Style::new().fg(Color::Cyan)),
        Span::styled(" quit", Style::new().dim()),
    ]);
    frame.render_widget(Paragraph::new(line), area);
}
