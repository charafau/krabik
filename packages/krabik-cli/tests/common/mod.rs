//! Helpers shared by the integration tests.

#![cfg(unix)]

use assert_cmd::Command;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

/// A directory with fake executables that are put on the `PATH` when the CLI
/// under test is run.
pub struct FakeToolchain {
    _dir: TempDir,
    bin: PathBuf,
}

impl FakeToolchain {
    /// Creates a toolchain with the given executables, e.g. `("android", script)`.
    pub fn new(tools: &[(&str, &str)]) -> Self {
        let dir = TempDir::new().unwrap();
        let bin = dir.path().join("bin");
        fs::create_dir_all(&bin).unwrap();

        for (name, script) in tools {
            write_executable(&bin.join(name), script);
        }

        Self { _dir: dir, bin }
    }

    /// The `PATH` to run the CLI with: the fake tools first, the real ones after.
    pub fn path(&self) -> String {
        let current = std::env::var("PATH").unwrap_or_default();
        format!("{}:{current}", self.bin.display())
    }
}

/// The CLI under test, with the fake toolchain on the `PATH`.
pub fn cli(tools: &FakeToolchain) -> Command {
    let mut cmd = Command::cargo_bin("krabik").expect("binary should build");
    cmd.env("PATH", tools.path());
    cmd
}

/// Writes a script and makes it executable.
pub fn write_executable(path: &Path, script: &str) {
    fs::write(path, script).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
}

/// A fake `gradlew` that records its arguments in `FAKE_LOG`.
#[allow(dead_code)]
pub const FAKE_GRADLEW: &str = "#!/bin/sh\necho \"gradlew $*\" >> \"$FAKE_LOG\"\nexit 0\n";

/// Creates a directory that looks like the krabik-ui-dsl directory of a
/// checkout, as far as `create --with-dsl` reads it, and returns its path.
///
/// The Android project of the template app has a `gradlew` that records its
/// calls, and files that must not end up in a created project.
#[allow(dead_code)]
pub fn fake_dsl(parent: &Path) -> PathBuf {
    let dsl = parent.join("krabik-ui-dsl");

    let files = [
        ("rust/Cargo.toml", "[package]\nname = \"krabik-ui-dsl\"\n"),
        ("shared/android/rust.gradle.kts", "// builds ../rust\n"),
        ("shared/ios/KrabikIOS.h", "// the C interface\n"),
        ("shared/build-rust-ios.sh", "#!/bin/sh\n"),
        ("example-todo-minimal/android/gradlew.bat", "@rem gradle\n"),
        (
            "example-todo-minimal/android/gradle/libs.versions.toml",
            "[versions]\nagp = \"9.0.1\"\n",
        ),
        (
            "example-todo-minimal/android/gradle/wrapper/gradle-wrapper.properties",
            "distributionUrl=gradle-9.1.0-bin.zip\n",
        ),
        (
            "example-todo-minimal/android/gradle.properties",
            "android.useAndroidX=true\n",
        ),
        (
            "example-todo-minimal/android/build.gradle.kts",
            "// top-level build file\n",
        ),
        ("example-todo-minimal/android/.gitignore", "/build\n"),
        (
            "example-todo-minimal/android/app/src/main/AndroidManifest.xml",
            "<manifest><application android:label=\"@string/app_name\" /></manifest>\n",
        ),
        (
            "example-todo-minimal/android/app/src/main/res/values/themes.xml",
            "<resources />\n",
        ),
        (
            "example-todo-minimal/android/app/src/main/res/values/strings.xml",
            "<resources><string name=\"app_name\">Todo Minimal Example</string></resources>\n",
        ),
        // the template's own sources, tests and build output are not boilerplate
        (
            "example-todo-minimal/android/settings.gradle.kts",
            "rootProject.name = \"krabik-ui-dsl-todo-minimal\"\n",
        ),
        (
            "example-todo-minimal/android/app/build.gradle.kts",
            "android { namespace = \"com.example.krabik_ui_dsl\" }\n",
        ),
        (
            "example-todo-minimal/android/app/src/main/java/com/example/krabik_ui_dsl/MinimalTodoBridge.kt",
            "package com.example.krabik_ui_dsl\n",
        ),
        (
            "example-todo-minimal/android/app/src/androidTest/java/MinimalTodoTest.kt",
            "// a test\n",
        ),
        ("example-todo-minimal/android/app/build/output.apk", "apk"),
    ];

    for (path, content) in files {
        let path = dsl.join(path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, content).unwrap();
    }

    write_executable(
        &dsl.join("example-todo-minimal/android/gradlew"),
        FAKE_GRADLEW,
    );

    dsl
}
