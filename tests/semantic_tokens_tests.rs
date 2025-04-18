use bazel_lsp::parser::BazelParser;

#[test]
fn test_semantic_tokens_targets() {
    let parser = BazelParser::default();
    let text = r#"
cc_binary(
    name = "hello_world",
    srcs = ["hello_world.cc"],
)
"#;

    // Extract targets
    let targets = parser.extract_targets(text).unwrap();

    // Check that we have at least one target
    assert!(!targets.is_empty());

    // Check that the first target is "cc_binary"
    let first_target = &targets[0];
    assert_eq!(first_target.rule_type, "cc_binary");

    // Check that the range has the correct length for "cc_binary"
    let range_length = first_target.range.end.character - first_target.range.start.character;
    assert_eq!(range_length, 9);
}

#[test]
fn test_semantic_tokens_attributes() {
    let source = r#"
go_binary(
    name = "hello_world",
    srcs = ["hello_world.cc"],
    deps = ["//base:base"],
)
"#;

    let parser = BazelParser::new().unwrap();
    let attributes = parser.extract_attributes(source).unwrap();

    assert!(!attributes.is_empty());
    for attr in attributes {
        assert!(attr.range.start.line >= 2);
        assert!(attr.range.end.line <= 5);
    }
}

#[test]
fn test_semantic_tokens_strings() {
    let parser = BazelParser::default();
    let text = r#"
cc_binary(
    name = "hello_world",
    srcs = ["hello_world.cc"],
)
"#;

    // Extract strings
    let strings = parser.extract_strings(text).unwrap();

    // Check that we have at least two strings ("hello_world" and "hello_world.cc")
    assert!(strings.len() >= 2);

    // Check that the strings have the correct length
    for string in strings {
        let range_length = string.range.end.character - string.range.start.character;
        // The length should be either 13 (for "hello_world") or 16 (for "hello_world.cc")
        assert!(
            range_length == 13 || range_length == 16,
            "Unexpected string length: {}",
            range_length
        );
    }
}

#[test]
fn test_semantic_tokens_all() {
    let parser = BazelParser::default();
    let text = r#"
cc_binary(
    name = "hello_world",
    srcs = ["hello_world.cc"],
)
"#;

    // Extract all token types
    let targets = parser.extract_targets(text).unwrap();
    let attributes = parser.extract_attributes(text).unwrap();
    let strings = parser.extract_strings(text).unwrap();

    // Check that we have at least one of each token type
    assert!(!targets.is_empty(), "No targets found");
    assert!(!attributes.is_empty(), "No attributes found");
    assert!(!strings.is_empty(), "No strings found");

    // Check that we have the expected number of tokens
    assert_eq!(targets.len(), 1, "Expected 1 target");
    assert_eq!(attributes.len(), 2, "Expected 2 attributes");
    assert_eq!(strings.len(), 2, "Expected 2 strings");
}
