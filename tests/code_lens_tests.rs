use anyhow::Result;
use bazel_lsp::parser::BazelParser;
use tower_lsp::lsp_types::{CodeLens, Command};

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

    for target in &targets {
        assert!(!target.name.is_empty(), "Target name is empty");
        assert!(!target.rule_type.is_empty(), "Target rule type is empty");
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
    let parser = BazelParser::default();

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
fn test_code_lens_command_types() -> Result<()> {
    let parser = BazelParser::default();

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

    let targets = parser.extract_targets(build_file)?;

    assert!(
        !targets.is_empty(),
        "No targets were extracted from the BUILD file"
    );

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

    assert!(has_binary, "No binary target found");
    assert!(has_test, "No test target found");
    assert!(has_library, "No library target found");

    Ok(())
}

#[test]
fn test_code_lens_commands() -> Result<()> {
    let parser = BazelParser::default();

    let build_file = r#"
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

    let targets = parser.extract_targets(build_file)?;
    assert!(
        !targets.is_empty(),
        "No targets were extracted from the BUILD file"
    );

    let mut has_run_lens = false;
    let mut has_test_lens = false;
    let mut has_build_lens = false;

    for target in &targets {
        if target.rule_type.ends_with("_binary") {
            let lens = CodeLens {
                range: target.range.clone(),
                command: Some(Command {
                    title: format!("▶ Run {}", target.name),
                    command: "bazel.run".into(),
                    arguments: Some(vec![serde_json::json!({
                        "target": target.name
                    })]),
                }),
                data: None,
            };
            assert_eq!(lens.command.as_ref().unwrap().command, "bazel.run");
            assert!(lens.command.as_ref().unwrap().title.starts_with("▶ Run"));
            has_run_lens = true;
        } else if target.rule_type.ends_with("_test") {
            let lens = CodeLens {
                range: target.range.clone(),
                command: Some(Command {
                    title: format!("Test {}", target.name),
                    command: "bazel.test".into(),
                    arguments: Some(vec![serde_json::json!({
                        "target": target.name
                    })]),
                }),
                data: None,
            };
            assert_eq!(lens.command.as_ref().unwrap().command, "bazel.test");
            assert!(lens.command.as_ref().unwrap().title.starts_with("Test"));
            has_test_lens = true;
        }

        let build_lens = CodeLens {
            range: target.range.clone(),
            command: Some(Command {
                title: format!("Build {}", target.name),
                command: "bazel.build".into(),
                arguments: Some(vec![serde_json::json!({
                    "target": target.name
                })]),
            }),
            data: None,
        };
        assert_eq!(build_lens.command.as_ref().unwrap().command, "bazel.build");
        assert!(build_lens
            .command
            .as_ref()
            .unwrap()
            .title
            .starts_with("Build"));
        has_build_lens = true;
    }

    assert!(has_run_lens, "No run code lens found");
    assert!(has_test_lens, "No test code lens found");
    assert!(has_build_lens, "No build code lens found");

    Ok(())
}

#[test]
fn test_code_lens_target_names() -> Result<()> {
    let parser = BazelParser::default();

    let build_file = r#"
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

    let targets = parser.extract_targets(build_file)?;
    assert!(
        !targets.is_empty(),
        "No targets were extracted from the BUILD file"
    );

    let expected_targets = vec![
        ("hello_world", "cc_binary"),
        ("go_test", "go_test"),
        ("python_lib", "py_library"),
    ];

    for (target, (expected_name, expected_type)) in targets.iter().zip(expected_targets.iter()) {
        let build_lens = CodeLens {
            range: target.range.clone(),
            command: Some(Command {
                title: format!("Build {}", target.name),
                command: "bazel.build".into(),
                arguments: Some(vec![serde_json::json!({
                    "target": target.name
                })]),
            }),
            data: None,
        };

        let command = build_lens.command.as_ref().unwrap();
        let target_name = command.arguments.as_ref().unwrap()[0]["target"]
            .as_str()
            .unwrap();

        assert_eq!(
            target_name, *expected_name,
            "Target name '{}' should match expected name '{}'",
            target_name, expected_name
        );

        assert_eq!(
            target.rule_type, *expected_type,
            "Rule type '{}' should match expected type '{}'",
            target.rule_type, expected_type
        );

        match target.rule_type.as_str() {
            rule if rule.ends_with("_binary") => {
                let run_lens = CodeLens {
                    range: target.range.clone(),
                    command: Some(Command {
                        title: format!("▶ Run {}", target.name),
                        command: "bazel.run".into(),
                        arguments: Some(vec![serde_json::json!({
                            "target": target.name
                        })]),
                    }),
                    data: None,
                };
                let run_target = run_lens
                    .command
                    .as_ref()
                    .unwrap()
                    .arguments
                    .as_ref()
                    .unwrap()[0]["target"]
                    .as_str()
                    .unwrap();
                assert_eq!(
                    run_target, *expected_name,
                    "Run lens target name should match expected name"
                );
            }
            rule if rule.ends_with("_test") => {
                let test_lens = CodeLens {
                    range: target.range.clone(),
                    command: Some(Command {
                        title: format!("Test {}", target.name),
                        command: "bazel.test".into(),
                        arguments: Some(vec![serde_json::json!({
                            "target": target.name
                        })]),
                    }),
                    data: None,
                };
                let test_target = test_lens
                    .command
                    .as_ref()
                    .unwrap()
                    .arguments
                    .as_ref()
                    .unwrap()[0]["target"]
                    .as_str()
                    .unwrap();
                assert_eq!(
                    test_target, *expected_name,
                    "Test lens target name should match expected name"
                );
            }
            _ => {}
        }
    }

    Ok(())
}
