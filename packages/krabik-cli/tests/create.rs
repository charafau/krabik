//! End-to-end tests for the `create` command.
//!
//! The `android` and `xcodegen` executables are replaced by small shell
//! scripts so that the generated project can be inspected without a full
//! Android/Xcode toolchain being installed.

#![cfg(unix)]

mod common;

use common::{FakeToolchain, cli, fake_dsl};
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
fn create_inside_a_workspace_does_not_register_the_project() {
    let tools = toolchain(FAKE_ANDROID);
    let work = TempDir::new().unwrap();
    let manifest = work.path().join("Cargo.toml");
    let original = "[workspace]\nmembers = []\nresolver = \"2\"\n";
    fs::write(&manifest, original).unwrap();

    cli(&tools)
        .current_dir(work.path())
        .args(["create", "my-app"])
        .assert()
        .success();

    // the generated crate is standalone, but the enclosing workspace is untouched
    assert!(work.path().join("my-app/rust/Cargo.toml").is_file());
    assert_eq!(fs::read_to_string(&manifest).unwrap(), original);
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

/// A toolchain without the `android` CLI: a krabik-ui-dsl project is created
/// without it. The fake `xcodegen` records the directory it was run in.
fn dsl_toolchain() -> FakeToolchain {
    FakeToolchain::new(&[(
        "xcodegen",
        "#!/bin/sh\necho \"xcodegen $* in $(basename \"$PWD\")\" >> \"$FAKE_LOG\"\nexit 0\n",
    )])
}

/// Runs `create --with-dsl` with a fake krabik-ui-dsl directory and returns
/// the working directory, the project root and the framework's directory.
fn create_dsl(args: &[&str]) -> (TempDir, PathBuf, PathBuf) {
    let tools = dsl_toolchain();
    let work = TempDir::new().unwrap();
    // the path is written into the project in its canonical form
    let dsl = fs::canonicalize(fake_dsl(work.path())).unwrap();

    cli(&tools)
        .current_dir(work.path())
        .env("FAKE_LOG", work.path().join("calls.log"))
        .args(["create", args[0], "--with-dsl", "--dsl-path"])
        .arg(&dsl)
        .args(&args[1..])
        .assert()
        .success();

    let project = work.path().join(args[0]);
    (work, project, dsl)
}

#[test]
fn create_with_dsl_creates_an_app_crate_that_uses_the_framework() {
    let (_work, project, dsl) = create_dsl(&["my-app"]);

    let cargo_toml = fs::read_to_string(project.join("rust/Cargo.toml")).unwrap();
    assert!(cargo_toml.contains("name = \"my-app\""), "{cargo_toml}");
    assert!(
        cargo_toml.contains(&format!(
            "krabik-ui-dsl = {{ path = \"{}/rust\", features = [\"hot-reload\"] }}",
            dsl.display()
        )),
        "{cargo_toml}"
    );

    let lib = fs::read_to_string(project.join("rust/src/lib.rs")).unwrap();
    assert!(lib.contains("fn my_app_install()"), "{lib}");
    assert!(
        lib.contains("fn Java_com_example_my_1app_AppBridge_install"),
        "{lib}"
    );
    assert!(lib.contains("hot_reload!(Counter)"), "{lib}");

    // nothing of the UniFFI template is created
    assert!(!project.join("rust/uniffi-bindgen").exists());
    assert!(!project.join("rust/uniffi-bindgen-swift").exists());
    assert!(!project.join("rust/build.rs").exists());
    assert!(!project.join("ios/Generated").exists());

    // the targets are declared like in every other project
    let toolchain = fs::read_to_string(project.join("rust-toolchain.toml")).unwrap();
    assert!(toolchain.contains("aarch64-linux-android"), "{toolchain}");
    assert!(toolchain.contains("aarch64-apple-ios-sim"), "{toolchain}");
}

#[test]
fn create_with_dsl_creates_an_android_shell_around_the_framework() {
    use std::os::unix::fs::PermissionsExt;

    let (_work, project, dsl) = create_dsl(&["my-app", "--package", "io.acme"]);
    let android = project.join("android");

    // the boilerplate comes from the template app of the framework
    for file in [
        "gradlew.bat",
        "gradle/libs.versions.toml",
        "gradle/wrapper/gradle-wrapper.properties",
        "gradle.properties",
        "build.gradle.kts",
        ".gitignore",
        "app/src/main/AndroidManifest.xml",
        "app/src/main/res/values/themes.xml",
    ] {
        assert!(android.join(file).is_file(), "missing {file}");
    }
    let gradlew = fs::metadata(android.join("gradlew")).unwrap();
    assert!(gradlew.permissions().mode() & 0o111 != 0, "not executable");

    // the template's own sources, tests and build output are left out
    assert!(!android.join("app/build").exists());
    assert!(!android.join("app/src/androidTest").exists());
    assert!(!android.join("app/src/main/java/com/example").exists());

    let settings = fs::read_to_string(android.join("settings.gradle.kts")).unwrap();
    assert!(
        settings.contains("rootProject.name = \"my-app\""),
        "{settings}"
    );
    assert!(
        settings.contains(&format!("file(\"{}/shared/android\")", dsl.display())),
        "{settings}"
    );

    let gradle = fs::read_to_string(android.join("app/build.gradle.kts")).unwrap();
    assert!(
        gradle.contains("namespace = \"io.acme.my_app\""),
        "{gradle}"
    );
    assert!(
        gradle.contains("applicationId = \"io.acme.my_app\""),
        "{gradle}"
    );

    let sources = android.join("app/src/main/java/io/acme/my_app");
    let activity = fs::read_to_string(sources.join("MainActivity.kt")).unwrap();
    assert!(
        activity.starts_with("package io.acme.my_app\n"),
        "{activity}"
    );
    assert!(
        activity.contains("System.loadLibrary(\"my_app\")"),
        "{activity}"
    );
    let bridge = fs::read_to_string(sources.join("AppBridge.kt")).unwrap();
    assert!(bridge.starts_with("package io.acme.my_app\n"), "{bridge}");

    // the JNI function is the one the bridge declares
    let lib = fs::read_to_string(project.join("rust/src/lib.rs")).unwrap();
    assert!(
        lib.contains("fn Java_io_acme_my_1app_AppBridge_install"),
        "{lib}"
    );

    // the name of the template app is replaced
    let strings = fs::read_to_string(android.join("app/src/main/res/values/strings.xml")).unwrap();
    assert!(strings.contains(">my-app<"), "{strings}");
}

#[test]
fn create_with_dsl_creates_an_ios_shell_around_the_framework() {
    let (work, project, dsl) = create_dsl(&["my-app", "--package", "io.acme"]);
    let ios = project.join("ios");

    let yml = fs::read_to_string(ios.join("project.yml")).unwrap();
    assert!(yml.contains("\nname: MyApp\n"), "{yml}");
    assert!(
        yml.contains("PRODUCT_BUNDLE_IDENTIFIER: io.acme.my-app"),
        "{yml}"
    );
    assert!(yml.contains("RUST_LIB: my_app"), "{yml}");
    assert!(
        yml.contains(&format!("- path: \"{}/shared/ios\"", dsl.display())),
        "{yml}"
    );
    assert!(
        yml.contains(&format!("\"{}/shared/build-rust-ios.sh\"", dsl.display())),
        "{yml}"
    );

    let header = fs::read_to_string(ios.join("Sources/Bridging.h")).unwrap();
    assert!(header.contains("void my_app_install(void);"), "{header}");
    let swift = fs::read_to_string(ios.join("Sources/MyAppApp.swift")).unwrap();
    assert!(swift.contains("my_app_install()"), "{swift}");
    assert!(ios.join("Signing.xcconfig").is_file());

    // the Xcode project is generated in the iOS directory
    let log = fs::read_to_string(work.path().join("calls.log")).unwrap();
    assert!(log.contains("xcodegen generate in ios"), "{log}");
}

#[test]
fn create_with_dsl_uses_the_checkout_of_the_cli_by_default() {
    let tools = dsl_toolchain();
    let work = TempDir::new().unwrap();

    cli(&tools)
        .current_dir(work.path())
        .env("FAKE_LOG", work.path().join("calls.log"))
        .args(["create", "demo", "--with-dsl"])
        .assert()
        .success();

    let project = work.path().join("demo");
    let cargo_toml = fs::read_to_string(project.join("rust/Cargo.toml")).unwrap();
    assert!(
        cargo_toml.contains("/packages/krabik-ui-dsl/rust\""),
        "{cargo_toml}"
    );

    // the real Gradle wrapper and launcher icons are copied
    assert!(
        project
            .join("android/gradle/wrapper/gradle-wrapper.jar")
            .is_file()
    );
    assert!(
        project
            .join("android/app/src/main/res/mipmap-hdpi/ic_launcher.webp")
            .is_file()
    );
    assert!(!project.join("android/app/src/androidTest").exists());
}

#[test]
fn create_with_dsl_fails_for_a_directory_that_is_not_the_framework() {
    let tools = dsl_toolchain();
    let work = TempDir::new().unwrap();
    let other = TempDir::new().unwrap();

    cli(&tools)
        .current_dir(work.path())
        .args(["create", "demo", "--with-dsl", "--dsl-path"])
        .arg(other.path())
        .assert()
        .failure()
        .code(1)
        .stderr(predicate::str::contains("is not a krabik-ui-dsl directory"))
        .stderr(predicate::str::contains("--dsl-path"));

    // nothing is left behind, not even the staging directory
    let leftovers: Vec<_> = fs::read_dir(work.path())
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert!(leftovers.is_empty(), "left over: {leftovers:?}");
}

#[test]
fn create_rejects_a_dsl_path_without_with_dsl() {
    let tools = dsl_toolchain();
    let work = TempDir::new().unwrap();

    cli(&tools)
        .current_dir(work.path())
        .args(["create", "demo", "--dsl-path", "."])
        .assert()
        .failure()
        .stderr(predicate::str::contains("--with-dsl"));

    assert!(!work.path().join("demo").exists());
}
