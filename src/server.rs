use crate::parser::BazelParser;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::SemanticTokensOptions;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer};
use url;

pub struct Backend {
    pub client: Client,
    parser: BazelParser,
    documents: Arc<RwLock<HashMap<String, String>>>,
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::INCREMENTAL,
                )),
                code_lens_provider: Some(CodeLensOptions {
                    resolve_provider: Some(true),
                }),
                semantic_tokens_provider: Some(
                    SemanticTokensServerCapabilities::SemanticTokensOptions(
                        SemanticTokensOptions {
                            work_done_progress_options: WorkDoneProgressOptions {
                                work_done_progress: Some(true),
                            },
                            legend: SemanticTokensLegend {
                                token_types: vec![
                                    SemanticTokenType::new("function"),
                                    SemanticTokenType::new("property"),
                                    SemanticTokenType::new("string"),
                                ],
                                token_modifiers: vec![],
                            },
                            range: Some(true),
                            full: None,
                        },
                    ),
                ),
                document_formatting_provider: Some(OneOf::Left(true)),
                execute_command_provider: Some(ExecuteCommandOptions {
                    commands: vec![
                        "bazel.build".into(),
                        "bazel.test".into(),
                        "bazel.run".into(),
                        "bazel.execute".into(),
                    ],
                    work_done_progress_options: WorkDoneProgressOptions {
                        work_done_progress: Some(true),
                    },
                }),
                ..Default::default()
            },
            server_info: Some(ServerInfo {
                name: "bazel-lsp".into(),
                version: Some("0.1.0".into()),
            }),
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "Bazel LSP server initialized!")
            .await;
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri.clone();
        let text = params.text_document.text.clone();

        let mut documents = self.documents.write().await;
        documents.insert(uri.to_string(), text.clone());

        let message = format!("Opened: {}", uri);
        self.client.log_message(MessageType::INFO, message).await;

        self.publish_diagnostics(&uri, &text).await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri.clone();

        self.update_document_content(&uri, &params.content_changes)
            .await;

        let documents = self.documents.read().await;
        let text = documents.get(uri.as_str()).cloned().unwrap_or_default();

        self.publish_diagnostics(&uri, &text).await;

        self.client
            .send_request::<request::SemanticTokensRefresh>(())
            .await
            .ok();
        self.client
            .send_request::<request::CodeLensRefresh>(())
            .await
            .ok();
    }

    async fn code_lens(&self, params: CodeLensParams) -> Result<Option<Vec<CodeLens>>> {
        let uri = params.text_document.uri.clone();

        let documents = self.documents.read().await;
        let text = documents.get(&uri.to_string()).cloned().unwrap_or_default();

        let mut lenses = Vec::new();

        match self.parser.extract_targets(&text) {
            Ok(targets) => {
                for target in targets {
                    match target.rule_type.as_str() {
                        rule if rule.ends_with("_test") => {
                            lenses.push(CodeLens {
                                range: target.range.clone(),
                                command: None,
                                data: Some(serde_json::json!({
                                    "type": "test",
                                    "target": target.name,
                                    "rule_type": target.rule_type,
                                })),
                            });
                        }
                        rule if rule.ends_with("_binary") => {
                            lenses.push(CodeLens {
                                range: target.range.clone(),
                                command: None,
                                data: Some(serde_json::json!({
                                    "type": "run",
                                    "target": target.name,
                                    "rule_type": target.rule_type,
                                })),
                            });
                        }
                        _ => {}
                    }
                    lenses.push(CodeLens {
                        range: target.range,
                        command: None,
                        data: Some(serde_json::json!({
                            "type": "build",
                            "target": target.name,
                            "rule_type": target.rule_type,
                        })),
                    });
                }
            }
            Err(err) => {
                self.client
                    .log_message(
                        MessageType::ERROR,
                        format!("Failed to extract targets: {}", err),
                    )
                    .await;
            }
        }

        Ok(Some(lenses))
    }

    async fn code_lens_resolve(&self, lens: CodeLens) -> Result<CodeLens> {
        let data = lens.data.clone().unwrap();
        let lens_type = data["type"].as_str().unwrap();
        let target = data["target"].as_str().unwrap();

        let command = match lens_type {
            "run" => Command {
                title: format!("▶ Run {}", target),
                command: "bazel.run".into(),
                arguments: Some(vec![serde_json::json!({
                    "target": target
                })]),
            },
            "test" => Command {
                title: format!("Test {}", target),
                command: "bazel.test".into(),
                arguments: Some(vec![serde_json::json!({
                    "target": target
                })]),
            },
            "build" => Command {
                title: format!("Build {}", target),
                command: "bazel.build".into(),
                arguments: Some(vec![serde_json::json!({
                    "target": target
                })]),
            },
            _ => panic!("Unknown lens type: {}", lens_type),
        };

        Ok(CodeLens {
            range: lens.range,
            command: Some(command),
            data: lens.data,
        })
    }

    async fn semantic_tokens_full(
        &self,
        params: SemanticTokensParams,
    ) -> Result<Option<SemanticTokensResult>> {
        let uri = params.text_document.uri.clone();
        let documents = self.documents.read().await;
        let text = documents.get(&uri.to_string()).cloned().unwrap_or_default();

        let tokens = self.get_semantic_tokens(&text);
        Ok(Some(SemanticTokensResult::Tokens(tokens)))
    }

    async fn semantic_tokens_range(
        &self,
        params: SemanticTokensRangeParams,
    ) -> Result<Option<SemanticTokensRangeResult>> {
        let uri = params.text_document.uri.clone();
        let documents = self.documents.read().await;
        let text = documents.get(&uri.to_string()).cloned().unwrap_or_default();

        let tokens = self.get_semantic_tokens(&text);
        Ok(Some(SemanticTokensRangeResult::Tokens(tokens)))
    }

    async fn formatting(&self, params: DocumentFormattingParams) -> Result<Option<Vec<TextEdit>>> {
        let uri = params.text_document.uri;
        let documents = self.documents.read().await;
        let text = documents.get(&uri.to_string()).cloned().unwrap_or_default();

        let formatted_text = self.parser.sort_deps_in_text(&text).map_err(|e| {
            let mut error =
                tower_lsp::jsonrpc::Error::new(tower_lsp::jsonrpc::ErrorCode::InternalError);
            error.data = Some(serde_json::json!({ "message": e.to_string() }));
            error
        })?;

        Ok(Some(vec![TextEdit {
            range: Range {
                start: Position {
                    line: 0,
                    character: 0,
                },
                end: Position {
                    line: text.lines().count() as u32,
                    character: 0,
                },
            },
            new_text: formatted_text,
        }]))
    }
}

impl Backend {
    pub fn new(client: Client) -> Self {
        Self {
            client,
            parser: BazelParser::default(),
            documents: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn publish_diagnostics(&self, uri: &url::Url, text: &str) {
        let mut diagnostics = Vec::new();

        match self.parser.parse(text) {
            Ok(_) => {
                self.client
                    .publish_diagnostics(uri.clone(), diagnostics, None)
                    .await;
            }
            Err(err) => {
                let diagnostic = Diagnostic {
                    range: Range {
                        start: Position {
                            line: 0,
                            character: 0,
                        },
                        end: Position {
                            line: 0,
                            character: 0,
                        },
                    },
                    severity: Some(DiagnosticSeverity::ERROR),
                    code: Some(NumberOrString::String("parse_error".to_string())),
                    code_description: None,
                    source: Some("bazel-lsp".to_string()),
                    message: err.to_string(),
                    related_information: None,
                    tags: None,
                    data: None,
                };

                diagnostics.push(diagnostic);
                self.client
                    .publish_diagnostics(uri.clone(), diagnostics, None)
                    .await;
            }
        }
    }

    pub async fn update_document_content(
        &self,
        uri: &url::Url,
        content_changes: &[TextDocumentContentChangeEvent],
    ) {
        let mut documents = self.documents.write().await;
        let current_text = documents.get(&uri.to_string()).cloned().unwrap_or_default();

        let mut new_text = current_text;
        for change in content_changes {
            if let Some(range) = &change.range {
                let start_byte = self.position_to_byte_index(&new_text, &range.start);
                let end_byte = self.position_to_byte_index(&new_text, &range.end);

                new_text.replace_range(start_byte..end_byte, &change.text);
            } else {
                new_text = change.text.clone();
            }
        }

        documents.insert(uri.to_string(), new_text);
    }

    fn position_to_byte_index(&self, text: &str, position: &Position) -> usize {
        let lines: Vec<&str> = text.lines().collect();
        let mut byte_index = 0;

        for i in 0..position.line as usize {
            if i < lines.len() {
                byte_index += lines[i].len() + 1; // +1 for the newline character
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

    pub fn get_semantic_tokens(&self, text: &str) -> SemanticTokens {
        let mut tokens = Vec::new();

        let targets = match self.parser.extract_targets(text) {
            Ok(targets) => targets,
            Err(_) => Vec::new(),
        };

        let attributes = match self.parser.extract_attributes(text) {
            Ok(attributes) => attributes,
            Err(_) => Vec::new(),
        };

        let strings = match self.parser.extract_strings(text) {
            Ok(strings) => strings,
            Err(_) => Vec::new(),
        };

        let mut all_tokens: Vec<(Range, u32)> = Vec::new();

        for target in targets {
            all_tokens.push((target.range, 0));
        }

        for attr in attributes {
            all_tokens.push((attr.range, 1));
        }

        for string in strings {
            all_tokens.push((string.range, 2));
        }

        all_tokens.sort_by(|a, b| {
            let line_cmp = a.0.start.line.cmp(&b.0.start.line);
            if line_cmp == std::cmp::Ordering::Equal {
                a.0.start.character.cmp(&b.0.start.character)
            } else {
                line_cmp
            }
        });

        let mut prev_line = 0;
        let mut prev_start = 0;

        for (range, token_type) in all_tokens {
            let delta_line = range.start.line;
            let delta_start = if delta_line == prev_line {
                if range.start.character >= prev_start {
                    range.start.character - prev_start
                } else {
                    0
                }
            } else {
                range.start.character
            };

            let delta_line_value = if tokens.is_empty() {
                delta_line
            } else {
                if delta_line >= prev_line {
                    delta_line - prev_line
                } else {
                    0
                }
            };

            tokens.push(SemanticToken {
                delta_line: delta_line_value,
                delta_start: delta_start as u32,
                length: (range.end.character - range.start.character) as u32,
                token_type,
                token_modifiers_bitset: 0,
            });

            prev_line = delta_line;
            prev_start = range.start.character;
        }

        SemanticTokens {
            result_id: None,
            data: tokens,
        }
    }
}
