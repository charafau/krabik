//! Scaffold and build cross-platform Rust mobile apps with UniFFI.
//!
//! The CLI lives in [`run`], which dispatches the parsed [`Args`] to the
//! platform modules. The logic is in the library so that it can be tested
//! without spawning a process.

pub mod android;
pub mod ios;
pub mod naming;
pub mod rust;
pub mod watch;

use anyhow::{Context, Result, anyhow};
use clap::{Parser, Subcommand, ValueEnum};

use crate::naming::ProjectName;

use std::ffi::OsStr;
use std::fs;

use std::io::IsTerminal;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Checks whether an executable command is available in the `PATH`.
pub(crate) fn command_exists(command: &str) -> bool {
    let Some(path) = std::env::var_os("PATH") else {
        return false;
    };

    find_in_path(&path, command)
}

/// Searches the given `PATH` for an executable with the given name.
fn find_in_path(path: &OsStr, command: &str) -> bool {
    std::env::split_paths(path)
        .filter(|dir| !dir.as_os_str().is_empty())
        .any(|dir| {
            let candidate = dir.join(command);
            if is_executable(&candidate) {
                return true;
            }

            #[cfg(windows)]
            {
                is_executable(&dir.join(format!("{command}.exe")))
            }
            #[cfg(not(windows))]
            {
                false
            }
        })
}

/// Whether the given path is a file that can be executed.
#[cfg(unix)]
fn is_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;

    fs::metadata(path)
        .map(|meta| meta.is_file() && meta.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}

/// Whether the given path is a file that can be executed.
#[cfg(not(unix))]
fn is_executable(path: &Path) -> bool {
    path.is_file()
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

/// Prints the result of a check and returns whether it passed.
pub(crate) fn report_check(ok: bool, passed: &str, failed: &str) -> bool {
    println!("{} {}", check_mark(ok), if ok { passed } else { failed });
    ok
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

/// The command line arguments of the CLI.
#[derive(Parser)]
#[command(version, about, long_about=None)]
pub struct Args {
    /// The command to run.
    #[command(subcommand)]
    pub command: Commands,
}

/// The mobile platform a command applies to.
#[derive(Clone, Copy, ValueEnum)]
pub enum Platform {
    /// Android, using Gradle and the Android NDK.
    Android,
    /// iOS, using Xcode and XcodeGen.
    Ios,
}

/// The platforms a command applies to.
#[derive(Clone, Copy, ValueEnum, PartialEq, Eq, Debug)]
pub enum Platforms {
    /// Android only.
    Android,
    /// iOS only.
    Ios,
    /// Android and iOS.
    Both,
}

impl Platforms {
    /// Whether Android is included.
    pub fn includes_android(self) -> bool {
        matches!(self, Platforms::Android | Platforms::Both)
    }

    /// Whether iOS is included.
    pub fn includes_ios(self) -> bool {
        matches!(self, Platforms::Ios | Platforms::Both)
    }
}

/// The commands of the CLI.
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
        /// Include devices that are not online.
        #[arg(short, long, visible_alias = "offline")]
        all: bool,
    },

    /// Launch given emulator
    Launch {
        /// The name of the emulator or simulator to boot.
        emulator: String,
        /// The platform the emulator belongs to.
        #[arg(long, value_enum, default_value_t = Platform::Android)]
        platform: Platform,
    },

    /// Run on given device (build and launch)
    Run {
        /// The device to run on. The only online device is used when omitted.
        #[arg(default_value = "")]
        device: String,
        /// The platform of the device.
        #[arg(long, value_enum, default_value_t = Platform::Android)]
        platform: Platform,
        /// Build for this ABI instead of the one of the device (Android only)
        #[arg(long, value_enum)]
        abi: Option<android::Abi>,
    },

    /// Build release build
    Release {
        /// The platform to build for.
        #[arg(long, value_enum, default_value_t = Platform::Android)]
        platform: Platform,
    },

    /// Generate UniFFI bindings
    Bindings {
        /// The platform to generate the bindings for.
        #[arg(long, value_enum, default_value_t = Platform::Android)]
        platform: Platform,
    },

    /// Watch the Rust sources and rebuild on every change
    Watch {
        /// The device to run on (Android) or the simulator to use (iOS)
        #[arg(default_value = "")]
        device: String,
        /// The platform to build for
        #[arg(long, value_enum, default_value = "android")]
        platform: Platforms,
    },
}

/// Creates the Rust, Android and iOS parts of the project in `root`.
fn create_project(project: &ProjectName, package_prefix: &str, root: &Path) -> Result<()> {
    rust::create_rust(project, root)?;
    android::create_android(project, package_prefix, root)?;
    ios::create_ios(project, package_prefix, root)?;

    // The project declares the targets it needs, so it does not depend on the
    // machine being set up beforehand.
    let mut targets: Vec<&str> = android::ANDROID_TARGETS.iter().map(|t| t.arch).collect();
    targets.extend(ios::IOS_TARGETS.iter().copied());
    rust::write_toolchain_file(root, &targets)?;

    Ok(())
}

/// A directory that holds the project while it is being created.
///
/// The project is only moved to its final location once it was created
/// successfully, so a failure never leaves a half-created project behind.
struct StagingDir {
    path: PathBuf,
}

impl StagingDir {
    /// Creates a staging directory next to the final project directory.
    ///
    /// The directory is created in the current working directory so that
    /// moving it to its final location is a rename within the same
    /// filesystem, which is atomic.
    fn create(name: &str) -> Result<Self> {
        let path = PathBuf::from(format!(".mobile_cli-{name}-{}.tmp", std::process::id()));

        // a directory of a previous, interrupted run may still be around
        if path.exists() {
            fs::remove_dir_all(&path).with_context(|| {
                format!("failed to remove the stale directory {}", path.display())
            })?;
        }

        fs::create_dir(&path).with_context(|| format!("failed to create {}", path.display()))?;
        Ok(Self { path })
    }

    /// The path of the staging directory.
    fn path(&self) -> &Path {
        &self.path
    }

    /// Removes the staging directory and everything in it.
    fn cleanup(&self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

/// Main CLI dispatcher that executes the requested subcommand.
pub fn run(args: Args) -> Result<()> {
    match args.command {
        Commands::Doctor => {
            println!("doctor\n");
            let mut ok = rust::check_rust();
            println!();

            // the installed targets are read once and shared by both platforms
            let targets = rust::InstalledTargets::read();

            ok &= android::check_android(&targets);
            println!();
            ok &= ios::check_ios(&targets);
            if !ok {
                return Err(anyhow!("one or more doctor checks failed"));
            }
        }
        Commands::Create { name, package } => {
            println!("create: {name}");

            naming::validate_package_prefix(&package)?;

            // Validate the name before touching the filesystem so that an
            // invalid name never leaves a half-created project behind.
            let project = ProjectName::parse(&name)?;

            let target = PathBuf::from(format!("./{}", project.dir_name()));
            if fs::metadata(&target).is_ok() {
                return Err(anyhow!("{} already exists", target.display()));
            }

            // The project is built in a staging directory and only moved to
            // its final location once every part was created successfully.
            let staging = StagingDir::create(project.dir_name())?;
            if let Err(err) = create_project(&project, &package, staging.path()) {
                staging.cleanup();
                return Err(err);
            }

            fs::rename(staging.path(), &target).with_context(|| {
                format!(
                    "failed to move {} to {}",
                    staging.path().display(),
                    target.display()
                )
            })?;
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
        Commands::Run {
            device,
            platform,
            abi,
        } => match platform {
            Platform::Android => android::build_android_debug(&device, abi)?,
            Platform::Ios => {
                if abi.is_some() {
                    return Err(anyhow!("--abi is only supported for Android"));
                }
                ios::build_ios_debug(&device)?
            }
        },
        Commands::Release { platform } => match platform {
            Platform::Android => android::build_android_release()?,
            Platform::Ios => ios::build_ios_release()?,
        },
        Commands::Bindings { platform } => match platform {
            Platform::Android => android::generate_android_bindings()?,
            Platform::Ios => ios::generate_ios_bindings()?,
        },
        Commands::Watch { device, platform } => watch::watch(&device, platform)?,
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

    #[test]
    fn test_find_in_path() {
        let dir = std::env::temp_dir().join(format!(
            "test_find_in_path_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();

        let executable = dir.join("mytool");
        std::fs::write(&executable, "#!/bin/sh\n").unwrap();

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&executable, std::fs::Permissions::from_mode(0o755)).unwrap();
        }

        assert!(find_in_path(dir.as_os_str(), "mytool"));
        assert!(!find_in_path(dir.as_os_str(), "missing-tool"));

        // a file that cannot be executed is not found
        let not_executable = dir.join("data");
        std::fs::write(&not_executable, "data").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&not_executable, std::fs::Permissions::from_mode(0o644))
                .unwrap();
        }
        assert!(!find_in_path(dir.as_os_str(), "data"));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_find_in_path_empty_path() {
        assert!(!find_in_path(std::ffi::OsStr::new(""), "cargo"));
    }
}
