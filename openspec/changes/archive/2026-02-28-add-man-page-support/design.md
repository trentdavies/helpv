## Context

helpv currently fetches help via a fallback chain: configured help flags → `--help` → `-h` → `base help sub` → `man`. Man pages only appear when everything else fails. But many commands (e.g., `zoxide`, `tmux`, `curl`) have thin `--help` output — a few lines of usage — while their man pages contain detailed documentation. The viewer should automatically detect this and show the better content.

The fetcher already has `try_man_page()` which joins command parts with `-` and strips formatting. The app tracks `current_command: Vec<String>` and a `History` stack for back-navigation. Subcommand discovery runs via `ToolPacks` (label/run/pattern/invoke config). The pager status bar shows a breadcrumb and subcommand count.

## Goals / Non-Goals

**Goals:**
- Model sources as a graph: each command has multiple sources (--help, man page, toolpack discovery), each producing its own content and its own children
- Present all discovered sources in the finder, labeled by origin — selecting one fetches that source's content
- Auto-detect thin help output on initial load and upgrade to man page content
- Track content source per navigation level for correct back-navigation and status display
- Indicate content source in the status bar

**Non-Goals:**
- Separate `helpv man <cmd>` invocation mode — no CLI changes
- Man page section selection (e.g., `man 3 printf`) — always use default section
- Rendering man page formatting (bold, underline) — continue stripping to plain text
- Changing the existing help fallback chain order — upgrade happens after fetch, not during

## Decisions

### 1. Upgrade check lives in a new function, not inside `fetch_help`

`fetch_help` stays as-is — it returns whatever the fallback chain produces. A new function `fetch_best_content` wraps it: call `fetch_help`, check if the result is thin, try the man page, return whichever is better along with a source tag.

**Why not modify `fetch_help`**: The fallback chain already includes man pages as a last resort (when `--help` fails entirely). The upgrade logic is different — it's "help succeeded but was thin, and man is richer." Mixing these concerns makes the fallback chain harder to reason about. A wrapper keeps both paths clean.

**Return type**: A struct or tuple of `(content: String, source: ContentSource)` where `ContentSource` is an enum `{ Help, Man }`.

### 2. Thin-content threshold: line count after trimming blanks

A help output is "thin" if it has fewer than ~10 non-blank lines. This catches the typical pattern of a one-line usage string plus a few flag descriptions.

**Why line count**: Simple, predictable, easy to test. Character count penalizes verbose single-line usage strings. Section-based heuristics ("does it have an OPTIONS section?") are fragile across tools.

**Why 10**: Most thin help is 1-5 lines (usage + a couple flags). Most useful help has 15+ lines with sections. 10 is a safe middle ground — conservative enough to avoid false upgrades. Can be tuned later without architectural changes.

### 3. Content source tracked in `HistoryEntry`, not `App`

Add a `source: ContentSource` field to `HistoryEntry` and track the current source in `App`. When going back, restore the source from history so the status bar and context are correct.

**Why per-entry**: The source can change at each navigation level. `helpv curl` might show the man page (thin --help), but drilling into a subcommand might use --help. Each level needs its own source tag.

### 4. Man page discovery as a source branch in the graph

Man pages form their own branch of discoverable items, parallel to help-parsed subcommands and toolpack discoveries. Two complementary strategies populate this branch:

**SEE ALSO parsing**: Man pages conventionally end with a SEE ALSO section listing related pages (e.g., `git-log(1), git-commit(1)`). Parse this section with a regex like `([\w-]+)\(\d+\)` to extract page names. Filter to pages sharing the base command prefix.

**Prefix search**: Run `man -k "^<base>-"` (or search the filesystem) to find all man pages matching `<base>-*`. This catches pages not listed in SEE ALSO.

Both produce `Subcommand` entries with `invoke_command` set to `man {name}` and `label` set to "Man Pages". These appear alongside help-parsed subcommands in the finder — they are separate items, not merged or deduplicated with their --help counterparts.

**Why both**: SEE ALSO gives curated, relevant pages with the relationships the author intended. Prefix search catches completeness. Together they cover both quality and coverage.

### 5. Status bar shows content source as a short indicator

When content came from a man page, show `[man]` in the status bar next to the breadcrumb. No indicator for help (it's the default/expected case).

**Why minimal**: The user doesn't need to think about modes. The indicator is informational — "oh, this is the man page" — not a mode switch. Keeping it subtle avoids cluttering the status bar.

### 6. Sources form a graph — each item carries its own source

A command's finder shows items from multiple sources simultaneously. A subcommand parsed from `--help` output is a different item from the same-named page found in `man`'s SEE ALSO. They coexist in the finder with different labels, and selecting one fetches that source's content.

**How it works**: The existing `Subcommand` struct already distinguishes these cases. Items parsed from help text have no `invoke_command` — selecting them fetches via the help chain. Items discovered from man pages carry `invoke_command = "man {name}"` — selecting them fetches the man page. The `label` field identifies the origin ("Man Pages" vs no label for parsed subcommands).

**Example**: `helpv git` shows the finder with:
- `commit`, `log`, `push` — parsed from `git --help` (no label)
- `git-commit`, `git-log`, `git-rebase` — discovered from man pages (label: "Man Pages")
- `attributes`, `revisions` — discovered from toolpack (label: "Guides")

Selecting `commit` (from --help) fetches `git commit --help`. Selecting `git-commit` (from man) fetches `man git-commit`. Different content, different source, same finder.

**Drill-in sets its own source**: When you select an item and drill in, the resulting view's `ContentSource` is determined by how that item was fetched — not inherited from the parent. This is tracked in the history entry so back-navigation restores correctly.

## Risks / Trade-offs

**False upgrades** → The threshold might cause helpv to show a man page when the user expected --help output. Mitigation: conservative threshold (10 lines), and the `[man]` indicator tells users what they're seeing. If this becomes a problem, make the threshold configurable or add a keybinding to toggle between help and man.

**Missing man pages** → Some commands have --help but no man page. The upgrade check handles this naturally — if `try_man_page` returns None, keep the original help. No degradation.

**Man page discovery noise** → `man -k` can return unrelated pages on systems with broad man databases. Mitigation: filter results to only pages starting with the base command name + hyphen.

**SEE ALSO format variation** → Not all man pages follow the `name(section)` convention. Mitigation: the regex is lenient, and prefix search provides backup discovery. Missing some SEE ALSO entries is acceptable.

**Performance** → The upgrade check adds one `man` invocation on every initial load. Mitigation: `man` is fast (~10-50ms), and we only call it when the help output is thin (most commands with rich --help skip the check entirely).
