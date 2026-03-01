use helpv::app::App;
use helpv::config::Config;
use serde_json::json;
use std::process::Command;
use std::time::Instant;

const COMMANDS: &[&str] = &["git", "tmux"];
const SAMPLES: usize = 3;

fn time_helpv(cmd: &str) -> f64 {
    let command = vec![cmd.to_string()];
    let config = Config::load().expect("failed to load config");
    let start = Instant::now();
    let _ = App::new(command, config);
    start.elapsed().as_secs_f64() * 1000.0
}

fn time_man(cmd: &str) -> f64 {
    let start = Instant::now();
    let _ = Command::new("man")
        .arg(cmd)
        .env("MANPAGER", "cat")
        .env("PAGER", "cat")
        .env("MAN_KEEP_FORMATTING", "0")
        .output();
    start.elapsed().as_secs_f64() * 1000.0
}

fn median(values: &mut [f64]) -> f64 {
    values.sort_by(|a, b| a.partial_cmp(b).unwrap());
    values[values.len() / 2]
}

fn main() {
    // Warm filesystem cache by running man once for each command
    for cmd in COMMANDS {
        let _ = Command::new("man")
            .arg(cmd)
            .env("MANPAGER", "cat")
            .env("PAGER", "cat")
            .output();
    }

    let mut all_passed = true;
    let mut results = Vec::new();

    for cmd in COMMANDS {
        let mut man_times = Vec::with_capacity(SAMPLES);
        let mut helpv_times = Vec::with_capacity(SAMPLES);

        for _ in 0..SAMPLES {
            man_times.push(time_man(cmd));
            helpv_times.push(time_helpv(cmd));
        }

        let man_ms = median(&mut man_times);
        let helpv_ms = median(&mut helpv_times);
        let ratio = helpv_ms / man_ms;
        // Allow 10% margin: tools with thin --help output fall back to man internally,
        // so helpv can't be faster than man in that case — only roughly equal.
        let passed = ratio <= 1.10;

        if !passed {
            all_passed = false;
        }

        results.push(json!({
            "command": cmd,
            "helpv_ms": (helpv_ms * 100.0).round() / 100.0,
            "man_ms": (man_ms * 100.0).round() / 100.0,
            "ratio": (ratio * 100.0).round() / 100.0,
            "passed": passed,
        }));
    }

    let output = json!({
        "benchmark": "load_time",
        "samples": SAMPLES,
        "results": results,
        "passed": all_passed,
    });

    println!("{}", serde_json::to_string_pretty(&output).unwrap());

    if !all_passed {
        std::process::exit(1);
    }
}
