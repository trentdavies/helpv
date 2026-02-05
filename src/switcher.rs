use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use nucleo::{Config as NucleoConfig, Matcher, Utf32Str};
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::Span,
    widgets::{Block, Borders, Clear, Widget},
};

pub struct CommandSwitcher {
    history: Vec<String>,
    pub query: String,
    filtered: Vec<(u16, usize)>, // (score, index into history)
    pub selected: usize,
    matcher: Matcher,
}

impl CommandSwitcher {
    pub fn new(history: Vec<String>) -> Self {
        let mut switcher = Self {
            history,
            query: String::new(),
            filtered: Vec::new(),
            selected: 0,
            matcher: Matcher::new(NucleoConfig::DEFAULT),
        };
        switcher.update_filtered();
        switcher
    }

    pub fn push_char(&mut self, c: char) {
        self.query.push(c);
        self.update_filtered();
        self.selected = 0;
    }

    pub fn pop_char(&mut self) {
        self.query.pop();
        self.update_filtered();
        self.selected = 0;
    }

    fn update_filtered(&mut self) {
        self.filtered.clear();

        if self.query.is_empty() {
            // Show all history items when query is empty
            self.filtered = self
                .history
                .iter()
                .enumerate()
                .map(|(i, _)| (0, i))
                .collect();
            return;
        }

        let mut needle_buf = Vec::new();
        let needle = Utf32Str::new(&self.query, &mut needle_buf);

        for (i, cmd) in self.history.iter().enumerate() {
            let mut haystack_buf = Vec::new();
            let haystack = Utf32Str::new(cmd, &mut haystack_buf);

            if let Some(score) = self.matcher.fuzzy_match(haystack, needle) {
                self.filtered.push((score, i));
            }
        }

        // Sort by score (highest first)
        self.filtered.sort_by(|a, b| b.0.cmp(&a.0));
    }

    pub fn move_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
    }

    pub fn move_down(&mut self) {
        let max_idx = if self.filtered.is_empty() && !self.query.is_empty() {
            0 // Allow selecting the typed query as new command
        } else {
            self.filtered.len().saturating_sub(1)
        };
        if self.selected < max_idx {
            self.selected += 1;
        }
    }

    pub fn selected_command(&self) -> Option<String> {
        // If we have filtered results, return the selected one
        if let Some((_, idx)) = self.filtered.get(self.selected) {
            return Some(self.history[*idx].clone());
        }

        // If query is not empty but no matches, return the query as a new command
        if !self.query.is_empty() {
            return Some(self.query.clone());
        }

        None
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> SwitcherAction {
        match key.code {
            KeyCode::Esc => SwitcherAction::Close,
            KeyCode::Enter => {
                if let Some(cmd) = self.selected_command() {
                    SwitcherAction::Select(cmd)
                } else {
                    SwitcherAction::None
                }
            }
            KeyCode::Up => {
                self.move_up();
                SwitcherAction::None
            }
            KeyCode::Down => {
                self.move_down();
                SwitcherAction::None
            }
            KeyCode::Char('p') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.move_up();
                SwitcherAction::None
            }
            KeyCode::Char('n') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.move_down();
                SwitcherAction::None
            }
            KeyCode::Backspace => {
                self.pop_char();
                SwitcherAction::None
            }
            KeyCode::Char(c) => {
                self.push_char(c);
                SwitcherAction::None
            }
            _ => SwitcherAction::None,
        }
    }

    #[allow(dead_code)]
    pub fn filtered_count(&self) -> usize {
        self.filtered.len()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SwitcherAction {
    None,
    Close,
    Select(String),
}

pub struct SwitcherWidget<'a> {
    switcher: &'a CommandSwitcher,
}

impl<'a> SwitcherWidget<'a> {
    pub fn new(switcher: &'a CommandSwitcher) -> Self {
        Self { switcher }
    }
}

impl Widget for SwitcherWidget<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // Calculate overlay dimensions
        let width = (area.width * 2 / 3).min(60).max(30);
        let height = (area.height * 2 / 3).min(20).max(8);

        let x = area.x + (area.width - width) / 2;
        let y = area.y + (area.height - height) / 2;

        let overlay_area = Rect::new(x, y, width, height);

        // Clear the area
        Clear.render(overlay_area, buf);

        // Draw border
        let title = if self.switcher.history.is_empty() {
            " Open Command ".to_string()
        } else {
            format!(
                " Open Command ({} recent) ",
                self.switcher.history.len()
            )
        };
        let block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Magenta))
            .style(Style::default().bg(Color::Black));

        let inner = block.inner(overlay_area);
        block.render(overlay_area, buf);

        // Draw search input
        let input_line = format!("> {}", self.switcher.query);
        let input_span = Span::styled(&input_line, Style::default().fg(Color::Yellow));
        buf.set_span(inner.x, inner.y, &input_span, inner.width);

        // Draw hint
        let hint = if self.switcher.query.is_empty() {
            "Type a command name..."
        } else {
            ""
        };
        if !hint.is_empty() {
            let hint_x = inner.x + input_line.len() as u16;
            let hint_span = Span::styled(hint, Style::default().fg(Color::DarkGray));
            buf.set_span(hint_x, inner.y, &hint_span, inner.width.saturating_sub(input_line.len() as u16));
        }

        // Draw separator
        let separator = "─".repeat(inner.width as usize);
        let sep_span = Span::styled(separator, Style::default().fg(Color::DarkGray));
        buf.set_span(inner.x, inner.y + 1, &sep_span, inner.width);

        // Draw items
        let items_start_y = inner.y + 2;
        let items_height = inner.height.saturating_sub(2) as usize;

        if self.switcher.filtered.is_empty() && !self.switcher.query.is_empty() {
            // Show the query as a new command option
            let style = Style::default()
                .fg(Color::Black)
                .bg(Color::Magenta)
                .add_modifier(Modifier::BOLD);
            let mut line = format!("▶ {} (new)", self.switcher.query);
            while line.len() < inner.width as usize {
                line.push(' ');
            }
            let span = Span::styled(line, style);
            buf.set_span(inner.x, items_start_y, &span, inner.width);
        } else {
            for (i, (_, idx)) in self
                .switcher
                .filtered
                .iter()
                .take(items_height)
                .enumerate()
            {
                let cmd = &self.switcher.history[*idx];
                let y = items_start_y + i as u16;

                let is_selected = i == self.switcher.selected;
                let style = if is_selected {
                    Style::default()
                        .fg(Color::Black)
                        .bg(Color::Magenta)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White)
                };

                let mut line = if is_selected { "▶ " } else { "  " }.to_string();
                line.push_str(cmd);

                // Truncate if too long
                if line.len() > inner.width as usize {
                    line.truncate(inner.width as usize - 3);
                    line.push_str("...");
                }

                // Pad to full width for selection highlight
                while line.len() < inner.width as usize {
                    line.push(' ');
                }

                let span = Span::styled(line, style);
                buf.set_span(inner.x, y, &span, inner.width);
            }
        }

        // Show help hint at bottom if space allows
        if inner.height > 4 {
            let help_y = overlay_area.bottom() - 2;
            let help_text = "Enter: select │ Esc: cancel";
            let help_span = Span::styled(help_text, Style::default().fg(Color::DarkGray));
            let help_x = inner.x + (inner.width.saturating_sub(help_text.len() as u16)) / 2;
            buf.set_span(help_x, help_y, &help_span, help_text.len() as u16);
        }
    }
}
