use clap::{Parser, Subcommand};
use color_eyre::eyre::Result;

mod game;
mod install;
mod state;
mod tui;

#[derive(Debug, Parser)]
#[command(name = "claude-arcade", about = "Terminal Minesweeper for Claude Code sessions")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Launch the Minesweeper game
    Play {
        /// Game difficulty
        #[arg(long, value_enum, default_value = "medium")]
        difficulty: Difficulty,
    },
    /// Wire up Claude Code hooks (requires tmux)
    Install {
        /// Apply without prompting
        #[arg(long)]
        yes: bool,
        /// Show changes without applying
        #[arg(long)]
        dry_run: bool,
    },
    /// Remove Claude Code hooks
    Uninstall {
        /// Apply without prompting
        #[arg(long)]
        yes: bool,
    },
}

#[derive(Debug, Clone, clap::ValueEnum)]
pub enum Difficulty {
    Easy,
    Medium,
    Hard,
}

impl Difficulty {
    /// Returns (width, height, mines)
    pub fn board_params(&self) -> (usize, usize, usize) {
        match self {
            Difficulty::Easy => (9, 9, 10),
            Difficulty::Medium => (16, 16, 40),
            Difficulty::Hard => (30, 16, 99),
        }
    }

    pub fn score_multiplier(&self) -> u32 {
        match self {
            Difficulty::Easy => 1,
            Difficulty::Medium => 2,
            Difficulty::Hard => 4,
        }
    }
}

fn main() -> Result<()> {
    color_eyre::install()?;
    let cli = Cli::parse();

    match cli.command {
        Commands::Play { difficulty } => tui::run(difficulty),
        Commands::Install { yes, dry_run } => install::install(yes, dry_run),
        Commands::Uninstall { yes } => install::uninstall(yes),
    }
}
