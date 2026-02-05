# helpv

A terminal-based help documentation viewer with subcommand navigation. I built this because reading `--help` output in a raw terminal is painful—no search, no navigation, no way to drill into subcommands without running another command.

## The Problem

CLI tools have gotten complex. `git` has 150+ subcommands. `kubectl` has nested subcommands three levels deep. The standard workflow—run `git --help`, scroll with terminal history, run `git remote --help`, repeat—breaks down at scale. You lose context, you lose your place, you're constantly switching between reading and typing.

## The Solution

`helpv` gives you a TUI pager with:

- **Vim-style navigation** (j/k, gg/G, Ctrl-d/u) for moving through help text
- **Live search** with match highlighting (/, n/N)
- **Fuzzy subcommand finder** (f) using nucleo for instant filtering
- **Drill-down navigation** into subcommands with breadcrumb tracking
- **Command switching** (o) to jump between different tools without exiting

## Installation

```bash
cargo install --path .
```

Or build from source:

```bash
cargo build --release
# Binary at ./target/release/helpv
```

## Usage

```bash
helpv git              # View git help
helpv kubectl get      # View kubectl get help
helpv cargo build      # View cargo build help
```

Once inside:

| Key | Action |
|-----|--------|
| j, ↓ | Scroll down |
| k, ↑ | Scroll up |
| d, Ctrl-d | Half page down |
| u, Ctrl-u | Half page up |
| gg, Home | Jump to top |
| G, End | Jump to bottom |
| / | Start search |
| n | Next match |
| N | Previous match |
| f | Open subcommand finder |
| o | Open different command |
| Backspace | Go back to parent |
| ? | Show help overlay |
| q, Esc | Quit |

## How It Works

`helpv` fetches help text by trying strategies in order:

1. `{command} --help`
2. `{command} -h`
3. `{base} help {subcommand}` (for git-style CLIs)
4. `man {command}` (with formatting stripped)

The parser extracts subcommands by detecting section headers (e.g., "Commands:", "Subcommands:") and parsing indented entries beneath them. This is more art than science—CLI help formats vary wildly—but the default patterns handle most common tools.

## Configuration

Config lives at `~/.config/helpv/config.toml`. Optional—sensible defaults work out of the box.

```toml
# Override help-fetching strategy for specific tools
[tools.kubectl]
help_flags = ["{cmd} --help", "kubectl help {cmd}"]

[tools.npm]
help_flags = ["{cmd} --help", "{cmd} -h"]

# Custom subcommand detection patterns
[[subcommand_patterns]]
section = "(?i)commands?:|subcommands?:"
entry = "^\\s{2,4}([\\w-]+)\\s+(.*)$"

# Keybinding overrides
[keys]
quit = ["q", "Escape"]
search = ["/"]
find_subcommand = ["f"]
open_command = ["o"]
back = ["Backspace"]
```
## Limitations

- Subcommand parsing relies on heuristics. Tools with non-standard help formats may not parse correctly—use custom patterns in config as a workaround.
- Man page stripping handles basic formatting but may miss edge cases.
- Command history persists only within a session (i.e., not written to disk).
