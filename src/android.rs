//! Android support: the project template, the toolchain and the build.

use anyhow::{Context, Result, anyhow};
use clap::ValueEnum;

use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::naming::ProjectName;
use crate::remove::{remove_files_with_extension, remove_if_exists};
use crate::rust::InstalledTargets;
use crate::{check_mark, command_exists, report_check, run_command, rust};

/// A Rust target that can be built for Android.
pub struct AndroidTarget {
    /// The Rust target triple, e.g. `aarch64-linux-android`.
    pub arch: &'static str,
    /// The Android ABI, which is also the name of the jniLibs folder.
    pub jni_folder: &'static str,
    /// The Cargo environment variable that selects the linker for the target.
    pub linker_env: &'static str,
    /// The name of the NDK clang compiler for the target.
    pub clang: &'static str,
}

/// The Rust targets a project can be built for on Android.
pub const ANDROID_TARGETS: [AndroidTarget; 4] = [
    AndroidTarget {
        arch: "aarch64-linux-android",
        jni_folder: "arm64-v8a",
        linker_env: "CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER",
        clang: "aarch64-linux-android",
    },
    AndroidTarget {
        arch: "armv7-linux-androideabi",
        jni_folder: "armeabi-v7a",
        linker_env: "CARGO_TARGET_ARMV7_LINUX_ANDROIDEABI_LINKER",
        clang: "armv7a-linux-androideabi",
    },
    AndroidTarget {
        arch: "x86_64-linux-android",
        jni_folder: "x86_64",
        linker_env: "CARGO_TARGET_X86_64_LINUX_ANDROID_LINKER",
        clang: "x86_64-linux-android",
    },
    AndroidTarget {
        arch: "i686-linux-android",
        jni_folder: "x86",
        linker_env: "CARGO_TARGET_I686_LINUX_ANDROID_LINKER",
        clang: "i686-linux-android",
    },
];

/// The Android ABI a device runs on, as reported by `ro.product.cpu.abi`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum Abi {
    /// 64 bit ARM
    #[value(name = "arm64-v8a")]
    Arm64V8a,
    /// 32 bit ARM
    #[value(name = "armeabi-v7a")]
    ArmeabiV7a,
    /// 64 bit x86
    #[value(name = "x86_64")]
    X86_64,
    /// 32 bit x86
    #[value(name = "x86")]
    X86,
}

impl Abi {
    /// The ABI as it is named by Android, e.g. `arm64-v8a`.
    pub fn as_str(self) -> &'static str {
        match self {
            Abi::Arm64V8a => "arm64-v8a",
            Abi::ArmeabiV7a => "armeabi-v7a",
            Abi::X86_64 => "x86_64",
            Abi::X86 => "x86",
        }
    }
}

/// Returns the Rust target for the given Android ABI, e.g. `arm64-v8a`.
fn target_for_abi(abi: &str) -> Result<&'static AndroidTarget> {
    ANDROID_TARGETS
        .iter()
        .find(|target| target.jni_folder == abi)
        .ok_or_else(|| anyhow!("unsupported Android ABI {abi:?}"))
}

/// Reads the primary ABI of a device via `adb shell getprop ro.product.cpu.abi`.
fn device_abi(serial: &str) -> Result<String> {
    let output = Command::new("adb")
        .args(["-s", serial, "shell", "getprop", "ro.product.cpu.abi"])
        .output()
        .with_context(|| format!("failed to run adb shell getprop for {serial}"))?;

    if !output.status.success() {
        return Err(anyhow!("adb shell getprop failed ({})", output.status));
    }

    parse_device_abi(&String::from_utf8_lossy(&output.stdout)).ok_or_else(|| {
        anyhow!("could not determine the ABI of {serial}; the device may not be ready yet")
    })
}

/// Extracts the ABI from the output of `adb shell getprop ro.product.cpu.abi`.
fn parse_device_abi(output: &str) -> Option<String> {
    let abi = output.trim();
    if abi.is_empty() {
        None
    } else {
        Some(abi.to_string())
    }
}

/// Removes the shared libraries of other architectures from the jniLibs
/// directory, they may be outdated.
fn clear_jni_libs(jni_libs: &Path) -> Result<()> {
    if !jni_libs.is_dir() {
        return Ok(());
    }

    for abi_dir in fs::read_dir(jni_libs)? {
        let abi_dir = abi_dir?.path();
        if !abi_dir.is_dir() {
            continue;
        }

        for entry in fs::read_dir(&abi_dir)? {
            let path = entry?.path();
            if path.extension().and_then(|ext| ext.to_str()) == Some("so") {
                fs::remove_file(&path)
                    .with_context(|| format!("failed to remove {}", path.display()))?;
            }
        }
    }

    Ok(())
}

/// Adds required Android Rust compilation targets via `rustup target add`.
pub fn install_android_toolchain() -> Result<()> {
    let mut cmd = Command::new("rustup");
    cmd.args(["target", "add"])
        .args(ANDROID_TARGETS.iter().map(|t| t.arch));
    run_command(&mut cmd, "rustup target add")
}

/// Doctor check verifying the Android toolchain: the Android CLI, adb, a JDK,
/// the SDK, the NDK and the Rust targets.
pub fn check_android(targets: &InstalledTargets) -> bool {
    println!("[Android]");
    let mut ok = true;

    ok &= report_check(
        command_exists("android"),
        "android CLI is installed",
        "android CLI is not installed",
    );
    ok &= report_check(
        command_exists("adb"),
        "adb is installed",
        "adb is not installed",
    );
    ok &= report_check(
        command_exists("java"),
        "java is installed",
        "java is not installed",
    );
    ok &= check_java_home();
    ok &= check_android_sdk();
    ok &= check_ndk();

    for target in ANDROID_TARGETS {
        let installed = targets.contains(target.arch);
        println!("{} target {}", check_mark(installed), target.arch);
        ok &= installed;
    }
    ok
}

/// Checks that `JAVA_HOME` points at an existing directory when it is set.
///
/// Gradle fails with a confusing error when `JAVA_HOME` is set but invalid,
/// even when a `java` binary is available on the `PATH`.
fn check_java_home() -> bool {
    let Some(java_home) = std::env::var_os("JAVA_HOME") else {
        // Gradle falls back to the java binary on the PATH
        return true;
    };

    if Path::new(&java_home).is_dir() {
        println!(
            "{} JAVA_HOME points at an existing directory",
            check_mark(true)
        );
        true
    } else {
        println!(
            "{} JAVA_HOME points to a missing directory: {}",
            check_mark(false),
            java_home.to_string_lossy()
        );
        false
    }
}

/// The location of the Android SDK.
fn android_sdk_dir() -> Option<PathBuf> {
    // the Android CLI knows best where it finds the SDK
    if let Some(dir) = sdk_from_cli_info() {
        return Some(dir);
    }

    for var in ["ANDROID_HOME", "ANDROID_SDK_ROOT"] {
        if let Some(dir) = std::env::var_os(var) {
            return Some(PathBuf::from(dir));
        }
    }

    None
}

/// Reads the SDK location from the output of `android info`.
fn sdk_from_cli_info() -> Option<PathBuf> {
    let output = Command::new("android").args(["info"]).output().ok()?;
    if !output.status.success() {
        return None;
    }

    parse_sdk_info(&String::from_utf8_lossy(&output.stdout))
}

/// Extracts the SDK path from the output of `android info`.
fn parse_sdk_info(output: &str) -> Option<PathBuf> {
    output.lines().find_map(|line| {
        let value = line.trim().strip_prefix("sdk:")?.trim();
        if value.is_empty() {
            None
        } else {
            Some(PathBuf::from(value))
        }
    })
}

/// Checks that the Android SDK is available.
fn check_android_sdk() -> bool {
    match android_sdk_dir() {
        Some(dir) if dir.is_dir() => {
            println!(
                "{} Android SDK is installed at {}",
                check_mark(true),
                dir.display()
            );
            true
        }
        Some(dir) => {
            println!(
                "{} Android SDK directory is missing: {}",
                check_mark(false),
                dir.display()
            );
            false
        }
        None => {
            println!(
                "{} Android SDK could not be found, set ANDROID_HOME",
                check_mark(false)
            );
            false
        }
    }
}

/// Checks that the NDK is configured and usable.
fn check_ndk() -> bool {
    let Ok(ndk_root) = std::env::var("ANDROID_NDK_HOME") else {
        println!(
            "{} ANDROID_NDK_HOME environment variable is not set",
            check_mark(false)
        );
        return false;
    };

    println!(
        "{} ANDROID_NDK_HOME environment variable is set",
        check_mark(true)
    );

    match find_ndk_prebuilt(&ndk_root) {
        Ok(prebuilt) => {
            println!(
                "{} ANDROID_NDK_HOME points at an NDK ({prebuilt})",
                check_mark(true)
            );
            true
        }
        Err(err) => {
            println!(
                "{} ANDROID_NDK_HOME does not point at an NDK: {err}",
                check_mark(false)
            );
            false
        }
    }
}

/// Paths that are only removed when `clean` is run with `--purge`.
const ANDROID_PURGE_PATHS: &[&str] = &["android/.gradle", "android/.kotlin"];

/// Removes the Android build artifacts.
pub fn clean_android(purge: bool) -> Result<()> {
    println!("\nAndroid:");

    let mut paths = vec![
        "android/build",
        "android/app/build",
        "android/app/src/main/java/uniffi",
    ];

    if purge {
        paths.extend(ANDROID_PURGE_PATHS);
    }

    for path in paths {
        remove_if_exists(path)?;
    }

    // The shared libraries are copied into jniLibs by `run` and `release`.
    remove_files_with_extension(Path::new("android/app/src/main/jniLibs"), "so")?;

    Ok(())
}

/// Creates the Android project template, cleans up default files, and configures jniLibs and MainActivity.
pub fn create_android(name: &ProjectName, package_prefix: &str, root: &Path) -> Result<()> {
    let package_id = name.java_package(package_prefix);
    let android_dir = root.join("android");

    // The `android` CLI puts the Kotlin sources into the namespace it is given,
    // so the package of the generated sources always matches the one we write.
    let mut output = android_dir.clone().into_os_string();
    output.push("/");

    let mut cmd = Command::new("android");
    cmd.arg("create")
        .arg("--name")
        .arg(name.dir_name())
        .arg("--namespace")
        .arg(&package_id)
        .arg("--application-id")
        .arg(&package_id)
        .arg("-o")
        .arg(output);
    run_command(&mut cmd, "android create")?;

    remove_kotlin_files(&android_dir).context("failed to remove Kotlin files")?;

    verify_android_package(&android_dir.join("app/build.gradle.kts"), &package_id)?;

    for target in ANDROID_TARGETS {
        let path = android_dir
            .join("app/src/main/jniLibs")
            .join(target.jni_folder);
        fs::create_dir_all(&path)
            .with_context(|| format!("failed to create {}", path.display()))?;
    }

    create_main_activity(name, package_prefix, root).context("failed to create MainActivity")?;

    Ok(())
}

/// Verifies that the generated Gradle project uses the package id we asked for.
fn verify_android_package(gradle_path: &Path, expected: &str) -> Result<()> {
    let content = fs::read_to_string(gradle_path)
        .with_context(|| format!("failed to read {}", gradle_path.display()))?;
    let (namespace, application_id) = parse_android_packages(&content);

    for (label, actual) in [
        ("namespace", namespace.as_deref()),
        ("applicationId", application_id.as_deref()),
    ] {
        match actual {
            Some(actual) if actual == expected => {}
            Some(actual) => {
                return Err(anyhow!(
                    "the generated Gradle project declares {label} {actual:?}, expected {expected:?}"
                ));
            }
            None => {
                return Err(anyhow!(
                    "could not find {label} in {}",
                    gradle_path.display()
                ));
            }
        }
    }

    Ok(())
}

/// The `android create` command creates a template with sample Kotlin files; this removes them.
fn remove_kotlin_files<P: AsRef<Path>>(path: P) -> Result<()> {
    let path = path.as_ref();
    if !path.exists() {
        return Ok(());
    }

    if path.is_dir() {
        for entry in fs::read_dir(path)? {
            let entry = entry?;
            let entry_path = entry.path();
            if entry_path.is_dir() {
                remove_kotlin_files(&entry_path)?;
                if !entry_path.components().any(|c| c.as_os_str() == "jniLibs")
                    && fs::read_dir(&entry_path)?.next().is_none()
                {
                    let _ = fs::remove_dir(&entry_path);
                }
            } else if entry_path.extension().and_then(|ext| ext.to_str()) == Some("kt") {
                fs::remove_file(&entry_path)?;
            }
        }
    } else if path.extension().and_then(|ext| ext.to_str()) == Some("kt") {
        fs::remove_file(path)?;
    }

    Ok(())
}

/// Creates the Android `MainActivity` and layout XML that load the Rust shared library and display a greeting.
fn create_main_activity(name: &ProjectName, package_prefix: &str, root: &Path) -> Result<()> {
    // 1. Update rust/src/lib.rs with UniFFI proc macros
    let lib_rs_path = root.join("rust/src/lib.rs");
    fs::write(&lib_rs_path, RUST_LIB_CONTENT)
        .with_context(|| format!("failed to write {}", lib_rs_path.display()))?;

    // 2. Create activity_main.xml layout with LinearLayout and TextView
    let layout_dir = root.join("android/app/src/main/res/layout");
    fs::create_dir_all(&layout_dir)
        .with_context(|| format!("failed to create {}", layout_dir.display()))?;
    let layout_xml_path = layout_dir.join("activity_main.xml");
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
    fs::write(&layout_xml_path, layout_content)
        .with_context(|| format!("failed to write {}", layout_xml_path.display()))?;

    // 3. Create MainActivity.kt to load the Rust library and set layout text
    //    The source directory has to mirror the package of the file.
    let main_activity_dir = root
        .join("android/app/src/main/java")
        .join(name.java_package_path(package_prefix));
    fs::create_dir_all(&main_activity_dir)
        .with_context(|| format!("failed to create {}", main_activity_dir.display()))?;
    let main_activity_path = main_activity_dir.join("MainActivity.kt");
    fs::write(
        &main_activity_path,
        main_activity_content(name, package_prefix),
    )
    .with_context(|| format!("failed to write {}", main_activity_path.display()))?;

    Ok(())
}

const RUST_LIB_CONTENT: &str = r#"uniffi::setup_scaffolding!();

include!(concat!(env!("OUT_DIR"), "/uniffi_bindgen_kotlin_jni.uniffi.rs"));

#[uniffi::export]
pub fn get_greeting() -> String {
    "Hello from Rust!".to_string()
}
"#;

/// Generates the Kotlin source code for `MainActivity` loading and calling the Rust library.
fn main_activity_content(name: &ProjectName, package_prefix: &str) -> String {
    format!(
        r#"package {package}

import android.app.Activity
import android.os.Bundle
import android.widget.TextView
import uniffi.{lib_name}.getGreeting

class MainActivity : Activity() {{

    override fun onCreate(savedInstanceState: Bundle?) {{
        super.onCreate(savedInstanceState)
        setContentView(R.layout.activity_main)

        val tv = findViewById<TextView>(R.id.sample_text)
        tv.text = getGreeting()
    }}
}}
"#,
        package = name.java_package(package_prefix),
        lib_name = name.identifier(),
    )
}

/// Android device information.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct AndroidDevice {
    /// The model of the device or the name of the emulator.
    pub name: String,
    /// The serial of the device, when it differs from the name.
    pub id: Option<String>,
    /// The state of the device, e.g. `online` or `offline`.
    pub state: String,
}

/// Lists connected Android devices and running emulators using `adb devices -l`.
/// If `all` is true, offline AVD emulators are also queried and listed.
pub fn list_android_devices(all: bool) {
    println!("Available Android Devices:\n");

    let online_devices = get_online_android_devices();
    let mut devices = online_devices.clone();

    if all {
        let offline_avds = get_offline_android_emulators();
        for avd in offline_avds {
            if !devices.iter().any(|d| d.name == avd) {
                devices.push(AndroidDevice {
                    name: avd,
                    id: None,
                    state: "offline".to_string(),
                });
            }
        }
    }

    println!("{}", format_android_devices(&devices, all));
}

/// Queries `adb devices -l` and returns a list of online Android devices and emulators.
fn get_online_android_devices() -> Vec<AndroidDevice> {
    let output = Command::new("adb").args(["devices", "-l"]).output();
    let stdout = match output {
        Ok(out) if out.status.success() => String::from_utf8_lossy(&out.stdout).to_string(),
        _ => return Vec::new(),
    };

    let mut devices = parse_adb_devices(&stdout);

    for dev in &mut devices {
        if let Some(id) = &dev.id {
            if id.starts_with("emulator-")
                && let Some(avd_name) = get_emulator_avd_name(id)
            {
                dev.name = avd_name;
            }
        } else if dev.name.starts_with("emulator-")
            && let Some(avd_name) = get_emulator_avd_name(&dev.name)
        {
            dev.id = Some(dev.name.clone());
            dev.name = avd_name;
        }
    }

    devices
}

/// Queries the AVD name for an emulator serial via `adb -s <serial> emu avd name`.
fn get_emulator_avd_name(serial: &str) -> Option<String> {
    let output = Command::new("adb")
        .args(["-s", serial, "emu", "avd", "name"])
        .output()
        .ok()?;

    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let first_line = stdout.lines().next()?.trim();
        if !first_line.is_empty() && first_line != "OK" {
            return Some(first_line.to_string());
        }
    }
    None
}

/// Queries the Android CLI for known emulator AVD names.
fn get_offline_android_emulators() -> Vec<String> {
    let output = Command::new("android").args(["emulator", "list"]).output();
    let stdout = match output {
        Ok(out) if out.status.success() => String::from_utf8_lossy(&out.stdout).to_string(),
        _ => return Vec::new(),
    };

    parse_emulator_list(&stdout)
}

/// Parses the stdout of `adb devices -l` into a vector of [`AndroidDevice`].
fn parse_adb_devices(output: &str) -> Vec<AndroidDevice> {
    let mut devices = Vec::new();

    for line in output.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with("List of devices attached") || line.starts_with('*')
        {
            continue;
        }

        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 2 {
            let id = parts[0];
            let status = parts[1];
            if status == "device" {
                let model = parts
                    .iter()
                    .find_map(|p| p.strip_prefix("model:"))
                    .unwrap_or(id);
                devices.push(AndroidDevice {
                    name: model.to_string(),
                    id: if model != id {
                        Some(id.to_string())
                    } else {
                        None
                    },
                    state: "online".to_string(),
                });
            }
        }
    }

    devices
}

/// Parses output from `android emulator list` into a list of AVD names.
fn parse_emulator_list(output: &str) -> Vec<String> {
    output
        .lines()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .collect()
}

/// Formats a slice of [`AndroidDevice`] into a user-friendly list string.
fn format_android_devices(devices: &[AndroidDevice], all: bool) -> String {
    if devices.is_empty() {
        if all {
            return "  No Android devices found".to_string();
        } else {
            return "  No online Android devices found".to_string();
        }
    }

    devices
        .iter()
        .map(|d| match &d.id {
            Some(id) => format!("  • {} ({}) [{}]", d.name, id, d.state),
            None => format!("  • {} [{}]", d.name, d.state),
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Starts an Android emulator by name via `android emulator start <name>`.
pub fn launch_android_emulator(name: &str) -> Result<()> {
    println!("Launching Android Emulator...\n");
    let mut cmd = Command::new("android");
    cmd.args(["emulator", "start", name]);
    run_command(&mut cmd, "android emulator start")
}

/// Resolves the serial of the device to work on.
///
/// Matches by model/AVD name or serial when `wanted` is given, otherwise the
/// only online device is used, or the user is asked when several are online.
pub fn resolve_device(wanted: &str) -> Result<String> {
    let wanted = if wanted.is_empty() {
        None
    } else {
        Some(wanted)
    };
    resolve_android_device(wanted)
}

/// Resolve the adb serial of the device to run on.
/// Matches by model/AVD name or serial when `wanted` is given; otherwise picks the
/// only online device, or prompts interactively when several are online.
fn resolve_android_device(wanted: Option<&str>) -> Result<String> {
    let devices: Vec<(String, String)> = get_online_android_devices()
        .into_iter()
        .filter(|d| d.state == "online")
        .map(|d| {
            let serial = d.id.clone().unwrap_or_else(|| d.name.clone());
            (d.name, serial)
        })
        .collect();

    let index = select_device_index(&devices, wanted)?;
    Ok(devices[index].1.clone())
}

/// Selects a device from the list, prompting the user interactively if multiple devices are available.
fn select_device_index(devices: &[(String, String)], wanted: Option<&str>) -> Result<usize> {
    if let Some(wanted) = wanted {
        return devices
            .iter()
            .position(|(name, serial)| name == wanted || serial == wanted)
            .ok_or_else(|| anyhow!("no online Android device named {wanted}"));
    }

    match devices.len() {
        0 => Err(anyhow!(
            "no online Android devices found; connect a device or start an emulator"
        )),
        1 => Ok(0),
        _ => {
            println!("More than one Android device is online:");
            for (i, (name, serial)) in devices.iter().enumerate() {
                println!("  {}. {name} ({serial})", i + 1);
            }
            print!("\nSelect a device [1-{}]: ", devices.len());
            io::stdout()
                .flush()
                .context("failed to flush device prompt")?;

            let mut input = String::new();
            io::stdin()
                .read_line(&mut input)
                .context("failed to read device selection")?;
            let index = parse_device_selection(&input, devices.len())?;
            Ok(index - 1)
        }
    }
}

/// Parses a 1-based numerical device selection string from user input.
fn parse_device_selection(input: &str, count: usize) -> Result<usize> {
    let selection: usize = input
        .trim()
        .parse()
        .map_err(|_| anyhow!("invalid device selection: {}", input.trim()))?;
    if selection == 0 || selection > count {
        return Err(anyhow!(
            "device selection {selection} is out of range 1-{count}"
        ));
    }
    Ok(selection)
}

const ANDROID_API_LEVEL: u32 = 24;

/// Maps Cargo linker environment variables to NDK clang compiler names for the given SDK level.
fn get_ndk_toolchain_compilers(
    sdk_version: u32,
) -> std::collections::HashMap<&'static str, String> {
    let mut compilers = std::collections::HashMap::new();

    for target in ANDROID_TARGETS {
        compilers.insert(
            target.linker_env,
            format!("{}{sdk_version}-clang", target.clang),
        );
    }

    compilers
}

/// Locates the NDK toolchain and returns the map of Cargo linker environment variables.
fn ndk_linker_env() -> Result<std::collections::HashMap<&'static str, String>> {
    let ndk_root = std::env::var("ANDROID_NDK_HOME")
        .map_err(|_| anyhow!("ANDROID_NDK_HOME is not set; point it at your Android NDK"))?;
    let prebuilt = find_ndk_prebuilt(&ndk_root)?;

    Ok(get_ndk_toolchain_compilers(ANDROID_API_LEVEL)
        .into_iter()
        .map(|(env_var, compiler)| {
            (
                env_var,
                format!("{ndk_root}/toolchains/llvm/prebuilt/{prebuilt}/bin/{compiler}"),
            )
        })
        .collect())
}

/// Locates the prebuilt host toolchain directory inside `$ANDROID_NDK_HOME`.
fn find_ndk_prebuilt(ndk_root: &str) -> Result<String> {
    let prebuilt_dir = format!("{ndk_root}/toolchains/llvm/prebuilt");
    let host = if cfg!(target_os = "macos") {
        "darwin"
    } else if cfg!(target_os = "windows") {
        "windows"
    } else {
        "linux"
    };

    let entries: Vec<String> = fs::read_dir(&prebuilt_dir)
        .with_context(|| format!("NDK toolchains not found at {prebuilt_dir}"))?
        .filter_map(Result::ok)
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .filter(|name| name.starts_with(host))
        .collect();

    let arch = if cfg!(target_arch = "aarch64") {
        "arm64"
    } else {
        "x86_64"
    };

    entries
        .iter()
        .find(|name| name.contains(arch))
        .or_else(|| entries.first())
        .cloned()
        .ok_or_else(|| anyhow!("no {host} prebuilt toolchain in {prebuilt_dir}"))
}

/// Extracts the `namespace` and `applicationId` values from `build.gradle.kts`.
fn parse_android_packages(content: &str) -> (Option<String>, Option<String>) {
    let mut namespace = None;
    let mut application_id = None;

    for line in content.lines() {
        let line = line.trim();
        if let Some(value) = gradle_string_value(line, "namespace") {
            namespace = Some(value);
        }
        if let Some(value) = gradle_string_value(line, "applicationId") {
            application_id = Some(value);
        }
    }

    (namespace, application_id)
}

/// Parses the string value of a Gradle property, e.g. `namespace = "com.example.app"`.
fn gradle_string_value(line: &str, key: &str) -> Option<String> {
    let rest = line.strip_prefix(key)?;

    // The key must not be the prefix of a longer identifier, e.g. `namespaceSuffix`.
    match rest.chars().next() {
        Some('=' | ' ' | '\t' | '"' | '\'') => {}
        _ => return None,
    }

    let rest = rest.trim_start();
    let rest = rest.strip_prefix('=').unwrap_or(rest).trim_start();

    let quote = rest.chars().next()?;
    if quote != '"' && quote != '\'' {
        return None;
    }

    let value = &rest[1..];
    let end = value.find(quote)?;
    let value = value[..end].trim();

    if value.is_empty() {
        None
    } else {
        Some(value.to_string())
    }
}

/// Reads `./android/app/build.gradle.kts` and returns the parsed Android package identifier.
fn get_android_package_name() -> Result<String> {
    let content = fs::read_to_string("./android/app/build.gradle.kts")
        .context("failed to read ./android/app/build.gradle.kts")?;
    let (namespace, application_id) = parse_android_packages(&content);
    namespace
        .or(application_id)
        .ok_or_else(|| anyhow!("Could not find Android package name in build.gradle.kts"))
}

/// Launches the app on a connected Android device or emulator using `adb shell am start`.
fn launch_android_app(serial: &str) -> Result<()> {
    let package_name = get_android_package_name()?;
    println!("\nLaunching app {package_name} via adb...");

    let activity = format!("{package_name}/{package_name}.MainActivity");
    let mut cmd = Command::new("adb");
    cmd.args(["-s", serial, "shell", "am", "start", "-n", &activity]);
    run_command(&mut cmd, "adb shell am start")
}

/// Generates Kotlin UniFFI bindings using `uniffi-bindgen`.
fn generate_kotlin_bindings() -> Result<()> {
    println!("\nGenerating Kotlin UniFFI bindings...");

    // uniffi-bindgen-kotlin-jni parses the Rust sources directly (`src:<crate>`),
    // so no compiled library is needed. Output goes straight into the Android source set
    // (`uniffi/Uniffi.kt` and `uniffi/<crate>/uniffi.<crate>.kt`).
    let args = bindgen_args(&rust::get_lib_name()?);

    println!("Running command: cargo {}", args.join(" "));

    let mut cmd = Command::new("cargo");
    cmd.args(args).current_dir("./rust");
    run_command(&mut cmd, "cargo run -p uniffi-bindgen")
}

/// Builds the cargo command arguments for generating Kotlin UniFFI bindings.
fn bindgen_args(lib_name: &str) -> Vec<String> {
    [
        "run",
        "-p",
        "uniffi-bindgen",
        "--",
        "bindings",
        &format!("src:{lib_name}"),
        "../android/app/src/main/java",
    ]
    .map(String::from)
    .to_vec()
}

/// Builds the Rust library for the architecture of the target device, generates
/// Kotlin bindings, installs and launches the app.
pub fn build_android_debug(device: &str, abi: Option<Abi>) -> Result<()> {
    println!("\nBuilding android...");

    let wanted = if device.is_empty() {
        None
    } else {
        Some(device)
    };
    let serial = resolve_android_device(wanted)?;

    // The library has to be built for the architecture of the device it is
    // installed on, otherwise the app cannot load it.
    let target = match abi {
        Some(abi) => target_for_abi(abi.as_str())?,
        None => target_for_abi(&device_abi(&serial)?)?,
    };

    println!(
        "\nBuilding rust lib for {} ({})...",
        target.arch, target.jni_folder
    );

    let ndk_linkers = ndk_linker_env()?;

    let mut cmd = Command::new("cargo");
    cmd.args([
        "build",
        "--manifest-path",
        "./rust/Cargo.toml",
        "--target",
        target.arch,
    ])
    .envs(&ndk_linkers);
    run_command(&mut cmd, "cargo build")?;

    // Copy rust libs
    let lib_name = rust::get_lib_name()?;

    let so_filename = format!("lib{lib_name}.so");
    let src_path = format!("./rust/target/{}/debug/{so_filename}", target.arch);
    let jni_libs = Path::new("android/app/src/main/jniLibs");
    let dst_dir = jni_libs.join(target.jni_folder);

    // The app is installed on a single device, so only its architecture is
    // needed and the libraries of an earlier run may be outdated.
    clear_jni_libs(jni_libs)?;

    fs::create_dir_all(&dst_dir)
        .with_context(|| format!("failed to create {}", dst_dir.display()))?;
    fs::copy(&src_path, dst_dir.join(&so_filename)).with_context(|| {
        format!(
            "failed to copy {src_path} to {}",
            dst_dir.join(&so_filename).display()
        )
    })?;

    generate_kotlin_bindings()?;

    // Compile Android project
    let mut gradle = Command::new("./android/gradlew");
    gradle
        .args(["-p", "android", "app:installDebug"])
        .env("ANDROID_SERIAL", &serial);
    run_command(&mut gradle, "gradlew app:installDebug")?;

    println!("\nAndroid build done.");

    launch_android_app(&serial)?;

    Ok(())
}

/// Builds release Rust shared libraries for all Android targets, generates bindings, and assembles the release APK.
pub fn build_android_release() -> Result<()> {
    println!("\nBuilding android release...");

    let ndk_linkers = ndk_linker_env()?;

    println!("\nBuilding rust lib...");

    let lib_name = rust::get_lib_name()?;
    let so_filename = format!("lib{lib_name}.so");

    for target in ANDROID_TARGETS {
        let mut cmd = Command::new("cargo");
        cmd.args([
            "build",
            "--manifest-path",
            "./rust/Cargo.toml",
            "--target",
            target.arch,
            "--release",
        ])
        .envs(&ndk_linkers);
        run_command(
            &mut cmd,
            &format!("cargo build --release --target {}", target.arch),
        )?;

        let src_path = format!("./rust/target/{}/release/{so_filename}", target.arch);
        let dst_dir = format!("android/app/src/main/jniLibs/{}", target.jni_folder);

        fs::create_dir_all(&dst_dir).with_context(|| format!("failed to create {dst_dir}"))?;
        fs::copy(&src_path, format!("{dst_dir}/{so_filename}"))
            .with_context(|| format!("failed to copy {src_path} to {dst_dir}"))?;
        println!("\nAndroid build for {} done.", target.arch);
    }

    generate_kotlin_bindings()?;

    println!("\nBuilding release APK...");
    let mut gradle = Command::new("./android/gradlew");
    gradle.args(["-p", "android", "app:assembleRelease"]);
    run_command(&mut gradle, "gradlew app:assembleRelease")?;

    println!(
        "\nAndroid release APK: android/app/build/outputs/apk/release/app-release-unsigned.apk"
    );

    Ok(())
}

/// Generates Android Kotlin bindings by invoking [`generate_kotlin_bindings`].
pub fn generate_android_bindings() -> Result<()> {
    generate_kotlin_bindings()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_target_for_abi() {
        let expected = [
            ("arm64-v8a", "aarch64-linux-android"),
            ("armeabi-v7a", "armv7-linux-androideabi"),
            ("x86_64", "x86_64-linux-android"),
            ("x86", "i686-linux-android"),
        ];

        for (abi, triple) in expected {
            let target = target_for_abi(abi).unwrap();
            assert_eq!(target.arch, triple, "{abi}");
            assert_eq!(target.jni_folder, abi);
        }
    }

    #[test]
    fn test_target_for_abi_unknown() {
        assert!(target_for_abi("mips").is_err());
        assert!(target_for_abi("").is_err());
        assert!(target_for_abi("arm64").is_err());
    }

    #[test]
    fn test_abi_as_str() {
        assert_eq!(Abi::Arm64V8a.as_str(), "arm64-v8a");
        assert_eq!(Abi::ArmeabiV7a.as_str(), "armeabi-v7a");
        assert_eq!(Abi::X86_64.as_str(), "x86_64");
        assert_eq!(Abi::X86.as_str(), "x86");
    }

    #[test]
    fn test_parse_device_abi() {
        assert_eq!(
            parse_device_abi("arm64-v8a\n").as_deref(),
            Some("arm64-v8a")
        );
        assert_eq!(parse_device_abi("x86_64\r\n").as_deref(), Some("x86_64"));
        assert_eq!(
            parse_device_abi("  armeabi-v7a  ").as_deref(),
            Some("armeabi-v7a")
        );
        assert!(parse_device_abi("").is_none());
        assert!(parse_device_abi("\n").is_none());
        assert!(parse_device_abi("   \r\n").is_none());
    }

    #[test]
    fn test_clear_jni_libs_removes_shared_libraries() {
        let unique_id = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let temp_dir = std::env::temp_dir().join(format!("test_jni_libs_{unique_id}"));

        let arm64 = temp_dir.join("arm64-v8a");
        let x86_64 = temp_dir.join("x86_64");
        fs::create_dir_all(&arm64).unwrap();
        fs::create_dir_all(&x86_64).unwrap();

        fs::write(arm64.join("libdemo.so"), "arm64").unwrap();
        fs::write(x86_64.join("libdemo.so"), "x86_64").unwrap();
        // other files have to be kept
        fs::write(x86_64.join("keep.txt"), "keep").unwrap();

        clear_jni_libs(&temp_dir).unwrap();

        assert!(!arm64.join("libdemo.so").exists());
        assert!(!x86_64.join("libdemo.so").exists());
        assert!(x86_64.join("keep.txt").exists());

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_clear_jni_libs_missing_directory() {
        let missing = std::env::temp_dir().join("jni_libs_missing_test_12345");
        assert!(clear_jni_libs(&missing).is_ok());
    }

    #[test]
    fn test_parse_sdk_info() {
        let info = "sdk: /Users/someone/Android/sdk\nversion: 1.0.0\nlauncher_version: 1.0.0\n";
        assert_eq!(
            parse_sdk_info(info),
            Some(PathBuf::from("/Users/someone/Android/sdk"))
        );
    }

    #[test]
    fn test_parse_sdk_info_without_sdk() {
        assert_eq!(parse_sdk_info("version: 1.0.0\n"), None);
        assert_eq!(parse_sdk_info("sdk:\n"), None);
        assert_eq!(parse_sdk_info("sdk:   \n"), None);
        assert_eq!(parse_sdk_info(""), None);
    }

    #[test]
    fn test_parse_sdk_info_ignores_other_keys() {
        // a key that merely ends in "sdk" must not be mistaken for the sdk
        assert_eq!(parse_sdk_info("toolsdk: /some/path\n"), None);
    }

    #[test]
    fn test_parse_android_packages() {
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

        let (namespace, application_id) = parse_android_packages(sample_gradle_kts);
        assert_eq!(namespace.as_deref(), Some("com.example.isitgood"));
        assert_eq!(application_id.as_deref(), Some("com.example.isitgood"));
    }

    #[test]
    fn test_parse_android_packages_groovy() {
        let gradle = "android {\n    namespace 'com.example.groovy'\n}\n";
        let (namespace, application_id) = parse_android_packages(gradle);
        assert_eq!(namespace.as_deref(), Some("com.example.groovy"));
        assert_eq!(application_id, None);
    }

    #[test]
    fn test_parse_android_packages_application_id_only() {
        let gradle = "defaultConfig {\n    applicationId = \"com.example.appid\"\n}\n";
        let (namespace, application_id) = parse_android_packages(gradle);
        assert_eq!(namespace, None);
        assert_eq!(application_id.as_deref(), Some("com.example.appid"));
    }

    #[test]
    fn test_parse_android_packages_skips_empty_value() {
        let gradle = "namespace = \"\"\napplicationId = \"com.example.fallback\"\n";
        let (namespace, application_id) = parse_android_packages(gradle);
        assert_eq!(namespace, None);
        assert_eq!(application_id.as_deref(), Some("com.example.fallback"));
    }

    #[test]
    fn test_parse_android_packages_missing() {
        let (namespace, application_id) =
            parse_android_packages("android {\n    compileSdk = 34\n}\n");
        assert_eq!(namespace, None);
        assert_eq!(application_id, None);
    }

    #[test]
    fn test_parse_android_packages_ignores_longer_keys() {
        // `namespaceSuffix` and `testApplicationId` must not be mistaken for
        // `namespace` and `applicationId`.
        let gradle =
            "namespaceSuffix = \"com.example.suffix\"\ntestApplicationId = \"com.example.test\"\n";
        let (namespace, application_id) = parse_android_packages(gradle);
        assert_eq!(namespace, None);
        assert_eq!(application_id, None);
    }

    #[test]
    fn test_parse_android_packages_keeps_equals_in_value() {
        let gradle = "namespace = \"com.example=a=b\"\n";
        let (namespace, _) = parse_android_packages(gradle);
        assert_eq!(namespace.as_deref(), Some("com.example=a=b"));
    }

    #[test]
    fn test_get_ndk_toolchain_compilers() {
        let compilers = get_ndk_toolchain_compilers(28);
        assert_eq!(compilers.len(), 4);
        assert_eq!(
            compilers["CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER"],
            "aarch64-linux-android28-clang"
        );
        assert_eq!(
            compilers["CARGO_TARGET_ARMV7_LINUX_ANDROIDEABI_LINKER"],
            "armv7a-linux-androideabi28-clang"
        );
        assert_eq!(
            compilers["CARGO_TARGET_I686_LINUX_ANDROID_LINKER"],
            "i686-linux-android28-clang"
        );
        assert_eq!(
            compilers["CARGO_TARGET_X86_64_LINUX_ANDROID_LINKER"],
            "x86_64-linux-android28-clang"
        );
    }

    #[test]
    fn test_ndk_linker_env_vars_match_targets() {
        // CARGO_TARGET_<TRIPLE>_LINKER, triple upper-cased with '-' -> '_'
        for target in ANDROID_TARGETS {
            let env_var = format!(
                "CARGO_TARGET_{}_LINKER",
                target.arch.to_uppercase().replace('-', "_")
            );
            assert_eq!(target.linker_env, env_var, "{}", target.arch);
        }
    }

    #[test]
    fn test_android_target_table() {
        let expected = [
            ("aarch64-linux-android", "arm64-v8a"),
            ("armv7-linux-androideabi", "armeabi-v7a"),
            ("x86_64-linux-android", "x86_64"),
            ("i686-linux-android", "x86"),
        ];
        assert_eq!(ANDROID_TARGETS.len(), expected.len());
        for (target, (triple, folder)) in ANDROID_TARGETS.iter().zip(expected) {
            assert_eq!(target.arch, triple);
            assert_eq!(target.jni_folder, folder, "{triple}");
        }
    }

    #[test]
    fn test_bindgen_args() {
        assert_eq!(
            bindgen_args("demo"),
            [
                "run",
                "-p",
                "uniffi-bindgen",
                "--",
                "bindings",
                "src:demo",
                "../android/app/src/main/java",
            ]
        );
    }

    #[test]
    fn test_main_activity_content() {
        let name = ProjectName::parse("demo").unwrap();
        let kt = main_activity_content(&name, "com.example");
        assert!(kt.starts_with("package com.example.demo\n"));
        assert!(kt.contains("import uniffi.demo.getGreeting"));
        assert!(kt.contains("tv.text = getGreeting()"));
        // the generated bindings load the native library themselves
        assert!(!kt.contains("System.loadLibrary"));
    }

    #[test]
    fn test_main_activity_content_custom_package() {
        let name = ProjectName::parse("demo").unwrap();
        let kt = main_activity_content(&name, "io.acme");
        assert!(kt.starts_with("package io.acme.demo\n"));
        assert!(kt.contains("import uniffi.demo.getGreeting"));
    }

    #[test]
    fn test_main_activity_content_hyphenated_name() {
        let name = ProjectName::parse("my-app").unwrap();
        let kt = main_activity_content(&name, "com.example");
        // a hyphen is not valid in a Kotlin package, it has to become an underscore
        assert!(kt.starts_with("package com.example.my_app\n"));
        assert!(kt.contains("import uniffi.my_app.getGreeting"));
    }

    #[test]
    fn test_main_activity_package_matches_source_dir() {
        for raw in ["demo", "my-app", "my_app"] {
            let name = ProjectName::parse(raw).unwrap();
            for prefix in ["com.example", "io.acme"] {
                let kt = main_activity_content(&name, prefix);
                let package = kt
                    .lines()
                    .next()
                    .and_then(|line| line.strip_prefix("package "))
                    .expect("first line declares the package");

                // the source directory has to mirror the package of the file
                assert_eq!(
                    name.java_package_path(prefix),
                    package.replace('.', "/"),
                    "{raw} / {prefix}"
                );
                // every segment has to be a valid Java identifier
                for segment in package.split('.') {
                    assert!(!segment.contains('-'), "{segment}");
                    assert!(!segment.is_empty(), "{package}");
                }
            }
        }
    }

    #[test]
    fn test_verify_android_package() {
        let unique_id = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("test_pkg_{unique_id}.gradle.kts"));
        fs::write(
            &path,
            "android {\n    namespace = \"com.example.my_app\"\n    defaultConfig {\n        applicationId = \"com.example.my_app\"\n    }\n}\n",
        )
        .unwrap();

        verify_android_package(&path, "com.example.my_app").unwrap();

        fs::remove_file(&path).unwrap();
    }

    #[test]
    fn test_verify_android_package_rejects_mismatch() {
        let unique_id = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("test_pkg_bad_{unique_id}.gradle.kts"));
        fs::write(
            &path,
            "android {\n    namespace = \"com.example.my_app\"\n    defaultConfig {\n        applicationId = \"com.example.my_app\"\n    }\n}\n",
        )
        .unwrap();

        let err = verify_android_package(&path, "io.acme.my_app")
            .unwrap_err()
            .to_string();
        assert!(err.contains("com.example.my_app"), "{err}");

        fs::remove_file(&path).unwrap();
    }

    #[test]
    fn test_rust_lib_content() {
        assert!(RUST_LIB_CONTENT.contains("uniffi::setup_scaffolding!();"));
        assert!(RUST_LIB_CONTENT.contains(
            r#"include!(concat!(env!("OUT_DIR"), "/uniffi_bindgen_kotlin_jni.uniffi.rs"));"#
        ));
        assert!(RUST_LIB_CONTENT.contains("#[uniffi::export]\npub fn get_greeting() -> String"));
    }

    #[test]
    fn test_remove_kotlin_files() {
        let unique_id = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let temp_dir = std::env::temp_dir().join(format!("test_remove_kt_{unique_id}"));
        let sub_dir = temp_dir.join("app/src/main/java/com/example/demo/theme");
        let jni_dir = temp_dir.join("app/src/main/jniLibs/arm64-v8a");
        let res_dir = temp_dir.join("app/src/main/res");
        fs::create_dir_all(&sub_dir).unwrap();
        fs::create_dir_all(&jni_dir).unwrap();
        fs::create_dir_all(&res_dir).unwrap();

        let kt_file1 = temp_dir.join("app/src/main/java/com/example/demo/MainActivity.kt");
        let kt_file2 = sub_dir.join("Theme.kt");
        let kts_file = temp_dir.join("build.gradle.kts");
        let xml_file = res_dir.join("AndroidManifest.xml");

        fs::write(&kt_file1, "package com.example.demo").unwrap();
        fs::write(&kt_file2, "package com.example.demo.theme").unwrap();
        fs::write(&kts_file, "plugins {}").unwrap();
        fs::write(&xml_file, "<manifest/>").unwrap();

        assert!(kt_file1.exists());
        assert!(kt_file2.exists());
        assert!(kts_file.exists());
        assert!(xml_file.exists());
        assert!(jni_dir.exists());

        remove_kotlin_files(&temp_dir).unwrap();

        assert!(!kt_file1.exists(), "MainActivity.kt should be removed");
        assert!(!kt_file2.exists(), "Theme.kt should be removed");
        assert!(
            !sub_dir.exists(),
            "empty theme package directory should be removed"
        );
        assert!(
            !temp_dir.join("app/src/main/java").exists(),
            "empty java directory tree should be removed"
        );
        assert!(kts_file.exists(), "build.gradle.kts should NOT be removed");
        assert!(
            xml_file.exists(),
            "AndroidManifest.xml should NOT be removed"
        );
        assert!(
            res_dir.exists(),
            "directory with non-kt files should NOT be removed"
        );
        assert!(jni_dir.exists(), "jniLibs directory should be preserved");

        let _ = fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_remove_kotlin_files_nonexistent_path() {
        let temp_dir = std::env::temp_dir().join("nonexistent_path_test_12345");
        assert!(remove_kotlin_files(&temp_dir).is_ok());
    }

    #[test]
    fn test_parse_adb_devices() {
        let sample = "List of devices attached\nemulator-5554          device product:sdk_gphone64_arm64 model:sdk_gphone64_arm64 device:emulator64_arm64 transport_id:1\n";
        let devices = parse_adb_devices(sample);
        assert_eq!(devices.len(), 1);
        assert_eq!(devices[0].name, "sdk_gphone64_arm64");
        assert_eq!(devices[0].id.as_deref(), Some("emulator-5554"));
        assert_eq!(devices[0].state, "online");
    }

    #[test]
    fn test_parse_adb_devices_empty() {
        let sample = "List of devices attached\n\n";
        let devices = parse_adb_devices(sample);
        assert!(devices.is_empty());
    }

    #[test]
    fn test_parse_emulator_list() {
        let sample = "Pixel_10\nPixel_Fold\n";
        let avds = parse_emulator_list(sample);
        assert_eq!(avds, vec!["Pixel_10", "Pixel_Fold"]);
    }

    #[test]
    fn test_format_android_devices() {
        let devices = vec![
            AndroidDevice {
                name: "Pixel_10".to_string(),
                id: Some("emulator-5554".to_string()),
                state: "online".to_string(),
            },
            AndroidDevice {
                name: "Pixel_Fold".to_string(),
                id: None,
                state: "offline".to_string(),
            },
        ];

        let formatted = format_android_devices(&devices, true);
        assert_eq!(
            formatted,
            "  • Pixel_10 (emulator-5554) [online]\n  • Pixel_Fold [offline]"
        );

        assert_eq!(
            format_android_devices(&[], false),
            "  No online Android devices found"
        );
        assert_eq!(
            format_android_devices(&[], true),
            "  No Android devices found"
        );
    }

    #[test]
    fn test_parse_device_selection() {
        assert_eq!(parse_device_selection("1\n", 3).unwrap(), 1);
        assert_eq!(parse_device_selection("3", 3).unwrap(), 3);
        assert!(parse_device_selection("0", 3).is_err());
        assert!(parse_device_selection("4", 3).is_err());
        assert!(parse_device_selection("abc", 3).is_err());
        assert!(parse_device_selection("", 3).is_err());
    }

    #[test]
    fn test_select_device_index_by_name_or_serial() {
        let devices = vec![
            ("Pixel_7".to_string(), "SERIAL1".to_string()),
            ("emulator".to_string(), "emulator-5554".to_string()),
        ];
        assert_eq!(select_device_index(&devices, Some("Pixel_7")).unwrap(), 0);
        assert_eq!(
            select_device_index(&devices, Some("emulator-5554")).unwrap(),
            1
        );
        assert!(select_device_index(&devices, Some("Nope")).is_err());
    }

    #[test]
    fn test_select_device_index_defaults_to_single_device() {
        let devices = vec![("only".to_string(), "SERIAL".to_string())];
        assert_eq!(select_device_index(&devices, None).unwrap(), 0);
        assert!(select_device_index(&[], None).is_err());
    }

    #[test]
    fn test_find_ndk_prebuilt_missing_ndk() {
        let missing = std::env::temp_dir().join("ndk_prebuilt_missing_test_12345");
        assert!(find_ndk_prebuilt(missing.to_str().unwrap()).is_err());
    }

    #[test]
    fn test_find_ndk_prebuilt_prefers_host_arch() {
        let unique_id = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let temp_dir = std::env::temp_dir().join(format!("test_ndk_prebuilt_{unique_id}"));
        let prebuilt = temp_dir.join("toolchains/llvm/prebuilt");

        let host = if cfg!(target_os = "macos") {
            "darwin"
        } else if cfg!(target_os = "windows") {
            "windows"
        } else {
            "linux"
        };
        let arch = if cfg!(target_arch = "aarch64") {
            "arm64"
        } else {
            "x86_64"
        };

        fs::create_dir_all(prebuilt.join(format!("{host}-other-arch"))).unwrap();
        fs::create_dir_all(prebuilt.join(format!("{host}-{arch}"))).unwrap();
        fs::create_dir_all(prebuilt.join("foreign-host")).unwrap();

        let found = find_ndk_prebuilt(temp_dir.to_str().unwrap()).unwrap();
        assert_eq!(found, format!("{host}-{arch}"));

        let _ = fs::remove_dir_all(&temp_dir);
    }
}
