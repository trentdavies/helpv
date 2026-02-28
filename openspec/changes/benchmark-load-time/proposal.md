## Why

helpv has no performance regression testing. We need a benchmark that enforces an aggressive startup target — faster than `man` — and catches regressions before they ship. This also gives agents a concrete, automatable gate for future optimization work.

## What Changes

- Add a load-time benchmark suite using `cargo bench` (criterion or similar)
- Benchmark measures time from process start to first content ready (pre-render), not including terminal I/O
- Target: helpv must load faster than `man` for the same command
- Baseline commands: `helpv git` and `helpv tmux` — both expected to **fail** the target with current implementation
- Benchmark outputs machine-readable results (JSON) so agents can parse pass/fail and regressions
- Can be invoked via `cargo bench --bench load_time` with no interactive terminal required

## Capabilities

### New Capabilities
- `load-time-benchmark`: Automated benchmark suite that measures helpv startup latency against `man` as a baseline, outputs structured results, and enforces pass/fail thresholds

### Modified Capabilities
<!-- None — this is a new testing capability, not a behavioral change to helpv -->

## Impact

- New `benches/` directory with benchmark harness
- Possible refactor of startup path in `src/main.rs` / `src/app.rs` to expose a measurable entrypoint (non-TUI) for benchmarking
- New dev-dependency (criterion or similar bench framework)
- CI pipeline may gain a benchmark step (optional, not required for this change)
