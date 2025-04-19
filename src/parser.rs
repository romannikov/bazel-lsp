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

        Ok(Self {
            parser: Mutex::new(parser),
            target_query: target_query,
            attribute_query: attribute_query,
            string_query: string_query,
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
            let mut rule_start_line = 0;
            let mut rule_start_char = 0;
            let mut rule_end_line = 0;
            let mut rule_end_char = 0;
            let mut rule_call_node = None;

            for capture in m.captures {
                let node = capture.node;
                let text = &source[node.start_byte()..node.end_byte()];

                match capture.index {
                    0 => {
                        rule_type = text.to_string();

                        rule_start_line = node.start_position().row;
                        rule_start_char = node.start_position().column;
                        rule_end_line = node.end_position().row;
                        rule_end_char = node.end_position().column;

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
                        targets.push(BazelTarget {
                            name: target_name,
                            rule_type,
                            range: Range {
                                start: Position {
                                    line: rule_start_line as u32,
                                    character: rule_start_char as u32,
                                },
                                end: Position {
                                    line: rule_end_line as u32,
                                    character: rule_end_char as u32,
                                },
                            },
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
}

impl Default for BazelParser {
    fn default() -> Self {
        Self::new().expect("Failed to initialize Bazel parser")
    }
}
