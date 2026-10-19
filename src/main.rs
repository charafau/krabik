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

fn main() {
    let args = Args::parse();

    match args.command {
        Commands::Doctor => {
            println!("doctor");
            if command_exists("android") {
                println!("\x1b[32m[✓]\x1b[0m android is installed");
            } else {
                println!("\x1b[31m[✗]\x1b[0m android is not installed");
            };

            if command_exists("xcodegen") {
                println!("\x1b[32m[✓]\x1b[0m xcodegen is installed");
            } else {
                println!("\x1b[31m[✗]\x1b[0m xcodegen is not installed");
            };

            if command_exists("cargo") {
                println!("\x1b[32m[✓]\x1b[0m cargo is installed");
            } else {
                println!("\x1b[31m[✗]\x1b[0m cargo is not installed");
            };

            for target in ANDROID_TARGETS {
                println!(
                    "{} {target}",
                    if target_installed(target) {
                        "\x1b[32m[✓]\x1b[0m"
                    } else {
                        "\x1b[31m[✗]\x1b[0m"
                    }
                );
            }

            for target in IOS_TARGETS {
                println!(
                    "{} {target}",
                    if target_installed(target) {
                        "\x1b[32m[✓]\x1b[0m"
                    } else {
                        "\x1b[31m[✗]\x1b[0m"
                    }
                );
            }
        }
        Commands::Create { name } => {
            println!("create: {}", name);

            //create subdir
            fs::create_dir(&format!("./{name}/")).expect("failed to create folder");
            // for rust

            Command::new("cargo")
                .args(["new", &format!("{name}/rust")])
                .status()
                .expect("failed to run flutter");
            // for android
            Command::new("android")
                .args([
                    "create",
                    "--name",
                    &format!("{name}"),
                    "-o",
                    &format!("./{name}/android/"),
                ])
                .status()
                .expect("failed to run flutter");
        }
    }
}
