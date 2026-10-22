//! Tests for the `doctor` command.

#![cfg(unix)]

mod common;

use common::FakeToolchain;
use std::fs;
use tempfile::TempDir;

/// A fake `rustup` that records every invocation in `RUSTUP_LOG`.
const FAKE_RUSTUP: &str = r#"#!/bin/sh
echo "called" >> "$RUSTUP_LOG"
if [ "$1" = "target" ] && [ "$2" = "list" ]; then
    printf 'aarch64-linux-android\naarch64-apple-ios\naarch64-apple-ios-sim\n'
fi
exit 0
"#;

#[test]
fn doctor_queries_rustup_only_once() {
    let dir = TempDir::new().unwrap();
    let log = dir.path().join("rustup.log");

    let tools = FakeToolchain::new(&[("rustup", FAKE_RUSTUP)]);

    let output = common::cli(&tools)
        .env("RUSTUP_LOG", &log)
        .arg("doctor")
        .output()
        .expect("doctor should run");

    let calls = fs::read_to_string(&log).unwrap_or_default().lines().count();
    assert_eq!(calls, 1, "rustup should be queried once, got {calls}");

    // the targets reported by the fake rustup are shown as installed
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("[✓] target aarch64-linux-android"),
        "{stdout}"
    );
    assert!(stdout.contains("[✓] target aarch64-apple-ios"), "{stdout}");

    // the targets that are missing are reported as such
    assert!(stdout.contains("[✗] target i686-linux-android"), "{stdout}");
}

#[test]
fn doctor_reports_a_missing_ndk() {
    let dir = TempDir::new().unwrap();
    let tools = FakeToolchain::new(&[("rustup", FAKE_RUSTUP)]);

    let output = common::cli(&tools)
        .env("ANDROID_NDK_HOME", dir.path().join("no-such-ndk"))
        .arg("doctor")
        .output()
        .expect("doctor should run");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("[✗] ANDROID_NDK_HOME does not point at an NDK"),
        "{stdout}"
    );
}

#[test]
fn doctor_reports_a_missing_java_home() {
    let dir = TempDir::new().unwrap();
    let tools = FakeToolchain::new(&[("rustup", FAKE_RUSTUP)]);

    let output = common::cli(&tools)
        .env("JAVA_HOME", dir.path().join("no-such-jdk"))
        .arg("doctor")
        .output()
        .expect("doctor should run");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("[✗] JAVA_HOME points to a missing directory"),
        "{stdout}"
    );
}
