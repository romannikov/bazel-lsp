use bazel_lsp::parser::BazelParser;

#[test]
fn test_sort_deps_basic() {
    let parser = BazelParser::default();
    let input = r#"
cc_binary(
    name = "my_binary",
    deps = [
        "//third_party:lib3",
        "//base:lib1",
        "//core:lib2",
    ],
)
"#;

    let expected = r#"
cc_binary(
    name = "my_binary",
    deps = [
        "//base:lib1",
        "//core:lib2",
        "//third_party:lib3",
    ],
)
"#;

    let result = parser.sort_deps_in_text(input).unwrap();
    assert_eq!(result, expected);
}

#[test]
fn test_sort_deps_multiple_targets() {
    let parser = BazelParser::default();
    let input = r#"
cc_binary(
    name = "binary1",
    deps = [
        "//third_party:lib3",
        "//base:lib1",
    ],
)

cc_binary(
    name = "binary2",
    deps = [
        "//core:lib2",
        "//base:lib1",
    ],
)
"#;

    let expected = r#"
cc_binary(
    name = "binary1",
    deps = [
        "//base:lib1",
        "//third_party:lib3",
    ],
)

cc_binary(
    name = "binary2",
    deps = [
        "//base:lib1",
        "//core:lib2",
    ],
)
"#;

    let result = parser.sort_deps_in_text(input).unwrap();
    assert_eq!(result, expected);
}

#[test]
fn test_sort_deps_empty_list() {
    let parser = BazelParser::default();
    let input = r#"
cc_binary(
    name = "my_binary",
    deps = [],
)
"#;

    let expected = r#"
cc_binary(
    name = "my_binary",
    deps = [],
)
"#;

    let result = parser.sort_deps_in_text(input).unwrap();
    assert_eq!(result, expected);
}

#[test]
fn test_sort_deps_single_dep() {
    let parser = BazelParser::default();
    let input = r#"
cc_binary(
    name = "my_binary",
    deps = ["//base:lib1"],
)
"#;

    let expected = r#"
cc_binary(
    name = "my_binary",
    deps = [
        "//base:lib1",
    ],
)
"#;

    let result = parser.sort_deps_in_text(input).unwrap();
    assert_eq!(result, expected);
}

#[test]
fn test_sort_deps_no_deps() {
    let parser = BazelParser::default();
    let input = r#"
cc_binary(
    name = "my_binary",
    srcs = ["main.cc"],
)
"#;

    let expected = r#"
cc_binary(
    name = "my_binary",
    srcs = ["main.cc"],
)
"#;

    let result = parser.sort_deps_in_text(input).unwrap();
    assert_eq!(result, expected);
}

#[test]
fn test_sort_deps_complex() {
    let parser = BazelParser::default();
    let input = r#"
cc_binary(
    name = "my_binary",
    srcs = ["main.cc"],
    deps = [
        "//third_party:lib3",
        "//base:lib1",
        "//core:lib2",
        "//base:lib1",  # duplicate
    ],
)

go_library(
    name = "go_lib",
    srcs = ["lib.go"],
    deps = [
        "//go:go111",
        "//go:go",
        "//aaa:go111",
    ],
)
"#;

    let expected = r#"
cc_binary(
    name = "my_binary",
    srcs = ["main.cc"],
    deps = [
        "//base:lib1",
        "//core:lib2",
        "//third_party:lib3",
    ],
)

go_library(
    name = "go_lib",
    srcs = ["lib.go"],
    deps = [
        "//aaa:go111",
        "//go:go",
        "//go:go111",
    ],
)
"#;

    let result = parser.sort_deps_in_text(input).unwrap();
    assert_eq!(result, expected);
}

#[test]
fn test_sort_deps_remove_duplicates() {
    let parser = BazelParser::default();
    let input = r#"
cc_binary(
    name = "my_binary",
    deps = [
        "//third_party:lib3",
        "//base:lib1",
        "//core:lib2",
        "//base:lib1",  # duplicate should be removed
    ],
)
"#;

    let expected = r#"
cc_binary(
    name = "my_binary",
    deps = [
        "//base:lib1",
        "//core:lib2",
        "//third_party:lib3",
    ],
)
"#;

    let result = parser.sort_deps_in_text(input).unwrap();
    assert_eq!(result, expected);
}

#[test]
fn test_sort_deps_remove_duplicates_multiple_targets() {
    let parser = BazelParser::default();
    let input = r#"
cc_binary(
    name = "binary1",
    deps = [
        "//third_party:lib3",
        "//base:lib1",
        "//base:lib1",  # duplicate
    ],
)

cc_binary(
    name = "binary2",
    deps = [
        "//core:lib2",
        "//base:lib1",
        "//base:lib1",  # duplicate
    ],
)
"#;

    let expected = r#"
cc_binary(
    name = "binary1",
    deps = [
        "//base:lib1",
        "//third_party:lib3",
    ],
)

cc_binary(
    name = "binary2",
    deps = [
        "//base:lib1",
        "//core:lib2",
    ],
)
"#;

    let result = parser.sort_deps_in_text(input).unwrap();
    assert_eq!(result, expected);
}
