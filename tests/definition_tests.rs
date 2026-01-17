use bazel_lsp::bazel::{parse_label, resolve_label_path, BazelLabel};
use std::fs;
use std::path::PathBuf;
use tempfile::tempdir;

#[test]
fn test_parse_label() {
    assert_eq!(
        parse_label("//foo:bar"),
        Some(BazelLabel {
            package: Some("foo".to_string()),
            target: "bar".to_string(),
            is_relative: false,
        })
    );

    assert_eq!(
        parse_label("//foo/bar"),
        Some(BazelLabel {
            package: Some("foo/bar".to_string()),
            target: "bar".to_string(),
            is_relative: false,
        })
    );

    assert_eq!(
        parse_label(":bar"),
        Some(BazelLabel {
            package: None,
            target: "bar".to_string(),
            is_relative: true,
        })
    );

    assert_eq!(
        parse_label("bar"),
        Some(BazelLabel {
            package: None,
            target: "bar".to_string(),
            is_relative: true,
        })
    );
    
    assert_eq!(
        parse_label("@repo//foo:bar"),
        Some(BazelLabel {
            package: Some("foo".to_string()),
            target: "bar".to_string(),
            is_relative: false,
        })
    );
}

#[test]
fn test_resolve_label_path() {
    let dir = tempdir().unwrap();
    let root = dir.path();

    // Create a mock workspace
    // root/WORKSPACE
    // root/pkg/BUILD
    // root/pkg/src/lib.rs
    
    fs::File::create(root.join("WORKSPACE")).unwrap();
    fs::create_dir(root.join("pkg")).unwrap();
    fs::File::create(root.join("pkg").join("BUILD")).unwrap();
    
    let label = BazelLabel {
        package: Some("pkg".to_string()),
        target: "my_target".to_string(),
        is_relative: false,
    };
    
    let result = resolve_label_path(&label, &root.join("other_file.rs"), root);
    assert!(result.is_ok());
    let path_opt = result.unwrap();
    assert!(path_opt.is_some());
    let (path, target) = path_opt.unwrap();
    
    assert_eq!(path, root.join("pkg").join("BUILD"));
    assert_eq!(target, "my_target");
}

#[test]
fn test_resolve_relative_label() {
    let dir = tempdir().unwrap();
    let root = dir.path();

    // root/WORKSPACE
    // root/pkg/BUILD
    // root/pkg/lib.rs
    
    fs::File::create(root.join("WORKSPACE")).unwrap();
    fs::create_dir(root.join("pkg")).unwrap();
    fs::File::create(root.join("pkg").join("BUILD")).unwrap();
    
    let label = BazelLabel {
        package: None,
        target: "my_target".to_string(),
        is_relative: true,
    };
    
    // Simulating call from root/pkg/lib.rs
    let current_file = root.join("pkg").join("lib.rs");
    
    let result = resolve_label_path(&label, &current_file, root);
    assert!(result.is_ok());
    let path_opt = result.unwrap();
    assert!(path_opt.is_some());
    let (path, target) = path_opt.unwrap();
    
    assert_eq!(path, root.join("pkg").join("BUILD"));
    assert_eq!(target, "my_target");
}
