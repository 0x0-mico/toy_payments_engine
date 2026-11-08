//! Exit-status contract: status 0 on success, status 1 on fatal IO, and a
//! stream full of rejected rows is still a success. Usage errors are outside
//! the processing contract: missing or extra arguments make the argument
//! parser print a usage message and exit 2.

use std::fs;
use std::path::PathBuf;
use std::process::Command;

const BIN: &str = env!("CARGO_BIN_EXE_toy_engine");

fn run(path: &PathBuf) -> (Option<i32>, String, String) {
    let output = Command::new(BIN)
        .arg(path)
        .output()
        .unwrap_or_else(|e| panic!("cannot run {BIN}: {e}"));
    (
        output.status.code(),
        String::from_utf8_lossy(&output.stdout).into_owned(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    )
}

fn temp(name: &str, content: &str) -> PathBuf {
    let path = std::env::temp_dir()
        .join(format!("toy_engine_exit_{}_{}", std::process::id(), name));
    fs::write(&path, content).unwrap();
    path
}

fn cases_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/cases")
}

#[test]
fn a_valid_stream_exits_zero() {
    let input = cases_dir().join("deposit_single/input.csv");
    let (code, _, stderr) = run(&input);
    assert_eq!(code, Some(0));
    assert!(stderr.is_empty(), "unexpected stderr: {stderr}");
}

#[test]
fn rejected_rows_do_not_change_the_exit_status() {
    let input = cases_dir().join("pdf_sample/input.csv");
    let (code, _, stderr) = run(&input);
    assert_eq!(code, Some(0));
    assert!(!stderr.is_empty(), "pdf_sample rejects its last row");
}

#[test]
fn every_row_rejected_still_exits_zero() {
    let path = temp(
        "all_rejected.csv",
        "type,client,tx,amount\nbogus,1,1,1\ndeposit,1,2\nwithdrawal,1,3,-5\n",
    );
    let (code, stdout, _) = run(&path);
    fs::remove_file(&path).unwrap();
    assert_eq!(code, Some(0));
    assert_eq!(stdout, "client,available,held,total,locked\n");
}

#[test]
fn an_empty_file_exits_zero() {
    let path = temp("empty.csv", "");
    let (code, stdout, _) = run(&path);
    fs::remove_file(&path).unwrap();
    assert_eq!(code, Some(0));
    assert_eq!(stdout, "client,available,held,total,locked\n");
}

#[test]
fn a_missing_input_file_exits_one() {
    let path = std::env::temp_dir().join("toy_engine_exit_does_not_exist.csv");
    let (code, _, stderr) = run(&path);
    assert_eq!(code, Some(1));
    assert!(stderr.contains("Fatal error"), "stderr: {stderr}");
}

#[test]
fn a_missing_argument_is_a_usage_error() {
    let output = Command::new(BIN).output().unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(2));
    assert!(stderr.contains("Usage"), "stderr: {stderr}");
}

#[test]
fn an_additional_argument_is_a_usage_error() {
    let input = cases_dir().join("deposit_single/input.csv");
    let output = Command::new(BIN)
        .arg(&input)
        .arg("extra.csv")
        .output()
        .unwrap();
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(2));
    assert!(stderr.contains("unexpected argument"), "stderr: {stderr}");
}

#[test]
fn help_exits_zero() {
    let output = Command::new(BIN).arg("--help").output().unwrap();
    assert_eq!(output.status.code(), Some(0));
}
