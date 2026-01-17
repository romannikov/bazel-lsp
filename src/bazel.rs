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

#[derive(Debug, Clone, PartialEq)]
pub struct BazelLabel {
    pub package: Option<String>,
    pub target: String,
    pub is_relative: bool,
}

/// Parses a Bazel label string
///
/// Supports:
/// - //path/to/package:target
/// - //path/to/package (shorthand for //path/to/package:package)
/// - :target
/// - target (same as :target)
pub fn parse_label(label: &str) -> Option<BazelLabel> {
    let label = label.trim();
    
    // Handle external repos (simplified: ignore repo part and treat as global path if possible, or just fail for now if complex)
    // For now, let's strip @repo if present to just handle the path part if it starts with @
    let label = if label.starts_with('@') {
        if let Some(idx) = label.find("//") {
            &label[idx..]
        } else {
            return None;
        }
    } else {
        label
    };

    if label.starts_with("//") {
        let content = &label[2..];
        if let Some(colon_idx) = content.find(':') {
            let package = content[..colon_idx].to_string();
            let target = content[colon_idx + 1..].to_string();
            Some(BazelLabel {
                package: Some(package),
                target,
                is_relative: false,
            })
        } else {
            // //path/to/package -> target is package name
            let package = content.to_string();
            let target = Path::new(&package)
                .file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default();
            
            Some(BazelLabel {
                package: Some(package),
                target,
                is_relative: false,
            })
        }
    } else if label.starts_with(':') {
        Some(BazelLabel {
            package: None,
            target: label[1..].to_string(),
            is_relative: true,
        })
    } else {
        // Assume relative target if it doesn't contain other indicators, but typically labels start with : in BUILD files for local rules
        // However, in deps lists, strict labels used.
        // Let's support "target" as ":target"
        Some(BazelLabel {
            package: None,
            target: label.to_string(),
            is_relative: true,
        })
    }
}

/// Resolves a Bazel label to a file path (BUILD file) and target name
///
/// Returns (PathBuf to BUILD file, target name)
pub fn resolve_label_path(
    label: &BazelLabel,
    current_file: &Path,
    workspace_root: &Path,
) -> Result<Option<(PathBuf, String)>> {
    let package_path = if label.is_relative {
        // Relative to current file's package
        if let Some(parent) = current_file.parent() {
            parent.to_path_buf()
        } else {
            return Ok(None);
        }
    } else {
        // Absolute package path from workspace root
        if let Some(pkg) = &label.package {
            workspace_root.join(pkg)
        } else {
            workspace_root.to_path_buf()
        }
    };

    // Check for BUILD or BUILD.bazel
    let build_file = package_path.join("BUILD");
    let build_bazel_file = package_path.join("BUILD.bazel");

    if build_bazel_file.exists() {
        Ok(Some((build_bazel_file, label.target.clone())))
    } else if build_file.exists() {
        Ok(Some((build_file, label.target.clone())))
    } else {
        Ok(None)
    }
}
