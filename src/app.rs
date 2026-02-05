use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent};
use ratatui::{buffer::Buffer, layout::Rect, widgets::Widget, Frame};
use std::time::Duration;

use crate::{
    config::Config,
    fetcher::fetch_help,
    finder::{Finder, FinderAction, FinderWidget},
    history::History,
    keys::{Action, KeyHandler},
    pager::{HelpOverlay, Pager, PagerWidget, SearchInput},
    parser::{parse_subcommands, Subcommand},
    switcher::{CommandSwitcher, SwitcherAction, SwitcherWidget},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppState {
    Paging,
    Searching,
    Finding,
    Switching,
    Help,
}

pub struct App {
    pub state: AppState,
    pub pager: Pager,
    pub finder: Option<Finder>,
    pub switcher: Option<CommandSwitcher>,
    pub history: History,
    pub command_history: Vec<String>,
    pub config: Config,
    pub current_command: Vec<String>,
    pub subcommands: Vec<Subcommand>,
    pub search_input: String,
    pub key_handler: KeyHandler,
    pub should_quit: bool,
    pub error_message: Option<String>,
}

impl App {
    pub fn new(command: Vec<String>, config: Config) -> Result<Self> {
        let help_text = fetch_help(&command, &config)?;
        let subcommands = parse_subcommands(&help_text, &config);
        let key_handler = KeyHandler::new(config.keys.clone());
        let initial_cmd = command[0].clone();

        Ok(Self {
            state: AppState::Paging,
            pager: Pager::new(help_text),
            finder: None,
            switcher: None,
            history: History::new(),
            command_history: vec![initial_cmd],
            config,
            current_command: command,
            subcommands,
            search_input: String::new(),
            key_handler,
            should_quit: false,
            error_message: None,
        })
    }

    pub fn run(&mut self, terminal: &mut ratatui::Terminal<impl ratatui::backend::Backend>) -> Result<()> {
        while !self.should_quit {
            terminal.draw(|frame| self.draw(frame))?;
            self.handle_events()?;
        }
        Ok(())
    }

    fn draw(&mut self, frame: &mut Frame) {
        let area = frame.area();

        // Clamp scroll based on current viewport
        self.pager.clamp_scroll(area.height.saturating_sub(1) as usize);

        // Draw the pager
        let breadcrumb = self.history.full_breadcrumb(&self.current_command);
        let pager_widget = PagerWidget::new(&self.pager, &breadcrumb, self.subcommands.len());
        frame.render_widget(pager_widget, area);

        // Draw overlays based on state
        match self.state {
            AppState::Searching => {
                let status_area = Rect::new(area.x, area.bottom() - 1, area.width, 1);
                frame.render_widget(SearchInput::new(&self.search_input), status_area);
            }
            AppState::Finding => {
                if let Some(ref finder) = self.finder {
                    frame.render_widget(FinderWidget::new(finder), area);
                }
            }
            AppState::Switching => {
                if let Some(ref switcher) = self.switcher {
                    frame.render_widget(SwitcherWidget::new(switcher), area);
                }
            }
            AppState::Help => {
                frame.render_widget(HelpOverlay, area);
            }
            AppState::Paging => {}
        }

        // Show error message if any
        if let Some(ref msg) = self.error_message {
            let error_area = Rect::new(area.x, area.bottom() - 1, area.width, 1);
            frame.render_widget(ErrorMessage(msg), error_area);
        }
    }

    fn handle_events(&mut self) -> Result<()> {
        if event::poll(Duration::from_millis(100))?
            && let Event::Key(key) = event::read()? {
                self.error_message = None; // Clear error on any key press
                self.handle_key(key)?;
            }
        Ok(())
    }

    fn handle_key(&mut self, key: KeyEvent) -> Result<()> {
        match self.state {
            AppState::Paging => self.handle_paging_key(key),
            AppState::Searching => self.handle_searching_key(key),
            AppState::Finding => self.handle_finding_key(key),
            AppState::Switching => self.handle_switching_key(key),
            AppState::Help => self.handle_help_key(key),
        }
    }

    fn handle_paging_key(&mut self, key: KeyEvent) -> Result<()> {
        if let Some(action) = self.key_handler.handle(key) {
            match action {
                Action::Quit => {
                    self.should_quit = true;
                }
                Action::ScrollUp => {
                    self.pager.scroll_up(1);
                }
                Action::ScrollDown => {
                    self.pager.scroll_down(1);
                }
                Action::HalfPageUp => {
                    self.pager.scroll_up(10);
                }
                Action::HalfPageDown => {
                    self.pager.scroll_down(10);
                }
                Action::PageUp => {
                    self.pager.scroll_up(20);
                }
                Action::PageDown => {
                    self.pager.scroll_down(20);
                }
                Action::Top => {
                    self.pager.scroll_to_top();
                }
                Action::Bottom => {
                    self.pager.scroll_to_bottom(20); // Will be clamped in draw
                }
                Action::Search => {
                    self.state = AppState::Searching;
                    self.search_input.clear();
                    self.key_handler.reset_pending();
                }
                Action::NextMatch => {
                    self.pager.next_match();
                }
                Action::PrevMatch => {
                    self.pager.prev_match();
                }
                Action::OpenFinder => {
                    if !self.subcommands.is_empty() {
                        self.finder = Some(Finder::new(self.subcommands.clone()));
                        self.state = AppState::Finding;
                        self.key_handler.reset_pending();
                    } else {
                        self.error_message = Some("No subcommands found".to_string());
                    }
                }
                Action::OpenCommand => {
                    self.switcher = Some(CommandSwitcher::new(self.command_history.clone()));
                    self.state = AppState::Switching;
                    self.key_handler.reset_pending();
                }
                Action::Back => {
                    self.go_back()?;
                }
                Action::ShowHelp => {
                    self.state = AppState::Help;
                    self.key_handler.reset_pending();
                }
            }
        }
        Ok(())
    }

    fn handle_searching_key(&mut self, key: KeyEvent) -> Result<()> {
        match key.code {
            KeyCode::Esc => {
                self.state = AppState::Paging;
                self.search_input.clear();
            }
            KeyCode::Enter => {
                self.pager.set_search(&self.search_input);
                self.state = AppState::Paging;
            }
            KeyCode::Backspace => {
                self.search_input.pop();
                // Live search update
                self.pager.set_search(&self.search_input);
            }
            KeyCode::Char(c) => {
                self.search_input.push(c);
                // Live search update
                self.pager.set_search(&self.search_input);
            }
            _ => {}
        }
        Ok(())
    }

    fn handle_finding_key(&mut self, key: KeyEvent) -> Result<()> {
        if let Some(ref mut finder) = self.finder {
            match finder.handle_key(key) {
                FinderAction::Close => {
                    self.finder = None;
                    self.state = AppState::Paging;
                }
                FinderAction::Select => {
                    if let Some(item) = finder.selected_item() {
                        let subcmd = item.name.clone();
                        self.drill_into_subcommand(&subcmd)?;
                    }
                }
                FinderAction::None => {}
            }
        }
        Ok(())
    }

    fn handle_switching_key(&mut self, key: KeyEvent) -> Result<()> {
        if let Some(ref mut switcher) = self.switcher {
            match switcher.handle_key(key) {
                SwitcherAction::Close => {
                    self.switcher = None;
                    self.state = AppState::Paging;
                }
                SwitcherAction::Select(cmd) => {
                    self.switch_to_command(&cmd)?;
                }
                SwitcherAction::None => {}
            }
        }
        Ok(())
    }

    fn handle_help_key(&mut self, key: KeyEvent) -> Result<()> {
        // Any key closes the help overlay
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('?') | KeyCode::Enter => {
                self.state = AppState::Paging;
            }
            _ => {}
        }
        Ok(())
    }

    fn drill_into_subcommand(&mut self, subcmd: &str) -> Result<()> {
        // Save current state to history
        self.history.push(
            self.current_command.clone(),
            self.pager.scroll,
        );

        // Build new command
        let mut new_cmd = self.current_command.clone();
        new_cmd.push(subcmd.to_string());

        // Fetch help for new command
        match fetch_help(&new_cmd, &self.config) {
            Ok(help_text) => {
                self.subcommands = parse_subcommands(&help_text, &self.config);
                self.pager = Pager::new(help_text);
                self.current_command = new_cmd;
                self.finder = None;
                self.state = AppState::Paging;
            }
            Err(e) => {
                // Restore from history on failure
                self.history.pop();
                self.error_message = Some(format!("Could not fetch help: {}", e));
                self.finder = None;
                self.state = AppState::Paging;
            }
        }

        Ok(())
    }

    fn go_back(&mut self) -> Result<()> {
        if let Some(entry) = self.history.pop() {
            match fetch_help(&entry.command, &self.config) {
                Ok(help_text) => {
                    self.subcommands = parse_subcommands(&help_text, &self.config);
                    self.pager = Pager::new(help_text);
                    self.pager.scroll = entry.scroll_position;
                    self.current_command = entry.command;
                }
                Err(e) => {
                    self.error_message = Some(format!("Could not go back: {}", e));
                }
            }
        }
        Ok(())
    }

    fn switch_to_command(&mut self, cmd: &str) -> Result<()> {
        let new_command = vec![cmd.to_string()];

        match fetch_help(&new_command, &self.config) {
            Ok(help_text) => {
                // Add to command history if not already present
                if !self.command_history.contains(&cmd.to_string()) {
                    self.command_history.push(cmd.to_string());
                }

                // Clear navigation history since we're switching to a new command
                self.history = History::new();

                self.subcommands = parse_subcommands(&help_text, &self.config);
                self.pager = Pager::new(help_text);
                self.current_command = new_command;
                self.switcher = None;
                self.state = AppState::Paging;
            }
            Err(e) => {
                self.error_message = Some(format!("Could not fetch help for '{}': {}", cmd, e));
                self.switcher = None;
                self.state = AppState::Paging;
            }
        }

        Ok(())
    }
}

struct ErrorMessage<'a>(&'a str);

impl Widget for ErrorMessage<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        use ratatui::style::{Color, Style};
        use ratatui::text::Span;

        let style = Style::default().fg(Color::White).bg(Color::Red);

        // Clear the line
        for x in area.left()..area.right() {
            buf[(x, area.y)].set_style(style);
            buf[(x, area.y)].set_char(' ');
        }

        let msg = format!(" Error: {} ", self.0);
        let span = Span::styled(msg, style);
        buf.set_span(area.x, area.y, &span, area.width);
    }
}
