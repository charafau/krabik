//! Hot reload of a krabik-ui-dsl app.
//!
//! The app's crate is rebuilt and the library is handed to the running app,
//! which continues with the new code and the state it had. The app looks for
//! new libraries in a directory of its own storage; see `hot.rs` of
//! krabik-ui-dsl for that side and for the limits.

use anyhow::{Context, Result, anyhow};

use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::{android, ios, run_command, rust};

/// The cargo feature of krabik-ui-dsl that makes an app take new libraries.
const HOT_RELOAD_FEATURE: &str = "hot-reload";

/// The directory the app watches, below its code cache on Android and below
/// its temporary directory on iOS.
const HOT_DIRECTORY: &str = "krabik-hot";

/// Where a library is put on an Android device before the app's user copies
/// it; `adb` cannot write into the storage of an app.
const ANDROID_STAGING: &str = "/data/local/tmp";

/// Whether the manifest describes an app that can be hot reloaded: it depends
/// on krabik-ui-dsl with the `hot-reload` feature.
pub fn is_hot_reload_project(cargo_toml: &str) -> bool {
    let Some(dependency) = rust::dsl_dependency(cargo_toml) else {
        return false;
    };

    // the list after `features =`, up to its closing bracket
    dependency
        .split_once("features")
        .and_then(|(_, rest)| rest.split_once('['))
        .and_then(|(_, list)| list.split_once(']'))
        .is_some_and(|(features, _)| {
            features
                .split(',')
                .any(|feature| feature.trim().trim_matches('"') == HOT_RELOAD_FEATURE)
        })
}

/// A running app that takes new libraries.
pub enum HotTarget {
    /// An app on an Android emulator.
    Android {
        /// The `adb` serial of the device.
        serial: String,
        /// The Rust target of the device.
        triple: &'static str,
        /// The NDK linkers, as cargo environment variables.
        linkers: HashMap<&'static str, String>,
        /// The id the app is installed under.
        application_id: String,
    },
    /// An app on an iOS simulator.
    Ios {
        /// The UDID of the simulator.
        udid: String,
        /// The bundle id of the app.
        bundle_id: String,
    },
}

impl HotTarget {
    /// The app of the project on the given Android device.
    pub fn android(device: &str) -> Result<Self> {
        let serial = android::resolve_device(device)?;
        let triple = android::target_for_abi(&android::device_abi(&serial)?)?.arch;

        Ok(Self::Android {
            triple,
            linkers: android::ndk_linker_env()?,
            application_id: android::get_application_id()?,
            serial,
        })
    }

    /// The app of the project on the given iOS simulator.
    pub fn ios(device: &str) -> Result<Self> {
        Ok(Self::Ios {
            udid: ios::resolve_simulator_for(device)?,
            bundle_id: ios::get_bundle_id()?,
        })
    }

    /// The platform, for messages.
    pub fn platform(&self) -> &'static str {
        match self {
            Self::Android { .. } => "Android",
            Self::Ios { .. } => "iOS",
        }
    }

    /// The Rust target the library is built for.
    fn triple(&self) -> &'static str {
        match self {
            Self::Android { triple, .. } => triple,
            Self::Ios { .. } => ios::IOS_SIM_TARGET,
        }
    }

    /// The extension of a shared library on the platform.
    fn extension(&self) -> &'static str {
        match self {
            Self::Android { .. } => "so",
            Self::Ios { .. } => "dylib",
        }
    }

    /// Builds the app's crate for the target. Only the app's crate is
    /// compiled again, the framework is a dependency cargo has cached.
    pub fn build(&self) -> Result<()> {
        let mut cmd = Command::new("cargo");
        cmd.args([
            "build",
            "--manifest-path",
            "./rust/Cargo.toml",
            "--target",
            self.triple(),
        ]);
        if let Self::Android { linkers, .. } = self {
            cmd.envs(linkers);
        }
        run_command(&mut cmd, "cargo build")
    }

    /// Hands the library that [`build`](Self::build) produced to the running
    /// app.
    ///
    /// The library gets a new name every time, because the app loads every
    /// name only once. It is written under another name first and renamed,
    /// so that the app never sees half a file.
    pub fn deliver(&self) -> Result<()> {
        let lib_name = rust::get_lib_name()?;
        let extension = self.extension();
        let built = format!(
            "./rust/target/{}/debug/lib{lib_name}.{extension}",
            self.triple()
        );
        if !Path::new(&built).is_file() {
            return Err(anyhow!(
                "{built} was not built; the crate has to be a cdylib"
            ));
        }

        let millis = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|elapsed| elapsed.as_millis())
            .unwrap_or_default();
        let name = delivery_name(millis, std::process::id(), extension);

        match self {
            Self::Android {
                serial,
                application_id,
                ..
            } => deliver_android(serial, application_id, &built, &name),
            Self::Ios { udid, bundle_id } => deliver_ios(udid, bundle_id, &built, &name),
        }
    }
}

/// The name a library is delivered under.
///
/// The app takes the name that sorts last, so the time comes first and has a
/// fixed width.
fn delivery_name(millis: u128, pid: u32, extension: &str) -> String {
    format!("app-{millis:013}-{pid}.{extension}")
}

/// The shell command that copies a delivered library into the app's storage.
/// It is run as the app's user, in the app's data directory.
fn install_command(application_id: &str, name: &str) -> String {
    format!(
        "run-as {application_id} sh -c 'mkdir -p code_cache/{HOT_DIRECTORY} \
         && cp {ANDROID_STAGING}/{name} code_cache/{HOT_DIRECTORY}/{name}.part \
         && mv code_cache/{HOT_DIRECTORY}/{name}.part code_cache/{HOT_DIRECTORY}/{name}'"
    )
}

/// Runs a command without showing its output; the output is part of the
/// error when the command fails.
fn run_quietly(cmd: &mut Command, what: &str) -> Result<String> {
    let output = cmd
        .output()
        .with_context(|| format!("failed to run {what}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let details = [stderr.trim(), stdout.trim()]
            .into_iter()
            .find(|text| !text.is_empty())
            .unwrap_or("no output");
        return Err(anyhow!("{what} failed ({}): {details}", output.status));
    }

    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

/// Puts the library into the code cache of the app on the device.
fn deliver_android(serial: &str, application_id: &str, built: &str, name: &str) -> Result<()> {
    let staged = format!("{ANDROID_STAGING}/{name}");

    let mut push = Command::new("adb");
    push.args(["-s", serial, "push", built, &staged]);
    run_quietly(&mut push, "adb push")?;

    let mut install = Command::new("adb");
    install.args([
        "-s",
        serial,
        "shell",
        &install_command(application_id, name),
    ]);
    let installed = run_quietly(&mut install, "adb shell run-as").with_context(|| {
        format!(
            "could not reach the storage of {application_id}; \
             install and start a debug build first with `krabik run`"
        )
    });

    // the staged copy is removed whether the app took it or not
    let mut remove = Command::new("adb");
    remove.args(["-s", serial, "shell", "rm", "-f", &staged]);
    let _ = run_quietly(&mut remove, "adb shell rm");

    installed.map(|_| ())
}

/// Puts the library into the temporary directory of the app on the simulator.
fn deliver_ios(udid: &str, bundle_id: &str, built: &str, name: &str) -> Result<()> {
    let mut cmd = Command::new("xcrun");
    cmd.args(["simctl", "get_app_container", udid, bundle_id, "data"]);
    let container = run_quietly(&mut cmd, "xcrun simctl get_app_container").with_context(|| {
        format!(
            "could not find {bundle_id} on the simulator; \
             install and start it first with `krabik run --platform ios`"
        )
    })?;

    let directory = Path::new(container.trim()).join("tmp").join(HOT_DIRECTORY);
    fs::create_dir_all(&directory)
        .with_context(|| format!("failed to create {}", directory.display()))?;

    let part = directory.join(format!("{name}.part"));
    fs::copy(built, &part)
        .with_context(|| format!("failed to copy {built} to {}", part.display()))?;
    fs::rename(&part, directory.join(name))
        .with_context(|| format!("failed to rename {}", part.display()))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_hot_reload_project_inline() {
        let toml = "[dependencies]\n\
                    krabik-ui-dsl = { path = \"../../rust\", features = [\"hot-reload\"] }\n";
        assert!(is_hot_reload_project(toml));

        let toml = "[dependencies]\n\
                    krabik-ui-dsl = { path = \"../../rust\", features = [\"svg\", \"hot-reload\"] }\n";
        assert!(is_hot_reload_project(toml));
    }

    #[test]
    fn test_is_hot_reload_project_table() {
        let toml = "[dependencies.krabik-ui-dsl]\npath = \"../../rust\"\n\
                    features = [\"hot-reload\"]\n";
        assert!(is_hot_reload_project(toml));
    }

    #[test]
    fn test_is_hot_reload_project_without_the_feature() {
        let toml = "[dependencies]\nkrabik-ui-dsl = { path = \"../../rust\" }\n";
        assert!(!is_hot_reload_project(toml));

        let toml = "[dependencies]\n\
                    krabik-ui-dsl = { path = \"../../rust\", features = [\"svg\"] }\n";
        assert!(!is_hot_reload_project(toml));

        // a path that happens to contain the name of the feature
        let toml = "[dependencies]\n\
                    krabik-ui-dsl = { path = \"../hot-reload/rust\", features = [] }\n";
        assert!(!is_hot_reload_project(toml));
    }

    #[test]
    fn test_is_hot_reload_project_ignores_other_crates() {
        let toml = "[dependencies]\nother = { version = \"1\", features = [\"hot-reload\"] }\n";
        assert!(!is_hot_reload_project(toml));

        // a UniFFI project
        let toml = "[dependencies]\nuniffi = { version = \"1\" }\n";
        assert!(!is_hot_reload_project(toml));
    }

    #[test]
    fn test_delivery_name_sorts_by_time() {
        let earlier = delivery_name(999, 42, "so");
        let later = delivery_name(1_795_000_000_000, 7, "so");
        assert_eq!(earlier, "app-0000000000999-42.so");
        assert_eq!(later, "app-1795000000000-7.so");
        assert!(earlier < later);

        // the app only takes names with the extension of a library
        assert!(delivery_name(1, 1, "dylib").ends_with(".dylib"));
    }

    #[test]
    fn test_install_command() {
        assert_eq!(
            install_command("com.example.demo", "app-1-2.so"),
            "run-as com.example.demo sh -c 'mkdir -p code_cache/krabik-hot \
             && cp /data/local/tmp/app-1-2.so code_cache/krabik-hot/app-1-2.so.part \
             && mv code_cache/krabik-hot/app-1-2.so.part code_cache/krabik-hot/app-1-2.so'"
        );
    }
}
