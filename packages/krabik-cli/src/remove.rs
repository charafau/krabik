//! Helpers for removing files and directories from a generated project.

use anyhow::{Context, Result};

use std::fs;
use std::path::Path;

/// Removes a file or a directory if it exists, without following symlinks.
pub(crate) fn remove_if_exists(path: impl AsRef<Path>) -> Result<()> {
    let path = path.as_ref();
    let Ok(metadata) = fs::symlink_metadata(path) else {
        return Ok(());
    };

    if metadata.is_dir() {
        fs::remove_dir_all(path).with_context(|| format!("failed to remove {}", path.display()))?;
    } else {
        fs::remove_file(path).with_context(|| format!("failed to remove {}", path.display()))?;
    }

    println!("  removed {}", path.display());
    Ok(())
}

/// Removes every file with the given extension below `root`, recursively.
pub(crate) fn remove_files_with_extension(root: &Path, extension: &str) -> Result<()> {
    remove_matching(root, |path| {
        path.is_file() && path.extension().and_then(|ext| ext.to_str()) == Some(extension)
    })
}

// /// Removes every direct child of `root` whose extension matches.
// pub(crate) fn remove_entries_with_extension(root: &Path, extension: &str) -> Result<()> {
//     remove_matching(root, |path| {
//         path.parent() == Some(root)
//             && path.extension().and_then(|ext| ext.to_str()) == Some(extension)
//     })
// }

/// Removes everything below (and including) `root` that matches `predicate`.
fn remove_matching(root: &Path, predicate: impl Fn(&Path) -> bool) -> Result<()> {
    let Ok(metadata) = fs::symlink_metadata(root) else {
        return Ok(());
    };
    if !metadata.is_dir() {
        return Ok(());
    }

    let mut matches = Vec::new();
    collect_matching(root, &predicate, &mut matches)?;
    for path in matches {
        remove_if_exists(path)?;
    }

    Ok(())
}

/// Collects paths matching `predicate` below `directory`.
fn collect_matching(
    directory: &Path,
    predicate: &impl Fn(&Path) -> bool,
    matches: &mut Vec<std::path::PathBuf>,
) -> Result<()> {
    for entry in fs::read_dir(directory)
        .with_context(|| format!("failed to read {}", directory.display()))?
    {
        let path = entry?.path();
        let metadata = fs::symlink_metadata(&path)
            .with_context(|| format!("failed to inspect {}", path.display()))?;

        if predicate(&path) {
            matches.push(path);
        } else if metadata.is_dir() {
            collect_matching(&path, predicate, matches)?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    /// A temporary directory that is removed when the test ends.
    struct TempDir(PathBuf);

    impl TempDir {
        fn new(name: &str) -> Self {
            let path = std::env::temp_dir().join(format!(
                "krabik_remove_{name}_{}",
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ));
            fs::create_dir_all(&path).unwrap();
            Self(path)
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn test_remove_files_with_extension() {
        let tmp = TempDir::new("files");
        let nested = tmp.0.join("arm64-v8a");
        fs::create_dir_all(&nested).unwrap();
        fs::write(nested.join("libapp.so"), "").unwrap();
        fs::write(tmp.0.join("keep.txt"), "").unwrap();

        remove_files_with_extension(&tmp.0, "so").unwrap();

        assert!(!nested.join("libapp.so").exists());
        assert!(tmp.0.join("keep.txt").exists());
        assert!(nested.is_dir());
    }

    // #[test]
    // fn test_remove_entries_with_extension_only_touches_direct_children() {
    //     let tmp = TempDir::new("entries");
    //     let nested = tmp.0.join("nested");
    //     fs::create_dir_all(nested.join("Inner.xcodeproj")).unwrap();
    //     fs::create_dir_all(tmp.0.join("App.xcodeproj")).unwrap();
    //     fs::write(tmp.0.join("project.yml"), "").unwrap();

    //     remove_entries_with_extension(&tmp.0, "xcodeproj").unwrap();

    //     assert!(!tmp.0.join("App.xcodeproj").exists());
    //     assert!(nested.join("Inner.xcodeproj").exists());
    //     assert!(tmp.0.join("project.yml").exists());
    // }

    #[test]
    fn test_remove_ignores_missing_paths() {
        let tmp = TempDir::new("missing");
        remove_if_exists(tmp.0.join("does-not-exist")).unwrap();
    }
}
