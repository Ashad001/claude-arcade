use clap::{Parser, Subcommand};
use color_eyre::eyre::Result;

mod game;
mod install;
mod state;
mod stats;
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
    /// Print the local leaderboard and exit
    Stats,
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
        Commands::Stats => print_stats(),
    }
}

fn print_stats() -> Result<()> {
    let records = stats::leaderboard_top(10);
    if records.is_empty() {
        println!("No games recorded yet. Play some games first!");
        return Ok(());
    }
    println!("{:<8} {:>7}  {:>5}  {:<3}  {}", "DIFF", "SCORE", "TIME", "WIN", "DATE");
    println!("{}", "─".repeat(42));
    for r in &records {
        let time_str = format!("{:02}:{:02}", r.time_secs / 60, r.time_secs % 60);
        let won_str = if r.won { "yes" } else { "no " };
        let date = if r.timestamp.len() >= 10 { &r.timestamp[..10] } else { &r.timestamp };
        println!(
            "{:<8} {:>7}  {:>5}  {:<3}  {}",
            r.difficulty, r.score, time_str, won_str, date
        );
    }
    Ok(())
}
