//! Monitor commands integration tests (T059)

use assert_cmd::Command;
use predicates::prelude::*;

fn canlink() -> Command {
    assert_cmd::cargo::cargo_bin_cmd!("canlink")
}

#[test]
fn test_monitor_help() {
    canlink()
        .arg("monitor")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Monitor connection status"))
        .stdout(predicate::str::contains("status"))
        .stdout(predicate::str::contains("reconnect"))
        .stdout(predicate::str::contains("config"));
}

#[test]
fn test_monitor_status() {
    canlink()
        .args(["monitor", "status"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Connection Monitor Status"))
        .stdout(predicate::str::contains("State:"))
        .stdout(predicate::str::contains("Connected"))
        .stdout(predicate::str::contains("Can send:"))
        .stdout(predicate::str::contains("Can receive:"))
        .stdout(predicate::str::contains("Heartbeat interval:"));
}

#[test]
fn test_monitor_status_shows_backends() {
    canlink()
        .args(["monitor", "status"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Registered Backends:"))
        .stdout(predicate::str::contains("mock"));
}

#[test]
fn test_monitor_status_json() {
    canlink()
        .args(["-j", "monitor", "status"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"state\": \"connected\""))
        .stdout(predicate::str::contains("\"can_send\": true"))
        .stdout(predicate::str::contains("\"can_receive\": true"))
        .stdout(predicate::str::contains("\"heartbeat_interval_ms\""))
        .stdout(predicate::str::contains("\"auto_reconnect\": false"))
        .stdout(predicate::str::contains("\"backends\""));
}

#[test]
fn test_monitor_reconnect_mock() {
    canlink()
        .args(["monitor", "reconnect", "mock"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Reconnected to backend 'mock'"));
}

#[test]
fn test_monitor_reconnect_invalid_backend() {
    canlink()
        .args(["monitor", "reconnect", "nonexistent"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Backend not found"));
}

#[test]
fn test_monitor_reconnect_json() {
    canlink()
        .args(["-j", "monitor", "reconnect", "mock"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"status\": \"success\""))
        .stdout(predicate::str::contains("\"backend\": \"mock\""))
        .stdout(predicate::str::contains("\"new_state\": \"connected\""));
}

#[test]
fn test_monitor_config_default() {
    canlink()
        .args(["monitor", "config"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Monitor configuration updated"))
        .stdout(predicate::str::contains("Heartbeat interval: 1000 ms"))
        .stdout(predicate::str::contains("Auto-reconnect: disabled"));
}

#[test]
fn test_monitor_config_custom_heartbeat() {
    canlink()
        .args(["monitor", "config", "--heartbeat-ms", "500"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Heartbeat interval: 500 ms"));
}

#[test]
fn test_monitor_config_enable_reconnect() {
    canlink()
        .args(["monitor", "config", "--auto-reconnect"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Auto-reconnect: enabled"));
}

#[test]
fn test_monitor_config_with_max_retries() {
    canlink()
        .args([
            "monitor",
            "config",
            "--auto-reconnect",
            "--max-retries",
            "5",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Auto-reconnect: enabled"));
}

#[test]
fn test_monitor_config_json() {
    canlink()
        .args(["-j", "monitor", "config", "--heartbeat-ms", "2000"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"status\": \"success\""))
        .stdout(predicate::str::contains("\"heartbeat_interval_ms\": 2000"));
}

#[test]
fn test_monitor_config_json_with_reconnect() {
    canlink()
        .args(["-j", "monitor", "config", "--auto-reconnect"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"auto_reconnect\": true"));
}
