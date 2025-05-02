use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::RwLock;
use tower_lsp::{LspService, Server};

use bazel_lsp::server::Backend;
use bazel_lsp::target_trie::{RuleInfo, TargetTrie};

async fn setup_server() -> (
    tokio::io::WriteHalf<tokio::io::DuplexStream>,
    tokio::io::ReadHalf<tokio::io::DuplexStream>,
) {
    let (service, socket) = LspService::new(|client| {
        let mut backend = Backend::new(client);
        let mut trie = TargetTrie::new();
        trie.insert_target(
            "//a:inside_a",
            RuleInfo::new("inside_a".into(), "//a:inside_a".into()),
        );
        trie.insert_target(
            "//a:inside_b",
            RuleInfo::new("inside_b".into(), "//a:inside_b".into()),
        );
        trie.insert_target(
            "//a/b:target1",
            RuleInfo::new("target1".into(), "//a/b:target1".into()),
        );
        trie.insert_target(
            "//a/c:target2",
            RuleInfo::new("target2".into(), "//a/c:target2".into()),
        );
        trie.insert_target(
            "//a/b:target2",
            RuleInfo::new("target2".into(), "//a/b:target2".into()),
        );
        backend.target_trie = Arc::new(RwLock::new(trie));
        backend
    });

    let (stdin, stdout) = tokio::io::duplex(1024);
    let (stdin_read, stdin_write) = tokio::io::split(stdin);
    let (stdout_read, stdout_write) = tokio::io::split(stdout);
    let server_fut = Server::new(stdin_read, stdout_write, socket).serve(service);
    tokio::spawn(server_fut);

    (stdin_write, stdout_read)
}

async fn send_message(
    writer: &mut tokio::io::WriteHalf<tokio::io::DuplexStream>,
    message: serde_json::Value,
) -> Result<(), anyhow::Error> {
    let message_str = message.to_string();
    let header = format!("Content-Length: {}\r\n\r\n", message_str.len());
    writer.write_all(header.as_bytes()).await?;
    writer.write_all(message_str.as_bytes()).await?;
    writer.flush().await?;
    Ok(())
}

async fn read_message(
    reader: &mut tokio::io::ReadHalf<tokio::io::DuplexStream>,
) -> Result<serde_json::Value, anyhow::Error> {
    let mut header = String::new();
    loop {
        let mut buf = [0; 1];
        reader.read_exact(&mut buf).await?;
        header.push(buf[0] as char);
        if header.ends_with("\r\n\r\n") {
            break;
        }
    }

    let content_length = header
        .lines()
        .find(|line| line.starts_with("Content-Length: "))
        .and_then(|line| line.split(": ").nth(1))
        .and_then(|len| len.parse::<usize>().ok())
        .ok_or_else(|| anyhow::anyhow!("Invalid Content-Length header"))?;

    let mut content = vec![0; content_length];
    reader.read_exact(&mut content).await?;
    let response = serde_json::from_slice(&content)?;
    println!("Received response: {}", response);
    Ok(response)
}

#[tokio::test]
async fn test_completion_with_colon() -> Result<(), anyhow::Error> {
    let (mut stdin, mut stdout) = setup_server().await;

    let init_params = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "capabilities": {},
            "rootUri": "file:///",
            "processId": 1
        }
    });
    send_message(&mut stdin, init_params).await?;
    let init_response = read_message(&mut stdout).await?;
    assert_eq!(init_response["id"], 1);

    let initialized_params = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "initialized",
        "params": {}
    });
    send_message(&mut stdin, initialized_params).await?;

    let did_open_params = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "textDocument/didOpen",
        "params": {
            "textDocument": {
                "uri": "file:///test.bzl",
                "languageId": "starlark",
                "version": 1,
                "text": "a:"
            }
        }
    });
    send_message(&mut stdin, did_open_params).await?;

    // Read and ignore the echoed notifications
    let _ = read_message(&mut stdout).await?; // initialized
    let _ = read_message(&mut stdout).await?; // didOpen

    let completion_params = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "textDocument/completion",
        "params": {
            "textDocument": {
                "uri": "file:///test.bzl"
            },
            "position": {
                "line": 0,
                "character": 2
            }
        }
    });
    send_message(&mut stdin, completion_params).await?;

    // Read the completion response
    let response = read_message(&mut stdout).await?;
    println!("Completion response: {}", response);

    assert_eq!(response["id"], 2);
    assert!(response["result"].is_array() || response["result"].is_null());
    if response["result"].is_array() {
        let items = response["result"].as_array().unwrap();
        assert_eq!(items.len(), 2);
        assert!(items.iter().any(|item| item["label"] == "//a:inside_a"));
        assert!(items.iter().any(|item| item["label"] == "//a:inside_b"));
    }

    Ok(())
}

#[tokio::test]
async fn test_completion_with_double_slash() -> Result<(), anyhow::Error> {
    let (mut stdin, mut stdout) = setup_server().await;

    let init_params = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "processId": 1,
            "clientInfo": {
                "name": "test-client",
                "version": "1.0.0"
            },
            "rootUri": "file:///",
            "capabilities": {
                "textDocument": {
                    "completion": {
                        "dynamicRegistration": true,
                        "completionItem": {
                            "snippetSupport": true,
                            "resolveSupport": {
                                "properties": ["documentation", "detail", "additionalTextEdits"]
                            }
                        }
                    }
                }
            }
        }
    });
    send_message(&mut stdin, init_params).await?;
    let init_response = read_message(&mut stdout).await?;
    assert_eq!(init_response["id"], 1);

    let initialized_params = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "initialized",
        "params": {}
    });
    send_message(&mut stdin, initialized_params).await?;

    let did_open_params = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "textDocument/didOpen",
        "params": {
            "textDocument": {
                "uri": "file:///test.bzl",
                "languageId": "starlark",
                "version": 1,
                "text": "//"
            }
        }
    });
    send_message(&mut stdin, did_open_params).await?;

    // Read and ignore the echoed notifications
    let _ = read_message(&mut stdout).await?; // initialized
    let _ = read_message(&mut stdout).await?; // didOpen

    let completion_params = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "textDocument/completion",
        "params": {
            "textDocument": {
                "uri": "file:///test.bzl"
            },
            "position": {
                "line": 0,
                "character": 2
            }
        }
    });
    send_message(&mut stdin, completion_params).await?;

    let response = read_message(&mut stdout).await?;
    println!("Completion response: {}", response);

    assert_eq!(response["id"], 2);
    assert!(response["result"].is_array() || response["result"].is_null());
    if response["result"].is_array() {
        let items = response["result"].as_array().unwrap();
        assert_eq!(items.len(), 2);
        assert!(items.iter().any(|item| item["label"] == "//a/b:target1"));
        assert!(items.iter().any(|item| item["label"] == "//a/c:target2"));
    }

    Ok(())
}

#[tokio::test]
async fn test_completion_with_existing_path() -> Result<(), anyhow::Error> {
    let (mut stdin, mut stdout) = setup_server().await;

    let init_params = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "capabilities": {},
            "rootUri": "file:///",
            "processId": 1
        }
    });
    send_message(&mut stdin, init_params).await?;
    let init_response = read_message(&mut stdout).await?;
    assert_eq!(init_response["id"], 1);

    let initialized_params = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "initialized",
        "params": {}
    });
    send_message(&mut stdin, initialized_params).await?;

    let did_open_params = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "textDocument/didOpen",
        "params": {
            "textDocument": {
                "uri": "file:///test.bzl",
                "languageId": "starlark",
                "version": 1,
                "text": "//a/b"
            }
        }
    });
    send_message(&mut stdin, did_open_params).await?;

    // Read and ignore the echoed notifications
    let _ = read_message(&mut stdout).await?; // initialized
    let _ = read_message(&mut stdout).await?; // didOpen

    let completion_params = serde_json::json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "textDocument/completion",
        "params": {
            "textDocument": {
                "uri": "file:///test.bzl"
            },
            "position": {
                "line": 0,
                "character": 5
            }
        }
    });
    send_message(&mut stdin, completion_params).await?;

    let response = read_message(&mut stdout).await?;
    println!("Completion response: {}", response);

    assert_eq!(response["id"], 2);
    assert!(response["result"].is_array() || response["result"].is_null());
    if response["result"].is_array() {
        let items = response["result"].as_array().unwrap();
        assert_eq!(items.len(), 2);
        assert!(items.iter().any(|item| item["label"] == "//a/b:target1"));
        assert!(items.iter().any(|item| item["label"] == "//a/b:target2"));
    }

    Ok(())
}
