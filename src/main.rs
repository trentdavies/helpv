mod app;
mod config;
mod fetcher;
mod finder;
mod history;
mod keys;
mod pager;
mod parser;
mod switcher;
mod toolpacks;

use anyhow::Result;
use clap::Parser;
use crossterm::{
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::prelude::*;
use std::io::{self, stdout};

use app::App;
use config::Config;

#[derive(Parser, Debug)]
#[command(name = "helpv")]
#[command(version)]
#[command(about = "A TUI help viewer with vim-style navigation and fuzzy subcommand finder")]
#[command(
    long_about = "A TUI help viewer with vim-style navigation and fuzzy subcommand finder.

helpv fetches and displays help text for any command, letting you navigate
with familiar vim keybindings. Press 'f' to fuzzy-find subcommands, Enter
to drill into them, and Backspace to go back."
)]
#[command(after_long_help = "KEYBINDINGS:
    j/k, Up/Down      Scroll line by line
    d/u, Ctrl-d/u     Scroll half page
    Space, Ctrl-f/b   Scroll full page
    gg, G             Jump to top/bottom
    /                 Search in help text
    n/N               Next/previous search match
    f                 Fuzzy find subcommands
    o                 Open arbitrary command
    Enter             Drill into selected subcommand
    Backspace         Go back to parent command
    ?                 Show keybindings help
    q, Escape         Quit

EXAMPLES:
    helpv git                View git help
    helpv git commit         View git commit help
    helpv cargo build        View cargo build help

CONFIGURATION:
    Config file: ~/.config/helpv/config.toml
    Customize keybindings, help flags, and subcommand patterns.")]
struct Args {
    /// Command (and optional subcommands) to show help for
    #[arg(required = true, value_name = "COMMAND")]
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
