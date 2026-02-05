use anyhow::Result;
use regex::Regex;
use serde::Deserialize;
use std::collections::HashMap;
use std::process::Command;

/// Embedded default tool packs
const DEFAULT_TOOLPACKS: &str = include_str!("toolpacks.toml");

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ToolPacks {
    #[serde(flatten)]
    pub tools: HashMap<String, ToolPack>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ToolPack {
    /// Commands to try for base help (e.g., ["git --help"])
    #[serde(default)]
    pub help: Vec<String>,

    /// Commands to try for subcommand help
    /// Use {base} for base command, {sub} for subcommand
    #[serde(default)]
    pub subcommand: Vec<String>,

    /// Additional discovery sources
    #[serde(default)]
    pub discover: Vec<DiscoverySource>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DiscoverySource {
    /// Label shown in the finder (e.g., "All Commands", "Guides")
    pub label: String,

    /// Command to run to get the listing
    pub run: String,

    /// Regex pattern to extract items
    /// Group 1 = name, Group 2 (optional) = description
    pub pattern: String,

    /// Command to invoke when selecting an item
    /// Use {name} for the item name, {base} for base command
    pub invoke: String,

    /// Optional section header pattern - only parse after matching this
    #[serde(default)]
    pub section: Option<String>,
}

/// An item discovered from a discovery source
#[derive(Debug, Clone)]
pub struct DiscoveredItem {
    pub name: String,
    pub description: Option<String>,
    pub label: String,
    pub invoke_template: String,
}

impl ToolPacks {
    pub fn load() -> Result<Self> {
        // Load embedded defaults
        let mut packs: ToolPacks = toml::from_str(DEFAULT_TOOLPACKS)?;

        // Load user overrides from ~/.config/helpv/tools/*.toml
        if let Some(config_dir) = dirs::config_dir() {
            let tools_dir = config_dir.join("helpv").join("tools");
            if tools_dir.exists()
                && let Ok(entries) = std::fs::read_dir(&tools_dir)
            {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension().map(|e| e == "toml").unwrap_or(false)
                        && let Ok(content) = std::fs::read_to_string(&path)
                        && let Ok(user_packs) = toml::from_str::<ToolPacks>(&content)
                    {
                        // User packs override defaults
                        for (name, pack) in user_packs.tools {
                            packs.tools.insert(name, pack);
                        }
                    }
                }
            }
        }

        Ok(packs)
    }

    pub fn get(&self, tool: &str) -> Option<&ToolPack> {
        self.tools.get(tool)
    }
}

impl ToolPack {
    /// Get help flags for fetching base help
    pub fn get_help_commands(&self) -> Vec<String> {
        if self.help.is_empty() {
            vec!["{cmd} --help".to_string(), "{cmd} -h".to_string()]
        } else {
            self.help.clone()
        }
    }

    /// Get help flags for fetching subcommand help
    pub fn get_subcommand_commands(&self) -> Vec<String> {
        if self.subcommand.is_empty() {
            vec!["{cmd} --help".to_string(), "{base} help {sub}".to_string()]
        } else {
            self.subcommand.clone()
        }
    }

    /// Run all discovery sources and collect items
    pub fn discover_items(&self, base_cmd: &str) -> Vec<DiscoveredItem> {
        let mut items = Vec::new();

        for source in &self.discover {
            if let Ok(discovered) = source.run_discovery(base_cmd) {
                items.extend(discovered);
            }
        }

        items
    }
}

impl DiscoverySource {
    /// Run this discovery source and extract items
    pub fn run_discovery(&self, base_cmd: &str) -> Result<Vec<DiscoveredItem>> {
        let mut items = Vec::new();

        // Build and run the command
        let cmd_str = self.run.replace("{base}", base_cmd);
        let parts: Vec<&str> = cmd_str.split_whitespace().collect();
        if parts.is_empty() {
            return Ok(items);
        }

        let output = Command::new(parts[0]).args(&parts[1..]).output()?;

        if !output.status.success() {
            return Ok(items);
        }

        let text = String::from_utf8_lossy(&output.stdout);

        // Compile patterns
        let entry_re = Regex::new(&self.pattern)?;
        let section_re = self.section.as_ref().and_then(|s| Regex::new(s).ok());

        // Parse the output
        let mut in_section = section_re.is_none(); // If no section pattern, parse everything

        for line in text.lines() {
            // Check for section header
            if let Some(ref re) = section_re
                && re.is_match(line)
            {
                in_section = true;
                continue;
            }

            if !in_section {
                continue;
            }

            // Try to match entry
            if let Some(caps) = entry_re.captures(line)
                && let Some(name_match) = caps.get(1)
            {
                let name = name_match.as_str().to_string();
                let description = caps.get(2).map(|m| m.as_str().trim().to_string());

                // Skip if looks like a flag
                if name.starts_with('-') {
                    continue;
                }

                items.push(DiscoveredItem {
                    name,
                    description,
                    label: self.label.clone(),
                    invoke_template: self.invoke.clone(),
                });
            }
        }

        Ok(items)
    }
}
