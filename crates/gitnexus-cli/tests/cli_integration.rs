//! Integration tests for the `gitnexus` CLI binary.
//!
//! These tests run the compiled binary and check exit codes + output.

use std::process::Command;

fn gitnexus() -> Command {
    Command::new(env!("CARGO_BIN_EXE_gitnexus"))
}

#[test]
fn cli_help_shows_usage() {
    let output = gitnexus()
        .arg("--help")
        .output()
        .expect("failed to run gitnexus");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("analyze"),
        "help should mention analyze command"
    );
    assert!(
        stdout.contains("generate"),
        "help should mention generate command"
    );
    assert!(
        stdout.contains("report"),
        "help should mention report command"
    );
}

#[test]
fn cli_analyze_help() {
    let output = gitnexus()
        .args(["analyze", "--help"])
        .output()
        .expect("failed to run gitnexus analyze --help");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("--force"),
        "analyze help should mention --force"
    );
}

#[test]
fn cli_report_help() {
    let output = gitnexus()
        .args(["report", "--help"])
        .output()
        .expect("failed to run gitnexus report --help");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("--json"),
        "report help should mention --json"
    );
}

#[test]
fn cli_config_test_without_config() {
    // Should succeed even without config (graceful error message)
    let output = gitnexus()
        .args(["config", "test"])
        .output()
        .expect("failed to run gitnexus config test");
    assert!(output.status.success());
}

#[test]
fn cli_status_runs() {
    // Status should succeed (may say no index found, but shouldn't crash)
    let output = gitnexus()
        .arg("status")
        .output()
        .expect("failed to run gitnexus status");
    assert!(output.status.success());
}

#[test]
fn cli_list_runs() {
    let output = gitnexus()
        .arg("list")
        .output()
        .expect("failed to run gitnexus list");
    assert!(output.status.success());
}

#[test]
fn cli_hotspots_on_self() {
    // Run hotspots on the gitnexus-rs repo itself
    let repo = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();

    let output = gitnexus()
        .args(["hotspots", "--path", repo.to_str().unwrap()])
        .output()
        .expect("failed to run gitnexus hotspots");
    assert!(
        output.status.success(),
        "hotspots should succeed on a git repo"
    );
}

#[test]
fn cli_coupling_on_self() {
    let repo = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();

    let output = gitnexus()
        .args(["coupling", "--path", repo.to_str().unwrap()])
        .output()
        .expect("failed to run gitnexus coupling");
    assert!(
        output.status.success(),
        "coupling should succeed on a git repo"
    );
}

#[test]
fn cli_ownership_on_self() {
    let repo = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap();

    let output = gitnexus()
        .args(["ownership", "--path", repo.to_str().unwrap()])
        .output()
        .expect("failed to run gitnexus ownership");
    assert!(
        output.status.success(),
        "ownership should succeed on a git repo"
    );
}

#[test]
fn cli_cypher_no_index() {
    // Cypher without an index should exit gracefully (not panic)
    let output = gitnexus()
        .args(["cypher", "MATCH (n) RETURN n LIMIT 1"])
        .output()
        .expect("failed to run gitnexus cypher");
    // Should succeed (prints error message but exits 0)
    assert!(output.status.success());
}
