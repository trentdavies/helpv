use regex::Regex;

use crate::config::Config;

#[derive(Debug, Clone)]
pub struct Subcommand {
    pub name: String,
    pub description: Option<String>,
    /// Category label for discovered items (e.g., "All Commands", "Guides")
    pub label: Option<String>,
    /// Custom invoke command for discovered items (e.g., "git help {name}")
    pub invoke_command: Option<String>,
}

pub fn parse_subcommands(help_text: &str, config: &Config) -> Vec<Subcommand> {
    let mut subcommands = Vec::new();

    for pattern in &config.subcommand_patterns {
        let section_re = match Regex::new(&pattern.section) {
            Ok(re) => re,
            Err(_) => continue,
        };

        let entry_re = match Regex::new(&pattern.entry) {
            Ok(re) => re,
            Err(_) => continue,
        };

        let mut in_section = false;
        let mut blank_line_count = 0;

        for line in help_text.lines() {
            if section_re.is_match(line) {
                in_section = true;
                blank_line_count = 0;
                continue;
            }

            if in_section {
                if line.trim().is_empty() {
                    blank_line_count += 1;
                    if blank_line_count >= 2 {
                        in_section = false;
                    }
                    continue;
                }

                // Check if this looks like a new section header
                if !line.starts_with(' ') && !line.starts_with('\t') && line.ends_with(':') {
                    in_section = false;
                    continue;
                }

                blank_line_count = 0;

                if let Some(captures) = entry_re.captures(line)
                    && let Some(name_match) = captures.get(1)
                {
                    let name = name_match.as_str().to_string();
                    let description = captures.get(2).map(|m| m.as_str().trim().to_string());

                    // Skip if this looks like a flag rather than a subcommand
                    if name.starts_with('-') {
                        continue;
                    }

                    // Avoid duplicates
                    if !subcommands.iter().any(|s: &Subcommand| s.name == name) {
                        subcommands.push(Subcommand {
                            name,
                            description,
                            label: None,
                            invoke_command: None,
                        });
                    }
                }
            }
        }
    }

    // Try git-style parsing if we found nothing
    if subcommands.is_empty() {
        subcommands = parse_git_style(help_text);
    }

    // Try aggressive pattern if still nothing
    if subcommands.is_empty() {
        subcommands = parse_aggressive(help_text);
    }

    subcommands
}

/// Parse git-style help format where:
/// - Section headers are non-indented descriptive text (possibly with parenthetical)
/// - Commands are 3-space indented: `   cmd      Description`
fn parse_git_style(help_text: &str) -> Vec<Subcommand> {
    let mut subcommands = Vec::new();

    // Git uses exactly 3 spaces, then command, then 2+ spaces, then description
    let entry_re = Regex::new(r"^   ([a-z][\w-]*)\s{2,}(.+)$").unwrap();

    // Track if we're past the usage block and into command listings
    let mut past_usage = false;
    let mut in_command_section = false;

    for line in help_text.lines() {
        // Skip until we're past the usage: block
        if line.starts_with("usage:") || line.starts_with("Usage:") {
            past_usage = false;
            continue;
        }

        // Blank line after usage block signals we might be entering commands
        if !past_usage && line.trim().is_empty() {
            past_usage = true;
            continue;
        }

        if !past_usage {
            continue;
        }

        // Non-indented, non-empty line could be a section header
        // Git uses lines like "start a working area (see also: git help tutorial)"
        if !line.starts_with(' ') && !line.starts_with('\t') && !line.trim().is_empty() {
            // Check if this looks like a git section header (lowercase, possibly with parenthetical)
            let trimmed = line.trim();
            if trimmed
                .chars()
                .next()
                .map(|c| c.is_lowercase())
                .unwrap_or(false)
                || trimmed.contains("(see also:")
            {
                in_command_section = true;
                continue;
            }

            // Check for the footer lines that signal end of commands
            if trimmed.starts_with('\'') || trimmed.starts_with('"') {
                in_command_section = false;
                continue;
            }
        }

        if in_command_section
            && let Some(captures) = entry_re.captures(line)
            && let Some(name_match) = captures.get(1)
        {
            let name = name_match.as_str().to_string();
            let description = captures.get(2).map(|m| m.as_str().trim().to_string());

            if !subcommands.iter().any(|s: &Subcommand| s.name == name) {
                subcommands.push(Subcommand {
                    name,
                    description,
                    label: None,
                    invoke_command: None,
                });
            }
        }
    }

    subcommands
}

fn parse_aggressive(help_text: &str) -> Vec<Subcommand> {
    let mut subcommands = Vec::new();

    // Look for common patterns like "  command    Description" or "  command:   Description"
    let entry_re = Regex::new(r"^\s{2,6}([a-z][\w-]*):?\s{2,}(.*)$").unwrap();

    let mut in_likely_section = false;

    for line in help_text.lines() {
        let lower = line.to_lowercase();

        // Detect section headers - broader matching
        if lower.contains("command") || lower.contains("subcommand") || lower.contains("available")
        {
            in_likely_section = true;
            continue;
        }

        if in_likely_section {
            if line.trim().is_empty() {
                continue;
            }

            // End section on non-indented, non-empty line that doesn't look like a category
            if !line.starts_with(' ') && !line.starts_with('\t') {
                // Keep going if it looks like a category header (lowercase start, or contains 'see also')
                let trimmed = line.trim();
                if !trimmed
                    .chars()
                    .next()
                    .map(|c| c.is_lowercase())
                    .unwrap_or(false)
                    && !lower.contains("command")
                    && !lower.contains("see also")
                {
                    in_likely_section = false;
                }
                continue;
            }

            if let Some(captures) = entry_re.captures(line)
                && let Some(name_match) = captures.get(1)
            {
                let name = name_match.as_str().to_string();
                let description = captures.get(2).map(|m| m.as_str().trim().to_string());

                if !name.starts_with('-')
                    && !subcommands.iter().any(|s: &Subcommand| s.name == name)
                {
                    subcommands.push(Subcommand {
                        name,
                        description,
                        label: None,
                        invoke_command: None,
                    });
                }
            }
        }
    }

    subcommands
}
