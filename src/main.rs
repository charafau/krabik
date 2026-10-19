mod android;
mod rust;

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

const IOS_TARGETS: [&str; 2] = ["aarch64-apple-ios", "aarch64-apple-ios-sim"];

fn install_ios_toolchain() {
    Command::new("rustup")
        .args(["target", "add"])
        .args(IOS_TARGETS)
        .status()
        .expect("failed to run rustup");
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
            if rust::target_installed(target) {
                "\x1b[32m[✓]\x1b[0m"
            } else {
                "\x1b[31m[✗]\x1b[0m"
            }
        );
    }
}

fn main() {
    let args = Args::parse();

    match args.command {
        Commands::Doctor => {
            println!("doctor\n");
            rust::check_rust();
            println!();
            android::check_android();
            println!();
            check_ios();
        }
        Commands::Create { name } => {
            println!("create: {}", name);

            //create subdir
            fs::create_dir(format!("./{name}/")).expect("failed to create folder");

            rust::create_rust(&name);
            android::create_android(&name);
        }
        Commands::Toolchain => {
            android::install_android_toolchain();
            install_ios_toolchain();
        }
        Commands::Devices => {
            android::list_android_devices();
            // list_ios_devices();
        }
        Commands::Launch { emulator } => {
            android::launch_android_emulator(&emulator);
        }
        Commands::Run { device } => {
            let _ = android::build_android_debug(&device);
        }
        Commands::Release => {
            let _ = android::build_android_release();
        }
        Commands::Bindings => {
            let _ = android::generate_android_bindings();
        }
    }
}
