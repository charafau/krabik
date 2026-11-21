//! Tests for the Android architecture detection of the `run` command.
//!
//! A fake `adb` reports a device with a configurable ABI, so the CLI can be
//! checked without an emulator or a device being connected.

#![cfg(unix)]

mod common;

use common::{FakeToolchain, cli, fake_dsl};
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

/// A fake `adb` that reports a single arm64 device and records its calls.
const FAKE_ADB_LOGGING: &str = r#"#!/bin/sh
echo "adb $*" >> "$FAKE_LOG"
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

/// The tools of a krabik-ui-dsl project: `cargo` must not be called, Gradle
/// builds the Rust library.
fn dsl_tools() -> FakeToolchain {
    FakeToolchain::new(&[
        ("adb", FAKE_ADB_LOGGING),
        (
            "cargo",
            "#!/bin/sh\necho \"cargo $*\" >> \"$FAKE_LOG\"\nexit 0\n",
        ),
        ("xcodegen", "#!/bin/sh\nexit 0\n"),
    ])
}

/// Creates a krabik-ui-dsl project named `demo` and returns its root.
fn create_dsl_project(tools: &FakeToolchain, work: &TempDir) -> std::path::PathBuf {
    let dsl = fake_dsl(work.path());
    cli(tools)
        .current_dir(work.path())
        .env("FAKE_LOG", work.path().join("create.log"))
        .args(["create", "demo", "--with-dsl", "--dsl-path"])
        .arg(&dsl)
        .assert()
        .success();
    work.path().join("demo")
}

#[test]
fn run_lets_gradle_build_a_dsl_project() {
    let tools = dsl_tools();
    let work = TempDir::new().unwrap();
    let project = create_dsl_project(&tools, &work);
    let log = work.path().join("calls.log");

    cli(&tools)
        .current_dir(&project)
        .env("FAKE_LOG", &log)
        .env_remove("ANDROID_NDK_HOME")
        .arg("run")
        .assert()
        .success();

    let calls = std::fs::read_to_string(&log).unwrap();
    // Gradle builds the library for the device and installs the app
    assert!(
        calls.contains("gradlew -p android app:installDebug -PkrabikAbis=arm64-v8a"),
        "{calls}"
    );
    assert!(
        calls.contains(
            "adb -s TEST1234 shell am start -n com.example.demo/com.example.demo.MainActivity"
        ),
        "{calls}"
    );
    // neither the library nor UniFFI bindings are built by the CLI
    assert!(!calls.contains("cargo"), "{calls}");
}

#[test]
fn run_passes_the_abi_flag_to_gradle_in_a_dsl_project() {
    let tools = dsl_tools();
    let work = TempDir::new().unwrap();
    let project = create_dsl_project(&tools, &work);
    let log = work.path().join("calls.log");

    cli(&tools)
        .current_dir(&project)
        .env("FAKE_LOG", &log)
        .args(["run", "--abi", "x86_64"])
        .assert()
        .success();

    let calls = std::fs::read_to_string(&log).unwrap();
    assert!(calls.contains("-PkrabikAbis=x86_64"), "{calls}");
}

#[test]
fn bindings_and_release_are_refused_in_a_dsl_project() {
    let tools = dsl_tools();
    let work = TempDir::new().unwrap();
    let project = create_dsl_project(&tools, &work);

    cli(&tools)
        .current_dir(&project)
        .arg("bindings")
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("no UniFFI bindings"));

    cli(&tools)
        .current_dir(&project)
        .arg("release")
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("not supported for krabik-ui-dsl"));
}
