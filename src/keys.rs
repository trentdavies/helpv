use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::config::KeyConfig;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    Quit,
    ScrollUp,
    ScrollDown,
    HalfPageUp,
    HalfPageDown,
    PageUp,
    PageDown,
    Top,
    Bottom,
    Search,
    NextMatch,
    PrevMatch,
    OpenFinder,
    OpenCommand,
    Back,
    ShowHelp,
}

pub struct KeyHandler {
    config: KeyConfig,
    pending_g: bool,
}

impl KeyHandler {
    pub fn new(config: KeyConfig) -> Self {
        Self {
            config,
            pending_g: false,
        }
    }

    pub fn handle(&mut self, key: KeyEvent) -> Option<Action> {
        // Handle 'gg' sequence for going to top
        if self.pending_g {
            self.pending_g = false;
            if key.code == KeyCode::Char('g') {
                return Some(Action::Top);
            }
        }

        // Check for 'g' to start 'gg' sequence
        if key.code == KeyCode::Char('g') && key.modifiers.is_empty() {
            self.pending_g = true;
            return None;
        }

        self.match_key(key)
    }

    fn match_key(&self, key: KeyEvent) -> Option<Action> {
        let key_str = key_to_string(&key);

        if self.config.quit.iter().any(|k| matches_key(k, &key_str, &key)) {
            return Some(Action::Quit);
        }
        if self.config.scroll_up.iter().any(|k| matches_key(k, &key_str, &key)) {
            return Some(Action::ScrollUp);
        }
        if self.config.scroll_down.iter().any(|k| matches_key(k, &key_str, &key)) {
            return Some(Action::ScrollDown);
        }
        if self.config.half_page_up.iter().any(|k| matches_key(k, &key_str, &key)) {
            return Some(Action::HalfPageUp);
        }
        if self.config.half_page_down.iter().any(|k| matches_key(k, &key_str, &key)) {
            return Some(Action::HalfPageDown);
        }
        if self.config.page_up.iter().any(|k| matches_key(k, &key_str, &key)) {
            return Some(Action::PageUp);
        }
        if self.config.page_down.iter().any(|k| matches_key(k, &key_str, &key)) {
            return Some(Action::PageDown);
        }
        if self.config.top.iter().any(|k| matches_key(k, &key_str, &key)) {
            return Some(Action::Top);
        }
        if self.config.bottom.iter().any(|k| matches_key(k, &key_str, &key)) {
            return Some(Action::Bottom);
        }
        if self.config.search.iter().any(|k| matches_key(k, &key_str, &key)) {
            return Some(Action::Search);
        }
        if self.config.next_match.iter().any(|k| matches_key(k, &key_str, &key)) {
            return Some(Action::NextMatch);
        }
        if self.config.prev_match.iter().any(|k| matches_key(k, &key_str, &key)) {
            return Some(Action::PrevMatch);
        }
        if self.config.find_subcommand.iter().any(|k| matches_key(k, &key_str, &key)) {
            return Some(Action::OpenFinder);
        }
        if self.config.open_command.iter().any(|k| matches_key(k, &key_str, &key)) {
            return Some(Action::OpenCommand);
        }
        if self.config.back.iter().any(|k| matches_key(k, &key_str, &key)) {
            return Some(Action::Back);
        }
        if self.config.help.iter().any(|k| matches_key(k, &key_str, &key)) {
            return Some(Action::ShowHelp);
        }

        None
    }

    pub fn reset_pending(&mut self) {
        self.pending_g = false;
    }
}

fn key_to_string(key: &KeyEvent) -> String {
    let mut s = String::new();

    if key.modifiers.contains(KeyModifiers::CONTROL) {
        s.push_str("Ctrl-");
    }
    if key.modifiers.contains(KeyModifiers::ALT) {
        s.push_str("Alt-");
    }
    if key.modifiers.contains(KeyModifiers::SHIFT) {
        // For characters, shift is usually implicit in the character
        if !matches!(key.code, KeyCode::Char(_)) {
            s.push_str("Shift-");
        }
    }

    match key.code {
        KeyCode::Char(c) => s.push(c),
        KeyCode::Esc => s.push_str("Escape"),
        KeyCode::Enter => s.push_str("Enter"),
        KeyCode::Backspace => s.push_str("Backspace"),
        KeyCode::Tab => s.push_str("Tab"),
        KeyCode::Up => s.push_str("Up"),
        KeyCode::Down => s.push_str("Down"),
        KeyCode::Left => s.push_str("Left"),
        KeyCode::Right => s.push_str("Right"),
        KeyCode::Home => s.push_str("Home"),
        KeyCode::End => s.push_str("End"),
        KeyCode::PageUp => s.push_str("PageUp"),
        KeyCode::PageDown => s.push_str("PageDown"),
        KeyCode::F(n) => s.push_str(&format!("F{}", n)),
        _ => s.push_str("Unknown"),
    }

    s
}

fn matches_key(pattern: &str, key_str: &str, key: &KeyEvent) -> bool {
    // Direct match
    if pattern == key_str {
        return true;
    }

    // Handle special cases
    match pattern {
        "Space" => key.code == KeyCode::Char(' '),
        "Escape" | "Esc" => key.code == KeyCode::Esc,
        _ if pattern.starts_with("Ctrl-") => {
            let char_part = &pattern[5..];
            if let KeyCode::Char(c) = key.code {
                key.modifiers.contains(KeyModifiers::CONTROL)
                    && c.to_ascii_lowercase().to_string() == char_part.to_lowercase()
            } else {
                false
            }
        }
        _ => pattern.to_lowercase() == key_str.to_lowercase(),
    }
}
