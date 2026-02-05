use anyhow::{anyhow, Result};
use std::process::Command;

use crate::config::Config;

pub fn fetch_help(cmd: &[String], config: &Config) -> Result<String> {
    if cmd.is_empty() {
        return Err(anyhow!("No command specified"));
    }

    let base_cmd = &cmd[0];
    let help_flags = config.get_help_flags(base_cmd);

    for flag_pattern in &help_flags {
        if let Some(output) = try_help_pattern(cmd, flag_pattern)
            && !output.trim().is_empty() {
                return Ok(output);
            }
    }

    // Try man page as fallback
    if let Some(output) = try_man_page(cmd)
        && !output.trim().is_empty() {
            return Ok(output);
        }

    Err(anyhow!(
        "Could not fetch help for '{}'",
        cmd.join(" ")
    ))
}

fn try_help_pattern(cmd: &[String], pattern: &str) -> Option<String> {
    let full_cmd = cmd.join(" ");
    let base = &cmd[0];
    let subcmd = if cmd.len() > 1 {
        cmd[1..].join(" ")
    } else {
        String::new()
    };

    let expanded = pattern
        .replace("{cmd}", &full_cmd)
        .replace("{base}", base)
        .replace("{subcmd}", &subcmd);

    let parts: Vec<&str> = expanded.split_whitespace().collect();
    if parts.is_empty() {
        return None;
    }

    let result = Command::new(parts[0])
        .args(&parts[1..])
        .output()
        .ok()?;

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
