use anyhow::Result;
use serde::Deserialize;
use std::collections::HashMap;
use std::path::PathBuf;

use crate::toolpacks::ToolPacks;

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
pub struct Config {
    pub tools: HashMap<String, ToolConfig>,
    #[serde(default)]
    pub subcommand_patterns: Vec<SubcommandPattern>,
    #[serde(default)]
    pub keys: KeyConfig,
    #[serde(skip)]
    pub toolpacks: ToolPacks,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ToolConfig {
    pub help_flags: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SubcommandPattern {
    pub section: String,
    pub entry: String,
}

#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
pub struct KeyConfig {
    pub quit: Vec<String>,
    pub scroll_up: Vec<String>,
    pub scroll_down: Vec<String>,
    pub half_page_up: Vec<String>,
    pub half_page_down: Vec<String>,
    pub page_up: Vec<String>,
    pub page_down: Vec<String>,
    pub top: Vec<String>,
    pub bottom: Vec<String>,
    pub search: Vec<String>,
    pub next_match: Vec<String>,
    pub prev_match: Vec<String>,
    pub find_subcommand: Vec<String>,
    pub open_command: Vec<String>,
    pub back: Vec<String>,
    pub help: Vec<String>,
}

impl Config {
    pub fn load() -> Result<Self> {
        let config_path = Self::config_path();

        let mut config = if config_path.exists() {
            let content = std::fs::read_to_string(&config_path)?;
            let mut config: Config = toml::from_str(&content)?;
            config.apply_defaults();
            config
        } else {
            Self::default_config()
        };

        // Load tool packs
        config.toolpacks = ToolPacks::load()?;

        Ok(config)
    }

    fn config_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("helpv")
            .join("config.toml")
    }

    fn default_config() -> Self {
        let mut config = Config::default();
        config.apply_defaults();
        config
    }

    fn apply_defaults(&mut self) {
        if self.subcommand_patterns.is_empty() {
            self.subcommand_patterns = Self::default_subcommand_patterns();
        }

        self.keys.apply_defaults();
    }

    fn default_subcommand_patterns() -> Vec<SubcommandPattern> {
        vec![
            SubcommandPattern {
                section: r"(?im)^(commands?|subcommands?|available\s+commands?):?\s*$".to_string(),
                entry: r"^\s{2,4}([\w][\w-]*)\s+(.*)$".to_string(),
            },
            SubcommandPattern {
                section: r"(?im)^(usage|options):?\s*$".to_string(),
                entry: r"^\s{2,4}([\w][\w-]*)\s{2,}(.*)$".to_string(),
            },
            // gh-style: "GENERAL COMMANDS" section header with "  cmd:  description" entries
            SubcommandPattern {
                section: r"(?i)^\w+\s+COMMANDS?\s*$".to_string(),
                entry: r"^\s{2}([\w][\w-]*):\s+(.*)$".to_string(),
            },
        ]
    }

    /// Get help flags for a tool (base command only)
    pub fn get_help_flags(&self, tool: &str) -> Vec<String> {
        // User config in config.toml takes precedence
        if let Some(tool_config) = self.tools.get(tool) {
            return tool_config.help_flags.clone();
        }

        // Check toolpacks
        if let Some(pack) = self.toolpacks.get(tool) {
            return pack.get_help_commands();
        }

        // Generic fallback
        vec!["{cmd} --help".to_string(), "{cmd} -h".to_string()]
    }

    /// Get help flags for a subcommand
    pub fn get_subcommand_help_flags(&self, tool: &str) -> Vec<String> {
        // Check toolpacks
        if let Some(pack) = self.toolpacks.get(tool) {
            return pack.get_subcommand_commands();
        }

        // Generic fallback
        vec![
            "{cmd} --help".to_string(),
            "{base} help {sub}".to_string(),
            "{cmd} -h".to_string(),
        ]
    }
}

impl KeyConfig {
    fn apply_defaults(&mut self) {
        if self.quit.is_empty() {
            self.quit = vec!["q".to_string(), "Escape".to_string()];
        }
        if self.scroll_up.is_empty() {
            self.scroll_up = vec!["k".to_string(), "Up".to_string()];
        }
        if self.scroll_down.is_empty() {
            self.scroll_down = vec!["j".to_string(), "Down".to_string()];
        }
        if self.half_page_up.is_empty() {
            self.half_page_up = vec!["Ctrl-u".to_string(), "u".to_string()];
        }
        if self.half_page_down.is_empty() {
            self.half_page_down = vec!["Ctrl-d".to_string(), "d".to_string()];
        }
        if self.page_up.is_empty() {
            self.page_up = vec!["Ctrl-b".to_string(), "b".to_string()];
        }
        if self.page_down.is_empty() {
            self.page_down = vec!["Ctrl-f".to_string(), "Space".to_string()];
        }
        if self.top.is_empty() {
            self.top = vec!["gg".to_string(), "Home".to_string()];
        }
        if self.bottom.is_empty() {
            self.bottom = vec!["G".to_string(), "End".to_string()];
        }
        if self.search.is_empty() {
            self.search = vec!["/".to_string()];
        }
        if self.next_match.is_empty() {
            self.next_match = vec!["n".to_string()];
        }
        if self.prev_match.is_empty() {
            self.prev_match = vec!["N".to_string()];
        }
        if self.find_subcommand.is_empty() {
            self.find_subcommand = vec!["f".to_string()];
        }
        if self.open_command.is_empty() {
            self.open_command = vec!["o".to_string()];
        }
        if self.back.is_empty() {
            self.back = vec!["Backspace".to_string()];
        }
        if self.help.is_empty() {
            self.help = vec!["?".to_string()];
        }
    }
}
