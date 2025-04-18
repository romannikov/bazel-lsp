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

pub struct BazelParser {
    parser: Mutex<Parser>,
    query: Query,
}

impl BazelParser {
    pub fn new() -> Result<Self> {
        let mut parser = Parser::new();
        let language = tree_sitter_starlark::LANGUAGE;
        parser
            .set_language(&language.into())
            .expect("Error loading Starlark parser");

        let query = Query::new(
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

        Ok(Self {
            parser: Mutex::new(parser),
            query,
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
        let mut matches = cursor.matches(&self.query, tree.root_node(), source.as_bytes());

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
                            target_name = format!("{}_target", &text[1..text.len() - 1]);
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
}

impl Default for BazelParser {
    fn default() -> Self {
        Self::new().expect("Failed to initialize Bazel parser")
    }
}
