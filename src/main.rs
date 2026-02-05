mod app;
mod config;
mod fetcher;
mod finder;
mod history;
mod keys;
mod pager;
mod parser;

use anyhow::Result;
use clap::Parser;
use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::prelude::*;
use std::io::{self, stdout};

use app::App;
use config::Config;

#[derive(Parser, Debug)]
#[command(name = "helpv")]
#[command(about = "A help viewer with subcommand navigation")]
#[command(version)]
struct Args {
    /// The command to show help for
    #[arg(required = true)]
    command: Vec<String>,
}

fn main() -> Result<()> {
    let args = Args::parse();

    if args.command.is_empty() {
        eprintln!("Usage: helpv <COMMAND> [SUBCOMMANDS...]");
        eprintln!("Example: helpv git");
        std::process::exit(1);
    }

    let config = Config::load()?;

    // Initialize terminal
    enable_raw_mode()?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen)?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Run the app
    let result = run_app(&mut terminal, args.command, config);

    // Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    // Handle any errors
    if let Err(e) = result {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }

    Ok(())
}

fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    command: Vec<String>,
    config: Config,
) -> Result<()> {
    let mut app = App::new(command, config)?;
    app.run(terminal)
}
