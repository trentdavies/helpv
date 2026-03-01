use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent};
use ratatui::{
    Frame,
    buffer::Buffer,
    layout::Rect,
    style::Color,
    widgets::{Clear, Widget},
};
use std::sync::mpsc;
use std::time::Duration;

use crate::{
    config::Config,
    fetcher::{ContentSource, fetch_best_content, fetch_help_with_invoke},
    finder::{Finder, FinderAction, FinderWidget},
    history::History,
    keys::{Action, KeyHandler},
    pager::{HelpOverlay, Pager, PagerWidget, SearchInput},
    parser::{Subcommand, parse_subcommands},
    switcher::{CommandSwitcher, SwitcherAction, SwitcherWidget},
    toolpacks::ToolPacks,
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
    prev_state: AppState,
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
    pub content_source: ContentSource,
    discovery_receiver: Option<mpsc::Receiver<Vec<Subcommand>>>,
}

impl App {
    pub fn new(command: Vec<String>, config: Config) -> Result<Self> {
        let (content, source) = fetch_best_content(&command, &config)?;
        let subcommands = parse_subcommands(&content, &config);

        let key_handler = KeyHandler::new(config.keys.clone());
        let initial_cmd = command[0].clone();

        // Spawn background discovery (man -k + toolpacks) — results arrive via channel
        let receiver = spawn_discovery(&command[0], &config.toolpacks);

        Ok(Self {
            state: AppState::Paging,
            prev_state: AppState::Paging,
            pager: Pager::new(content),
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
            content_source: source,
            discovery_receiver: Some(receiver),
        })
    }

    pub fn run(
        &mut self,
        terminal: &mut ratatui::Terminal<impl ratatui::backend::Backend>,
    ) -> Result<()> {
        while !self.should_quit {
            terminal.draw(|frame| self.draw(frame))?;
            self.handle_events()?;
        }
        Ok(())
    }

    fn draw(&mut self, frame: &mut Frame) {
        let area = frame.area();

        // Clear entire screen on any state transition to prevent artifacts
        if self.prev_state != self.state {
            frame.render_widget(Clear, area);
        }
        self.prev_state = self.state;

        // Clamp scroll based on current viewport
        self.pager
            .clamp_scroll(area.height.saturating_sub(1) as usize);

        // Draw the pager
        let breadcrumb = self.history.full_breadcrumb(&self.current_command);
        let pager_widget = PagerWidget::new(
            &self.pager,
            &breadcrumb,
            self.subcommands.len(),
            self.content_source,
        );
        frame.render_widget(pager_widget, area);

        // Draw overlays based on state
        match self.state {
            AppState::Searching => {
                let status_area = Rect::new(area.x, area.bottom() - 1, area.width, 1);
                frame.render_widget(SearchInput::new(&self.search_input), status_area);
            }
            AppState::Finding => {
                frame.render_widget(Dim, area);
                if let Some(ref mut finder) = self.finder {
                    frame.render_widget(FinderWidget::new(finder), area);
                }
            }
            AppState::Switching => {
                frame.render_widget(Dim, area);
                if let Some(ref switcher) = self.switcher {
                    frame.render_widget(SwitcherWidget::new(switcher), area);
                }
            }
            AppState::Help => {
                frame.render_widget(Dim, area);
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
        self.poll_discovery();

        if event::poll(Duration::from_millis(100))? {
            match event::read()? {
                Event::Key(key) => {
                    self.error_message = None; // Clear error on any key press
                    self.handle_key(key)?;
                }
                Event::Resize(_, _) => {
                    // Terminal resize: redraw will happen automatically on next frame
                    // Just need to handle the event to trigger the redraw cycle
                }
                _ => {}
            }
        }
        Ok(())
    }

    fn poll_discovery(&mut self) {
        if let Some(ref rx) = self.discovery_receiver {
            match rx.try_recv() {
                Ok(discovered) => {
                    merge_discovered_items(&mut self.subcommands, discovered);
                    self.discovery_receiver = None;
                }
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.discovery_receiver = None;
                }
                Err(mpsc::TryRecvError::Empty) => {}
            }
        }
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
                        let item_clone = item.clone();
                        self.drill_into_item(&item_clone)?;
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

    fn drill_into_item(&mut self, item: &Subcommand) -> Result<()> {
        // Save current state to history (including content source)
        self.history.push(
            self.current_command.clone(),
            self.pager.scroll,
            self.content_source,
        );

        let base_cmd = self.current_command[0].clone();
        let is_man_invoke = item
            .invoke_command
            .as_ref()
            .is_some_and(|cmd| cmd.starts_with("man "));

        // Check if this item has a custom invoke command
        let result = if let Some(ref invoke_cmd) = item.invoke_command {
            // Use custom invoke command (e.g., for git guides or man pages)
            fetch_help_with_invoke(&base_cmd, &item.name, invoke_cmd).map(|text| {
                (
                    text,
                    if is_man_invoke {
                        ContentSource::Man
                    } else {
                        ContentSource::Help
                    },
                )
            })
        } else {
            // Standard subcommand navigation with thin-content upgrade
            let mut new_cmd = self.current_command.clone();
            new_cmd.push(item.name.clone());
            fetch_best_content(&new_cmd, &self.config)
        };

        match result {
            Ok((content, source)) => {
                let mut subcommands = parse_subcommands(&content, &self.config);

                // If using custom invoke, we stay at the same command level
                // Otherwise, we're drilling into a subcommand
                if item.invoke_command.is_some() {
                    // For custom invokes (like guides or man pages), don't change current_command
                    // Discover man pages from SEE ALSO if this is man page content
                    if source == ContentSource::Man {
                        let see_also = parse_see_also(&content, &base_cmd);
                        merge_discovered_items(&mut subcommands, see_also);
                    }
                } else {
                    let mut new_cmd = self.current_command.clone();
                    new_cmd.push(item.name.clone());
                    self.current_command = new_cmd;

                    // Spawn background discovery for the base command
                    self.discovery_receiver =
                        Some(spawn_discovery(&base_cmd, &self.config.toolpacks));
                }

                self.content_source = source;
                self.subcommands = subcommands;
                self.pager = Pager::new(content);
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
            match fetch_best_content(&entry.command, &self.config) {
                Ok((content, _source)) => {
                    let subcommands = parse_subcommands(&content, &self.config);
                    let base_cmd = entry.command[0].clone();

                    self.subcommands = subcommands;
                    self.pager = Pager::new(content);
                    self.pager.scroll = entry.scroll_position;
                    self.current_command = entry.command;
                    self.content_source = entry.source;

                    // Spawn background discovery
                    self.discovery_receiver =
                        Some(spawn_discovery(&base_cmd, &self.config.toolpacks));
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

        match fetch_best_content(&new_command, &self.config) {
            Ok((content, source)) => {
                // Add to command history if not already present
                if !self.command_history.contains(&cmd.to_string()) {
                    self.command_history.push(cmd.to_string());
                }

                // Clear navigation history since we're switching to a new command
                self.history = History::new();

                let subcommands = parse_subcommands(&content, &self.config);

                self.subcommands = subcommands;
                self.pager = Pager::new(content);
                self.current_command = new_command;
                self.content_source = source;
                self.switcher = None;
                self.state = AppState::Paging;

                // Spawn background discovery for the new command
                self.discovery_receiver = Some(spawn_discovery(cmd, &self.config.toolpacks));
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

/// Spawn a background thread that runs both discovery sources (toolpacks + man -k)
/// and sends the combined results back via a channel.
fn spawn_discovery(base_cmd: &str, toolpacks: &ToolPacks) -> mpsc::Receiver<Vec<Subcommand>> {
    let (tx, rx) = mpsc::channel();
    let base_cmd = base_cmd.to_string();
    let toolpacks = toolpacks.clone();

    std::thread::spawn(move || {
        let results = run_discovery(&base_cmd, &toolpacks);
        // Send silently fails if receiver was dropped (e.g. user navigated away) — that's fine
        let _ = tx.send(results);
    });

    rx
}

/// Run both discovery sources in parallel using scoped threads.
fn run_discovery(base_cmd: &str, toolpacks: &ToolPacks) -> Vec<Subcommand> {
    let mut all = Vec::new();

    std::thread::scope(|s| {
        let toolpack_handle = s.spawn(|| discover_items(base_cmd, toolpacks));
        let man_handle = s.spawn(|| discover_man_pages(base_cmd));

        if let Ok(items) = toolpack_handle.join() {
            all.extend(items);
        }
        if let Ok(pages) = man_handle.join() {
            merge_discovered_items(&mut all, pages);
        }
    });

    all
}

/// Run discovery sources for a tool and return discovered items as Subcommands
fn discover_items(base_cmd: &str, toolpacks: &ToolPacks) -> Vec<Subcommand> {
    let Some(pack) = toolpacks.get(base_cmd) else {
        return Vec::new();
    };

    pack.discover_items(base_cmd)
        .into_iter()
        .map(|item| Subcommand {
            name: item.name,
            description: item.description,
            label: Some(item.label),
            invoke_command: Some(item.invoke_template),
        })
        .collect()
}

/// Discover man pages matching `<base>-*` via `man -k`
fn discover_man_pages(base_cmd: &str) -> Vec<Subcommand> {
    use regex::Regex;
    use std::process::Command;

    let pattern = format!("^{}-", regex::escape(base_cmd));
    let Ok(output) = Command::new("man").args(["-k", &pattern]).output() else {
        return Vec::new();
    };

    if !output.status.success() {
        return Vec::new();
    }

    let text = String::from_utf8_lossy(&output.stdout);
    // man -k output format: "name (section) - description" or "name(section) - description"
    let entry_re = Regex::new(r"^([\w][\w.-]*)\s*\(\d+\)\s*-\s*(.*)$").unwrap();

    text.lines()
        .filter_map(|line| {
            let caps = entry_re.captures(line.trim())?;
            let name = caps.get(1)?.as_str().to_string();
            let description = caps.get(2).map(|m| m.as_str().trim().to_string());

            // Only include pages that start with base_cmd-
            if !name.starts_with(&format!("{}-", base_cmd)) {
                return None;
            }

            Some(Subcommand {
                name: name.clone(),
                description,
                label: Some("Man Pages".to_string()),
                invoke_command: Some(format!("man {}", name)),
            })
        })
        .collect()
}

/// Parse SEE ALSO section from man page content to discover related pages
fn parse_see_also(content: &str, base_cmd: &str) -> Vec<Subcommand> {
    use regex::Regex;

    let mut in_see_also = false;
    let mut see_also_text = String::new();

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed == "SEE ALSO" || trimmed == "See Also" || trimmed == "SEE  ALSO" {
            in_see_also = true;
            continue;
        }
        if in_see_also {
            // End of section: next non-indented header
            if !trimmed.is_empty()
                && !trimmed.starts_with(' ')
                && trimmed.chars().next().is_some_and(|c| c.is_uppercase())
                && !trimmed.contains('(')
            {
                break;
            }
            see_also_text.push_str(line);
            see_also_text.push(' ');
        }
    }

    if see_also_text.is_empty() {
        return Vec::new();
    }

    let entry_re = Regex::new(r"([\w][\w.-]*)\(\d+\)").unwrap();
    let prefix = format!("{}-", base_cmd);

    entry_re
        .captures_iter(&see_also_text)
        .filter_map(|caps| {
            let name = caps.get(1)?.as_str().to_string();
            // Only include pages related to the base command
            if !name.starts_with(&prefix) && name != base_cmd {
                return None;
            }
            // Skip the base command itself
            if name == base_cmd {
                return None;
            }
            Some(Subcommand {
                name: name.clone(),
                description: None,
                label: Some("Man Pages".to_string()),
                invoke_command: Some(format!("man {}", name)),
            })
        })
        .collect()
}

/// Merge discovered items into the subcommands list, avoiding duplicates
fn merge_discovered_items(subcommands: &mut Vec<Subcommand>, discovered: Vec<Subcommand>) {
    for item in discovered {
        // Skip if there's already a subcommand with this name
        if !subcommands.iter().any(|s| s.name == item.name) {
            subcommands.push(item);
        }
    }
}

struct Dim;

impl Widget for Dim {
    fn render(self, area: Rect, buf: &mut Buffer) {
        for y in area.top()..area.bottom() {
            for x in area.left()..area.right() {
                buf[(x, y)].set_fg(Color::DarkGray);
            }
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================
    // parse_see_also tests
    // ========================================

    #[test]
    fn see_also_standard_format() {
        let content = "\
NAME
       git-log - Show commit logs

DESCRIPTION
       Shows the commit log.

SEE ALSO
       git-diff(1), git-show(1), git-format-patch(1), unrelated-tool(1)

AUTHOR
       Written by Linus Torvalds
";
        let results = parse_see_also(content, "git");
        let names: Vec<&str> = results.iter().map(|s| s.name.as_str()).collect();
        assert!(names.contains(&"git-diff"));
        assert!(names.contains(&"git-show"));
        assert!(names.contains(&"git-format-patch"));
        assert!(!names.contains(&"unrelated-tool"));
        assert!(
            results
                .iter()
                .all(|s| s.label.as_deref() == Some("Man Pages"))
        );
        assert!(results.iter().all(|s| s.invoke_command.is_some()));
    }

    #[test]
    fn see_also_mixed_entries() {
        let content = "\
SEE ALSO
       curl-config(1), libcurl(3), curl-easy-init(3)
";
        let results = parse_see_also(content, "curl");
        let names: Vec<&str> = results.iter().map(|s| s.name.as_str()).collect();
        assert!(names.contains(&"curl-config"));
        assert!(names.contains(&"curl-easy-init"));
        // libcurl doesn't start with "curl-"
        assert!(!names.contains(&"libcurl"));
    }

    #[test]
    fn see_also_no_section() {
        let content = "\
NAME
       foo - does things

DESCRIPTION
       It does things.
";
        let results = parse_see_also(content, "foo");
        assert!(results.is_empty());
    }

    #[test]
    fn see_also_skips_base_command_itself() {
        let content = "\
SEE ALSO
       git(1), git-log(1)
";
        let results = parse_see_also(content, "git");
        let names: Vec<&str> = results.iter().map(|s| s.name.as_str()).collect();
        assert!(!names.contains(&"git"));
        assert!(names.contains(&"git-log"));
    }
}
