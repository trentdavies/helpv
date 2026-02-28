## ADDED Requirements

### Requirement: Benchmark measures time-to-content-ready
The benchmark SHALL measure the elapsed time for helpv to reach content-ready state (help text fetched, parsed, subcommands discovered) WITHOUT terminal initialization or rendering.

#### Scenario: Benchmark times App::new() for a given command
- **WHEN** the benchmark runs for command "git"
- **THEN** it measures only the duration of `App::new(["git"], config)`, excluding terminal setup and rendering

### Requirement: Benchmark compares against man baseline
The benchmark SHALL time `man <command>` (with MANPAGER=cat) for the same command and compare helpv's load time against it.

#### Scenario: Man baseline is measured for comparison
- **WHEN** the benchmark runs for command "tmux"
- **THEN** it also times `man tmux` with MANPAGER=cat and reports both durations

### Requirement: Benchmark enforces pass/fail against man baseline
The benchmark SHALL fail (non-zero exit code) when helpv takes longer than `man` for the same command.

#### Scenario: helpv slower than man causes failure
- **WHEN** helpv takes 150ms and man takes 100ms for "git"
- **THEN** the benchmark exits with non-zero status and reports the failure

#### Scenario: helpv faster than man passes
- **WHEN** helpv takes 80ms and man takes 100ms for "git"
- **THEN** the benchmark exits with zero status

### Requirement: Benchmark outputs machine-readable results
The benchmark SHALL output JSON results containing: command name, helpv duration (ms), man duration (ms), pass/fail status, and ratio.

#### Scenario: JSON output for agent consumption
- **WHEN** the benchmark completes for commands ["git", "tmux"]
- **THEN** it prints a JSON object with per-command results and an overall pass/fail

### Requirement: Benchmark tests git and tmux
The benchmark SHALL include `git` and `tmux` as default benchmark targets. Both are expected to fail the baseline comparison with the current implementation.

#### Scenario: Default commands are benchmarked
- **WHEN** the benchmark runs with no arguments
- **THEN** it benchmarks both "git" and "tmux"

### Requirement: Benchmark is runnable without a terminal
The benchmark SHALL execute without requiring a TTY, alternate screen, or raw mode — enabling headless execution by CI or agents.

#### Scenario: Agent runs benchmark in non-interactive shell
- **WHEN** an agent executes `cargo bench --bench load_time`
- **THEN** it completes without hanging or requiring terminal input
