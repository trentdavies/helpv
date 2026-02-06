use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Widget, Wrap},
};
use regex::Regex;

pub struct Pager {
    pub content: Vec<String>,
    pub scroll: usize,
    pub search_query: Option<String>,
    pub search_matches: Vec<usize>,
    pub current_match: usize,
    search_regex: Option<Regex>,
}

impl Pager {
    pub fn new(content: String) -> Self {
        let lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();
        Self {
            content: lines,
            scroll: 0,
            search_query: None,
            search_matches: Vec::new(),
            current_match: 0,
            search_regex: None,
        }
    }

    pub fn scroll_down(&mut self, amount: usize) {
        self.scroll = self.scroll.saturating_add(amount);
    }

    pub fn scroll_up(&mut self, amount: usize) {
        self.scroll = self.scroll.saturating_sub(amount);
    }

    pub fn scroll_to_top(&mut self) {
        self.scroll = 0;
    }

    pub fn scroll_to_bottom(&mut self, viewport_height: usize) {
        if self.content.len() > viewport_height {
            self.scroll = self.content.len() - viewport_height;
        }
    }

    pub fn clamp_scroll(&mut self, viewport_height: usize) {
        let max_scroll = self.content.len().saturating_sub(viewport_height);
        self.scroll = self.scroll.min(max_scroll);
    }

    pub fn set_search(&mut self, query: &str) {
        if query.is_empty() {
            self.clear_search();
            return;
        }

        self.search_query = Some(query.to_string());
        self.search_regex = Regex::new(&regex::escape(query)).ok();
        self.search_matches.clear();
        self.current_match = 0;

        // Find all matching lines
        if self.search_regex.is_some() {
            for (i, line) in self.content.iter().enumerate() {
                if line.to_lowercase().contains(&query.to_lowercase()) {
                    self.search_matches.push(i);
                }
            }
        }
    }

    pub fn clear_search(&mut self) {
        self.search_query = None;
        self.search_regex = None;
        self.search_matches.clear();
        self.current_match = 0;
    }

    pub fn next_match(&mut self) {
        if self.search_matches.is_empty() {
            return;
        }

        self.current_match = (self.current_match + 1) % self.search_matches.len();
        self.scroll = self.search_matches[self.current_match];
    }

    pub fn prev_match(&mut self) {
        if self.search_matches.is_empty() {
            return;
        }

        if self.current_match == 0 {
            self.current_match = self.search_matches.len() - 1;
        } else {
            self.current_match -= 1;
        }
        self.scroll = self.search_matches[self.current_match];
    }

    pub fn match_count(&self) -> usize {
        self.search_matches.len()
    }

    pub fn current_match_index(&self) -> usize {
        self.current_match
    }

    pub fn scroll_percentage(&self, viewport_height: usize) -> u16 {
        if self.content.len() <= viewport_height {
            return 100;
        }

        let max_scroll = self.content.len() - viewport_height;
        ((self.scroll as f64 / max_scroll as f64) * 100.0) as u16
    }
}

pub struct PagerWidget<'a> {
    pager: &'a Pager,
    breadcrumb: &'a str,
    subcommand_count: usize,
}

impl<'a> PagerWidget<'a> {
    pub fn new(pager: &'a Pager, breadcrumb: &'a str, subcommand_count: usize) -> Self {
        Self {
            pager,
            breadcrumb,
            subcommand_count,
        }
    }
}

impl Widget for PagerWidget<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // Clear entire area first to prevent artifacts from persisting
        Clear.render(area, buf);

        let chunks = Layout::vertical([Constraint::Min(1), Constraint::Length(1)]).split(area);

        let content_area = chunks[0];
        let status_area = chunks[1];

        // Render content
        let viewport_height = content_area.height as usize;
        let visible_lines: Vec<Line> = self
            .pager
            .content
            .iter()
            .enumerate()
            .skip(self.pager.scroll)
            .take(viewport_height)
            .map(|(line_num, line)| {
                let is_match_line = self.pager.search_matches.contains(&line_num);
                let is_current_match = !self.pager.search_matches.is_empty()
                    && self.pager.search_matches.get(self.pager.current_match) == Some(&line_num);

                if let Some(ref query) = self.pager.search_query {
                    highlight_line(line, query, is_match_line, is_current_match)
                } else {
                    Line::raw(line.as_str())
                }
            })
            .collect();

        let content = Paragraph::new(visible_lines).wrap(Wrap { trim: false });
        content.render(content_area, buf);

        // Render status bar
        render_status_bar(
            status_area,
            buf,
            self.breadcrumb,
            self.subcommand_count,
            &self.pager.search_query,
            self.pager.match_count(),
            self.pager.current_match_index(),
            self.pager.scroll_percentage(viewport_height),
        );
    }
}

fn highlight_line(
    line: &str,
    query: &str,
    is_match_line: bool,
    is_current_match: bool,
) -> Line<'static> {
    if !is_match_line {
        return Line::raw(line.to_string());
    }

    let mut spans = Vec::new();
    let lower_line = line.to_lowercase();
    let lower_query = query.to_lowercase();
    let mut last_end = 0;

    for (start, _) in lower_line.match_indices(&lower_query) {
        if start > last_end {
            spans.push(Span::raw(line[last_end..start].to_string()));
        }

        let match_style = if is_current_match {
            Style::default()
                .fg(Color::Black)
                .bg(Color::Cyan)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Black).bg(Color::Yellow)
        };

        spans.push(Span::styled(
            line[start..start + query.len()].to_string(),
            match_style,
        ));
        last_end = start + query.len();
    }

    if last_end < line.len() {
        spans.push(Span::raw(line[last_end..].to_string()));
    }

    Line::from(spans)
}

#[allow(clippy::too_many_arguments)]
fn render_status_bar(
    area: Rect,
    buf: &mut Buffer,
    breadcrumb: &str,
    subcommand_count: usize,
    search_query: &Option<String>,
    match_count: usize,
    current_match: usize,
    scroll_pct: u16,
) {
    let status_style = Style::default().bg(Color::DarkGray).fg(Color::White);

    // Fill background
    for x in area.left()..area.right() {
        buf[(x, area.y)].set_style(status_style);
        buf[(x, area.y)].set_char(' ');
    }

    // Left: breadcrumb
    let breadcrumb_span = Span::styled(format!(" {} ", breadcrumb), status_style);
    buf.set_span(
        area.x,
        area.y,
        &breadcrumb_span,
        breadcrumb.len() as u16 + 2,
    );

    // Build right side info
    let mut right_parts = Vec::new();

    if let Some(query) = search_query {
        if match_count > 0 {
            right_parts.push(format!(
                "/{} ({}/{})",
                query,
                current_match + 1,
                match_count
            ));
        } else {
            right_parts.push(format!("/{} (no matches)", query));
        }
    }

    if subcommand_count > 0 {
        right_parts.push(format!("[f] {} subcmds", subcommand_count));
    }

    right_parts.push(format!("{}%", scroll_pct));
    right_parts.push("[?]help [q]quit".to_string());

    let right_str = right_parts.join(" │ ");
    let right_span = Span::styled(format!("{} ", right_str), status_style);

    let right_x = area.right().saturating_sub(right_str.len() as u16 + 1);
    if right_x > area.x + breadcrumb.len() as u16 + 3 {
        buf.set_span(right_x, area.y, &right_span, right_str.len() as u16 + 1);
    }
}

pub struct SearchInput<'a> {
    query: &'a str,
}

impl<'a> SearchInput<'a> {
    pub fn new(query: &'a str) -> Self {
        Self { query }
    }
}

impl Widget for SearchInput<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let style = Style::default().fg(Color::White).bg(Color::DarkGray);

        // Clear the line
        for x in area.left()..area.right() {
            buf[(x, area.y)].set_style(style);
            buf[(x, area.y)].set_char(' ');
        }

        let prompt = format!("/{}", self.query);
        let span = Span::styled(prompt, style);
        buf.set_span(area.x, area.y, &span, area.width);
    }
}

pub struct HelpOverlay;

impl Widget for HelpOverlay {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let help_text = r#"
  helpv - Help Viewer

  Navigation:
    j, ↓         Scroll down
    k, ↑         Scroll up
    d, Ctrl-d    Half page down
    u, Ctrl-u    Half page up
    f, Ctrl-f    Full page down
    b, Ctrl-b    Full page up
    gg, Home     Go to top
    G, End       Go to bottom

  Search:
    /            Start search
    n            Next match
    N            Previous match
    Escape       Clear search

  Subcommands:
    f            Open subcommand finder
    Enter        Select subcommand
    Backspace    Go back to parent
    o            Open different command

  General:
    ?            Show this help
    q, Escape    Quit / Close overlay
"#;

        let lines: Vec<&str> = help_text.lines().collect();
        let height = lines.len().min(area.height as usize);
        let width = lines
            .iter()
            .map(|l| l.len())
            .max()
            .unwrap_or(40)
            .min(area.width as usize - 4);

        let x = area.x + (area.width.saturating_sub(width as u16 + 4)) / 2;
        let y = area.y + (area.height.saturating_sub(height as u16 + 2)) / 2;

        let overlay_area = Rect::new(x, y, width as u16 + 4, height as u16 + 2);

        // Clear the area behind the overlay
        Clear.render(overlay_area, buf);

        // Draw border
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan))
            .style(Style::default().bg(Color::Black));

        let inner = block.inner(overlay_area);
        block.render(overlay_area, buf);

        // Draw content
        for (i, line) in lines.iter().take(inner.height as usize).enumerate() {
            let span = Span::styled(
                line.chars().take(inner.width as usize).collect::<String>(),
                Style::default().fg(Color::White),
            );
            buf.set_span(inner.x, inner.y + i as u16, &span, inner.width);
        }
    }
}
