//! Heavy end-to-end stress case, excluded from the default suite.
//!
//! Drives `scripts/generate_stress.py` (python3 required) to build a dataset of
//! `CYCLES` repeating 68-row blocks — 1,360,004 rows at the default 20000
//! cycles — that embeds every failure mode of the integration and unit suites,
//! then checks the engine's output against the generator's closed-form
//! expectation. Value-level comparison: the specification declares the printed
//! scale of round decimals irrelevant, so decimals are compared numerically.
//!
//! Run with:
//!     cargo test --test stress -- --ignored --nocapture

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::str::FromStr;
use std::time::Instant;

use rust_decimal::Decimal;

const CYCLES: u32 = 20000;

#[test]
#[ignore = "heavy: generates and processes ~1.36M rows"]
fn stress_dataset_matches_closed_form() {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let out_dir = manifest.join("target").join("stress");
    std::fs::create_dir_all(&out_dir).unwrap();

    let generator = Command::new("python3")
        .arg(manifest.join("scripts/generate_stress.py"))
        .arg("--cycles")
        .arg(CYCLES.to_string())
        .arg("--out-dir")
        .arg(&out_dir)
        .status()
        .expect("python3 is required to generate the stress dataset");
    assert!(generator.success(), "stress dataset generation failed");

    let start = Instant::now();
    let output = Command::new(env!("CARGO_BIN_EXE_toy_engine"))
        .arg(out_dir.join("input.csv"))
        .stderr(Stdio::null())
        .output()
        .expect("engine run failed");
    let elapsed = start.elapsed();
    assert!(output.status.success(), "engine must exit 0");

    let actual = parse(&String::from_utf8(output.stdout).unwrap());
    let expected = parse(&std::fs::read_to_string(out_dir.join("expected_output.csv")).unwrap());
    assert_eq!(actual, expected, "engine output differs from the closed form");
    eprintln!("stress: {} rows, {} cycles, verified in {:.2}s",
        count_rows(&out_dir.join("input.csv")),
        CYCLES,
        elapsed.as_secs_f64(),
    );
}

fn parse(text: &str) -> BTreeMap<u16, (Decimal, Decimal, Decimal, bool)> {
    let mut rows = BTreeMap::new();
    for (i, line) in text.lines().enumerate() {
        if i == 0 {
            continue; // header
        }
        let p: Vec<&str> = line.split(',').collect();
        assert_eq!(p.len(), 5, "bad output row: {line}");
        rows.insert(
            p[0].parse().unwrap(),
            (
                Decimal::from_str(p[1]).unwrap(),
                Decimal::from_str(p[2]).unwrap(),
                Decimal::from_str(p[3]).unwrap(),
                p[4] == "true",
            ),
        );
    }
    rows
}

fn count_rows(path: &PathBuf) -> usize {
    std::io::BufRead::lines(std::io::BufReader::new(
        std::fs::File::open(path).unwrap(),
    ))
    .count()
}
