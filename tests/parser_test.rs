use bazel_lsp::parser::BazelParser;
use tower_lsp::lsp_types::{Position, Range};

#[test]
fn test_multiple_name_attributes() {
    let parser = BazelParser::new().unwrap();
    let source = r#"
go_library(
    name = "lib1",
    name = "lib2",
    srcs = ["lib.go"],
    deps = ["//go:go"],
)
"#;

    let targets = parser.extract_targets(source).unwrap();
    assert_eq!(targets.len(), 1);

    let target = &targets[0];
    assert_eq!(target.name, "lib1");
    assert_eq!(target.rule_type, "go_library");

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

    assert_eq!(target.rule_type_range, expected_range);
}

#[test]
fn test_single_name_attribute() {
    let parser = BazelParser::new().unwrap();
    let source = r#"
go_library(
    name = "lib",
    srcs = ["lib.go"],
    deps = ["//go:go"],
)
"#;

    let targets = parser.extract_targets(source).unwrap();
    assert_eq!(targets.len(), 1);

    let target = &targets[0];
    assert_eq!(target.name, "lib");
    assert_eq!(target.rule_type, "go_library");
}

#[test]
fn test_no_name_attribute() {
    let parser = BazelParser::new().unwrap();
    let source = r#"
go_library(
    srcs = ["lib.go"],
    deps = ["//go:go"],
)
"#;

    let targets = parser.extract_targets(source).unwrap();
    assert_eq!(targets.len(), 0);
}

#[test]
fn test_rule_call_range() {
    let parser = BazelParser::new().unwrap();
    let source = r#"cc_binary(
    name = "my_target"
)"#;

    let targets = parser.extract_targets(source).unwrap();
    assert_eq!(targets.len(), 1);
    let target = &targets[0];

    assert_eq!(target.rule_call_range.start.line, 0);
    assert_eq!(target.rule_call_range.start.character, 0);
    assert_eq!(target.rule_call_range.end.line, 2);
    assert_eq!(target.rule_call_range.end.character, 1);
}

#[test]
fn test_rule_call_range_with_comments() {
    let parser = BazelParser::new().unwrap();
    let source = r#"# This is a comment
cc_binary(  # another comment
    name = "my_target"
)"#;

    let targets = parser.extract_targets(source).unwrap();
    assert_eq!(targets.len(), 1);
    let target = &targets[0];

    assert_eq!(target.rule_call_range.start.line, 1);
    assert_eq!(target.rule_call_range.start.character, 0);
    assert_eq!(target.rule_call_range.end.line, 3);
    assert_eq!(target.rule_call_range.end.character, 1);

    assert_eq!(target.range.start.line, 1);
    assert_eq!(target.range.start.character, 0);
    assert_eq!(target.range.end.line, 3);
    assert_eq!(target.range.end.character, 1);
}

#[test]
fn test_rule_call_range_multiple_targets() {
    let parser = BazelParser::new().unwrap();
    let source = r#"cc_binary(
    name = "target1"
)

go_library(
    name = "target2"
)"#;

    let targets = parser.extract_targets(source).unwrap();
    assert_eq!(targets.len(), 2);

    assert_eq!(targets[0].rule_call_range.start.line, 0);
    assert_eq!(targets[0].rule_call_range.start.character, 0);
    assert_eq!(targets[0].rule_call_range.end.line, 2);
    assert_eq!(targets[0].rule_call_range.end.character, 1);

    assert_eq!(targets[1].rule_call_range.start.line, 4);
    assert_eq!(targets[1].rule_call_range.start.character, 0);
    assert_eq!(targets[1].rule_call_range.end.line, 6);
    assert_eq!(targets[1].rule_call_range.end.character, 1);
}
