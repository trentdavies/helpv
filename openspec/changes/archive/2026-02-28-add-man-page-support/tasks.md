## 1. Content source type and fetch wrapper

- [x] 1.1 Add `ContentSource` enum (`Help`, `Man`) to `fetcher.rs`
- [x] 1.2 Add `is_thin()` function that counts non-blank lines and returns true if < 10
- [x] 1.3 Add `fetch_best_content()` wrapper: calls `fetch_help`, checks `is_thin`, tries `try_man_page` if thin, returns `(String, ContentSource)`
- [x] 1.4 Add tests for `is_thin` (empty, 3 lines, 8 non-blank with blanks, 10 lines, 25 lines)
- [x] 1.5 Add tests for `fetch_best_content` upgrade logic (mock or use fixture data)

## 2. Source tracking in app state and history

- [x] 2.1 Add `content_source: ContentSource` field to `App`
- [x] 2.2 Add `source: ContentSource` field to `HistoryEntry`
- [x] 2.3 Update `App::new()` to use `fetch_best_content` and store the returned source
- [x] 2.4 Update `drill_into_item()` to save current source in history entry on push, and set source from fetch result on the new view
- [x] 2.5 Update `go_back()` to restore source from the history entry
- [x] 2.6 Update `switch_to_command()` to use `fetch_best_content` and set source

## 3. Status bar source indicator

- [x] 3.1 Pass `content_source` to `PagerWidget` (or its breadcrumb rendering)
- [x] 3.2 Display `[man]` in the status bar when source is `Man`, nothing when `Help`
- [x] 3.3 Verify indicator updates on drill-in, back-navigation, and command switch

## 4. Man page discovery

- [x] 4.1 Add `discover_man_pages()` function: runs `man -k "^<base>-"` and parses results into `Subcommand` entries with `invoke_command = "man {name}"` and `label = "Man Pages"`
- [x] 4.2 Add SEE ALSO parser: given man page content, extract entries matching `([\w-]+)\(\d+\)`, filter to base command prefix, return `Subcommand` entries with `invoke_command` and label
- [x] 4.3 Merge man page discovery results into the subcommand list in `App::new()`, `drill_into_item()`, `go_back()`, and `switch_to_command()`
- [x] 4.4 Add tests for SEE ALSO parsing (standard format, mixed entries, no SEE ALSO section)
- [x] 4.5 Add tests for `discover_man_pages` prefix filtering (excludes unrelated pages)

## 5. Source-determined drill-in

- [x] 5.1 Update `drill_into_item()`: when item has `invoke_command` pointing to a man page, set `content_source` to `Man`; when item has no `invoke_command`, use `fetch_best_content` and take source from result
- [x] 5.2 When drilling into a man page item, run SEE ALSO parsing and man page discovery on the resulting content to populate subcommands for the new view
