## Context

helpv currently has no performance testing. The startup path — `App::new()` — does all the expensive work: spawning shell commands (`--help`, `man`), regex parsing, toolpack discovery. Terminal init and rendering happen after and are cheap by comparison.

`App::new()` already works without a terminal. It takes `(Vec<String>, Config)` and returns `Result<App>`. This means we can benchmark it directly in-process without any TUI setup.

## Goals / Non-Goals

**Goals:**
- Measure helpv time-to-content-ready vs `man` for the same command
- Fail when helpv is slower than `man` (currently expected for git, tmux)
- Output JSON so agents can parse results and track regressions
- Run headlessly — no TTY, no interactive input

**Non-Goals:**
- Statistical rigor (no criterion, no warmup iterations — single-shot is fine since we're comparing real-world cold-start)
- Benchmarking rendering/TUI performance
- Benchmarking fuzzy finder or search
- CI integration (can be added later)

## Decisions

**Custom bench harness over criterion:** Criterion adds complexity and measures hot-path performance. We care about cold-start, real-world latency — single-shot timing with `Instant` is the right tool. We also need JSON output and custom pass/fail logic that criterion doesn't provide. Use `harness = false` in Cargo.toml to run our own main().

**In-process App::new() vs shelling out to helpv:** Measuring `App::new()` directly avoids process startup overhead (binary loading, dynamic linking) that would add noise. It isolates the work we actually want to optimize.

**man with MANPAGER=cat as baseline:** This is how man loads content without rendering — apples-to-apples with our pre-render measurement. Same approach helpv already uses internally in `fetcher.rs`.

**Multiple samples with median:** Take 3 samples of each and use the median to reduce outlier noise from OS scheduling, without the overhead of a full statistical framework.

## Risks / Trade-offs

- **man timing varies by system** → Mitigated by relative comparison (ratio) rather than absolute thresholds
- **First run may be slower (cold cache)** → Run man first to warm the filesystem cache, then measure both
- **Background man page discovery in App::new()** → `spawn_man_page_discovery` fires a thread but doesn't block `App::new()` return. The benchmark captures the correct metric (time to content ready, not time to full discovery)
