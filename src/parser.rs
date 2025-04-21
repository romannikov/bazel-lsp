use anyhow::Result;
use std::sync::Mutex;
use tower_lsp::lsp_types::{Position, Range};
use tree_sitter::StreamingIterator;
use tree_sitter::{Parser, Query, QueryCursor};

#[derive(Debug, Clone)]
pub struct BazelTarget {
    pub name: String,
    pub rule_type: String,
    pub range: Range,
    pub rule_type_range: Range,
    pub rule_call_range: Range,
}

#[derive(Debug, Clone)]
pub struct BazelAttribute {
    pub range: Range,
}

#[derive(Debug, Clone)]
pub struct BazelString {
    pub range: Range,
}

pub struct BazelParser {
    parser: Mutex<Parser>,
    target_query: Query,
    attribute_query: Query,
    string_query: Query,
    deps_query: Query,
}

impl BazelParser {
    pub fn new() -> Result<Self> {
        let mut parser = Parser::new();
        let language = tree_sitter_starlark::LANGUAGE;
        parser
            .set_language(&language.into())
            .expect("Error loading Starlark parser");

        let target_query = Query::new(
            &language.into(),
            r#"
            (call
                function: (identifier) @rule_type
                arguments: (argument_list
                    (keyword_argument
                        name: (identifier) @arg_name
                        value: (string) @target_name
                    ) @first_name
                )
            )
            "#,
        )?;

        let attribute_query = Query::new(
            &language.into(),
            r#"
            (keyword_argument
                name: (identifier) @attr_name
            )
            "#,
        )?;

        let string_query = Query::new(
            &language.into(),
            r#"
            (string) @string
            "#,
        )?;

        let deps_query = Query::new(
            &language.into(),
            r#"
            (keyword_argument
                name: (identifier) @attr_name
                (#eq? @attr_name "deps")
                value: (list) @deps_list
            ) @deps_arg
            "#,
        )?;

        Ok(Self {
            parser: Mutex::new(parser),
            target_query: target_query,
            attribute_query: attribute_query,
            string_query: string_query,
            deps_query: deps_query,
        })
    }

    pub fn parse(&self, source: &str) -> Result<String> {
        self.parser
            .lock()
            .unwrap()
            .parse(source, None)
            .ok_or_else(|| anyhow::anyhow!("Failed to parse BUILD file"))?;
        Ok(source.to_string())
    }

    pub fn extract_targets(&self, source: &str) -> Result<Vec<BazelTarget>> {
        let tree = self
            .parser
            .lock()
            .unwrap()
            .parse(source, None)
            .ok_or_else(|| anyhow::anyhow!("Failed to parse BUILD file"))?;

        let mut targets = Vec::new();
        let mut cursor = QueryCursor::new();
        let mut matches = cursor.matches(&self.target_query, tree.root_node(), source.as_bytes());

        let mut processed_rule_calls = std::collections::HashSet::new();

        while let Some(m) = matches.next() {
            let mut rule_type = String::new();
            let mut target_name = String::new();
            let mut rule_call_node = None;
            let mut rule_type_node = None;

            for capture in m.captures {
                let node = capture.node;
                let text = &source[node.start_byte()..node.end_byte()];

                match capture.index {
                    0 => {
                        rule_type = text.to_string();
                        rule_type_node = Some(node);

                        // Find the parent call node which represents the entire target
                        let mut current = node.parent();
                        while let Some(parent) = current {
                            if parent.kind() == "call" {
                                rule_call_node = Some(parent);
                                break;
                            }
                            current = parent.parent();
                        }
                    }
                    2 => {
                        if text.starts_with('"') && text.ends_with('"') {
                            target_name = text[1..text.len() - 1].to_string();
                        }
                    }
                    _ => {}
                }
            }

            if let Some(rule_call) = rule_call_node {
                let rule_call_id = rule_call.id();
                if !processed_rule_calls.contains(&rule_call_id) {
                    processed_rule_calls.insert(rule_call_id);

                    if !rule_type.is_empty() && !target_name.is_empty() {
                        // Create the rule type range
                        let rule_type_range = if let Some(rule_type_node) = rule_type_node {
                            Range {
                                start: Position {
                                    line: rule_type_node.start_position().row as u32,
                                    character: rule_type_node.start_position().column as u32,
                                },
                                end: Position {
                                    line: rule_type_node.end_position().row as u32,
                                    character: rule_type_node.end_position().column as u32,
                                },
                            }
                        } else {
                            // Fallback to the start of the rule call if rule type node is not available
                            Range {
                                start: Position {
                                    line: rule_call.start_position().row as u32,
                                    character: rule_call.start_position().column as u32,
                                },
                                end: Position {
                                    line: rule_call.start_position().row as u32,
                                    character: rule_call.start_position().column as u32
                                        + rule_type.len() as u32,
                                },
                            }
                        };

                        // Create the rule call range (from rule type to closing parenthesis)
                        let rule_call_range = Range {
                            start: Position {
                                line: rule_type_range.start.line,
                                character: rule_type_range.start.character,
                            },
                            end: Position {
                                line: rule_call.end_position().row as u32,
                                character: rule_call.end_position().column as u32,
                            },
                        };

                        // Use the range of the entire call node instead of just the rule type
                        targets.push(BazelTarget {
                            name: target_name,
                            rule_type,
                            range: Range {
                                start: Position {
                                    line: rule_call.start_position().row as u32,
                                    character: rule_call.start_position().column as u32,
                                },
                                end: Position {
                                    line: rule_call.end_position().row as u32,
                                    character: rule_call.end_position().column as u32,
                                },
                            },
                            rule_type_range,
                            rule_call_range,
                        });
                    }
                }
            }
        }

        Ok(targets)
    }

    pub fn extract_attributes(&self, source: &str) -> Result<Vec<BazelAttribute>> {
        let tree = self
            .parser
            .lock()
            .unwrap()
            .parse(source, None)
            .ok_or_else(|| anyhow::anyhow!("Failed to parse BUILD file"))?;

        let mut attributes = Vec::new();
        let mut cursor = QueryCursor::new();
        let mut matches =
            cursor.matches(&self.attribute_query, tree.root_node(), source.as_bytes());

        while let Some(m) = matches.next() {
            for capture in m.captures {
                let node = capture.node;

                attributes.push(BazelAttribute {
                    range: Range {
                        start: Position {
                            line: node.start_position().row as u32,
                            character: node.start_position().column as u32,
                        },
                        end: Position {
                            line: node.end_position().row as u32,
                            character: node.end_position().column as u32,
                        },
                    },
                });
            }
        }

        Ok(attributes)
    }

    pub fn extract_strings(&self, source: &str) -> Result<Vec<BazelString>> {
        let tree = self
            .parser
            .lock()
            .unwrap()
            .parse(source, None)
            .ok_or_else(|| anyhow::anyhow!("Failed to parse BUILD file"))?;

        let mut strings = Vec::new();
        let mut cursor = QueryCursor::new();
        let mut matches = cursor.matches(&self.string_query, tree.root_node(), source.as_bytes());

        while let Some(m) = matches.next() {
            for capture in m.captures {
                let node = capture.node;

                strings.push(BazelString {
                    range: Range {
                        start: Position {
                            line: node.start_position().row as u32,
                            character: node.start_position().column as u32,
                        },
                        end: Position {
                            line: node.end_position().row as u32,
                            character: node.end_position().column as u32,
                        },
                    },
                });
            }
        }

        Ok(strings)
    }

    pub fn sort_deps_in_text(&self, source: &str) -> Result<String> {
        let tree = self
            .parser
            .lock()
            .unwrap()
            .parse(source, None)
            .ok_or_else(|| anyhow::anyhow!("Failed to parse BUILD file"))?;

        let mut cursor = QueryCursor::new();
        let mut matches = cursor.matches(&self.deps_query, tree.root_node(), source.as_bytes());

        let mut result = source.to_string();
        let mut changes = Vec::new();

        while let Some(m) = matches.next() {
            let mut deps: Vec<(String, String)> = Vec::new();
            let mut deps_range = None;

            for capture in m.captures {
                let node = capture.node;
                let text = &source[node.start_byte()..node.end_byte()];

                match capture.index {
                    0 => {
                        // This is the attr_name capture
                        continue;
                    }
                    1 => {
                        // This is the deps_list capture
                        let list_text = text.trim();
                        if list_text.starts_with('[') && list_text.ends_with(']') {
                            let content = &list_text[1..list_text.len() - 1];
                            for line in content.lines() {
                                let line = line.trim();
                                if line.is_empty() || line == "," {
                                    continue;
                                }

                                let dep_line = line.trim_end_matches(',').trim().to_string();
                                if dep_line.starts_with('"') {
                                    let mut dep = dep_line.clone();
                                    if let Some(comment_start) = dep_line.find('#') {
                                        dep = dep_line[..comment_start].trim().to_string();
                                    }
                                    if dep.starts_with('"') && dep.ends_with('"') {
                                        let dep_name = dep[1..dep.len() - 1].to_string();
                                        // Keep the first occurrence of each dependency with its comment
                                        if !deps.iter().any(|(name, _)| name == &dep_name) {
                                            deps.push((dep_name, dep_line));
                                        }
                                    }
                                }
                            }
                        }
                    }
                    2 => {
                        // This is the deps_arg capture (the entire keyword_argument node)
                        deps_range = Some(Range {
                            start: Position {
                                line: node.start_position().row as u32,
                                character: node.start_position().column as u32,
                            },
                            end: Position {
                                line: node.end_position().row as u32,
                                character: node.end_position().column as u32,
                            },
                        });
                    }
                    _ => {}
                }
            }

            if let Some(range) = deps_range {
                // Sort dependencies
                deps.sort_by(|a, b| a.0.cmp(&b.0));

                let formatted_deps = if deps.is_empty() {
                    "deps = []".to_string()
                } else {
                    let sorted_lines: Vec<String> =
                        deps.iter().map(|(_, line)| line.clone()).collect();
                    format!(
                        "deps = [\n        {}\n    ]",
                        sorted_lines.join(",\n        ") + ","
                    )
                };

                let start = self.position_to_byte_index(&result, &range.start);
                let end = self.position_to_byte_index(&result, &range.end);
                changes.push((start, end, formatted_deps));
            }
        }

        // Apply changes in reverse order to maintain correct indices
        changes.sort_by(|a, b| b.0.cmp(&a.0));
        for (start, end, formatted_deps) in changes {
            result.replace_range(start..end, &formatted_deps);
        }

        Ok(result)
    }

    fn position_to_byte_index(&self, text: &str, position: &Position) -> usize {
        let lines: Vec<&str> = text.lines().collect();
        let mut byte_index = 0;

        for i in 0..position.line as usize {
            if i < lines.len() {
                byte_index += lines[i].len() + 1;
            }
        }

        if (position.line as usize) < lines.len() {
            let line = lines[position.line as usize];
            let char_index = position.character as usize;
            let mut chars = 0;
            let mut bytes = 0;

            for c in line.chars() {
                if chars >= char_index {
                    break;
                }
                bytes += c.len_utf8();
                chars += 1;
            }

            byte_index += bytes;
        }

        byte_index
    }
}

impl Default for BazelParser {
    fn default() -> Self {
        Self::new().expect("Failed to initialize Bazel parser")
    }
}
