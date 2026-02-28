### Requirement: Thin-content detection

The system SHALL evaluate fetched help content and determine whether it is "thin" — too sparse to be useful as primary documentation. Content is thin when it has fewer than 10 non-blank lines.

#### Scenario: Help output with 3 non-blank lines

- **WHEN** `fetch_help` returns content with 3 non-blank lines (e.g., a usage string and two flags)
- **THEN** the system SHALL classify the content as thin

#### Scenario: Help output with 25 non-blank lines

- **WHEN** `fetch_help` returns content with 25 non-blank lines (sections, options, descriptions)
- **THEN** the system SHALL classify the content as not thin

#### Scenario: Help output with blank lines interspersed

- **WHEN** `fetch_help` returns 20 total lines but only 8 are non-blank
- **THEN** the system SHALL classify the content as thin (8 < 10)

### Requirement: Automatic upgrade to man page on initial load

The system SHALL automatically replace thin help content with man page content when a man page is available. This applies on initial load and on each navigation (drill-in, back).

#### Scenario: Thin help with available man page

- **WHEN** a command's help output is thin
- **AND** a man page exists for that command
- **THEN** the system SHALL display the man page content instead of the help output
- **AND** the content source SHALL be recorded as `Man`

#### Scenario: Thin help with no man page

- **WHEN** a command's help output is thin
- **AND** no man page exists for that command
- **THEN** the system SHALL display the original help output
- **AND** the content source SHALL be recorded as `Help`

#### Scenario: Rich help output

- **WHEN** a command's help output is not thin (>= 10 non-blank lines)
- **THEN** the system SHALL display the help output without attempting a man page lookup
- **AND** the content source SHALL be recorded as `Help`

### Requirement: Content source tracking

The system SHALL track the content source (`Help` or `Man`) for each navigation level. The source is determined by how the content was fetched, not inherited from the parent level.

#### Scenario: Source recorded on initial load with upgrade

- **WHEN** the initial command's help is thin and the man page is shown
- **THEN** the current content source SHALL be `Man`

#### Scenario: Source preserved in history on drill-in

- **WHEN** the user is viewing content with source `Man`
- **AND** the user drills into a subcommand
- **THEN** the parent's source (`Man`) SHALL be saved in the history entry
- **AND** the child's source SHALL be determined independently by its own fetch result

#### Scenario: Source restored on back-navigation

- **WHEN** the user navigates back
- **THEN** the content source SHALL be restored from the history entry
- **AND** the status bar SHALL reflect the restored source

### Requirement: Content source indicator in status bar

The system SHALL display a `[man]` indicator in the status bar when the current content came from a man page. No indicator is shown when content came from `--help`.

#### Scenario: Viewing man page content

- **WHEN** the current content source is `Man`
- **THEN** the status bar SHALL display `[man]` next to the breadcrumb

#### Scenario: Viewing help content

- **WHEN** the current content source is `Help`
- **THEN** the status bar SHALL NOT display any source indicator

### Requirement: Man page discovery populates the source graph

The system SHALL discover man pages as a separate branch of navigable items, parallel to help-parsed subcommands and toolpack discoveries. Man page items SHALL appear in the finder with the label "Man Pages" and SHALL NOT be deduplicated against help-parsed subcommands.

#### Scenario: Man pages discovered via SEE ALSO

- **WHEN** the current content is a man page
- **AND** the man page contains a SEE ALSO section with entries like `git-log(1), git-commit(1)`
- **THEN** the system SHALL parse those entries and create subcommand items with `invoke_command` set to fetch the referenced man page
- **AND** each item SHALL have the label "Man Pages"

#### Scenario: Man pages discovered via prefix search

- **WHEN** viewing a command (regardless of content source)
- **THEN** the system SHALL search for man pages matching `<base-command>-*`
- **AND** each matching page SHALL be added as a subcommand item with `invoke_command` set to fetch that man page
- **AND** each item SHALL have the label "Man Pages"

#### Scenario: Man page items coexist with help-parsed subcommands

- **WHEN** the finder contains `commit` parsed from `git --help` output
- **AND** the man page discovery finds `git-commit`
- **THEN** both items SHALL appear in the finder as separate entries
- **AND** `commit` SHALL have no label (help-parsed)
- **AND** `git-commit` SHALL have the label "Man Pages"

### Requirement: Source-determined drill-in

When the user selects an item from the finder, the system SHALL fetch content using the source associated with that item. The parent's content source does not influence the child's fetch strategy.

#### Scenario: Selecting a help-parsed subcommand

- **WHEN** the user selects a subcommand that was parsed from help text (no `invoke_command`)
- **THEN** the system SHALL fetch content via the help chain (with thin-content upgrade)
- **AND** the resulting content source SHALL be determined by the upgrade check

#### Scenario: Selecting a man page discovery item

- **WHEN** the user selects a subcommand with `invoke_command` pointing to a man page
- **THEN** the system SHALL fetch the man page content using the invoke command
- **AND** the content source SHALL be `Man`

### Requirement: fetch_help unchanged

The existing `fetch_help` function and its fallback chain (configured flags → `--help` → `-h` → `base help sub` → `man`) SHALL remain unchanged. The thin-content upgrade logic SHALL be implemented as a separate wrapper function.

#### Scenario: Commands with rich help

- **WHEN** `fetch_help` returns content with >= 10 non-blank lines
- **THEN** the behavior SHALL be identical to before this change

#### Scenario: Commands where help fails entirely

- **WHEN** `fetch_help` falls through to the man page fallback (because `--help` and `-h` both fail)
- **THEN** the man page SHALL be returned as before — the fallback chain is unchanged
