//! Integration tests for the canlink CLI.
//!
//! These tests verify the CLI commands work correctly end-to-end.

use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use tempfile::NamedTempFile;

fn canlink() -> Command {
    assert_cmd::cargo::cargo_bin_cmd!("canlink")
}

/// Test the list command.
#[test]
fn test_cli_list_command() {
    let mut cmd = canlink();
    cmd.arg("list");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("backends"));
}

/// Test the list command with JSON output.
#[test]
fn test_cli_list_json() {
    let mut cmd = canlink();
    cmd.arg("--json").arg("list");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("\"backends\""));
}

/// Test the info command with non-existent backend.
#[test]
fn test_cli_info_nonexistent() {
    let mut cmd = canlink();
    cmd.arg("info").arg("nonexistent");

    cmd.assert()
        .failure()
        .code(2)
        .stderr(predicate::str::contains("Backend not found"));
}

/// Test the validate command with valid config.
#[test]
fn test_cli_validate_valid() {
    let temp_file = NamedTempFile::new().unwrap();
    fs::write(
        temp_file.path(),
        r#"[backend]
backend_name = "tscan"
"#,
    )
    .unwrap();

    let mut cmd = canlink();
    cmd.arg("validate").arg(temp_file.path());

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Configuration file is valid"));
}

/// Test the validate command with invalid config.
#[test]
fn test_cli_validate_invalid() {
    let temp_file = NamedTempFile::new().unwrap();
    fs::write(temp_file.path(), "invalid toml [[[").unwrap();

    let mut cmd = canlink();
    cmd.arg("validate").arg(temp_file.path());

    cmd.assert()
        .failure()
        .code(4)
        .stderr(predicate::str::contains("Configuration error"));
}

/// Test the validate command with non-existent file.
#[test]
fn test_cli_validate_nonexistent() {
    let mut cmd = canlink();
    cmd.arg("validate").arg("/nonexistent/file.toml");

    cmd.assert()
        .failure()
        .code(4)
        .stderr(predicate::str::contains("Configuration file not found"));
}

/// Test the --help flag.
#[test]
fn test_cli_help() {
    let mut cmd = canlink();
    cmd.arg("--help");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Usage:"))
        .stdout(predicate::str::contains("Commands:"))
        .stdout(predicate::str::contains("list"))
        .stdout(predicate::str::contains("info"))
        .stdout(predicate::str::contains("send"))
        .stdout(predicate::str::contains("receive"))
        .stdout(predicate::str::contains("validate"))
        .stdout(predicate::str::contains("filter").not())
        .stdout(predicate::str::contains("monitor").not())
        .stdout(predicate::str::contains("isotp").not());
}

/// Test removed command: filter
#[test]
fn test_cli_filter_removed() {
    let mut cmd = canlink();
    cmd.arg("filter");

    cmd.assert()
        .failure()
        .code(2)
        .stderr(predicate::str::contains("unrecognized subcommand"));
}

/// Test removed command: monitor
#[test]
fn test_cli_monitor_removed() {
    let mut cmd = canlink();
    cmd.arg("monitor");

    cmd.assert()
        .failure()
        .code(2)
        .stderr(predicate::str::contains("unrecognized subcommand"));
}

/// Test removed command: isotp
#[test]
fn test_cli_isotp_removed() {
    let mut cmd = canlink();
    cmd.arg("isotp");

    cmd.assert()
        .failure()
        .code(2)
        .stderr(predicate::str::contains("unrecognized subcommand"));
}

/// Test the --version flag.
#[test]
fn test_cli_version() {
    let mut cmd = canlink();
    cmd.arg("--version");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("0.2.0"));
}

/// Test command with --json flag.
#[test]
fn test_cli_global_json_flag() {
    let mut cmd = canlink();
    cmd.arg("--json").arg("list");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("\"backends\""));
}
