use anyhow::Result;
use std::fs;
use std::path::{Path, PathBuf};

/// Checks if a directory is a Bazel workspace
///
/// A directory is considered a Bazel workspace if it contains a WORKSPACE or WORKSPACE.bazel file
/// at the root level.
pub fn is_workspace_dir(path: &Path) -> Result<bool> {
    if !path.is_dir() {
        return Ok(false);
    }

    // Check for WORKSPACE or WORKSPACE.bazel file
    let workspace_file = path.join("WORKSPACE");
    let workspace_bazel_file = path.join("WORKSPACE.bazel");

    Ok(workspace_file.exists() || workspace_bazel_file.exists())
}

/// Finds the root of a Bazel workspace from a given path
///
/// This function traverses up the directory tree from the given path
/// until it finds a directory containing a WORKSPACE or WORKSPACE.bazel file.
/// Returns None if no workspace root is found.
pub fn find_workspace_root(path: &Path) -> Result<Option<&Path>> {
    let mut current = Some(path);

    while let Some(dir) = current {
        if is_workspace_dir(dir)? {
            return Ok(Some(dir));
        }

        current = dir.parent();
    }

    Ok(None)
}

/// Gets the package path relative to the workspace root
///
/// Returns the package path as a string if the given path is within a Bazel workspace,
/// otherwise returns None.
pub fn get_package_path(path: &Path) -> Result<Option<String>> {
    if let Some(workspace_root) = find_workspace_root(path)? {
        if let Ok(relative_path) = path.strip_prefix(workspace_root) {
            return Ok(Some(relative_path.to_string_lossy().to_string()));
        }
    }

    Ok(None)
}

/// Finds all BUILD files in a directory recursively
///
/// This function searches for files named "BUILD" or "BUILD.bazel" in the given directory
/// and all its subdirectories, excluding hidden directories and bazel-out.
pub fn find_build_files(dir: &Path) -> Vec<PathBuf> {
    let mut build_files = Vec::new();

    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                if !path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .map(|name| name.starts_with('.') || name == "bazel-out")
                    .unwrap_or(false)
                {
                    build_files.extend(find_build_files(&path));
                }
            } else if path
                .file_name()
                .and_then(|name| name.to_str())
                .map(|name| name == "BUILD" || name == "BUILD.bazel")
                .unwrap_or(false)
            {
                build_files.push(path);
            }
        }
    }

    build_files
}
