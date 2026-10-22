//! Tests for the `watch` command.
//!
//! A fake toolchain stands in for the Android CLI and the Rust toolchain, so
//! that the watch loop can be exercised without a device.

#![cfg(unix)]

mod common;

use common::{FakeToolchain, cli};
use predicates::prelude::*;
use std::fs;
use std::path::Path;
use std::time::{Duration, Instant};
use tempfile::TempDir;

/// The CLI under test as a process command, with the fake toolchain on the
/// `PATH`.
///
/// Unlike [`cli`] this can be spawned, which is needed for a command that runs
/// until it is stopped.
fn command(tools: &FakeToolchain) -> std::process::Command {
    let mut cmd = std::process::Command::new(assert_cmd::cargo::cargo_bin("mobile_cli"));
    cmd.env("PATH", tools.path());
    cmd
}

/// A fake `android` CLI that records that it was called.
const FAKE_ANDROID: &str = r#"#!/bin/sh
echo "android $*" >> "$FAKE_LOG"
if [ "$1" = "create" ]; then
    namespace=""
    application_id=""
    output="."
    while [ $# -gt 0 ]; do
        case "$1" in
            --namespace) namespace="$2"; shift 2 ;;
            --application-id) application_id="$2"; shift 2 ;;
            --output|-o) output="$2"; shift 2 ;;
            *) shift ;;
        esac
    done
    package_path=$(printf '%s' "$namespace" | tr '.' '/')
    mkdir -p "$output/app/src/main/java/$package_path"
    printf 'package %s\n' "$namespace" > "$output/app/src/main/java/$package_path/MainActivity.kt"
    cat > "$output/app/build.gradle.kts" <<EOF
android {
    namespace = "$namespace"
    defaultConfig {
        applicationId = "$application_id"
    }
}
EOF
fi
exit 0
"#;

/// A fake `cargo` that records every invocation in `FAKE_LOG`.
const FAKE_CARGO: &str = r#"#!/bin/sh
echo "cargo $*" >> "$FAKE_LOG"

# `cargo new` has to create the crate
if [ "$1" = "new" ]; then
    target="$2"
    mkdir -p "$target/src"
    package="crate"
    while [ $# -gt 0 ]; do
        case "$1" in
            --name) package="$2"; shift 2 ;;
            *) shift ;;
        esac
    done
    printf '[package]\nname = "%s"\nversion = "0.1.0"\nedition = "2024"\n' "$package" > "$target/Cargo.toml"
    printf 'pub fn get_greeting() -> String { "Hello".to_string() }\n' > "$target/src/lib.rs"
    exit 0
fi

# a build produces the shared library the CLI copies
if [ "$1" = "build" ]; then
    manifest=""
    target="aarch64-linux-android"
    while [ $# -gt 0 ]; do
        case "$1" in
            --manifest-path) manifest="$2"; shift 2 ;;
            --target) target="$2"; shift 2 ;;
            *) shift ;;
        esac
    done
    dir=$(dirname "$manifest")
    mkdir -p "$dir/target/$target/debug"
    printf 'so' > "$dir/target/$target/debug/libdemo.so"
    exit 0
fi

exit 0
"#;

/// A fake `adb` that reports a single device.
const FAKE_ADB: &str = r#"#!/bin/sh
case "$1" in
    devices)
        printf 'List of devices attached\n'
        printf 'TEST1234               device product:sdk_gphone model:Pixel_7 device:emu64a transport_id:1\n'
        exit 0
        ;;
esac

if [ "$1" = "-s" ] && [ "$3" = "shell" ] && [ "$4" = "getprop" ] && [ "$5" = "ro.product.cpu.abi" ]; then
    printf 'arm64-v8a\n'
    exit 0
fi

exit 0
"#;

/// A fake `xcodegen`.
const FAKE_XCODEGEN: &str = r#"#!/bin/sh
echo "xcodegen $*" >> "$FAKE_LOG"
exit 0
"#;

/// The toolchain used by the watch tests.
fn toolchain() -> FakeToolchain {
    FakeToolchain::new(&[
        ("android", FAKE_ANDROID),
        ("cargo", FAKE_CARGO),
        ("adb", FAKE_ADB),
        ("xcodegen", FAKE_XCODEGEN),
    ])
}

/// Creates a project with the fake toolchain and returns its root.
fn create_project(tools: &FakeToolchain, work: &TempDir) -> std::path::PathBuf {
    cli(tools)
        .current_dir(work.path())
        .env("FAKE_LOG", work.path().join("calls.log"))
        .args(["create", "demo"])
        .assert()
        .success();

    work.path().join("demo")
}

/// Waits until the log contains the given text, or the timeout expires.
fn wait_for_log(log: &Path, needle: &str, timeout: Duration) -> bool {
    let start = Instant::now();
    while start.elapsed() < timeout {
        if let Ok(content) = fs::read_to_string(log)
            && content.contains(needle)
        {
            return true;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    false
}

#[test]
fn watch_rebuilds_when_a_rust_file_changes() {
    let tools = toolchain();
    let work = TempDir::new().unwrap();
    let project = create_project(&tools, &work);
    let log = work.path().join("calls.log");

    let mut child = command(&tools)
        .current_dir(&project)
        .env("FAKE_LOG", &log)
        .args(["watch"])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("watch should start");

    // the first build has to happen
    assert!(
        wait_for_log(&log, "cargo build", Duration::from_secs(20)),
        "the first build did not run: {:?}",
        fs::read_to_string(&log).unwrap_or_default()
    );

    // a change of a Rust file triggers another build
    let builds = fs::read_to_string(&log)
        .unwrap_or_default()
        .lines()
        .filter(|line| line.contains("cargo build"))
        .count();

    let source = project.join("rust/src/lib.rs");
    fs::write(
        &source,
        "pub fn get_greeting() -> String { \"Hi\".to_string() }\n",
    )
    .unwrap();

    let start = Instant::now();
    let mut rebuilt = false;
    while start.elapsed() < Duration::from_secs(20) {
        let count = fs::read_to_string(&log)
            .unwrap_or_default()
            .lines()
            .filter(|line| line.contains("cargo build"))
            .count();
        if count > builds {
            rebuilt = true;
            break;
        }
        std::thread::sleep(Duration::from_millis(50));
    }

    let _ = child.kill();
    let _ = child.wait();

    assert!(
        rebuilt,
        "the change did not trigger a build: {:?}",
        fs::read_to_string(&log).unwrap_or_default()
    );
}

#[test]
fn watch_does_not_rebuild_for_changes_outside_of_the_rust_sources() {
    let tools = toolchain();
    let work = TempDir::new().unwrap();
    let project = create_project(&tools, &work);
    let log = work.path().join("calls.log");

    let mut child = command(&tools)
        .current_dir(&project)
        .env("FAKE_LOG", &log)
        .args(["watch"])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("watch should start");

    assert!(
        wait_for_log(&log, "cargo build", Duration::from_secs(20)),
        "the first build did not run"
    );

    let builds = fs::read_to_string(&log)
        .unwrap_or_default()
        .lines()
        .filter(|line| line.contains("cargo build"))
        .count();

    // the target directory is written by every build and must not trigger one
    let target = project.join("rust/target");
    fs::create_dir_all(&target).unwrap();
    fs::write(target.join("something"), "x").unwrap();

    std::thread::sleep(Duration::from_secs(2));

    let _ = child.kill();
    let _ = child.wait();

    let after = fs::read_to_string(&log)
        .unwrap_or_default()
        .lines()
        .filter(|line| line.contains("cargo build"))
        .count();

    assert_eq!(
        after, builds,
        "a change outside of the Rust sources triggered a build"
    );
}

#[test]
fn watch_fails_outside_of_a_project() {
    let tools = toolchain();
    let work = TempDir::new().unwrap();

    cli(&tools)
        .current_dir(work.path())
        .args(["watch"])
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("no Rust sources found"));
}
