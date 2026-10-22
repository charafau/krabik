//! Tests for the Android architecture detection of the `run` command.
//!
//! A fake `adb` reports a device with a configurable ABI, so the CLI can be
//! checked without an emulator or a device being connected.

#![cfg(unix)]

mod common;

use common::{FakeToolchain, cli};
use predicates::prelude::*;
use tempfile::TempDir;

/// A fake `adb` that reports a single device with the ABI given in
/// `FAKE_ADB_ABI`.
const FAKE_ADB: &str = r#"#!/bin/sh
case "$1" in
    devices)
        printf 'List of devices attached\n'
        printf 'TEST1234               device product:sdk_gphone model:Pixel_7 device:emu64a transport_id:1\n'
        exit 0
        ;;
esac

# adb -s <serial> shell getprop ro.product.cpu.abi
if [ "$1" = "-s" ] && [ "$3" = "shell" ] && [ "$4" = "getprop" ] && [ "$5" = "ro.product.cpu.abi" ]; then
    printf '%s\n' "$FAKE_ADB_ABI"
    exit 0
fi

exit 0
"#;

/// A fake `adb` that cannot determine the ABI of the device.
const FAKE_ADB_WITHOUT_ABI: &str = r#"#!/bin/sh
case "$1" in
    devices)
        printf 'List of devices attached\n'
        printf 'TEST1234               device product:sdk_gphone model:Pixel_7 device:emu64a transport_id:1\n'
        exit 0
        ;;
esac

if [ "$1" = "-s" ] && [ "$3" = "shell" ] && [ "$4" = "getprop" ]; then
    printf '\n'
    exit 0
fi

exit 0
"#;

/// The CLI under test with a fake `adb`, and a working directory.
fn cli_with_abi() -> (FakeToolchain, TempDir) {
    let tools = FakeToolchain::new(&[("adb", FAKE_ADB)]);
    let work = TempDir::new().unwrap();
    (tools, work)
}

/// Runs `run` and returns the output of the CLI.
fn run(tools: &FakeToolchain, work: &TempDir, args: &[&str]) -> std::process::Output {
    let mut cmd = cli(tools);
    cmd.current_dir(work.path())
        .env("FAKE_ADB_ABI", args[0])
        .env_remove("ANDROID_NDK_HOME")
        .args(&args[1..]);
    cmd.output().expect("run should execute")
}

#[test]
fn run_builds_for_the_abi_of_the_device() {
    let expected = [
        ("arm64-v8a", "aarch64-linux-android"),
        ("armeabi-v7a", "armv7-linux-androideabi"),
        ("x86_64", "x86_64-linux-android"),
        ("x86", "i686-linux-android"),
    ];

    for (abi, triple) in expected {
        let (tools, work) = cli_with_abi();
        let output = run(&tools, &work, &[abi, "run"]);

        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains(triple),
            "expected {triple} for {abi}, got: {stdout}"
        );
    }
}

#[test]
fn run_reports_the_abi_it_builds_for() {
    let (tools, work) = cli_with_abi();
    let output = run(&tools, &work, &["x86_64", "run"]);

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("x86_64-linux-android (x86_64)"), "{stdout}");
}

#[test]
fn run_uses_the_abi_flag_instead_of_the_abi_of_the_device() {
    let (tools, work) = cli_with_abi();
    let output = run(&tools, &work, &["arm64-v8a", "run", "--abi", "x86_64"]);

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("x86_64-linux-android"), "{stdout}");
    assert!(!stdout.contains("aarch64-linux-android"), "{stdout}");
}

#[test]
fn run_fails_when_the_abi_of_the_device_is_unknown() {
    let tools = FakeToolchain::new(&[("adb", FAKE_ADB_WITHOUT_ABI)]);
    let work = TempDir::new().unwrap();

    cli(&tools)
        .current_dir(work.path())
        .env_remove("ANDROID_NDK_HOME")
        .arg("run")
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("could not determine the ABI"));
}

#[test]
fn run_rejects_the_abi_flag_for_ios() {
    let (tools, work) = cli_with_abi();

    cli(&tools)
        .current_dir(work.path())
        .args(["run", "--platform", "ios", "--abi", "x86_64"])
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains(
            "--abi is only supported for Android",
        ));
}

#[test]
fn run_help_documents_the_abi_flag() {
    let (tools, _work) = cli_with_abi();

    cli(&tools)
        .args(["run", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--abi"));
}
