//! Tests for the command line interface of `mobile_cli`.

use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::TempDir;

fn cli() -> Command {
    Command::cargo_bin("mobile_cli").expect("binary should build")
}

#[test]
fn help_shows_usage_and_subcommands() {
    cli()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Usage:"))
        .stdout(predicate::str::contains("doctor"))
        .stdout(predicate::str::contains("create"));
}

#[test]
fn version_prints_package_version() {
    cli()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains(env!("CARGO_PKG_VERSION")));
}

#[test]
fn unknown_subcommand_exits_with_code_2() {
    cli().arg("frobnicate").assert().failure().code(2);
}

#[test]
fn create_refuses_existing_directory() {
    let tmp = TempDir::new().unwrap();
    std::fs::create_dir(tmp.path().join("existing")).unwrap();

    cli()
        .current_dir(tmp.path())
        .args(["create", "existing"])
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("already exists"));
}

#[test]
fn release_outside_project_fails() {
    let tmp = TempDir::new().unwrap();
    cli()
        .current_dir(tmp.path())
        .arg("release")
        .assert()
        .failure()
        .code(1);
}

#[test]
fn launch_unknown_ios_simulator_fails() {
    cli()
        .args(["launch", "NoSuchSimulator", "--platform", "ios"])
        .assert()
        .failure()
        .code(1);
}

#[test]
fn doctor_prints_all_sections_and_exits_zero_or_one() {
    let output = Command::cargo_bin("mobile_cli")
        .expect("binary should build")
        .arg("doctor")
        .output()
        .expect("doctor should run");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("[Rust]"), "missing [Rust] section");
    assert!(stdout.contains("[Android]"), "missing [Android] section");
    assert!(stdout.contains("[iOS]"), "missing [iOS] section");

    let code = output
        .status
        .code()
        .expect("doctor should exit with a code");
    assert!(code == 0 || code == 1, "unexpected exit code {code}");
    if code == 1 {
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains("checks failed"));
    }
}

#[test]
fn doctor_checks_the_tools_the_build_needs() {
    let output = Command::cargo_bin("mobile_cli")
        .expect("binary should build")
        .arg("doctor")
        .output()
        .expect("doctor should run");

    let stdout = String::from_utf8_lossy(&output.stdout);
    for check in [
        "cargo",
        "rustup",
        "android CLI",
        "adb",
        "java",
        "Android SDK",
        "ANDROID_NDK_HOME",
        "xcodegen",
    ] {
        assert!(stdout.contains(check), "missing {check} check: {stdout}");
    }
}

#[test]
fn doctor_output_is_plain_when_piped() {
    let output = Command::cargo_bin("mobile_cli")
        .expect("binary should build")
        .arg("doctor")
        .output()
        .expect("doctor should run");

    assert!(!String::from_utf8_lossy(&output.stdout).contains('\u{1b}'));
}

#[test]
fn doctor_respects_no_color_env() {
    let output = Command::cargo_bin("mobile_cli")
        .expect("binary should build")
        .env("NO_COLOR", "1")
        .arg("doctor")
        .output()
        .expect("doctor should run");

    assert!(!String::from_utf8_lossy(&output.stdout).contains('\u{1b}'));
}

#[test]
fn create_rejects_invalid_package() {
    let tmp = TempDir::new().unwrap();
    cli()
        .current_dir(tmp.path())
        .args(["create", "demo", "--package", "com..bad"])
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("invalid --package"));
}

#[test]
fn create_rejects_invalid_project_name() {
    let tmp = TempDir::new().unwrap();
    for name in ["MyApp", "1app", "my app", "my.app", "fn", "class"] {
        cli()
            .current_dir(tmp.path())
            .args(["create", name])
            .assert()
            .failure()
            .code(1)
            .stderr(predicate::str::contains("invalid project name"));
    }
}

#[test]
fn create_rejects_invalid_name_without_creating_a_directory() {
    let tmp = TempDir::new().unwrap();
    cli()
        .current_dir(tmp.path())
        .args(["create", "MyApp"])
        .assert()
        .failure()
        .code(1);

    // nothing may be written before the name has been validated
    assert!(
        !tmp.path().join("MyApp").exists(),
        "no directory should be created for an invalid name"
    );
    assert_eq!(std::fs::read_dir(tmp.path()).unwrap().count(), 0);
}

#[test]
fn create_rejects_names_that_escape_the_working_directory() {
    let tmp = TempDir::new().unwrap();
    let working_dir = tmp.path().join("work");
    std::fs::create_dir(&working_dir).unwrap();

    for name in ["../escape", "..", "a/b"] {
        cli()
            .current_dir(&working_dir)
            .args(["create", name])
            .assert()
            .failure()
            .code(1)
            .stderr(predicate::str::contains("invalid project name"));
    }

    assert!(
        !tmp.path().join("escape").exists(),
        "no directory may be created outside the working directory"
    );
    assert_eq!(std::fs::read_dir(&working_dir).unwrap().count(), 0);
}

#[test]
fn create_help_documents_package_flag() {
    cli()
        .args(["create", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--package"));
}

#[test]
fn lib_run_doctor_errors_when_checks_fail() {
    let args = mobile_cli::Args {
        command: mobile_cli::Commands::Doctor,
    };
    if let Err(err) = mobile_cli::run(args) {
        assert!(err.to_string().contains("checks failed"));
    }
}

#[test]
fn lib_run_launch_unknown_simulator_errors() {
    let args = mobile_cli::Args {
        command: mobile_cli::Commands::Launch {
            emulator: "NoSuchSimulator".to_string(),
            platform: mobile_cli::Platform::Ios,
        },
    };
    assert!(mobile_cli::run(args).is_err());
}
