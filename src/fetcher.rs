use anyhow::{Result, anyhow};
use std::process::Command;

use crate::config::Config;

pub fn fetch_help(cmd: &[String], config: &Config) -> Result<String> {
    if cmd.is_empty() {
        return Err(anyhow!("No command specified"));
    }

    let base_cmd = &cmd[0];
    let is_subcommand = cmd.len() > 1;

    // Choose appropriate help flags based on whether this is a subcommand
    let help_flags = if is_subcommand {
        config.get_subcommand_help_flags(base_cmd)
    } else {
        config.get_help_flags(base_cmd)
    };

    for flag_pattern in &help_flags {
        if let Some(output) = try_help_pattern(cmd, flag_pattern)
            && !output.trim().is_empty()
        {
            return Ok(output);
        }
    }

    // Try man page as fallback
    if let Some(output) = try_man_page(cmd)
        && !output.trim().is_empty()
    {
        return Ok(output);
    }

    Err(anyhow!("Could not fetch help for '{}'", cmd.join(" ")))
}

/// Fetch help using a specific invoke command template
pub fn fetch_help_with_invoke(
    base_cmd: &str,
    item_name: &str,
    invoke_template: &str,
) -> Result<String> {
    let cmd_str = invoke_template
        .replace("{base}", base_cmd)
        .replace("{name}", item_name);

    let parts: Vec<&str> = cmd_str.split_whitespace().collect();
    if parts.is_empty() {
        return Err(anyhow!("Invalid invoke command"));
    }

    let result = Command::new(parts[0]).args(&parts[1..]).output()?;

    // Some tools write help to stderr
    let stdout = String::from_utf8_lossy(&result.stdout);
    let stderr = String::from_utf8_lossy(&result.stderr);

    if !stdout.trim().is_empty() {
        Ok(stdout.into_owned())
    } else if !stderr.trim().is_empty() && (result.status.success() || looks_like_help(&stderr)) {
        Ok(stderr.into_owned())
    } else {
        Err(anyhow!(
            "Could not fetch help for '{} {}'",
            base_cmd,
            item_name
        ))
    }
}

fn try_help_pattern(cmd: &[String], pattern: &str) -> Option<String> {
    let full_cmd = cmd.join(" ");
    let base = &cmd[0];
    let sub = if cmd.len() > 1 {
        cmd[1..].join(" ")
    } else {
        String::new()
    };

    let expanded = pattern
        .replace("{cmd}", &full_cmd)
        .replace("{base}", base)
        .replace("{sub}", &sub);

    let parts: Vec<&str> = expanded.split_whitespace().collect();
    if parts.is_empty() {
        return None;
    }

    let result = Command::new(parts[0]).args(&parts[1..]).output().ok()?;

    // Some tools write help to stderr
    let stdout = String::from_utf8_lossy(&result.stdout);
    let stderr = String::from_utf8_lossy(&result.stderr);

    if !stdout.trim().is_empty() {
        Some(stdout.into_owned())
    } else if !stderr.trim().is_empty() && result.status.success() {
        Some(stderr.into_owned())
    } else if !stderr.trim().is_empty() {
        // Some tools return non-zero but still output help to stderr
        let stderr_str = stderr.into_owned();
        if looks_like_help(&stderr_str) {
            Some(stderr_str)
        } else {
            None
        }
    } else {
        None
    }
}

fn try_man_page(cmd: &[String]) -> Option<String> {
    let man_page = cmd.join("-");

    let result = Command::new("man")
        .arg(&man_page)
        .env("MANPAGER", "cat")
        .env("PAGER", "cat")
        .env("MAN_KEEP_FORMATTING", "0")
        .output()
        .ok()?;

    if result.status.success() {
        let output = String::from_utf8_lossy(&result.stdout);
        // Strip man formatting (backspace sequences)
        Some(strip_man_formatting(&output))
    } else {
        // Try without joining for single commands
        if cmd.len() == 1 {
            let result = Command::new("man")
                .arg(&cmd[0])
                .env("MANPAGER", "cat")
                .env("PAGER", "cat")
                .env("MAN_KEEP_FORMATTING", "0")
                .output()
                .ok()?;

            if result.status.success() {
                let output = String::from_utf8_lossy(&result.stdout);
                return Some(strip_man_formatting(&output));
            }
        }
        None
    }
}

fn strip_man_formatting(text: &str) -> String {
    let mut result = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '\x08' {
            // Backspace - remove previous char
            result.pop();
        } else if c == '\x1b' {
            // Skip ANSI escape sequences
            if chars.peek() == Some(&'[') {
                chars.next();
                while let Some(&nc) = chars.peek() {
                    chars.next();
                    if nc.is_ascii_alphabetic() {
                        break;
                    }
                }
            }
        } else {
            result.push(c);
        }
    }

    result
}

fn looks_like_help(text: &str) -> bool {
    let lower = text.to_lowercase();
    lower.contains("usage:")
        || lower.contains("options:")
        || lower.contains("commands:")
        || lower.contains("--help")
        || lower.contains("synopsis")
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========================================
    // strip_man_formatting tests
    // ========================================

    #[test]
    fn strip_backspace_sequences() {
        // Bold in man pages: char + backspace + char (e.g., N\x08N = bold N)
        let input = "N\x08NA\x08AM\x08ME\x08E";
        let result = strip_man_formatting(input);
        assert_eq!(result, "NAME");
    }

    #[test]
    fn strip_underline_sequences() {
        // Underline in man pages: underscore + backspace + char
        let input = "_\x08f_\x08o_\x08o";
        let result = strip_man_formatting(input);
        assert_eq!(result, "foo");
    }

    #[test]
    fn strip_ansi_escape_codes() {
        let input = "\x1b[1mBold\x1b[0m and \x1b[32mgreen\x1b[0m text";
        let result = strip_man_formatting(input);
        assert_eq!(result, "Bold and green text");
    }

    #[test]
    fn strip_mixed_formatting() {
        let input = "\x1b[1mH\x08He\x08el\x08lp\x08p\x1b[0m - description";
        let result = strip_man_formatting(input);
        assert_eq!(result, "Help - description");
    }

    #[test]
    fn strip_preserves_normal_text() {
        let input = "This is normal text without any formatting";
        let result = strip_man_formatting(input);
        assert_eq!(result, input);
    }

    #[test]
    fn strip_handles_empty_string() {
        let result = strip_man_formatting("");
        assert_eq!(result, "");
    }

    #[test]
    fn strip_handles_newlines() {
        let input = "Line one\nLine two\nLine three";
        let result = strip_man_formatting(input);
        assert_eq!(result, "Line one\nLine two\nLine three");
    }

    #[test]
    fn strip_complex_ansi_sequences() {
        // ANSI with multiple parameters: \x1b[38;5;196m (256-color red)
        let input = "\x1b[38;5;196mred\x1b[0m";
        let result = strip_man_formatting(input);
        assert_eq!(result, "red");
    }

    // ========================================
    // looks_like_help tests
    // ========================================

    #[test]
    fn looks_like_help_usage_lowercase() {
        assert!(looks_like_help("usage: program [options]"));
    }

    #[test]
    fn looks_like_help_usage_titlecase() {
        assert!(looks_like_help("Usage: program [options]"));
    }

    #[test]
    fn looks_like_help_usage_uppercase() {
        assert!(looks_like_help("USAGE: program [options]"));
    }

    #[test]
    fn looks_like_help_options() {
        assert!(looks_like_help("options: -h, --help"));
    }

    #[test]
    fn looks_like_help_commands() {
        assert!(looks_like_help("commands:\n  build  Build the project"));
    }

    #[test]
    fn looks_like_help_double_dash_help() {
        assert!(looks_like_help("Use --help for more information"));
    }

    #[test]
    fn looks_like_help_synopsis() {
        assert!(looks_like_help("SYNOPSIS\n    program [options]"));
    }

    #[test]
    fn looks_like_help_random_text_false() {
        assert!(!looks_like_help("This is just random text"));
    }

    #[test]
    fn looks_like_help_error_message_false() {
        assert!(!looks_like_help("Error: command not found"));
    }

    #[test]
    fn looks_like_help_empty_false() {
        assert!(!looks_like_help(""));
    }

    #[test]
    fn looks_like_help_case_insensitive() {
        assert!(looks_like_help("USAGE: foo"));
        assert!(looks_like_help("Usage: foo"));
        assert!(looks_like_help("usage: foo"));
        assert!(looks_like_help("OPTIONS: bar"));
        assert!(looks_like_help("Options: bar"));
        assert!(looks_like_help("options: bar"));
    }
}
