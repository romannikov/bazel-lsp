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
