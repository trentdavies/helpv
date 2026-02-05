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
    pub scroll_offset: usize,
    visible_height: usize,
    matcher: Matcher,
}

impl Finder {
    pub fn new(items: Vec<Subcommand>) -> Self {
        let mut finder = Self {
            items,
            query: String::new(),
            filtered: Vec::new(),
            selected: 0,
            scroll_offset: 0,
            visible_height: 10, // Default, updated during render
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
        self.scroll_offset = 0;
    }

    pub fn push_char(&mut self, c: char) {
        self.query.push(c);
        self.update_filtered();
        self.selected = 0;
        self.scroll_offset = 0;
    }

    pub fn pop_char(&mut self) {
        self.query.pop();
        self.update_filtered();
        self.selected = 0;
        self.scroll_offset = 0;
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

        // Split query into space-separated terms (fzf style)
        let terms: Vec<&str> = self.query.split_whitespace().collect();

        if terms.is_empty() {
            // Query is all whitespace - show all
            self.filtered = self
                .items
                .iter()
                .enumerate()
                .map(|(i, _)| (0, i))
                .collect();
            return;
        }

        for (i, item) in self.items.iter().enumerate() {
            let searchable = match &item.label {
                Some(label) => format!("{} {}", label, item.name),
                None => item.name.clone(),
            };

            let mut haystack_buf = Vec::new();
            let haystack = Utf32Str::new(&searchable, &mut haystack_buf);

            // All terms must match (fzf AND semantics)
            let mut all_match = true;
            let mut total_score: u32 = 0;

            for term in &terms {
                let mut needle_buf = Vec::new();
                let needle = Utf32Str::new(term, &mut needle_buf);

                if let Some(score) = self.matcher.fuzzy_match(haystack, needle) {
                    total_score = total_score.saturating_add(score as u32);
                } else {
                    all_match = false;
                    break;
                }
            }

            if all_match {
                // Use u16::MAX if score overflows, otherwise cast
                let final_score = total_score.min(u16::MAX as u32) as u16;
                self.filtered.push((final_score, i));
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

    pub fn move_up_by(&mut self, n: usize) {
        self.selected = self.selected.saturating_sub(n);
    }

    pub fn move_down_by(&mut self, n: usize) {
        let max_idx = self.filtered.len().saturating_sub(1);
        self.selected = (self.selected + n).min(max_idx);
    }

    pub fn set_visible_height(&mut self, h: usize) {
        self.visible_height = h;
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
            KeyCode::Up if key.modifiers.is_empty() => {
                self.move_up();
                FinderAction::None
            }
            KeyCode::Down if key.modifiers.is_empty() => {
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
            KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.move_up_by(self.visible_height / 2);
                FinderAction::None
            }
            KeyCode::Char('d') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.move_down_by(self.visible_height / 2);
                FinderAction::None
            }
            KeyCode::Char('b') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.move_up_by(self.visible_height);
                FinderAction::None
            }
            KeyCode::Char('f') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.move_down_by(self.visible_height);
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
    finder: &'a mut Finder,
}

impl<'a> FinderWidget<'a> {
    pub fn new(finder: &'a mut Finder) -> Self {
        Self { finder }
    }
}

impl Widget for FinderWidget<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // Use most of the screen - 90% width and height with reasonable minimums
        let width = (area.width * 9 / 10).max(40);
        let height = (area.height * 9 / 10).max(10);

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

        // Draw items with scrolling
        let items_start_y = inner.y + 2;
        let items_height = inner.height.saturating_sub(2) as usize;

        // Update visible height for page navigation
        self.finder.set_visible_height(items_height);

        // Adjust scroll offset to keep selection visible
        if self.finder.selected < self.finder.scroll_offset {
            self.finder.scroll_offset = self.finder.selected;
        } else if self.finder.selected >= self.finder.scroll_offset + items_height {
            self.finder.scroll_offset = self.finder.selected.saturating_sub(items_height - 1);
        }

        let scroll_offset = self.finder.scroll_offset;

        // Render visible items
        for (render_idx, (_, idx)) in self
            .finder
            .filtered
            .iter()
            .skip(scroll_offset)
            .take(items_height)
            .enumerate()
        {
            let item = &self.finder.items[*idx];
            let y = items_start_y + render_idx as u16;
            let actual_idx = scroll_offset + render_idx;

            let is_selected = actual_idx == self.finder.selected;
            let style = if is_selected {
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };

            // Format: [label] name - description (truncated)
            let mut line = if is_selected { "▶ " } else { "  " }.to_string();

            // Show category label for discovered items
            if let Some(ref label) = item.label {
                line.push('[');
                // Abbreviate long labels
                let short_label = if label.len() > 8 {
                    &label[..8]
                } else {
                    label
                };
                line.push_str(short_label);
                line.push_str("] ");
            }

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
