use crate::fetcher::ContentSource;

#[derive(Debug, Clone)]
pub struct HistoryEntry {
    pub command: Vec<String>,
    pub scroll_position: usize,
    pub source: ContentSource,
}

#[derive(Debug, Default)]
pub struct History {
    entries: Vec<HistoryEntry>,
}

impl History {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, command: Vec<String>, scroll_position: usize, source: ContentSource) {
        self.entries.push(HistoryEntry {
            command,
            scroll_position,
            source,
        });
    }

    pub fn pop(&mut self) -> Option<HistoryEntry> {
        self.entries.pop()
    }

    #[allow(dead_code)]
    pub fn current(&self) -> Option<&HistoryEntry> {
        self.entries.last()
    }

    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    #[allow(dead_code)]
    pub fn breadcrumb_string(&self, current_cmd: &[String]) -> String {
        let mut parts: Vec<&str> = self
            .entries
            .iter()
            .filter_map(|e| e.command.last().map(|s| s.as_str()))
            .collect();

        if let Some(last) = current_cmd.last() {
            parts.push(last);
        }

        if parts.is_empty() {
            String::new()
        } else {
            parts.join(" > ")
        }
    }

    pub fn full_breadcrumb(&self, current_cmd: &[String]) -> String {
        current_cmd.join(" ")
    }
}
