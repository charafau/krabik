//! Watches the Rust sources of a project and brings the app up to date on
//! every change.
//!
//! A krabik-ui-dsl app with the `hot-reload` feature is hot reloaded: the
//! crate is rebuilt and the library is handed to the running app, which keeps
//! its state. That takes a few seconds.
//!
//! Every other project is rebuilt. The build itself is incremental, so a
//! cycle only has to recompile the changed crate, reinstall the app and
//! launch it again. That makes the edit-compile-run loop a lot shorter than
//! running the whole build by hand.

use anyhow::{Context, Result, anyhow};
use notify::event::{AccessKind, AccessMode};
use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::fs;
use std::path::PathBuf;
use std::sync::mpsc::{Receiver, RecvTimeoutError, channel};
use std::time::{Duration, Instant};

use crate::Platforms;
use crate::hot::{self, HotTarget};
use crate::{android, ios};

/// How long the file system has to be quiet before the project is rebuilt.
///
/// Editors write a file in several steps, and every step produces an event.
const DEBOUNCE: Duration = Duration::from_millis(200);

/// A path that is watched for changes.
struct WatchPath {
    path: PathBuf,
    recursive: bool,
}

/// Watches the Rust sources of the project and brings the app up to date on
/// every change: a krabik-ui-dsl app with the `hot-reload` feature is hot
/// reloaded, every other project is rebuilt.
///
/// The project is built once up front, so that the app is up to date when
/// the watch starts. Afterwards every change triggers another build. A build
/// that fails is reported, but does not stop the watch: the next save is
/// picked up as usual.
pub fn watch(device: &str, platforms: Platforms) -> Result<()> {
    let paths = watch_paths();
    if paths.is_empty() {
        return Err(anyhow!(
            "no Rust sources found, run this from the root of a project"
        ));
    }

    let hot_reload = fs::read_to_string("rust/Cargo.toml")
        .is_ok_and(|cargo_toml| hot::is_hot_reload_project(&cargo_toml));
    if hot_reload {
        return watch_hot(device, platforms, &paths);
    }

    // The device is resolved once, so that the watch does not ask for it
    // again on every build.
    let serial = if platforms.includes_android() {
        Some(android::resolve_device(device)?)
    } else {
        None
    };

    // The watcher has to be kept alive for as long as the events are read.
    let (_watcher, events) = start_watching(&paths)?;

    println!("Watching {} for changes...", describe(&paths));
    for path in &paths {
        println!("  {}", path.path.display());
    }

    // The first build, so that the app is up to date right away. A failure is
    // reported, but the watch continues: the project may simply not compile
    // yet.
    rebuild(device, platforms, serial.as_deref());

    loop {
        wait_for_change(&events)?;
        println!("\nChange detected, rebuilding...");
        rebuild(device, platforms, serial.as_deref());
    }
}

/// Hot reloads the running app on every change.
///
/// The app has to be running already; it is neither installed nor started
/// here. Its state survives a reload as long as the layout of its state and
/// message types stays the same.
fn watch_hot(device: &str, platforms: Platforms, paths: &[WatchPath]) -> Result<()> {
    // The devices are resolved once, so that the watch does not ask for them
    // again on every reload.
    let mut targets = Vec::new();
    if platforms.includes_android() {
        targets.push(HotTarget::android(device)?);
    }
    if platforms.includes_ios() {
        targets.push(HotTarget::ios(device)?);
    }

    // The watcher has to be kept alive for as long as the events are read.
    let (_watcher, events) = start_watching(paths)?;

    println!("Hot reloading on changes in {}...", describe(paths));
    println!("The app has to be running; start it with `krabik run`.");
    println!(
        "A library the app refuses, because the layout of its state or message type \
         changed, is only reported in the app's log (Android: `adb logcat -s krabik`); \
         restart the app with `krabik run` then."
    );

    // The first reload brings the app up to date with the sources. A failure
    // is reported, but the watch continues.
    hot_reload(&targets);

    loop {
        wait_for_change(&events)?;
        println!("\nChange detected, reloading...");
        hot_reload(&targets);
    }
}

/// Builds the app's library for every target and hands it to the app.
fn hot_reload(targets: &[HotTarget]) {
    for target in targets {
        let platform = target.platform();
        let started = Instant::now();

        if let Err(err) = target.build() {
            eprintln!(
                "{platform}: the build failed ({err:#}); the app keeps running the code it has"
            );
            continue;
        }

        match target.deliver() {
            Ok(()) => println!(
                "{platform}: reloaded in {:.1}s",
                started.elapsed().as_secs_f64()
            ),
            Err(err) => eprintln!("{platform}: the library was not delivered: {err:#}"),
        }
    }
}

/// Builds the project for the watched platforms.
fn rebuild(device: &str, platforms: Platforms, serial: Option<&str>) {
    if platforms.includes_android() {
        let device = serial.unwrap_or(device);
        if let Err(err) = android::build_android_debug(device, None) {
            eprintln!("Android build failed: {err:#}");
        }
    }

    if platforms.includes_ios()
        && let Err(err) = ios::build_ios_debug(device)
    {
        eprintln!("iOS build failed: {err:#}");
    }
}

/// The paths that are watched for changes: the sources and the manifests of
/// the crates of the project.
///
/// The target directory is deliberately not watched. The build writes to it
/// and every write would trigger another build.
fn watch_paths() -> Vec<WatchPath> {
    let mut paths = Vec::new();

    for dir in [
        "rust/src",
        "rust/uniffi-bindgen/src",
        "rust/uniffi-bindgen-swift/src",
    ] {
        let path = PathBuf::from(dir);
        if path.is_dir() {
            paths.push(WatchPath {
                path,
                recursive: true,
            });
        }
    }

    for file in [
        "rust/Cargo.toml",
        "rust/build.rs",
        "rust/uniffi-bindgen/Cargo.toml",
        "rust/uniffi-bindgen-swift/Cargo.toml",
    ] {
        let path = PathBuf::from(file);
        if path.exists() {
            paths.push(WatchPath {
                path,
                recursive: false,
            });
        }
    }

    paths
}

/// Starts the file watcher and returns it together with the channel the
/// events arrive on.
///
/// The watcher has to be kept alive for as long as the events are read.
fn start_watching(
    paths: &[WatchPath],
) -> Result<(RecommendedWatcher, Receiver<notify::Result<Event>>)> {
    let (tx, rx) = channel();

    let mut watcher = RecommendedWatcher::new(
        tx,
        // Comparing the contents catches the changes that the file system
        // events do not describe, e.g. on macOS.
        Config::default().with_compare_contents(true),
    )
    .context("failed to start the file watcher")?;

    for path in paths {
        let mode = if path.recursive {
            RecursiveMode::Recursive
        } else {
            RecursiveMode::NonRecursive
        };

        watcher
            .watch(&path.path, mode)
            .with_context(|| format!("failed to watch {}", path.path.display()))?;
    }

    Ok((watcher, rx))
}

/// Waits for a change in the watched paths.
///
/// The events of a single save are collected until the file system settles
/// down, so that a file that is written in several steps only triggers one
/// build.
fn wait_for_change(events: &Receiver<notify::Result<Event>>) -> Result<()> {
    loop {
        let event = events
            .recv()
            .map_err(|_| anyhow!("the file watcher stopped"))??;
        if !is_change(&event) {
            continue;
        }

        loop {
            match events.recv_timeout(DEBOUNCE) {
                Ok(Ok(_)) => {
                    // The file is still being written, wait for it to settle.
                }
                Ok(Err(err)) => return Err(err.into()),
                Err(RecvTimeoutError::Timeout) => return Ok(()),
                Err(RecvTimeoutError::Disconnected) => {
                    return Err(anyhow!("the file watcher stopped"));
                }
            }
        }
    }
}

/// Whether the event describes a change to a file.
fn is_change(event: &Event) -> bool {
    match event.kind {
        // A file that is closed after being written changed.
        EventKind::Access(AccessKind::Close(AccessMode::Write)) => true,
        // Reading a file is not a change.
        EventKind::Access(_) => false,
        _ => true,
    }
}

/// A short description of the watched paths.
fn describe(paths: &[WatchPath]) -> String {
    let mut names: Vec<String> = paths
        .iter()
        .map(|path| path.path.to_string_lossy().into_owned())
        .collect();
    names.sort();
    names.join(", ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_watch_paths_are_existing() {
        // the paths are relative to the project root, so this only checks that
        // the ones that exist are found
        for path in watch_paths() {
            assert!(path.path.exists(), "{}", path.path.display());
        }
    }

    #[test]
    fn test_watch_paths_exclude_the_target_directory() {
        for path in watch_paths() {
            assert!(
                !path.path.components().any(|c| c.as_os_str() == "target"),
                "{}",
                path.path.display()
            );
        }
    }

    #[test]
    fn test_is_change_ignores_reads() {
        assert!(!is_change(&Event::new(EventKind::Access(AccessKind::Read))));
        assert!(!is_change(&Event::new(EventKind::Access(
            AccessKind::Open(AccessMode::Read)
        ))));
    }

    #[test]
    fn test_is_change_detects_writes() {
        assert!(is_change(&Event::new(EventKind::Access(
            AccessKind::Close(AccessMode::Write)
        ))));
        assert!(is_change(&Event::new(EventKind::Create(
            notify::event::CreateKind::File
        ))));
        assert!(is_change(&Event::new(EventKind::Remove(
            notify::event::RemoveKind::File
        ))));
        assert!(is_change(&Event::new(EventKind::Modify(
            notify::event::ModifyKind::Data(notify::event::DataChange::Content)
        ))));
    }

    #[test]
    fn test_describe() {
        let paths = vec![
            WatchPath {
                path: PathBuf::from("rust/src"),
                recursive: true,
            },
            WatchPath {
                path: PathBuf::from("rust/Cargo.toml"),
                recursive: false,
            },
        ];
        assert_eq!(describe(&paths), "rust/Cargo.toml, rust/src");
    }
}
