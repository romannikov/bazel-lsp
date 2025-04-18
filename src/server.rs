use crate::parser::BazelParser;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_lsp::jsonrpc::Result;
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
                    resolve_provider: Some(false),
                }),
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

        // Store the document text
        let mut documents = self.documents.write().await;
        documents.insert(uri.to_string(), text.clone());

        let message = format!("Opened: {}", uri);
        self.client.log_message(MessageType::INFO, message).await;

        self.publish_diagnostics(&uri, &text).await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri.clone();
        let text = params.content_changes[0].text.clone();

        // Update the stored document text
        let mut documents = self.documents.write().await;
        documents.insert(uri.to_string(), text.clone());

        self.publish_diagnostics(&uri, &text).await;
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
                                command: Some(Command {
                                    title: format!("Test {}", target.name),
                                    command: "bazel.test".into(),
                                    arguments: Some(vec![serde_json::json!({
                                        "target": target.name
                                    })]),
                                }),
                                data: None,
                            });
                        }
                        rule if rule.ends_with("_binary") => {
                            lenses.push(CodeLens {
                                range: target.range.clone(),
                                command: Some(Command {
                                    title: format!("▶ Run {}", target.name),
                                    command: "bazel.run".into(),
                                    arguments: Some(vec![serde_json::json!({
                                        "target": target.name
                                    })]),
                                }),
                                data: None,
                            });
                        }
                        _ => {}
                    }
                    lenses.push(CodeLens {
                        range: target.range,
                        command: Some(Command {
                            title: format!("Build {}", target.name),
                            command: "bazel.build".into(),
                            arguments: Some(vec![serde_json::json!({
                                "target": target.name
                            })]),
                        }),
                        data: None,
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
}
