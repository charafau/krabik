use anyhow::{Ok, Result, anyhow};
use clap::{Parser, Subcommand};

use std::fs;
use std::intrinsics::copy;
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

    fs::rename(&format!("{name}/{name}"), &format!("{name}/rust")).unwrap();

    let cargo_toml_path = format!("{name}/rust/Cargo.toml");
    let buf = fs::read_to_string(&cargo_toml_path).expect("failed to read Cargo.toml");

    let mut lines: Vec<&str> = buf.lines().collect();
    lines.truncate(lines.len().saturating_sub(2));
    lines.push("");
    lines.push("[lib]");
    lines.push("crate-type = [\"cdylib\"]");
    lines.push("");
    lines.push("[dependencies]");

    fs::write(&cargo_toml_path, lines.join("\n")).expect("failed to write to file");
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
        .args(["emulator", "start", &format!("{name}")])
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

                std::env::set_var(linker, format!("{compiler_path}"));
            }
        }

        return Ok(());
    }

    return Err(anyhow!("ANDROID_NDK_HOME is not set"));
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

        // Compile Android project
        Command::new("./android/gradlew")
            .arg("app:installDebug")
            .status()
            .expect("failed to build android project");

        println!("\nAndroid build done.");

        return Ok(());
    }

    return Err(anyhow!("Android NDK not found"));

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

        return Ok(());
    }

    return Err(anyhow!("Android NDK not found"));
}

//TODO: add build_android_release

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
            fs::create_dir(&format!("./{name}/")).expect("failed to create folder");

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
            build_android_debug(&device);

            // launch_android_emulator(&device);
        }
        Commands::Release => {
            build_android_release();
        }
    }
}
