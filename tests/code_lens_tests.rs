use anyhow::Result;
use bazel_lsp::parser::BazelParser;

#[test]
fn test_extract_targets() -> Result<()> {
    let parser = BazelParser::default();

    let build_file = r#"
# This is a sample BUILD file
cc_binary(
    name = "hello_world",
    srcs = ["hello_world.cc"],
)

go_binary(
    name = "go_app",
    srcs = ["main.go"],
)

py_test(
    name = "python_test",
    srcs = ["test_python.py"],
)
"#;

    let targets = parser.extract_targets(build_file)?;

    assert!(
        !targets.is_empty(),
        "No targets were extracted from the BUILD file"
    );

    let rule_types: Vec<String> = targets.iter().map(|t| t.rule_type.clone()).collect();

    // Check for common rule types
    assert!(
        rule_types.contains(&"cc_binary".to_string()),
        "cc_binary target not found"
    );
    assert!(
        rule_types.contains(&"go_binary".to_string()),
        "go_binary target not found"
    );
    assert!(
        rule_types.contains(&"py_test".to_string()),
        "py_test target not found"
    );

    // Verify target structure
    for target in &targets {
        // Check that the target has a name
        assert!(!target.name.is_empty(), "Target name is empty");

        // Check that the target has a rule type
        assert!(!target.rule_type.is_empty(), "Target rule type is empty");

        // Check that the target has a valid range
        assert!(
            target.range.start.line <= target.range.end.line,
            "Invalid range: start line after end line"
        );
        assert!(
            target.range.start.character <= target.range.end.character,
            "Invalid range: start character after end character"
        );
    }

    Ok(())
}

#[test]
fn test_target_range() -> Result<()> {
    // Create a BazelParser instance
    let parser = BazelParser::default();

    // Sample BUILD file content
    let build_file = r#"
cc_binary(
    name = "hello_world",
    srcs = ["hello_world.cc"],
)
"#;

    let targets = parser.extract_targets(build_file)?;

    assert!(
        !targets.is_empty(),
        "No targets were extracted from the BUILD file"
    );

    let target = &targets[0];

    assert_eq!(target.range.start.line, 1, "Expected start line to be 1");
    assert_eq!(
        target.range.start.character, 0,
        "Expected start character to be 0"
    );
    assert_eq!(
        target.range.end.character, 9,
        "Expected end character to be 9"
    );

    Ok(())
}

#[test]
fn test_target_name_format() -> Result<()> {
    // Create a BazelParser instance
    let parser = BazelParser::default();

    // Sample BUILD file content
    let build_file = r#"
cc_binary(
    name = "hello_world",
    srcs = ["hello_world.cc"],
)
"#;

    // Extract targets from the BUILD file
    let targets = parser.extract_targets(build_file)?;

    // Verify that at least one target was extracted
    assert!(
        !targets.is_empty(),
        "No targets were extracted from the BUILD file"
    );

    // Get the first target
    let target = &targets[0];

    // Verify that the target name follows the expected format
    assert!(
        target.name.ends_with("_target"),
        "Target name should end with '_target'"
    );

    Ok(())
}

#[test]
fn test_code_lens_command_types() -> Result<()> {
    // Create a BazelParser instance
    let parser = BazelParser::default();

    // Sample BUILD file content with different rule types
    let build_file = r#"
# This is a sample BUILD file
cc_binary(
    name = "hello_world",
    srcs = ["hello_world.cc"],
)

go_test(
    name = "go_test",
    srcs = ["main_test.go"],
)

py_library(
    name = "python_lib",
    srcs = ["lib.py"],
)
"#;

    // Extract targets from the BUILD file
    let targets = parser.extract_targets(build_file)?;

    // Verify that targets were extracted
    assert!(
        !targets.is_empty(),
        "No targets were extracted from the BUILD file"
    );

    // Check for different rule types
    let mut has_binary = false;
    let mut has_test = false;
    let mut has_library = false;

    for target in &targets {
        if target.rule_type.ends_with("_binary") {
            has_binary = true;
        } else if target.rule_type.ends_with("_test") {
            has_test = true;
        } else if target.rule_type.ends_with("_library") {
            has_library = true;
        }
    }

    // Verify that we have at least one of each rule type
    assert!(has_binary, "No binary target found");
    assert!(has_test, "No test target found");
    assert!(has_library, "No library target found");

    Ok(())
}
