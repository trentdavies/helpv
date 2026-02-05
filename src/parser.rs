use regex::Regex;

use crate::config::Config;

#[derive(Debug, Clone)]
pub struct Subcommand {
    pub name: String,
    pub description: Option<String>,
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
                    && let Some(name_match) = captures.get(1) {
                        let name = name_match.as_str().to_string();
                        let description = captures.get(2).map(|m| m.as_str().trim().to_string());

                        // Skip if this looks like a flag rather than a subcommand
                        if name.starts_with('-') {
                            continue;
                        }

                        // Avoid duplicates
                        if !subcommands.iter().any(|s: &Subcommand| s.name == name) {
                            subcommands.push(Subcommand { name, description });
                        }
                    }
            }
        }
    }

    // Try a more aggressive pattern if we found nothing
    if subcommands.is_empty() {
        subcommands = parse_aggressive(help_text);
    }

    subcommands
}

fn parse_aggressive(help_text: &str) -> Vec<Subcommand> {
    let mut subcommands = Vec::new();

    // Look for common patterns like "  command    Description"
    let entry_re = Regex::new(r"^\s{2,6}([a-z][\w-]*)\s{2,}(.*)$").unwrap();

    let mut in_likely_section = false;

    for line in help_text.lines() {
        let lower = line.to_lowercase();

        // Detect section headers
        if lower.contains("command") || lower.contains("subcommand") {
            in_likely_section = true;
            continue;
        }

        if in_likely_section {
            if line.trim().is_empty() {
                continue;
            }

            // End section on non-indented, non-empty line
            if !line.starts_with(' ') && !line.starts_with('\t') {
                if !lower.contains("command") {
                    in_likely_section = false;
                }
                continue;
            }

            if let Some(captures) = entry_re.captures(line)
                && let Some(name_match) = captures.get(1) {
                    let name = name_match.as_str().to_string();
                    let description = captures.get(2).map(|m| m.as_str().trim().to_string());

                    if !name.starts_with('-') && !subcommands.iter().any(|s: &Subcommand| s.name == name) {
                        subcommands.push(Subcommand { name, description });
                    }
                }
        }
    }

    subcommands
}
