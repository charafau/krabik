use anyhow::{Result, anyhow};

use std::fs;
use std::process::Command;

use crate::{command_exists, rust};

const ANDROID_TARGETS: [&str; 4] = [
    "aarch64-linux-android",
    "armv7-linux-androideabi",
    "x86_64-linux-android",
    "i686-linux-android",
];

pub fn install_android_toolchain() {
    Command::new("rustup")
        .args(["target", "add"])
        .args(ANDROID_TARGETS)
        .status()
        .expect("failed to run rustup");
}

pub fn check_android() {
    println!("[Android]");
    if command_exists("android") {
        println!("\x1b[32m[✓]\x1b[0m android CLI is installed");
    } else {
        println!("\x1b[31m[✗]\x1b[0m android CLI is not installed");
    };
    if std::env::var("ANDROID_NDK_HOME").is_ok() {
        println!("\x1b[32m[✓]\x1b[0m ANDROID_NDK_HOME environment variable is set");
    } else {
        println!("\x1b[31m[✗]\x1b[0m ANDROID_NDK_HOME environment variable is not set");
    };
    for target in ANDROID_TARGETS {
        println!(
            "{} target {target}",
            if rust::target_installed(target) {
                "\x1b[32m[✓]\x1b[0m"
            } else {
                "\x1b[31m[✗]\x1b[0m"
            }
        );
    }
}

const ANDROID_JNI_FOLDERS: [&str; 4] = ["arm64-v8a", "armeabi-v7a", "x86", "x86_64"];

pub fn create_android(name: &str) {
    Command::new("android")
        .args([
            "create",
            "--name",
            name,
            "-o",
            &format!("./{name}/android/"),
        ])
        .status()
        .expect("failed to run android CLI");

    for folder in ANDROID_JNI_FOLDERS {
        let path = format!("./{name}/android/app/src/main/jniLibs/{folder}");
        fs::create_dir_all(path).expect("failed to create directory");
    }

    modify_main_activity(name).expect("failed to modify main activity");
}

fn modify_main_activity(name: &str) -> Result<()> {
    // 1. Update rust/src/lib.rs with UniFFI proc macros
    let lib_rs_path = format!("./{name}/rust/src/lib.rs");
    let rust_lib_content = r#"uniffi::setup_scaffolding!();

include!(concat!(env!("OUT_DIR"), "/uniffi_bindgen_kotlin_jni.uniffi.rs"));

#[uniffi::export]
pub fn get_greeting() -> String {
    "Hello from Rust!".to_string()
}
"#;
    fs::write(&lib_rs_path, rust_lib_content)?;

    // 2. Create activity_main.xml layout with LinearLayout and TextView
    let layout_dir = format!("./{name}/android/app/src/main/res/layout");
    fs::create_dir_all(&layout_dir)?;
    let layout_xml_path = format!("{layout_dir}/activity_main.xml");
    let layout_content = r#"<?xml version="1.0" encoding="utf-8"?>
<LinearLayout xmlns:android="http://schemas.android.com/apk/res/android"
    android:id="@+id/main_layout"
    android:layout_width="match_parent"
    android:layout_height="match_parent"
    android:orientation="vertical"
    android:gravity="center"
    android:padding="16dp">

    <TextView
        android:id="@+id/sample_text"
        android:layout_width="wrap_content"
        android:layout_height="wrap_content"
        android:textSize="20sp"
        android:textStyle="bold"
        android:text="Loading..." />

</LinearLayout>
"#;
    fs::write(&layout_xml_path, layout_content)?;

    // 3. Update MainActivity.kt to load the Rust library and set layout text
    let main_activity_dir = format!("./{name}/android/app/src/main/java/com/example/{name}");
    fs::create_dir_all(&main_activity_dir)?;
    let main_activity_path = format!("{main_activity_dir}/MainActivity.kt");
    let rust_lib_name = name.replace('-', "_");
    let kt_content = format!(
        r#"package com.example.{name}

import android.app.Activity
import android.os.Bundle
import android.widget.TextView
import uniffi.{rust_lib_name}.getGreeting

class MainActivity : Activity() {{

    override fun onCreate(savedInstanceState: Bundle?) {{
        super.onCreate(savedInstanceState)
        setContentView(R.layout.activity_main)

        val tv = findViewById<TextView>(R.id.sample_text)
        tv.text = getGreeting()
    }}
}}
"#
    );
    fs::write(&main_activity_path, kt_content)?;

    Ok(())
}

pub fn list_android_devices() {
    println!("Available Android Devices:\n");
    Command::new("android")
        .args(["emulator", "list"])
        .status()
        .expect("failed to list android devices");
}

pub fn launch_android_emulator(name: &str) {
    println!("Launching Adnroid Emulator...\n");
    Command::new("android")
        .args(["emulator", "start", name])
        .status()
        .expect("failed to launch emulator");
}

//NDK_TOOLCHAIN="$ANDROID_NDK_HOME/toolchains/llvm/prebuilt/darwin-x86_64/bin"

// for now only this for debug
// export CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER="$NDK_TOOLCHAIN/aarch64-linux-android28-clang"
// export CARGO_TARGET_ARMV7_LINUX_ANDROIDEABI_LINKER="$NDK_TOOLCHAIN/armv7a-linux-androideabi28-clang"
// export CARGO_TARGET_I686_LINUX_ANDROID_LINKER="$NDK_TOOLCHAIN/i686-linux-android28-clang"
// export CARGO_TARGET_X86_64_LINUX_ANDROID_LINKER="$NDK_TOOLCHAIN/x86_64-linux-android28-clang"

const ANDROID_NDK_LINKERS: [(&str, &str); 4] = [
    (
        "CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER",
        "aarch64-linux-android",
    ),
    (
        "CARGO_TARGET_ARMV7_LINUX_ANDROIDEABI_LINKER",
        "armv7a-linux-androideabi",
    ),
    (
        "CARGO_TARGET_I686_LINUX_ANDROID_LINKER",
        "i686-linux-android",
    ),
    (
        "CARGO_TARGET_X86_64_LINUX_ANDROID_LINKER",
        "x86_64-linux-android",
    ),
];

fn get_ndk_toolchain_compilers(
    sdk_version: u32,
) -> Result<std::collections::HashMap<&'static str, String>> {
    let mut compilers = std::collections::HashMap::new();

    for (env_var, compiler_prefix) in ANDROID_NDK_LINKERS {
        compilers.insert(env_var, format!("{compiler_prefix}{sdk_version}-clang"));
    }

    Ok(compilers)
}

fn export_ndk_variables() -> Result<()> {
    if std::env::var("ANDROID_NDK_HOME").is_ok() {
        let ndk_root = std::env::var("ANDROID_NDK_HOME").expect("ANDROID_NDK_HOME not set");

        for (linker, compiler) in get_ndk_toolchain_compilers(28)? {
            unsafe {
                //TODO: support linux and windows
                let compiler_path =
                    format!("{ndk_root}/toolchains/llvm/prebuilt/darwin-x86_64/bin/{compiler}");
                println!(
                    "Setting enviroment variable: linker: {} ; value: {}",
                    linker, compiler_path
                );

                std::env::set_var(linker, compiler_path.clone());
            }
        }

        Ok(())
    } else {
        Err(anyhow!("Android NDK not found"))
    }
}

fn parse_android_package_name(content: &str) -> Result<String> {
    for line in content.lines() {
        let line = line.trim();
        if line.starts_with("namespace") || line.starts_with("applicationId") {
            let value_part = if line.contains('=') {
                line.split('=').nth(1)
            } else {
                line.split_whitespace().nth(1)
            };

            if let Some(val) = value_part {
                let pkg = val.trim().trim_matches('"').trim_matches('\'');
                if !pkg.is_empty() {
                    return Ok(pkg.to_string());
                }
            }
        }
    }

    Err(anyhow!("Could not find Android package name in build.gradle.kts"))
}

fn get_android_package_name() -> Result<String> {
    let content = fs::read_to_string("./android/app/build.gradle.kts")?;
    parse_android_package_name(&content)
}

fn launch_android_app(device: Option<&str>) -> Result<()> {
    let package_name = get_android_package_name()?;
    println!("\nLaunching app {package_name} via adb...");

    let activity = format!("{package_name}/{package_name}.MainActivity");
    let mut cmd = Command::new("adb");

    let _ = device;

    // TODO - if more than 1 device, print a list and ask user to select one
    cmd.args(["shell", "am", "start", "-n", &activity]);
    cmd.status().expect("failed to launch app via adb");

    Ok(())
}

fn generate_kotlin_bindings() -> Result<()> {
    println!("\nGenerating Kotlin UniFFI bindings...");

    // uniffi-bindgen-kotlin-jni parses the Rust sources directly (`src:<crate>`),
    // so no compiled library is needed. Output goes straight into the Android source set
    // (`uniffi/Uniffi.kt` and `uniffi/<crate>/uniffi.<crate>.kt`).
    let lib_name = rust::get_lib_name()?;
    let source = format!("src:{lib_name}");
    let args = [
        "run",
        "-p",
        "uniffi-bindgen",
        "--",
        "bindings",
        &source,
        "../android/app/src/main/java",
    ];

    println!("Running command: cargo {}", args.join(" "));

    let status = Command::new("cargo")
        .args(args)
        .current_dir("./rust")
        .status()
        .expect("failed to generate UniFFI bindings");

    if !status.success() {
        return Err(anyhow!("UniFFI bindings generation failed"));
    }

    Ok(())
}

/// builds Android library (only debug), one target
pub fn build_android_debug(name: &str) -> Result<()> {
    println!("\nBuilding android...");

    if std::env::var("ANDROID_NDK_HOME").is_ok() {
        // let ndk_toolchain_path = std::env::var("ANDROID_NDK_HOME")? + "/aarch64-linux-android28-clang";
        export_ndk_variables()?;

        // cargo build --release --manifest-path "$RUST_DIR/Cargo.toml" --target aarch64-linux-android

        println!("\nBuilding rust lib...");
        Command::new("cargo")
            .args([
                "build",
                "--manifest-path",
                "./rust/Cargo.toml",
                "--target",
                "aarch64-linux-android",
            ])
            .status()
            .expect("failed to build");

        // Copy rust libs
        let lib_name = rust::get_lib_name()?;

        let so_filename = format!("lib{lib_name}.so");
        let src_path = format!("./rust/target/aarch64-linux-android/debug/{so_filename}");
        let dst_dir = "android/app/src/main/jniLibs/arm64-v8a";

        fs::create_dir_all(dst_dir)?;
        fs::copy(&src_path, format!("{dst_dir}/{so_filename}"))?;

        generate_kotlin_bindings()?;

        // Compile Android project
        Command::new("./android/gradlew")
            .args(["-p", "android", "app:installDebug"])
            .status()
            .expect("failed to build android project");

        println!("\nAndroid build done.");

        // Launch apk
        launch_android_app(if name.is_empty() { None } else { Some(name) })?;

        Ok(())
    } else {
        Err(anyhow!("Android NDK not found"))
    }

    // fs::copy(, to)

    // add gradle launch
}

pub fn build_android_release() -> Result<()> {
    println!("\nBuilding android release...");

    if std::env::var("ANDROID_NDK_HOME").is_ok() {
        export_ndk_variables()?;

        println!("\nBuilding rust lib...");

        let lib_name = rust::get_lib_name()?;
        let so_filename = format!("lib{lib_name}.so");

        for target in ANDROID_TARGETS {
            Command::new("cargo")
                .args([
                    "build",
                    "--manifest-path",
                    "./rust/Cargo.toml",
                    "--target",
                    target,
                    "--release",
                ])
                .status()
                .expect("failed to build");

            let jni_folder = match target {
                "aarch64-linux-android" => "arm64-v8a",
                "armv7-linux-androideabi" => "armeabi-v7a",
                "x86_64-linux-android" => "x86_64",
                "i686-linux-android" => "x86",
                _ => continue,
            };

            let src_path = format!("./rust/target/{target}/release/{so_filename}");
            let dst_dir = format!("android/app/src/main/jniLibs/{jni_folder}");

            fs::create_dir_all(&dst_dir)?;
            fs::copy(&src_path, format!("{dst_dir}/{so_filename}"))?;
            println!("\nAndroid build for {target} done.");
        }

        Ok(())
    } else {
        Err(anyhow!("Android NDK not found"))
    }
}

//TODO: add build_android_release

pub fn generate_android_bindings() -> Result<()> {
    generate_kotlin_bindings()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_android_package_name() {
        let sample_gradle_kts = r#"
plugins {
    alias(libs.plugins.android.application)
    alias(libs.plugins.kotlin.android)
}

android {
    namespace = "com.example.isitgood"
    compileSdk = 34

    defaultConfig {
        applicationId = "com.example.isitgood"
        minSdk = 24
        targetSdk = 34
        versionCode = 1
        versionName = "1.0"

        testInstrumentationRunner = "androidx.test.runner.AndroidJUnitRunner"
    }

    buildTypes {
        release {
            isMinifyEnabled = false
            proguardFiles(
                getDefaultProguardFile("proguard-android-optimize.txt"),
                "proguard-rules.pro"
            )
        }
    }
    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_1_8
        targetCompatibility = JavaVersion.VERSION_1_8
    }
    kotlinOptions {
        jvmTarget = "1.8"
    }
}

dependencies {
    implementation(libs.androidx.core.ktx)
    implementation(libs.androidx.appcompat)
    implementation(libs.material)
    testImplementation(libs.junit)
    androidTestImplementation(libs.androidx.junit)
    androidTestImplementation(libs.androidx.espresso.core)
}
"#;

        let pkg = parse_android_package_name(sample_gradle_kts).unwrap();
        assert_eq!(pkg, "com.example.isitgood");
    }
}
