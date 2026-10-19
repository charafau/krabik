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
    Launch { emulator: String }, // Package the project
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
        .args(["new", &format!("{name}/{name}")])
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
            // check_ios();
        }
        Commands::Launch { emulator } => {
            launch_android_emulator(&emulator);
        }
    }
}
