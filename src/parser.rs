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
                    )?
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

        while let Some(m) = matches.next() {
            let mut rule_type = String::new();
            let mut target_name = String::new();
            let mut start_line = 0;
            let mut start_char = 0;
            let mut end_line = 0;
            let mut end_char = 0;

            for capture in m.captures {
                let node = capture.node;
                let text = &source[node.start_byte()..node.end_byte()];

                match capture.index {
                    0 => {
                        // rule_type
                        rule_type = text.to_string();
                        start_line = node.start_position().row;
                        start_char = node.start_position().column;
                        end_line = node.end_position().row;
                        end_char = node.end_position().column;
                    }
                    2 => {
                        // target_name (if present)
                        if text.starts_with('"') && text.ends_with('"') {
                            target_name = text[1..text.len() - 1].to_string();
                        }
                    }
                    _ => {}
                }
            }

            if !rule_type.is_empty() {
                targets.push(BazelTarget {
                    name: target_name,
                    rule_type,
                    range: Range {
                        start: Position {
                            line: start_line as u32,
                            character: start_char as u32,
                        },
                        end: Position {
                            line: end_line as u32,
                            character: end_char as u32,
                        },
                    },
                });
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
