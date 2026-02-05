use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use nucleo::{Config as NucleoConfig, Matcher, Utf32Str};
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::Span,
    widgets::{Block, Borders, Clear, Widget},
};

use crate::parser::Subcommand;

pub struct Finder {
    items: Vec<Subcommand>,
    pub query: String,
    filtered: Vec<(u16, usize)>, // (score, index)
    pub selected: usize,
    matcher: Matcher,
}

impl Finder {
    pub fn new(items: Vec<Subcommand>) -> Self {
        let mut finder = Self {
            items,
            query: String::new(),
            filtered: Vec::new(),
            selected: 0,
            matcher: Matcher::new(NucleoConfig::DEFAULT),
        };
        finder.update_filtered();
        finder
    }

    #[allow(dead_code)]
    pub fn set_query(&mut self, query: String) {
        self.query = query;
        self.update_filtered();
        self.selected = 0;
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
            // Show all items when query is empty
            self.filtered = self
                .items
                .iter()
                .enumerate()
                .map(|(i, _)| (0, i))
                .collect();
            return;
        }

        let mut needle_buf = Vec::new();
        let needle = Utf32Str::new(&self.query, &mut needle_buf);

        for (i, item) in self.items.iter().enumerate() {
            let mut haystack_buf = Vec::new();
            let haystack = Utf32Str::new(&item.name, &mut haystack_buf);

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
        if self.selected + 1 < self.filtered.len() {
            self.selected += 1;
        }
    }

    pub fn selected_item(&self) -> Option<&Subcommand> {
        self.filtered
            .get(self.selected)
            .map(|(_, idx)| &self.items[*idx])
    }

    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn filtered_count(&self) -> usize {
        self.filtered.len()
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> FinderAction {
        match key.code {
            KeyCode::Esc => FinderAction::Close,
            KeyCode::Enter => {
                if self.selected_item().is_some() {
                    FinderAction::Select
                } else {
                    FinderAction::None
                }
            }
            KeyCode::Up | KeyCode::Char('k') if key.modifiers.is_empty() => {
                self.move_up();
                FinderAction::None
            }
            KeyCode::Down | KeyCode::Char('j') if key.modifiers.is_empty() => {
                self.move_down();
                FinderAction::None
            }
            KeyCode::Char('p') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.move_up();
                FinderAction::None
            }
            KeyCode::Char('n') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.move_down();
                FinderAction::None
            }
            KeyCode::Backspace => {
                self.pop_char();
                FinderAction::None
            }
            KeyCode::Char(c) => {
                self.push_char(c);
                FinderAction::None
            }
            _ => FinderAction::None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FinderAction {
    None,
    Close,
    Select,
}

pub struct FinderWidget<'a> {
    finder: &'a Finder,
}

impl<'a> FinderWidget<'a> {
    pub fn new(finder: &'a Finder) -> Self {
        Self { finder }
    }
}

impl Widget for FinderWidget<'_> {
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
        let title = format!(" Subcommands ({}/{}) ", self.finder.filtered_count(), self.finder.items.len());
        let block = Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan))
            .style(Style::default().bg(Color::Black));

        let inner = block.inner(overlay_area);
        block.render(overlay_area, buf);

        // Draw search input
        let input_line = format!("> {}", self.finder.query);
        let input_span = Span::styled(
            &input_line,
            Style::default().fg(Color::Yellow),
        );
        buf.set_span(inner.x, inner.y, &input_span, inner.width);

        // Draw separator
        let separator = "─".repeat(inner.width as usize);
        let sep_span = Span::styled(separator, Style::default().fg(Color::DarkGray));
        buf.set_span(inner.x, inner.y + 1, &sep_span, inner.width);

        // Draw items
        let items_start_y = inner.y + 2;
        let items_height = inner.height.saturating_sub(2) as usize;

        for (i, (_, idx)) in self
            .finder
            .filtered
            .iter()
            .take(items_height)
            .enumerate()
        {
            let item = &self.finder.items[*idx];
            let y = items_start_y + i as u16;

            let is_selected = i == self.finder.selected;
            let style = if is_selected {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };

            // Format: name - description (truncated)
            let mut line = if is_selected { "▶ " } else { "  " }.to_string();
            line.push_str(&item.name);

            if let Some(ref desc) = item.description {
                let remaining = inner.width as usize - line.len() - 3;
                if remaining > 10 {
                    line.push_str(" - ");
                    if desc.len() > remaining {
                        line.push_str(&desc[..remaining - 3]);
                        line.push_str("...");
                    } else {
                        line.push_str(desc);
                    }
                }
            }

            // Pad to full width for selection highlight
            while line.len() < inner.width as usize {
                line.push(' ');
            }

            let span = Span::styled(line, style);
            buf.set_span(inner.x, y, &span, inner.width);
        }

        // Show "no matches" if empty
        if self.finder.filtered.is_empty() && !self.finder.query.is_empty() {
            let msg = "No matching subcommands";
            let msg_span = Span::styled(msg, Style::default().fg(Color::DarkGray));
            buf.set_span(inner.x + 2, items_start_y, &msg_span, inner.width);
        }
    }
}
