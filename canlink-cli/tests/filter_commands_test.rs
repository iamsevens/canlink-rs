//! Filter commands integration tests (T058)

use assert_cmd::Command;
use predicates::prelude::*;

fn canlink() -> Command {
    assert_cmd::cargo::cargo_bin_cmd!("canlink")
}

#[test]
fn test_filter_help() {
    canlink()
        .arg("filter")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Manage message filters"))
        .stdout(predicate::str::contains("add"))
        .stdout(predicate::str::contains("list"))
        .stdout(predicate::str::contains("remove"))
        .stdout(predicate::str::contains("clear"));
}

#[test]
fn test_filter_add_id() {
    canlink()
        .args(["filter", "add", "id", "0x123"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Added id filter"))
        .stdout(predicate::str::contains("ID=0x123"));
}

#[test]
fn test_filter_add_id_decimal() {
    canlink()
        .args(["filter", "add", "id", "291"]) // 0x123 in decimal
        .assert()
        .success()
        .stdout(predicate::str::contains("Added id filter"))
        .stdout(predicate::str::contains("ID=0x123"));
}

#[test]
fn test_filter_add_mask() {
    canlink()
        .args(["filter", "add", "mask", "0x120", "0x7F0"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Added mask filter"))
        .stdout(predicate::str::contains("ID=0x120"))
        .stdout(predicate::str::contains("MASK=0x7F0"));
}

#[test]
fn test_filter_add_range() {
    canlink()
        .args(["filter", "add", "range", "0x100", "0x1FF"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Added range filter"))
        .stdout(predicate::str::contains("RANGE=0x100-0x1FF"));
}

#[test]
fn test_filter_add_extended() {
    canlink()
        .args(["filter", "add", "id", "0x12345678", "--extended"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Added id filter"))
        .stdout(predicate::str::contains("extended"));
}

#[test]
fn test_filter_add_id_missing_param() {
    canlink()
        .args(["filter", "add", "id"])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "ID filter requires an ID parameter",
        ));
}

#[test]
fn test_filter_add_mask_missing_params() {
    canlink()
        .args(["filter", "add", "mask", "0x120"])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "Mask filter requires ID and MASK parameters",
        ));
}

#[test]
fn test_filter_add_range_missing_params() {
    canlink()
        .args(["filter", "add", "range", "0x100"])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "Range filter requires START and END parameters",
        ));
}

#[test]
fn test_filter_add_range_invalid_order() {
    canlink()
        .args(["filter", "add", "range", "0x200", "0x100"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Start ID"));
}

#[test]
fn test_filter_add_invalid_type() {
    canlink()
        .args(["filter", "add", "invalid", "0x123"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Unknown filter type"));
}

#[test]
fn test_filter_add_invalid_id() {
    canlink()
        .args(["filter", "add", "id", "0xGGG"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Invalid"));
}

#[test]
fn test_filter_add_id_out_of_range() {
    // Standard ID max is 0x7FF
    canlink()
        .args(["filter", "add", "id", "0x800"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Invalid ID"));
}

#[test]
fn test_filter_list_empty() {
    canlink()
        .args(["filter", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("No filters configured"));
}

#[test]
fn test_filter_clear() {
    canlink()
        .args(["filter", "clear"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Cleared"));
}

#[test]
fn test_filter_remove_invalid_index() {
    canlink()
        .args(["filter", "remove", "0"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Failed to remove filter"));
}

#[test]
fn test_filter_add_json_output() {
    canlink()
        .args(["-j", "filter", "add", "id", "0x123"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"status\": \"success\""))
        .stdout(predicate::str::contains("\"filter_type\": \"id\""))
        .stdout(predicate::str::contains("\"index\": 0"));
}

#[test]
fn test_filter_list_json_output() {
    canlink()
        .args(["-j", "filter", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"total_count\""))
        .stdout(predicate::str::contains("\"hardware_count\""))
        .stdout(predicate::str::contains("\"filters\""));
}

#[test]
fn test_filter_clear_json_output() {
    canlink()
        .args(["-j", "filter", "clear"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"status\": \"success\""))
        .stdout(predicate::str::contains("\"cleared_count\""));
}
