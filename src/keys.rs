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

        if self
            .config
            .quit
            .iter()
            .any(|k| matches_key(k, &key_str, &key))
        {
            return Some(Action::Quit);
        }
        if self
            .config
            .scroll_up
            .iter()
            .any(|k| matches_key(k, &key_str, &key))
        {
            return Some(Action::ScrollUp);
        }
        if self
            .config
            .scroll_down
            .iter()
            .any(|k| matches_key(k, &key_str, &key))
        {
            return Some(Action::ScrollDown);
        }
        if self
            .config
            .half_page_up
            .iter()
            .any(|k| matches_key(k, &key_str, &key))
        {
            return Some(Action::HalfPageUp);
        }
        if self
            .config
            .half_page_down
            .iter()
            .any(|k| matches_key(k, &key_str, &key))
        {
            return Some(Action::HalfPageDown);
        }
        if self
            .config
            .page_up
            .iter()
            .any(|k| matches_key(k, &key_str, &key))
        {
            return Some(Action::PageUp);
        }
        if self
            .config
            .page_down
            .iter()
            .any(|k| matches_key(k, &key_str, &key))
        {
            return Some(Action::PageDown);
        }
        if self
            .config
            .top
            .iter()
            .any(|k| matches_key(k, &key_str, &key))
        {
            return Some(Action::Top);
        }
        if self
            .config
            .bottom
            .iter()
            .any(|k| matches_key(k, &key_str, &key))
        {
            return Some(Action::Bottom);
        }
        if self
            .config
            .search
            .iter()
            .any(|k| matches_key(k, &key_str, &key))
        {
            return Some(Action::Search);
        }
        if self
            .config
            .next_match
            .iter()
            .any(|k| matches_key(k, &key_str, &key))
        {
            return Some(Action::NextMatch);
        }
        if self
            .config
            .prev_match
            .iter()
            .any(|k| matches_key(k, &key_str, &key))
        {
            return Some(Action::PrevMatch);
        }
        if self
            .config
            .find_subcommand
            .iter()
            .any(|k| matches_key(k, &key_str, &key))
        {
            return Some(Action::OpenFinder);
        }
        if self
            .config
            .open_command
            .iter()
            .any(|k| matches_key(k, &key_str, &key))
        {
            return Some(Action::OpenCommand);
        }
        if self
            .config
            .back
            .iter()
            .any(|k| matches_key(k, &key_str, &key))
        {
            return Some(Action::Back);
        }
        if self
            .config
            .help
            .iter()
            .any(|k| matches_key(k, &key_str, &key))
        {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn make_key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn make_key_ctrl(c: char) -> KeyEvent {
        KeyEvent::new(KeyCode::Char(c), KeyModifiers::CONTROL)
    }

    fn make_key_shift(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::SHIFT)
    }

    // ========================================
    // key_to_string tests
    // ========================================

    #[test]
    fn key_to_string_plain_char() {
        let key = make_key(KeyCode::Char('a'));
        assert_eq!(key_to_string(&key), "a");
    }

    #[test]
    fn key_to_string_uppercase() {
        // Uppercase letters come through as-is
        let key = make_key(KeyCode::Char('G'));
        assert_eq!(key_to_string(&key), "G");
    }

    #[test]
    fn key_to_string_ctrl_modifier() {
        let key = make_key_ctrl('u');
        assert_eq!(key_to_string(&key), "Ctrl-u");
    }

    #[test]
    fn key_to_string_escape() {
        let key = make_key(KeyCode::Esc);
        assert_eq!(key_to_string(&key), "Escape");
    }

    #[test]
    fn key_to_string_enter() {
        let key = make_key(KeyCode::Enter);
        assert_eq!(key_to_string(&key), "Enter");
    }

    #[test]
    fn key_to_string_backspace() {
        let key = make_key(KeyCode::Backspace);
        assert_eq!(key_to_string(&key), "Backspace");
    }

    #[test]
    fn key_to_string_arrows() {
        assert_eq!(key_to_string(&make_key(KeyCode::Up)), "Up");
        assert_eq!(key_to_string(&make_key(KeyCode::Down)), "Down");
        assert_eq!(key_to_string(&make_key(KeyCode::Left)), "Left");
        assert_eq!(key_to_string(&make_key(KeyCode::Right)), "Right");
    }

    #[test]
    fn key_to_string_f_keys() {
        assert_eq!(key_to_string(&make_key(KeyCode::F(1))), "F1");
        assert_eq!(key_to_string(&make_key(KeyCode::F(12))), "F12");
    }

    #[test]
    fn key_to_string_page_keys() {
        assert_eq!(key_to_string(&make_key(KeyCode::PageUp)), "PageUp");
        assert_eq!(key_to_string(&make_key(KeyCode::PageDown)), "PageDown");
    }

    #[test]
    fn key_to_string_home_end() {
        assert_eq!(key_to_string(&make_key(KeyCode::Home)), "Home");
        assert_eq!(key_to_string(&make_key(KeyCode::End)), "End");
    }

    #[test]
    fn key_to_string_shift_special_key() {
        let key = make_key_shift(KeyCode::Up);
        assert_eq!(key_to_string(&key), "Shift-Up");
    }

    // ========================================
    // matches_key tests
    // ========================================

    #[test]
    fn matches_key_direct_string_match() {
        let key = make_key(KeyCode::Char('q'));
        let key_str = key_to_string(&key);
        assert!(matches_key("q", &key_str, &key));
    }

    #[test]
    fn matches_key_space() {
        let key = make_key(KeyCode::Char(' '));
        let key_str = key_to_string(&key);
        assert!(matches_key("Space", &key_str, &key));
    }

    #[test]
    fn matches_key_escape_full() {
        let key = make_key(KeyCode::Esc);
        let key_str = key_to_string(&key);
        assert!(matches_key("Escape", &key_str, &key));
    }

    #[test]
    fn matches_key_esc_shorthand() {
        let key = make_key(KeyCode::Esc);
        let key_str = key_to_string(&key);
        assert!(matches_key("Esc", &key_str, &key));
    }

    #[test]
    fn matches_key_ctrl_u() {
        let key = make_key_ctrl('u');
        let key_str = key_to_string(&key);
        assert!(matches_key("Ctrl-u", &key_str, &key));
    }

    #[test]
    fn matches_key_ctrl_case_insensitive() {
        let key = make_key_ctrl('u');
        let key_str = key_to_string(&key);
        assert!(matches_key("Ctrl-U", &key_str, &key));
    }

    #[test]
    fn matches_key_case_insensitive_fallback() {
        let key = make_key(KeyCode::Char('q'));
        let key_str = key_to_string(&key);
        assert!(matches_key("Q", &key_str, &key));
    }

    #[test]
    fn matches_key_no_false_positive() {
        let key = make_key(KeyCode::Char('a'));
        let key_str = key_to_string(&key);
        assert!(!matches_key("b", &key_str, &key));
    }

    // ========================================
    // KeyHandler gg sequence tests
    // ========================================

    fn default_key_config() -> KeyConfig {
        let mut config = KeyConfig::default();
        config.quit = vec!["q".to_string()];
        config.scroll_up = vec!["k".to_string()];
        config.scroll_down = vec!["j".to_string()];
        config.top = vec!["gg".to_string()];
        config.bottom = vec!["G".to_string()];
        config.half_page_up = vec!["Ctrl-u".to_string()];
        config.half_page_down = vec!["Ctrl-d".to_string()];
        config.page_up = vec!["Ctrl-b".to_string()];
        config.page_down = vec!["Ctrl-f".to_string()];
        config.search = vec!["/".to_string()];
        config.next_match = vec!["n".to_string()];
        config.prev_match = vec!["N".to_string()];
        config.find_subcommand = vec!["f".to_string()];
        config.open_command = vec!["o".to_string()];
        config.back = vec!["Backspace".to_string()];
        config.help = vec!["?".to_string()];
        config
    }

    #[test]
    fn gg_first_g_sets_pending() {
        let mut handler = KeyHandler::new(default_key_config());
        let result = handler.handle(make_key(KeyCode::Char('g')));
        assert!(result.is_none());
        assert!(handler.pending_g);
    }

    #[test]
    fn gg_second_g_returns_top() {
        let mut handler = KeyHandler::new(default_key_config());
        handler.handle(make_key(KeyCode::Char('g'))); // First g
        let result = handler.handle(make_key(KeyCode::Char('g'))); // Second g
        assert_eq!(result, Some(Action::Top));
        assert!(!handler.pending_g);
    }

    #[test]
    fn gg_non_g_after_g_clears_pending() {
        let mut handler = KeyHandler::new(default_key_config());
        handler.handle(make_key(KeyCode::Char('g'))); // First g
        let result = handler.handle(make_key(KeyCode::Char('j'))); // j instead of g
        assert_eq!(result, Some(Action::ScrollDown));
        assert!(!handler.pending_g);
    }

    #[test]
    fn gg_reset_pending_clears_state() {
        let mut handler = KeyHandler::new(default_key_config());
        handler.handle(make_key(KeyCode::Char('g'))); // Set pending
        handler.reset_pending();
        assert!(!handler.pending_g);
    }

    // ========================================
    // KeyHandler action mapping tests
    // ========================================

    #[test]
    fn handler_quit() {
        let mut handler = KeyHandler::new(default_key_config());
        let result = handler.handle(make_key(KeyCode::Char('q')));
        assert_eq!(result, Some(Action::Quit));
    }

    #[test]
    fn handler_scroll_up() {
        let mut handler = KeyHandler::new(default_key_config());
        let result = handler.handle(make_key(KeyCode::Char('k')));
        assert_eq!(result, Some(Action::ScrollUp));
    }

    #[test]
    fn handler_scroll_down() {
        let mut handler = KeyHandler::new(default_key_config());
        let result = handler.handle(make_key(KeyCode::Char('j')));
        assert_eq!(result, Some(Action::ScrollDown));
    }

    #[test]
    fn handler_half_page_up() {
        let mut handler = KeyHandler::new(default_key_config());
        let result = handler.handle(make_key_ctrl('u'));
        assert_eq!(result, Some(Action::HalfPageUp));
    }

    #[test]
    fn handler_half_page_down() {
        let mut handler = KeyHandler::new(default_key_config());
        let result = handler.handle(make_key_ctrl('d'));
        assert_eq!(result, Some(Action::HalfPageDown));
    }

    #[test]
    fn handler_bottom() {
        let mut handler = KeyHandler::new(default_key_config());
        let result = handler.handle(make_key(KeyCode::Char('G')));
        assert_eq!(result, Some(Action::Bottom));
    }

    #[test]
    fn handler_search() {
        let mut handler = KeyHandler::new(default_key_config());
        let result = handler.handle(make_key(KeyCode::Char('/')));
        assert_eq!(result, Some(Action::Search));
    }

    #[test]
    fn handler_open_finder() {
        let mut handler = KeyHandler::new(default_key_config());
        let result = handler.handle(make_key(KeyCode::Char('f')));
        assert_eq!(result, Some(Action::OpenFinder));
    }

    #[test]
    fn handler_unmapped_key_returns_none() {
        let mut handler = KeyHandler::new(default_key_config());
        let result = handler.handle(make_key(KeyCode::Char('z')));
        assert!(result.is_none());
    }
}
