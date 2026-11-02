//! End-to-end tests for the `create` command.
//!
//! The `android` and `xcodegen` executables are replaced by small shell
//! scripts so that the generated project can be inspected without a full
//! Android/Xcode toolchain being installed.

#![cfg(unix)]

mod common;

use common::{FakeToolchain, cli};
use predicates::prelude::*;
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

/// A fake `android` CLI that behaves like the real one: it puts the Kotlin
/// sources into the namespace it is given and mirrors it in the Gradle file.
/// It fails when `--namespace`/`--application-id` are missing, so a regression
/// that stops passing them is caught here.
const FAKE_ANDROID: &str = r#"#!/bin/sh
set -e

if [ "$1" != "create" ]; then
    echo "unsupported command: $1" >&2
    exit 1
fi
shift

name=""
namespace=""
application_id=""
output="."

while [ $# -gt 0 ]; do
    case "$1" in
        --name) name="$2"; shift 2 ;;
        --namespace) namespace="$2"; shift 2 ;;
        --application-id) application_id="$2"; shift 2 ;;
        --output|-o) output="$2"; shift 2 ;;
        *) shift ;;
    esac
done

if [ -z "$namespace" ] || [ -z "$application_id" ]; then
    echo "the fake android CLI requires --namespace and --application-id" >&2
    exit 1
fi

package_path=$(printf '%s' "$namespace" | tr '.' '/')
mkdir -p "$output/app/src/main/java/$package_path"
printf 'package %s\n' "$namespace" > "$output/app/src/main/java/$package_path/MainActivity.kt"

cat > "$output/app/build.gradle.kts" <<EOF
plugins {
    alias(libs.plugins.android.application)
}

android {
    namespace = "$namespace"
    compileSdk = 34

    defaultConfig {
        applicationId = "$application_id"
        minSdk = 24
        targetSdk = 34
    }
}
EOF
"#;

/// A fake `android` CLI from before `--namespace`/`--application-id` existed:
/// it always derives the package from the project name and ignores the flags.
const FAKE_ANDROID_LEGACY: &str = r#"#!/bin/sh
set -e

if [ "$1" != "create" ]; then
    echo "unsupported command: $1" >&2
    exit 1
fi
shift

name=""
output="."

while [ $# -gt 0 ]; do
    case "$1" in
        --name) name="$2"; shift 2 ;;
        --output|-o) output="$2"; shift 2 ;;
        *) shift ;;
    esac
done

mkdir -p "$output/app/src/main/java/com/example/legacy"
printf 'package com.example.legacy\n' > "$output/app/src/main/java/com/example/legacy/MainActivity.kt"

cat > "$output/app/build.gradle.kts" <<EOF
android {
    namespace = "com.example.legacy"

    defaultConfig {
        applicationId = "com.example.legacy"
    }
}
EOF
"#;

/// The fake `android`/`xcodegen` executables used to run the CLI under test.
fn toolchain(android_script: &str) -> FakeToolchain {
    FakeToolchain::new(&[
        ("android", android_script),
        ("xcodegen", "#!/bin/sh\nexit 0\n"),
    ])
}

/// Runs `create` in a fresh working directory and returns the project root.
fn create(tools: &FakeToolchain, args: &[&str]) -> (TempDir, PathBuf) {
    let work = TempDir::new().unwrap();
    cli(tools)
        .current_dir(work.path())
        .args(args)
        .assert()
        .success();
    let project = work.path().join(args[1]);
    (work, project)
}

#[test]
fn create_with_custom_package_generates_a_consistent_android_project() {
    let tools = toolchain(FAKE_ANDROID);
    let (_work, project) = create(&tools, &["create", "my-app", "--package", "io.acme"]);

    // the Gradle project uses the requested package
    let gradle = fs::read_to_string(project.join("android/app/build.gradle.kts")).unwrap();
    assert!(
        gradle.contains("namespace = \"io.acme.my_app\""),
        "{gradle}"
    );
    assert!(
        gradle.contains("applicationId = \"io.acme.my_app\""),
        "{gradle}"
    );

    // the Kotlin source lives in a directory that mirrors its package
    let kt_path = project.join("android/app/src/main/java/io/acme/my_app/MainActivity.kt");
    let kt = fs::read_to_string(&kt_path).unwrap();
    assert!(kt.starts_with("package io.acme.my_app\n"), "{kt}");
}

#[test]
fn create_with_default_package_generates_a_consistent_android_project() {
    let tools = toolchain(FAKE_ANDROID);
    let (_work, project) = create(&tools, &["create", "my-app"]);

    let gradle = fs::read_to_string(project.join("android/app/build.gradle.kts")).unwrap();
    assert!(
        gradle.contains("namespace = \"com.example.my_app\""),
        "{gradle}"
    );

    let kt_path = project.join("android/app/src/main/java/com/example/my_app/MainActivity.kt");
    let kt = fs::read_to_string(&kt_path).unwrap();
    assert!(kt.starts_with("package com.example.my_app\n"), "{kt}");
}

#[test]
fn create_with_custom_package_generates_a_consistent_ios_project() {
    let tools = toolchain(FAKE_ANDROID);
    let (_work, project) = create(&tools, &["create", "my-app", "--package", "io.acme"]);

    let yml = fs::read_to_string(project.join("ios/project.yml")).unwrap();
    // the prefix stays the prefix, the bundle id carries the project name
    assert!(yml.contains("bundleIdPrefix: io.acme"), "{yml}");
    assert!(
        yml.contains("PRODUCT_BUNDLE_IDENTIFIER: io.acme.my-app"),
        "{yml}"
    );
}

#[test]
fn create_with_custom_package_keeps_the_cargo_package_name() {
    let tools = toolchain(FAKE_ANDROID);
    let (_work, project) = create(&tools, &["create", "my-app", "--package", "io.acme"]);

    // the Cargo package name is the project name, the lib name is derived from it
    let cargo_toml = fs::read_to_string(project.join("rust/Cargo.toml")).unwrap();
    assert!(cargo_toml.contains("name = \"my-app\""), "{cargo_toml}");
}

#[test]
fn create_with_custom_package_creates_the_jni_libs_directories() {
    let tools = toolchain(FAKE_ANDROID);
    let (_work, project) = create(&tools, &["create", "my-app", "--package", "io.acme"]);

    for abi in ["arm64-v8a", "armeabi-v7a", "x86_64", "x86"] {
        let dir = project.join(format!("android/app/src/main/jniLibs/{abi}"));
        assert!(dir.is_dir(), "missing {}", dir.display());
    }
}

#[test]
fn create_fails_when_the_android_cli_uses_another_package() {
    let tools = toolchain(FAKE_ANDROID_LEGACY);
    let work = TempDir::new().unwrap();

    cli(&tools)
        .current_dir(work.path())
        .args(["create", "my-app", "--package", "io.acme"])
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("com.example.legacy"))
        .stderr(predicate::str::contains("io.acme.my_app"));
}

#[test]
fn create_declares_the_rust_targets_it_needs() {
    let tools = toolchain(FAKE_ANDROID);
    let (_work, project) = create(&tools, &["create", "my-app"]);

    // the project declares the targets it needs, so it does not depend on the
    // machine being set up beforehand
    let toolchain = fs::read_to_string(project.join("rust-toolchain.toml")).unwrap();
    for target in [
        "aarch64-linux-android",
        "armv7-linux-androideabi",
        "x86_64-linux-android",
        "i686-linux-android",
        "aarch64-apple-ios",
        "aarch64-apple-ios-sim",
    ] {
        assert!(toolchain.contains(target), "missing {target}: {toolchain}");
    }
    assert!(toolchain.contains("channel = \"stable\""), "{toolchain}");
}

#[test]
fn a_failed_create_leaves_nothing_behind() {
    let tools = toolchain(FAKE_ANDROID_LEGACY);
    let work = TempDir::new().unwrap();

    cli(&tools)
        .current_dir(work.path())
        .args(["create", "my-app", "--package", "io.acme"])
        .assert()
        .failure()
        .code(1);

    // neither the project nor the staging directory may be left over
    let leftovers: Vec<_> = fs::read_dir(work.path())
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert!(leftovers.is_empty(), "left over: {leftovers:?}");
}

#[test]
fn a_failed_create_can_be_retried() {
    let tools = toolchain(FAKE_ANDROID_LEGACY);
    let work = TempDir::new().unwrap();

    cli(&tools)
        .current_dir(work.path())
        .args(["create", "my-app"])
        .assert()
        .failure()
        .code(1);

    // the retry must not be blocked by the failed attempt
    cli(&tools)
        .current_dir(work.path())
        .args(["create", "my-app"])
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("already exists").not());
}
