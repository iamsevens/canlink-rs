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
        .stdout(predicate::str::contains("Available backends"))
        .stdout(predicate::str::contains("mock"));
}

/// Test the list command with JSON output.
#[test]
fn test_cli_list_json() {
    let mut cmd = canlink();
    cmd.arg("--json").arg("list");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("\"backends\""))
        .stdout(predicate::str::contains("\"mock\""));
}

/// Test the info command.
#[test]
fn test_cli_info_command() {
    let mut cmd = canlink();
    cmd.arg("info").arg("mock");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Backend: mock"))
        .stdout(predicate::str::contains("Version:"))
        .stdout(predicate::str::contains("Channels:"))
        .stdout(predicate::str::contains("CAN-FD Support:"));
}

/// Test the info command with JSON output.
#[test]
fn test_cli_info_json() {
    let mut cmd = canlink();
    cmd.arg("--json").arg("info").arg("mock");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("\"name\""))
        .stdout(predicate::str::contains("\"version\""))
        .stdout(predicate::str::contains("\"channel_count\""));
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

/// Test the send command.
#[test]
fn test_cli_send_command() {
    let mut cmd = canlink();
    cmd.arg("send")
        .arg("mock")
        .arg("0")
        .arg("0x123")
        .arg("01")
        .arg("02")
        .arg("03");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Message sent"))
        .stdout(predicate::str::contains("ID=0x123"));
}

/// Test the send command with invalid data.
#[test]
fn test_cli_send_invalid_data() {
    let mut cmd = canlink();
    cmd.arg("send").arg("mock").arg("0").arg("0x123").arg("ZZ"); // Invalid hex

    cmd.assert()
        .failure()
        .code(7)
        .stderr(predicate::str::contains("Parse error"));
}

/// Test the send command with too much data.
#[test]
fn test_cli_send_too_much_data() {
    let mut cmd = canlink();
    cmd.arg("send")
        .arg("mock")
        .arg("0")
        .arg("0x123")
        .arg("01")
        .arg("02")
        .arg("03")
        .arg("04")
        .arg("05")
        .arg("06")
        .arg("07")
        .arg("08")
        .arg("09"); // 9 bytes - too much for CAN 2.0

    cmd.assert()
        .failure()
        .code(5)
        .stderr(predicate::str::contains("Invalid argument"));
}

/// Test the validate command with valid config.
#[test]
fn test_cli_validate_valid() {
    let temp_file = NamedTempFile::new().unwrap();
    fs::write(temp_file.path(), "[backend]\nbackend_name = \"mock\"\n").unwrap();

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
        .stdout(predicate::str::contains("validate"));
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

    cmd.assert().success().stdout(predicate::str::contains("["));
}
