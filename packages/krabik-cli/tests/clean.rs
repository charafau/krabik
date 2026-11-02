//! Tests for the `clean` command.

use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use std::path::Path;
use tempfile::TempDir;

fn cli() -> Command {
    Command::cargo_bin("krabik").expect("binary should build")
}

/// Creates a minimal generated project with the given build artifacts.
fn project() -> TempDir {
    let tmp = TempDir::new().unwrap();
    for dir in [
        "rust/target/debug",
        "rust/src",
        "android/.gradle/8.0",
        "android/.kotlin/2.0",
        "android/build/outputs",
        "android/app/build/outputs",
        "android/app/src/main/java/uniffi",
        "android/app/src/main/java/com/example",
        "android/app/src/main/jniLibs/arm64-v8a",
        "ios/build/Build/Products",
        "ios/Generated",
        "ios/App.xcodeproj",
    ] {
        fs::create_dir_all(tmp.path().join(dir)).unwrap();
    }

    fs::write(tmp.path().join("rust/Cargo.toml"), "[workspace]\n").unwrap();
    fs::write(tmp.path().join("rust/src/lib.rs"), "").unwrap();
    fs::write(
        tmp.path()
            .join("android/app/src/main/java/com/example/MainActivity.kt"),
        "",
    )
    .unwrap();
    fs::write(
        tmp.path()
            .join("android/app/src/main/jniLibs/arm64-v8a/libapp.so"),
        "",
    )
    .unwrap();
    fs::write(
        tmp.path()
            .join("android/app/src/main/java/uniffi/Uniffi.kt"),
        "",
    )
    .unwrap();
    fs::write(tmp.path().join("ios/project.yml"), "name: App\n").unwrap();
    fs::write(tmp.path().join("ios/Generated/App.swift"), "").unwrap();

    tmp
}

fn assert_missing(path: &Path) {
    assert!(
        !path.exists(),
        "{} should have been removed",
        path.display()
    );
}

#[test]
fn clean_removes_build_artifacts() {
    let tmp = project();

    cli()
        .current_dir(tmp.path())
        .arg("clean")
        .assert()
        .success()
        .stdout(predicate::str::contains("Clean done."));

    for path in [
        "rust/target",
        "android/build",
        "android/app/build",
        "android/app/src/main/java/uniffi",
        "ios/build",
        "ios/Generated",
    ] {
        assert_missing(&tmp.path().join(path));
    }

    assert_missing(
        &tmp.path()
            .join("android/app/src/main/jniLibs/arm64-v8a/libapp.so"),
    );
}

#[test]
fn clean_keeps_sources_and_configuration() {
    let tmp = project();

    cli()
        .current_dir(tmp.path())
        .arg("clean")
        .assert()
        .success();

    for path in [
        "rust/Cargo.toml",
        "rust/src/lib.rs",
        "android/app/src/main/java/com/example/MainActivity.kt",
        "android/.gradle",
        "android/.kotlin",
        "ios/project.yml",
        "ios/App.xcodeproj",
    ] {
        assert!(
            tmp.path().join(path).exists(),
            "{} should have been kept",
            path
        );
    }
}

#[test]
fn purge_removes_cached_directories() {
    let tmp = project();

    cli()
        .current_dir(tmp.path())
        .args(["clean", "--purge"])
        .assert()
        .success();

    for path in ["android/.gradle", "android/.kotlin"] {
        assert_missing(&tmp.path().join(path));
    }

    // The regular build artifacts are removed as well.
    assert_missing(&tmp.path().join("android/build"));
    assert_missing(&tmp.path().join("rust/target"));
}

#[test]
fn clean_outside_project_fails() {
    let tmp = TempDir::new().unwrap();

    cli()
        .current_dir(tmp.path())
        .arg("clean")
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("no Rust sources found"));
}
