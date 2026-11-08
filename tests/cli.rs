//! Directory cases: one folder per scenario, three files each.
//!
//! `input.csv` is the only argument, `expected_output.csv` is the whole of
//! stdout, `expected_errors.txt` is the whole of stderr. Every case must exit 0:
//! a rejected row is logged, never fatal.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const BIN: &str = env!("CARGO_BIN_EXE_toy_engine");

fn cases_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/cases")
}

/// Line endings folded, trailing whitespace on every line and at the end dropped.
fn normalize(text: &str) -> String {
    text.replace("\r\n", "\n")
        .lines()
        .map(str::trim_end)
        .collect::<Vec<_>>()
        .join("\n")
}

fn read_normalized(path: &Path) -> String {
    let bytes = fs::read(path).unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
    normalize(&String::from_utf8_lossy(&bytes))
}

fn run_case(name: &str) {
    let dir = cases_root().join(name);
    let output = Command::new(BIN)
        .arg(dir.join("input.csv"))
        .output()
        .unwrap_or_else(|e| panic!("cannot run {BIN}: {e}"));

    let stdout = normalize(&String::from_utf8_lossy(&output.stdout));
    let stderr = normalize(&String::from_utf8_lossy(&output.stderr));

    assert_eq!(
        stdout,
        read_normalized(&dir.join("expected_output.csv")),
        "case {name}: stdout differs"
    );
    assert_eq!(
        stderr,
        read_normalized(&dir.join("expected_errors.txt")),
        "case {name}: stderr differs"
    );
    assert!(
        output.status.success(),
        "case {name}: exit status {:?}",
        output.status.code()
    );
}

macro_rules! cases {
    ($($name:ident),* $(,)?) => {
        const REGISTRY: &[&str] = &[$(stringify!($name)),*];

        $(
            #[test]
            fn $name() {
                run_case(stringify!($name));
            }
        )*
    };
}

cases!(
    amount_max_overflow,
    amount_missing_on_deposit,
    amount_negative_rejected,
    amount_non_numeric,
    amount_zero_rejected,
    big_dataset,
    chargeback_after_resolve_rejected,
    corrupt_row_mid_file,
    crlf_line_endings,
    deposit_single,
    dispute_after_chargeback_rejected,
    dispute_chargeback_locks,
    dispute_foreign_client,
    dispute_reopen_after_resolve,
    dispute_resolve,
    dispute_unknown_tx,
    dispute_withdrawal_rejected,
    double_chargeback_rejected,
    duplicate_tx_id,
    empty_file,
    header_only,
    locked_blocks_deposit,
    locked_blocks_withdrawal,
    max_ids,
    min_values,
    multiple_clients_interleaved,
    negative_available_on_dispute,
    negative_total_on_chargeback,
    pdf_sample,
    precision_four_places,
    precision_too_precise,
    precision_trailing_zeros,
    resolve_without_dispute,
    unknown_type_skipped,
    whitespace_padded,
    withdrawal_exact_balance,
    withdrawal_insufficient,
    withdrawal_unknown_client,
    withdrawal_within_funds,
);

/// A case folder without a test, or a test without a folder, fails here.
#[test]
fn registry_matches_case_directories() {
    let mut on_disk: Vec<String> = fs::read_dir(cases_root())
        .expect("tests/cases exists")
        .filter_map(Result::ok)
        .filter(|entry| entry.path().is_dir())
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect();
    on_disk.sort();

    let mut registered: Vec<String> = REGISTRY.iter().map(|name| (*name).to_owned()).collect();
    registered.sort();

    assert_eq!(
        on_disk, registered,
        "tests/cases and the case registry disagree"
    );
}
