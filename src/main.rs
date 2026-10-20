mod android;
mod ios;
mod rust;

use clap::{Parser, Subcommand, ValueEnum};

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

#[derive(Clone, Copy, ValueEnum)]
enum Platform {
    Android,
    Ios,
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
    Devices {
        #[arg(long, value_enum, default_value_t = Platform::Android)]
        platform: Platform,
    },

    /// Launch given emulator
    Launch {
        emulator: String,
        #[arg(long, value_enum, default_value_t = Platform::Android)]
        platform: Platform,
    },

    /// Run on given device (build and launch)
    Run {
        #[arg(default_value = "")]
        device: String,
        #[arg(long, value_enum, default_value_t = Platform::Android)]
        platform: Platform,
    },

    /// Build release build
    Release {
        #[arg(long, value_enum, default_value_t = Platform::Android)]
        platform: Platform,
    },

    /// Generate UniFFI bindings
    Bindings {
        #[arg(long, value_enum, default_value_t = Platform::Android)]
        platform: Platform,
    },
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

fn main() {
    let args = Args::parse();

    match args.command {
        Commands::Doctor => {
            println!("doctor\n");
            rust::check_rust();
            println!();
            android::check_android();
            println!();
            ios::check_ios();
        }
        Commands::Create { name } => {
            println!("create: {}", name);

            //create subdir
            fs::create_dir(format!("./{name}/")).expect("failed to create folder");

            rust::create_rust(&name);
            android::create_android(&name);
            ios::create_ios(&name);
        }
        Commands::Toolchain => {
            android::install_android_toolchain();
            ios::install_ios_toolchain();
        }
        Commands::Devices { platform } => match platform {
            Platform::Android => android::list_android_devices(),
            Platform::Ios => ios::list_ios_devices(),
        },
        Commands::Launch { emulator, platform } => match platform {
            Platform::Android => android::launch_android_emulator(&emulator),
            Platform::Ios => ios::launch_ios_emulator(&emulator),
        },
        Commands::Run { device, platform } => {
            let result = match platform {
                Platform::Android => android::build_android_debug(&device),
                Platform::Ios => ios::build_ios_debug(&device),
            };

            if let Err(e) = result {
                eprintln!("{e}");
            }
        }
        Commands::Release { platform } => {
            let result = match platform {
                Platform::Android => android::build_android_release(),
                Platform::Ios => ios::build_ios_release(),
            };

            if let Err(e) = result {
                eprintln!("{e}");
            }
        }
        Commands::Bindings { platform } => {
            let result = match platform {
                Platform::Android => android::generate_android_bindings(),
                Platform::Ios => ios::generate_ios_bindings(),
            };

            if let Err(e) = result {
                eprintln!("{e}");
            }
        }
    }
}
