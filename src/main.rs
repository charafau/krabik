use anyhow::{Result, anyhow};
use clap::{Parser, Subcommand};

use std::fs;

use std::process::Command;

fn command_exists(command: &str) -> bool {
    Command::new(command).arg("--version").output().is_ok()
}

#[derive(Parser)]
#[command(version, about, long_about=None)]
struct Args {
    #[command(subcommand)]
    command: Commands,
    // #[arg(short,long)]
    // doctor: bool,

    // #[arg(short,long)]
    // create: Option<String>,

    // #[arg(short,long)]
    // package: Option<String>,

    // #[arg(short,long)]
    // build: Option<String>,

    // #[arg(short,long)]
    // run: Option<String>,
}

#[derive(Subcommand)]
enum Commands {
    /// Check the development environment
    Doctor,

    /// Create a new project
    Create { name: String },

    /// Install necessary rust toolchains
    Toolchain,

    /// List available devices
    Devices,

    /// Launch given emulator
    Launch { emulator: String },

    /// Run on given device (build and launch)
    Run { device: String },

    /// Build release build
    Release,

    /// Generate UniFFI Android (Kotlin) bindings
    Bindings,
    // Package the project
    // Package {
    //     name: String,
    // },

    // /// Build the project
    // Build {
    //     name: String,
    // },

    // /// Run the project
    // Run {
    //     name: String,
    // },
}

fn target_installed(target: &str) -> bool {
    let output = Command::new("rustup")
        .args(["target", "list", "--installed"])
        .output()
        .expect("failed to execute rustup");

    String::from_utf8_lossy(&output.stdout)
        .lines()
        .any(|line| line == target)
}

const ANDROID_TARGETS: [&str; 4] = [
    "aarch64-linux-android",
    "armv7-linux-androideabi",
    "x86_64-linux-android",
    "i686-linux-android",
];

const IOS_TARGETS: [&str; 2] = ["aarch64-apple-ios", "aarch64-apple-ios-sim"];

fn install_android_toolchain() {
    Command::new("rustup")
        .args(["target", "add"])
        .args(ANDROID_TARGETS)
        .status()
        .expect("failed to run rustup");
}

fn install_ios_toolchain() {
    Command::new("rustup")
        .args(["target", "add"])
        .args(IOS_TARGETS)
        .status()
        .expect("failed to run rustup");
}

fn check_rust() {
    println!("[Rust]");
    if command_exists("cargo") {
        println!("\x1b[32m[✓]\x1b[0m cargo is installed");
    } else {
        println!("\x1b[31m[✗]\x1b[0m cargo is not installed");
    };
}

fn check_android() {
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
            if target_installed(target) {
                "\x1b[32m[✓]\x1b[0m"
            } else {
                "\x1b[31m[✗]\x1b[0m"
            }
        );
    }
}

fn check_ios() {
    println!("[iOS]");
    if command_exists("xcodebuild") {
        println!("\x1b[32m[✓]\x1b[0m xcode is installed");
    } else {
        println!("\x1b[31m[✗]\x1b[0m xcode is not installed");
    };
    if command_exists("xcodegen") {
        println!("\x1b[32m[✓]\x1b[0m xcodegen is installed");
    } else {
        println!("\x1b[31m[✗]\x1b[0m xcodegen is not installed");
    };
    for target in IOS_TARGETS {
        println!(
            "{} target {target}",
            if target_installed(target) {
                "\x1b[32m[✓]\x1b[0m"
            } else {
                "\x1b[31m[✗]\x1b[0m"
            }
        );
    }
}

fn create_rust(name: &str) {
    Command::new("cargo")
        .args(["new", &format!("{name}/{name}"), "--lib"])
        .status()
        .expect("failed to run cargo");

    fs::rename(format!("{name}/{name}"), format!("{name}/rust")).unwrap();

    let cargo_toml_path = format!("{name}/rust/Cargo.toml");
    let buf = fs::read_to_string(&cargo_toml_path).expect("failed to read Cargo.toml");

    let mut lines: Vec<&str> = buf.lines().collect();
    lines.truncate(lines.len().saturating_sub(2));
    lines.push("");
    lines.push("[lib]");
    lines.push("crate-type = [\"cdylib\"]");
    lines.push("");
    fs::write(&cargo_toml_path, lines.join("\n")).expect("failed to write to file");

    Command::new("cargo")
        .args(["add", "uniffi", "--features", "cli", "--manifest-path", &cargo_toml_path])
        .status()
        .expect("failed to add uniffi dependency");

    Command::new("cargo")
        .args(["add", "--build", "uniffi", "--features", "build", "--manifest-path", &cargo_toml_path])
        .status()
        .expect("failed to add uniffi build-dependency");

    Command::new("cargo")
        .args(["add", "uniffi-bindgen-kotlin-jni", "--manifest-path", &cargo_toml_path])
        .status()
        .expect("failed to add uniffi-bindgen-kotlin-jni dependency");

    // Command::new("cargo")
    //     .args(["add", "uniffi_bindgen", "--manifest-path", &cargo_toml_path])
    //     .status()
    //     .expect("failed to add uniffi_bindgen dependency");

    create_cargo_config(name).expect("failed to create cargo config");

    // Append [[bin]] target for uniffi-bindgen to Cargo.toml
    let mut cargo_toml_content = fs::read_to_string(&cargo_toml_path).expect("failed to read Cargo.toml");
    cargo_toml_content.push_str(
        "\n[[bin]]\nname = \"uniffi-bindgen\"\npath = \"src/uniffi-bindgen.rs\"\n"
    );
    fs::write(&cargo_toml_path, cargo_toml_content).expect("failed to update Cargo.toml with uniffi-bindgen bin target");

    // // Create build.rs
    // let build_rs_path = format!("{name}/rust/build.rs");
    // let build_rs_content = "fn main() {\n    uniffi::setup_scaffolding(\"src/lib.rs\").unwrap();\n}\n";
    // fs::write(build_rs_path, build_rs_content).expect("failed to write build.rs");

    // Create src/uniffi-bindgen.rs
    let bindgen_rs_path = format!("{name}/rust/src/uniffi-bindgen.rs");
    let bindgen_rs_content = "fn main() {\n    let _ = uniffi_bindgen_kotlin_jni::main();\n}\n";
    fs::write(bindgen_rs_path, bindgen_rs_content).expect("failed to write uniffi-bindgen.rs");
}

fn create_cargo_config(name: &str) -> Result<()> {
    let cargo_dir = format!("./{name}/rust/.cargo");
    fs::create_dir_all(&cargo_dir)?;

    let config_path = format!("{cargo_dir}/config.toml");
    let ndk_root = std::env::var("ANDROID_NDK_HOME")
        .unwrap_or_else(|_| "/Users/charafau/Utils/android-sdk/ndk/30.0.14904198".to_string());

    let config_content = format!(
        r#"[target.aarch64-linux-android]
linker = "{ndk_root}/toolchains/llvm/prebuilt/darwin-x86_64/bin/aarch64-linux-android24-clang"

[target.armv7-linux-androideabi]
linker = "{ndk_root}/toolchains/llvm/prebuilt/darwin-x86_64/bin/armv7a-linux-androideabi24-clang"

[target.i686-linux-android]
linker = "{ndk_root}/toolchains/llvm/prebuilt/darwin-x86_64/bin/i686-linux-android24-clang"

[target.x86_64-linux-android]
linker = "{ndk_root}/toolchains/llvm/prebuilt/darwin-x86_64/bin/x86_64-linux-android24-clang"
"#
    );

    fs::write(config_path, config_content)?;
    Ok(())
}

const ANDROID_JNI_FOLDERS: [&str; 4] = ["arm64-v8a", "armeabi-v7a", "x86", "x86_64"];

fn create_android(name: &str) {
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

    companion object {{
        init {{
            System.loadLibrary("{rust_lib_name}")
        }}
    }}
}}
"#
    );
    fs::write(&main_activity_path, kt_content)?;

    Ok(())
}

fn list_android_devices() {
    println!("Available Android Devices:\n");
    Command::new("android")
        .args(["emulator", "list"])
        .status()
        .expect("failed to list android devices");
}

fn launch_android_emulator(name: &str) {
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

fn get_lib_name() -> Result<String> {
    let cargo_toml = fs::read_to_string("./rust/Cargo.toml")?;
        let lib_name = cargo_toml
            .lines()
            .find_map(|line| {
                let line = line.trim();
                if line.starts_with("name =") || line.starts_with("name=") {
                    line.split('=').nth(1).map(|s| s.trim().trim_matches('"'))
                } else {
                    None
                }
            })
            .unwrap()
            .replace('-', "_");

    Ok(lib_name)
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

fn generate_kotlin_bindings(lib_path: &str) -> Result<()> {
    println!("\nGenerating Kotlin UniFFI bindings...");

    // UniFFI bindgen needs host library (.dylib/.so/.dll) or Android .so to parse UniFFI metadata.
    // If the given lib_path does not exist, build host target library first.
    let mut target_lib = lib_path.to_string();
    if !std::path::Path::new(&target_lib).exists() {
        println!("Target lib {} not found, building host library...", target_lib);
        Command::new("cargo")
            .args(["build", "--lib", "--target", "aarch64-linux-android"])
            .current_dir("./rust")
            .status()
            .expect("failed to build host rust library for binding generation");

        let lib_name = get_lib_name()?;
        let host_lib = format!("./rust/target/aarch64-linux-android/debug/lib{}.so", lib_name.replace('-', "_"));
        target_lib = host_lib;
    }

    // Adjust paths for running from rust directory
    let lib_path_for_bindgen = if target_lib.starts_with("./rust/") {
        target_lib.replacen("./rust/", "../rust/", 1)
    } else if target_lib.starts_with("rust/") {
        format!("../{}", target_lib)
    } else {
        target_lib.clone()
    };
    let args = [
        "run",
        "--bin",
        "uniffi-bindgen",
        "--",
        "bindings",
        &lib_path_for_bindgen,
        "../android",
    ];

    println!("Running command: cargo {}", args.join(" "));

    Command::new("cargo")
        .args(args)
        .current_dir("./rust")
        .status()
        .expect("failed to generate UniFFI bindings");

    // Generated bindings are placed according to the new bindgen; copy as needed
    let lib_name = get_lib_name()?;
    let android_java = format!("android/app/src/main/java");
    // Try various possible output locations for the new bindgen
    let candidates = vec![
        format!("android/uniffi"),
        format!("android/app/src/main/java/uniffi"),
        format!("android/src/main/java/uniffi"),
    ];
    for c in &candidates {
        if std::path::Path::new(c).exists() {
            // Copy the whole uniffi directory
            let entries = std::fs::read_dir(c)?;
            for entry in entries.flatten() {
                let src_path = entry.path();
                if src_path.is_dir() {
                    let dir_name = src_path.file_name().unwrap();
                    let dest_dir = format!("{}/uniffi/{}", android_java, dir_name.to_string_lossy());
                    std::fs::create_dir_all(&dest_dir)?;
                    if let Ok(kt_files) = std::fs::read_dir(&src_path) {
                        for kentry in kt_files.flatten() {
                            let kpath = kentry.path();
                            if kpath.extension().is_some_and(|ext| ext == "kt") {
                                let fname = kpath.file_name().unwrap();
                                std::fs::copy(&kpath, format!("{}/{}", dest_dir, fname.to_string_lossy()))?;
                            }
                        }
                    }
                } else if src_path.extension().is_some_and(|ext| ext == "kt") {
                    let fname = src_path.file_name().unwrap();
                    let dest_dir = format!("{}/uniffi", android_java);
                    std::fs::create_dir_all(&dest_dir)?;
                    std::fs::copy(&src_path, format!("{}/{}", dest_dir, fname.to_string_lossy()))?;
                }
            }
            break;
        }
    }

    Ok(())
}

/// builds Android library (only debug), one target
fn build_android_debug(name: &str) -> Result<()> {
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
        let lib_name = get_lib_name()?;

        let so_filename = format!("lib{lib_name}.so");
        let src_path = format!("./rust/target/aarch64-linux-android/debug/{so_filename}");
        let dst_dir = "android/app/src/main/jniLibs/arm64-v8a";

        fs::create_dir_all(dst_dir)?;
        fs::copy(&src_path, format!("{dst_dir}/{so_filename}"))?;

        generate_kotlin_bindings(&src_path)?;

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

fn build_android_release() -> Result<()> {
    println!("\nBuilding android release...");

    if std::env::var("ANDROID_NDK_HOME").is_ok() {
        export_ndk_variables()?;

        println!("\nBuilding rust lib...");

        let lib_name = get_lib_name()?;
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

fn generate_android_bindings() -> Result<()> {
    let lib_name = get_lib_name()?;
    let so_filename = format!("lib{lib_name}.so");
    let src_path = format!("./rust/target/aarch64-linux-android/debug/{so_filename}");
    generate_kotlin_bindings(&src_path)
}

fn main() {
    let args = Args::parse();

    match args.command {
        Commands::Doctor => {
            println!("doctor\n");
            check_rust();
            println!();
            check_android();
            println!();
            check_ios();
        }
        Commands::Create { name } => {
            println!("create: {}", name);

            //create subdir
            fs::create_dir(format!("./{name}/")).expect("failed to create folder");

            create_rust(&name);
            create_android(&name);
        }
        Commands::Toolchain => {
            install_android_toolchain();
            install_ios_toolchain();
        }
        Commands::Devices => {
            list_android_devices();
            // list_ios_devices();
        }
        Commands::Launch { emulator } => {
            launch_android_emulator(&emulator);
        }
        Commands::Run { device } => {
            let _ = build_android_debug(&device);
        }
        Commands::Release => {
            let _ = build_android_release();
        }
        Commands::Bindings => {
            let _ = generate_android_bindings();
        }
    }
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
