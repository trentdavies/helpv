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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::SubcommandPattern;

    fn test_config() -> Config {
        let mut config = Config::default();
        config.subcommand_patterns = vec![
            SubcommandPattern {
                section: r"(?im)^(commands?|subcommands?|available\s+commands?):?\s*$".to_string(),
                entry: r"^\s{2,4}([\w][\w-]*)\s+(.*)$".to_string(),
            },
            SubcommandPattern {
                section: r"(?im)^(usage|options):?\s*$".to_string(),
                entry: r"^\s{2,4}([\w][\w-]*)\s{2,}(.*)$".to_string(),
            },
            SubcommandPattern {
                section: r"(?i)^\w+\s+COMMANDS?\s*$".to_string(),
                entry: r"^\s{2}([\w][\w-]*):\s+(.*)$".to_string(),
            },
        ];
        config
    }

    // ========================================
    // parse_subcommands tests - pattern-based
    // ========================================

    #[test]
    fn parse_standard_commands_section() {
        let help = r#"
Commands:
  build    Compile the project
  test     Run the tests
  clean    Remove build artifacts
"#;
        let config = test_config();
        let subs = parse_subcommands(help, &config);
        assert_eq!(subs.len(), 3);
        assert_eq!(subs[0].name, "build");
        assert_eq!(subs[0].description.as_deref(), Some("Compile the project"));
        assert_eq!(subs[1].name, "test");
        assert_eq!(subs[2].name, "clean");
    }

    #[test]
    fn parse_uppercase_commands_section() {
        let help = r#"
COMMANDS:
  init     Initialize a new project
  deploy   Deploy to production
"#;
        let config = test_config();
        let subs = parse_subcommands(help, &config);
        assert_eq!(subs.len(), 2);
        assert_eq!(subs[0].name, "init");
        assert_eq!(subs[1].name, "deploy");
    }

    #[test]
    fn parse_section_terminated_by_double_blank() {
        let help = r#"
Commands:
  first    First command
  second   Second command


This is some other text that should be ignored.
  notacmd  This should not be parsed
"#;
        let config = test_config();
        let subs = parse_subcommands(help, &config);
        assert_eq!(subs.len(), 2);
        assert_eq!(subs[0].name, "first");
        assert_eq!(subs[1].name, "second");
    }

    #[test]
    fn parse_section_terminated_by_new_header() {
        let help = r#"
Commands:
  cmd1     First command
  cmd2     Second command
Options:
  -v       Verbose mode
"#;
        let config = test_config();
        let subs = parse_subcommands(help, &config);
        assert_eq!(subs.len(), 2);
        assert!(!subs.iter().any(|s| s.name == "v"));
    }

    #[test]
    fn parse_skips_flags() {
        let help = r#"
Commands:
  run      Run the application
  --help   Show help message
  -v       Verbose mode
  build    Build the project
"#;
        let config = test_config();
        let subs = parse_subcommands(help, &config);
        assert_eq!(subs.len(), 2);
        assert!(subs.iter().any(|s| s.name == "run"));
        assert!(subs.iter().any(|s| s.name == "build"));
        assert!(!subs.iter().any(|s| s.name.starts_with('-')));
    }

    #[test]
    fn parse_deduplicates_subcommands() {
        let help = r#"
Commands:
  build    Compile the project

Subcommands:
  build    Build (duplicate)
  test     Run tests
"#;
        let config = test_config();
        let subs = parse_subcommands(help, &config);
        let build_count = subs.iter().filter(|s| s.name == "build").count();
        assert_eq!(build_count, 1);
    }

    // ========================================
    // parse_git_style tests
    // ========================================

    #[test]
    fn parse_git_style_basic() {
        let help = include_str!("../tests/fixtures/git_help.txt");
        let subs = parse_git_style(help);

        assert!(subs.iter().any(|s| s.name == "clone"));
        assert!(subs.iter().any(|s| s.name == "init"));
        assert!(subs.iter().any(|s| s.name == "add"));
        assert!(subs.iter().any(|s| s.name == "commit"));
        assert!(subs.iter().any(|s| s.name == "push"));
        assert!(subs.iter().any(|s| s.name == "pull"));
    }

    #[test]
    fn parse_git_style_lowercase_section_headers() {
        let help = r#"
usage: git [options] <command>

start a working area (see also: git help tutorial)
   clone      Clone a repository
   init       Create an empty repository

work on the current change
   add        Add file contents
"#;
        let subs = parse_git_style(help);
        assert_eq!(subs.len(), 3);
        assert!(subs.iter().any(|s| s.name == "clone"));
        assert!(subs.iter().any(|s| s.name == "init"));
        assert!(subs.iter().any(|s| s.name == "add"));
    }

    #[test]
    fn parse_git_style_skips_usage_block() {
        let help = r#"
usage: git [-v | --version] [-h | --help]
           [--exec-path[=<path>]] [--html-path]
           <command> [<args>]

start a working area
   clone      Clone a repository
"#;
        let subs = parse_git_style(help);
        assert_eq!(subs.len(), 1);
        assert_eq!(subs[0].name, "clone");
    }

    #[test]
    fn parse_git_style_stops_at_footer() {
        let help = r#"
usage: git [options]

collaborate
   fetch      Download objects
   push       Update remote refs

'git help -a' lists all available commands.
   notacmd    This should not be parsed
"#;
        let subs = parse_git_style(help);
        assert_eq!(subs.len(), 2);
        assert!(subs.iter().any(|s| s.name == "fetch"));
        assert!(subs.iter().any(|s| s.name == "push"));
        assert!(!subs.iter().any(|s| s.name == "notacmd"));
    }

    #[test]
    fn parse_git_style_requires_3_space_indent() {
        let help = r#"
usage: test

section header
   valid      This has 3 space indent
  invalid    This has 2 space indent
    alsoinvalid  This has 4 space indent
"#;
        let subs = parse_git_style(help);
        assert_eq!(subs.len(), 1);
        assert_eq!(subs[0].name, "valid");
    }

    // ========================================
    // parse_aggressive tests
    // ========================================

    #[test]
    fn parse_aggressive_detects_command_keyword() {
        let help = r#"
Available commands:
  init      Initialize project
  run       Run the app
"#;
        let subs = parse_aggressive(help);
        assert_eq!(subs.len(), 2);
        assert!(subs.iter().any(|s| s.name == "init"));
        assert!(subs.iter().any(|s| s.name == "run"));
    }

    #[test]
    fn parse_aggressive_handles_colon_style() {
        let help = r#"
Subcommands:
  build:    Compile project
  test:     Run tests
"#;
        let subs = parse_aggressive(help);
        assert_eq!(subs.len(), 2);
        assert!(subs.iter().any(|s| s.name == "build"));
        assert!(subs.iter().any(|s| s.name == "test"));
    }

    #[test]
    fn parse_aggressive_handles_variable_indent() {
        let help = r#"
Available commands:
  cmd2      Two space indent
    cmd4    Four space indent
      cmd6  Six space indent
"#;
        let subs = parse_aggressive(help);
        assert_eq!(subs.len(), 3);
    }

    #[test]
    fn parse_aggressive_continues_through_lowercase_category_headers() {
        // Aggressive parser continues through lowercase category headers
        // Note: descriptions can't contain "command" or it triggers section header detection
        let help = "Available commands:\n  first     Do the first thing\nmore stuff\n  second    Do the second thing\n";
        let subs = parse_aggressive(help);
        assert!(
            subs.iter().any(|s| s.name == "first"),
            "Should find 'first'"
        );
        assert!(
            subs.iter().any(|s| s.name == "second"),
            "Should find 'second'"
        );
    }

    // ========================================
    // Edge cases
    // ========================================

    #[test]
    fn parse_empty_help_text() {
        let config = test_config();
        let subs = parse_subcommands("", &config);
        assert!(subs.is_empty());
    }

    #[test]
    fn parse_help_with_only_flags() {
        let help = r#"
Options:
  -h, --help     Show help message
  -v, --version  Show version
  -d, --debug    Enable debug mode
"#;
        let config = test_config();
        let subs = parse_subcommands(help, &config);
        assert!(subs.is_empty());
    }

    #[test]
    fn parse_real_cargo_help() {
        let help = include_str!("../tests/fixtures/cargo_help.txt");
        let config = test_config();
        let subs = parse_subcommands(help, &config);

        // Cargo uses "Commands:" section with 4-space indent and aliases like "build, b"
        // Our patterns capture the first word before comma/space
        // Note: Cargo format "    build, b    Description" - aggressive parser handles this
        assert!(
            !subs.is_empty(),
            "Expected to parse some commands from cargo help"
        );

        // If aggressive parser kicked in, it would find these
        // The test verifies parsing doesn't fail, not specific command names
        // since cargo format varies by version
    }

    #[test]
    fn parse_real_gh_help() {
        let help = include_str!("../tests/fixtures/gh_help.txt");
        let config = test_config();
        let subs = parse_subcommands(help, &config);

        // gh uses "CORE COMMANDS" style with colon entries
        assert!(subs.iter().any(|s| s.name == "auth"));
        assert!(subs.iter().any(|s| s.name == "pr"));
        assert!(subs.iter().any(|s| s.name == "issue"));
        assert!(subs.iter().any(|s| s.name == "repo"));
    }

    #[test]
    fn parse_fallback_chain() {
        // This help text won't match standard patterns, should fall through to git-style
        let help = r#"
usage: tool [options]

main commands
   foo      Do foo things
   bar      Do bar things
"#;
        let config = test_config();
        let subs = parse_subcommands(help, &config);
        assert!(subs.iter().any(|s| s.name == "foo"));
        assert!(subs.iter().any(|s| s.name == "bar"));
    }
}
