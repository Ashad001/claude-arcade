use clap::{Parser, Subcommand};
use color_eyre::eyre::Result;

mod games;
mod install;
mod menu;
mod state;
mod stats;
mod tui;

#[derive(Debug, Parser)]
#[command(
    name = "claude-arcade",
    about = "Retro terminal games for Claude Code sessions — Minesweeper · Tic Tac Toe · 2048"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Launch the game menu (Minesweeper, Tic Tac Toe, 2048)
    Play,
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

fn main() -> Result<()> {
    color_eyre::install()?;
    let cli = Cli::parse();

    match cli.command {
        Commands::Play => tui::run(),
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
    println!(
        "{:<14} {:>7}  {:>5}  {:<3}  DATE",
        "GAME / DIFF", "SCORE", "TIME", "WIN"
    );
    println!("{}", "─".repeat(46));
    for r in &records {
        let time_str = format!("{:02}:{:02}", r.time_secs / 60, r.time_secs % 60);
        let won_str = if r.won { "yes" } else { "no " };
        let date = if r.timestamp.len() >= 10 {
            &r.timestamp[..10]
        } else {
            &r.timestamp
        };
        let label = format!("{}/{}", r.game, r.difficulty);
        println!(
            "{:<14} {:>7}  {:>5}  {:<3}  {}",
            label, r.score, time_str, won_str, date
        );
    }
    Ok(())
}
