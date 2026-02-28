## Why

The viewer currently treats man pages as a last-resort fallback — only shown when `--help` fails entirely. But many commands produce thin `--help` output (a few lines of usage) while having rich, detailed man pages. Users shouldn't have to know to run `man` separately; helpv should automatically show the best available documentation.

## What Changes

- After fetching help text, evaluate whether the content is "thin" (just a few lines). If it is and a man page exists for the command, automatically upgrade to the man page content instead.
- The upgrade is transparent — no separate mode, no extra flags. `helpv <command>` always shows the best content available.
- When viewing man page content, subcommand discovery should parse the SEE ALSO section to find related man pages, and/or discover `<command>-*` man pages as navigable items.
- The status bar should indicate when content came from a man page vs `--help` so users understand what they're reading.
- Navigation (drill-in, back) should be content-source-aware: if the parent was upgraded to a man page, drilling into a subcommand should also attempt man page lookup (e.g., `git` man page → drill into `log` → try `man git-log`).

## Capabilities

### New Capabilities

- `man-page-upgrade`: Automatic detection of thin help output and transparent upgrade to man page content. Covers the thin-content threshold, man page availability check, content-source tracking, source-aware navigation, and man-specific subcommand discovery (SEE ALSO parsing).

### Modified Capabilities

_(none — the existing help fetch fallback chain stays as-is; the upgrade happens after fetching, not during)_

## Impact

- **Fetcher**: Needs a post-fetch evaluation step — check line count against a threshold, try man page, decide which to use
- **App state**: Track content source (help vs man) per navigation level so drill-in and back can route correctly
- **Subcommand discovery**: When viewing a man page, parse SEE ALSO for related pages and/or discover `<command>-*` man pages
- **Status bar**: Show content source indicator (e.g., "man" badge) when viewing upgraded content
- **No CLI changes**: `helpv <command>` interface is unchanged
- **No breaking changes**: Commands with sufficient `--help` output behave exactly as before
