use bazel_lsp::bazel::find_build_files;
use bazel_lsp::bazel::{find_workspace_root, get_package_path, is_workspace_dir};
use std::fs;
use tempfile::TempDir;

#[test]
fn test_is_workspace_dir() {
    let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path();

    assert!(!is_workspace_dir(temp_path).unwrap());

    fs::write(temp_path.join("WORKSPACE"), "").unwrap();
    assert!(is_workspace_dir(temp_path).unwrap());

    fs::remove_file(temp_path.join("WORKSPACE")).unwrap();
    fs::write(temp_path.join("WORKSPACE.bazel"), "").unwrap();
    assert!(is_workspace_dir(temp_path).unwrap());
}

#[test]
fn test_find_workspace_root() {
    let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path();

    let nested_dir = temp_path.join("a").join("b").join("c");
    fs::create_dir_all(&nested_dir).unwrap();

    assert!(find_workspace_root(&nested_dir).unwrap().is_none());

    fs::write(temp_path.join("WORKSPACE"), "").unwrap();

    let found_root = find_workspace_root(&nested_dir).unwrap().unwrap();
    assert_eq!(found_root, temp_path);
}

#[test]
fn test_get_package_path() {
    let temp_dir = TempDir::new().unwrap();
    let temp_path = temp_dir.path();

    fs::write(temp_path.join("WORKSPACE"), "").unwrap();

    let package_dir = temp_path.join("src").join("main");
    fs::create_dir_all(&package_dir).unwrap();

    let package_path = get_package_path(&package_dir).unwrap().unwrap();
    assert_eq!(package_path, "src/main");
}

#[test]
fn test_find_build_files_empty_dir() {
    let temp_dir = TempDir::new().unwrap();
    let build_files = find_build_files(temp_dir.path());
    assert!(build_files.is_empty());
}

#[test]
fn test_find_build_files_single_build() {
    let temp_dir = TempDir::new().unwrap();
    fs::write(temp_dir.path().join("BUILD"), "").unwrap();

    let build_files = find_build_files(temp_dir.path());
    assert_eq!(build_files.len(), 1);
    assert_eq!(build_files[0].file_name().unwrap(), "BUILD");
}

#[test]
fn test_find_build_files_build_bazel() {
    let temp_dir = TempDir::new().unwrap();
    fs::write(temp_dir.path().join("BUILD.bazel"), "").unwrap();

    let build_files = find_build_files(temp_dir.path());
    assert_eq!(build_files.len(), 1);
    assert_eq!(build_files[0].file_name().unwrap(), "BUILD.bazel");
}

#[test]
fn test_find_build_files_nested() {
    let temp_dir = TempDir::new().unwrap();
    let subdir = temp_dir.path().join("subdir");
    fs::create_dir(&subdir).unwrap();

    fs::write(temp_dir.path().join("BUILD"), "").unwrap();
    fs::write(subdir.join("BUILD"), "").unwrap();

    let build_files = find_build_files(temp_dir.path());
    assert_eq!(build_files.len(), 2);
}

#[test]
fn test_find_build_files_ignore_hidden() {
    let temp_dir = TempDir::new().unwrap();
    let hidden_dir = temp_dir.path().join(".git");
    fs::create_dir(&hidden_dir).unwrap();

    fs::write(temp_dir.path().join("BUILD"), "").unwrap();
    fs::write(hidden_dir.join("BUILD"), "").unwrap();

    let build_files = find_build_files(temp_dir.path());
    assert_eq!(build_files.len(), 1);
    assert_eq!(build_files[0].file_name().unwrap(), "BUILD");
}

#[test]
fn test_find_build_files_ignore_bazel_out() {
    let temp_dir = TempDir::new().unwrap();
    let bazel_out = temp_dir.path().join("bazel-out");
    fs::create_dir(&bazel_out).unwrap();

    fs::write(temp_dir.path().join("BUILD"), "").unwrap();
    fs::write(bazel_out.join("BUILD"), "").unwrap();

    let build_files = find_build_files(temp_dir.path());
    assert_eq!(build_files.len(), 1);
    assert_eq!(build_files[0].file_name().unwrap(), "BUILD");
}

#[test]
fn test_find_build_files_complex_structure() {
    let temp_dir = TempDir::new().unwrap();

    let dirs = [
        "src",
        "src/foo",
        "src/foo/bar",
        ".git",
        "bazel-out",
        "bazel-out/foo",
    ];

    for dir in dirs.iter() {
        fs::create_dir_all(temp_dir.path().join(dir)).unwrap();
    }

    let build_locations = [
        "BUILD",
        "src/BUILD",
        "src/foo/BUILD",
        "src/foo/bar/BUILD.bazel",
        ".git/BUILD",
        "bazel-out/BUILD",
        "bazel-out/foo/BUILD",
    ];

    for location in build_locations.iter() {
        fs::write(temp_dir.path().join(location), "").unwrap();
    }

    let build_files = find_build_files(temp_dir.path());
    assert_eq!(build_files.len(), 4); // Should only find the BUILD files in non-hidden, non-bazel-out directories
}
