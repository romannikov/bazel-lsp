use bazel_lsp::parser::BazelParser;
use tower_lsp::lsp_types::{Position, Range};

#[test]
fn test_multiple_name_attributes() {
    let parser = BazelParser::new().unwrap();

    // Test case with multiple name attributes
    let source = r#"
go_library(
    name = "lib1",
    name = "lib2",
    srcs = ["lib.go"],
    deps = ["//go:go"],
)
"#;

    let targets = parser.extract_targets(source).unwrap();

    // Should only extract one target with the first name attribute
    assert_eq!(targets.len(), 1);

    let target = &targets[0];
    assert_eq!(target.name, "lib1");
    assert_eq!(target.rule_type, "go_library");

    // Verify the range is correct (should point to the rule type)
    let expected_range = Range {
        start: Position {
            line: 1,
            character: 0,
        },
        end: Position {
            line: 1,
            character: 10,
        },
    };

    assert_eq!(target.range, expected_range);
}

#[test]
fn test_single_name_attribute() {
    let parser = BazelParser::new().unwrap();

    // Test case with a single name attribute
    let source = r#"
go_library(
    name = "lib",
    srcs = ["lib.go"],
    deps = ["//go:go"],
)
"#;

    let targets = parser.extract_targets(source).unwrap();

    // Should extract one target
    assert_eq!(targets.len(), 1);

    let target = &targets[0];
    assert_eq!(target.name, "lib");
    assert_eq!(target.rule_type, "go_library");
}

#[test]
fn test_no_name_attribute() {
    let parser = BazelParser::new().unwrap();

    // Test case with no name attribute
    let source = r#"
go_library(
    srcs = ["lib.go"],
    deps = ["//go:go"],
)
"#;

    let targets = parser.extract_targets(source).unwrap();

    // Should not extract any targets
    assert_eq!(targets.len(), 0);
}
