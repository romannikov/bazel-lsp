use crate::bazel::{find_build_files, find_workspace_root, is_workspace_dir};
use crate::parser::BazelParser;
use crate::target_trie::{RuleInfo, TargetTrie};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::SemanticTokensOptions;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer};
use url;

pub struct Backend {
    pub client: Client,
    pub parser: BazelParser,
    pub documents: Arc<RwLock<HashMap<String, String>>>,
    pub target_trie: Arc<RwLock<TargetTrie>>,
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, params: InitializeParams) -> Result<InitializeResult> {
        if let Some(workspace_folders) = &params.workspace_folders {
            for folder in workspace_folders {
                let uri = &folder.uri;
                let path = uri.to_file_path().unwrap_or_default();

                if let Ok(true) = is_workspace_dir(&path) {
                    let mut trie = self.target_trie.write().await;

                    let build_files: Vec<PathBuf> = find_build_files(&path).into_iter().collect();

                    for build_file in build_files.iter() {
                        let _ = self.populate_trie_from_build_file(build_file, &mut trie);
                    }
                }
            }
        }

        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::INCREMENTAL,
                )),
                code_lens_provider: Some(CodeLensOptions {
                    resolve_provider: Some(true),
                }),
                completion_provider: Some(CompletionOptions {
                    trigger_characters: Some(vec![':'.into()]),
                    all_commit_characters: None,
                    resolve_provider: Some(false),
                    work_done_progress_options: WorkDoneProgressOptions {
                        work_done_progress: Some(true),
                    },
                    completion_item: None,
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
                                range: target.rule_type_range.clone(),
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
                                range: target.rule_type_range.clone(),
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
                        range: target.rule_type_range,
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

    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        let uri = params.text_document_position.text_document.uri;
        let position = params.text_document_position.position;

        let documents = self.documents.read().await;
        let text = documents.get(&uri.to_string()).cloned().unwrap_or_default();

        let line = text.lines().nth(position.line as usize).unwrap_or("");
        let line_up_to_cursor = &line[..position.character as usize];

        let last_trigger = if line_up_to_cursor.ends_with("//") {
            Some(line_up_to_cursor.len() - 2)
        } else if let Some(pos) = line_up_to_cursor.rfind(':') {
            Some(pos)
        } else {
            None
        };

        if let Some(trigger_pos) = last_trigger {
            let text_after_trigger = &line_up_to_cursor[trigger_pos..];

            let trie = self.target_trie.read().await;
            let matching_rules = trie.starts_with(text_after_trigger);

            let mut completion_items = Vec::new();
            for rules in matching_rules {
                for rule in rules {
                    let edit_text = if text_after_trigger.starts_with("//") {
                        let slash_count =
                            text_after_trigger.chars().take_while(|&c| c == '/').count();
                        if slash_count > 2 {
                            format!("//{}", &rule.full_build_path[2..])
                        } else if text_after_trigger.len() > 2 {
                            let existing_path = &text_after_trigger[2..];
                            if rule.full_build_path.contains(existing_path) {
                                if let Some(pos) = rule.full_build_path.find(existing_path) {
                                    rule.full_build_path[pos..].to_string()
                                } else {
                                    rule.full_build_path.clone()
                                }
                            } else {
                                rule.full_build_path.clone()
                            }
                        } else {
                            rule.full_build_path.clone()
                        }
                    } else if text_after_trigger.starts_with(':') {
                        format!(":{}", rule.name)
                    } else {
                        rule.full_build_path.clone()
                    };

                    completion_items.push(CompletionItem {
                        label: rule.full_build_path.clone(),
                        kind: Some(CompletionItemKind::TEXT),
                        detail: Some(format!("Target: {}", rule.full_build_path)),
                        documentation: Some(Documentation::String(format!(
                            "Bazel target: {}",
                            rule.full_build_path
                        ))),
                        text_edit: Some(CompletionTextEdit::Edit(TextEdit {
                            range: Range {
                                start: Position {
                                    line: position.line,
                                    character: trigger_pos as u32,
                                },
                                end: position,
                            },
                            new_text: edit_text,
                        })),
                        ..Default::default()
                    });
                }
            }

            return Ok(Some(CompletionResponse::Array(completion_items)));
        }

        Ok(None)
    }
}

impl Backend {
    pub fn new(client: Client) -> Self {
        Self {
            client,
            parser: BazelParser::default(),
            documents: Arc::new(RwLock::new(HashMap::new())),
            target_trie: Arc::new(RwLock::new(TargetTrie::new())),
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

    fn get_semantic_tokens(&self, text: &str) -> SemanticTokens {
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
            all_tokens.push((target.rule_type_range, 0));
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

    fn populate_trie_from_build_file(
        &self,
        build_file: &Path,
        trie: &mut TargetTrie,
    ) -> anyhow::Result<()> {
        if let Ok(content) = fs::read_to_string(build_file) {
            if let Ok(targets) = self.parser.extract_targets(&content) {
                let package_path = if let Some(workspace_root) = find_workspace_root(build_file)? {
                    if let Ok(relative_path) =
                        build_file.parent().unwrap().strip_prefix(workspace_root)
                    {
                        relative_path.to_string_lossy().to_string()
                    } else {
                        String::new()
                    }
                } else {
                    String::new()
                };

                for target in targets {
                    let full_target_path = if package_path.is_empty() {
                        target.name.clone()
                    } else {
                        format!("{}:{}", package_path, target.name)
                    };

                    let rule = RuleInfo::new(
                        target.name.clone(),
                        format!("//{}:{}", package_path, target.name),
                    );

                    trie.insert_target(&full_target_path, rule);
                }
            }
        }
        Ok(())
    }
}
