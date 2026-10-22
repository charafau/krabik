//! Helpers shared by the integration tests.

#![cfg(unix)]

use assert_cmd::Command;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

/// A directory with fake executables that are put on the `PATH` when the CLI
/// under test is run.
pub struct FakeToolchain {
    _dir: TempDir,
    bin: PathBuf,
}

impl FakeToolchain {
    /// Creates a toolchain with the given executables, e.g. `("android", script)`.
    pub fn new(tools: &[(&str, &str)]) -> Self {
        let dir = TempDir::new().unwrap();
        let bin = dir.path().join("bin");
        fs::create_dir_all(&bin).unwrap();

        for (name, script) in tools {
            write_executable(&bin.join(name), script);
        }

        Self { _dir: dir, bin }
    }

    /// The `PATH` to run the CLI with: the fake tools first, the real ones after.
    pub fn path(&self) -> String {
        let current = std::env::var("PATH").unwrap_or_default();
        format!("{}:{current}", self.bin.display())
    }
}

/// The CLI under test, with the fake toolchain on the `PATH`.
pub fn cli(tools: &FakeToolchain) -> Command {
    let mut cmd = Command::cargo_bin("krabik").expect("binary should build");
    cmd.env("PATH", tools.path());
    cmd
}

/// Writes a script and makes it executable.
pub fn write_executable(path: &Path, script: &str) {
    fs::write(path, script).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o755)).unwrap();
}
