pub mod android;
pub mod ios;
pub mod naming;
pub mod rust;

use anyhow::{Context, Result, anyhow};
use clap::{Parser, Subcommand, ValueEnum};

use crate::naming::ProjectName;

use std::fs;

use std::io::IsTerminal;
use std::process::Command;

/// Checks whether an executable command is available on the system PATH by invoking it with `--version`.
pub(crate) fn command_exists(command: &str) -> bool {
    Command::new(command).arg("--version").output().is_ok()
}

/// Determines if terminal color output is enabled (respects `NO_COLOR` and requires a terminal stdout).
pub(crate) fn colors_enabled() -> bool {
    std::env::var_os("NO_COLOR").is_none() && std::io::stdout().is_terminal()
}

/// Formats text with ANSI escape color codes if terminal colors are enabled.
pub(crate) fn paint(fg: &str, text: &str) -> String {
    paint_with(colors_enabled(), fg, text)
}

/// Formats text with ANSI escape color codes if `colors` is true, otherwise returns the plain text.
fn paint_with(colors: bool, fg: &str, text: &str) -> String {
    if colors {
        format!("\x1b[{fg}m{text}\x1b[0m")
    } else {
        text.to_string()
    }
}

/// Returns a colored status checkmark: green `[✓]` for success or red `[✗]` for failure.
pub(crate) fn check_mark(ok: bool) -> String {
    paint(if ok { "32" } else { "31" }, if ok { "[✓]" } else { "[✗]" })
}

/// Executes a command synchronously, returning an error if spawning fails or the command exits with non-zero status.
pub(crate) fn run_command(cmd: &mut Command, what: &str) -> Result<()> {
    let status = cmd
        .status()
        .with_context(|| format!("failed to run {what}"))?;
    if !status.success() {
        return Err(anyhow!("{what} failed ({status})"));
    }
    Ok(())
}

#[derive(Parser)]
#[command(version, about, long_about=None)]
pub struct Args {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Clone, Copy, ValueEnum)]
pub enum Platform {
    Android,
    Ios,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Check the development environment
    Doctor,

    /// Create a new project
    Create {
        /// Project name: lowercase letters, digits, '-' and '_' (e.g. `my-app`)
        name: String,

        /// Package prefix for the generated app id (com.example.my_app by default)
        #[arg(long, default_value = "com.example")]
        package: String,
    },

    /// Install necessary rust toolchains
    Toolchain,

    /// List available devices (use -a / --all to include offline devices too)
    Devices {
        #[arg(short, long, visible_alias = "offline")]
        all: bool,
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
}

/// Main CLI dispatcher that executes the requested subcommand.
pub fn run(args: Args) -> Result<()> {
    match args.command {
        Commands::Doctor => {
            println!("doctor\n");
            let mut ok = rust::check_rust();
            println!();
            ok &= android::check_android();
            println!();
            ok &= ios::check_ios();
            if !ok {
                return Err(anyhow!("one or more doctor checks failed"));
            }
        }
        Commands::Create { name, package } => {
            println!("create: {}", name);

            naming::validate_package_prefix(&package)?;

            // Validate the name before touching the filesystem so that an
            // invalid name never leaves a half-created project behind.
            let project = ProjectName::parse(&name)?;

            let dir = format!("./{}", project.dir_name());
            if fs::metadata(&dir).is_ok() {
                return Err(anyhow!("{dir} already exists"));
            }
            fs::create_dir(&dir).with_context(|| format!("failed to create {dir}"))?;

            rust::create_rust(&project)?;
            android::create_android(&project, &package)?;
            ios::create_ios(&project, &package)?;
        }
        Commands::Toolchain => {
            android::install_android_toolchain()?;
            ios::install_ios_toolchain()?;
        }
        Commands::Devices { all } => {
            android::list_android_devices(all);
            println!();
            ios::list_ios_devices(all);
        }
        Commands::Launch { emulator, platform } => match platform {
            Platform::Android => android::launch_android_emulator(&emulator)?,
            Platform::Ios => ios::launch_ios_emulator(&emulator)?,
        },
        Commands::Run { device, platform } => match platform {
            Platform::Android => android::build_android_debug(&device)?,
            Platform::Ios => ios::build_ios_debug(&device)?,
        },
        Commands::Release { platform } => match platform {
            Platform::Android => android::build_android_release()?,
            Platform::Ios => ios::build_ios_release()?,
        },
        Commands::Bindings { platform } => match platform {
            Platform::Android => android::generate_android_bindings()?,
            Platform::Ios => ios::generate_ios_bindings()?,
        },
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_paint_with_colors() {
        assert_eq!(paint_with(true, "32", "[✓]"), "\x1b[32m[✓]\x1b[0m");
    }

    #[test]
    fn test_paint_without_colors() {
        assert_eq!(paint_with(false, "32", "[✓]"), "[✓]");
    }
}
