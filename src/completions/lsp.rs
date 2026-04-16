use crate::completions::CompletionItem;
use serde_json::{json, Value};
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::mpsc;

pub enum LspMessage {
    Completions(Vec<CompletionItem>),
    Diagnostic(Option<String>),
    Status(String),
}

pub struct LspProvider {
    child: Option<Child>,
    tx: Option<mpsc::Sender<Value>>,
}

impl Default for LspProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl LspProvider {
    pub fn new() -> Self {
        Self {
            child: None,
            tx: None,
        }
    }

    pub async fn start(&mut self, msg_tx: mpsc::Sender<LspMessage>) -> Result<(), Box<dyn std::error::Error>> {
        let lsp_bin = std::env::var("JQPP_LSP_BIN").unwrap_or_else(|_| "jq-lsp".to_string());
        let mut child = Command::new(lsp_bin)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()?;

        let mut stdin = child.stdin.take().ok_or("Failed to open stdin")?;
        let stdout = child.stdout.take().ok_or("Failed to open stdout")?;

        let (tx, mut rx) = mpsc::channel::<Value>(32);
        let reader_lsp_tx = tx.clone();
        self.tx = Some(tx);

        // Reader task — also holds a clone of `tx` so it can send responses
        // back to jq-lsp (e.g. workspace/configuration, client/registerCapability).
        let reader_msg_tx = msg_tx.clone();
        tokio::spawn(async move {
            let mut reader = BufReader::new(stdout);
            loop {
                let mut line = String::new();
                if reader.read_line(&mut line).await.unwrap_or(0) == 0 {
                    break;
                }
                if let Some(stripped) = line.strip_prefix("Content-Length: ") {
                    let len: usize = stripped.trim().parse().unwrap_or(0);
                    let mut dummy = String::new();
                    let _ = reader.read_line(&mut dummy).await;

                    let mut body = vec![0u8; len];
                    if reader.read_exact(&mut body).await.is_err() { break; }
                    if let Ok(msg) = serde_json::from_slice::<Value>(&body) {
                        handle_lsp_message(msg, &reader_msg_tx, &reader_lsp_tx).await;
                    }
                }
            }
        });

        // Writer task
        tokio::spawn(async move {
            while let Some(msg) = rx.recv().await {
                let body = serde_json::to_string(&msg).unwrap();
                let header = format!("Content-Length: {}\r\n\r\n", body.len());
                if stdin.write_all(header.as_bytes()).await.is_err() {
                    break;
                }
                if stdin.write_all(body.as_bytes()).await.is_err() {
                    break;
                }
                let _ = stdin.flush().await;
            }
        });

        self.child = Some(child);

        // Handshake: Initialize
        self.send(json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "processId": std::process::id(),
                "rootUri": null,
                "capabilities": {}
            }
        })).await;

        Ok(())
    }

    pub async fn send(&self, msg: Value) {
        if let Some(ref tx) = self.tx {
            let _ = tx.send(msg).await;
        }
    }

    pub async fn did_change(&self, text: &str) {
        self.send(json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didChange",
            "params": {
                "textDocument": {
                    "uri": "file:///query.jq",
                    "version": 1
                },
                "contentChanges": [{ "text": text }]
            }
        })).await;
    }

    pub async fn completion(&self, full_text: &str) {
        // jq-lsp only completes bare function names — it returns nothing when
        // given a full pipe expression like `"x" | asc`.  Extract just the
        // token being typed (text after the last `|`) and send that as a
        // separate snippet document so the LSP sees a bare identifier.
        let token = full_text
            .rfind('|')
            .map(|p| full_text[p + 1..].trim_start())
            .unwrap_or(full_text);

        // Update the snippet document with the bare token.
        self.send(json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didChange",
            "params": {
                "textDocument": { "uri": "file:///completion-snippet.jq", "version": 1 },
                "contentChanges": [{ "text": token }]
            }
        })).await;

        // Request completions from the snippet document at the end of the token.
        self.send(json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "textDocument/completion",
            "params": {
                "textDocument": { "uri": "file:///completion-snippet.jq" },
                "position": { "line": 0, "character": token.len() }
            }
        })).await;
    }

    pub async fn shutdown(&mut self) {
        self.send(json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "shutdown"
        })).await;
        self.send(json!({
            "jsonrpc": "2.0",
            "method": "exit"
        })).await;
        if let Some(mut child) = self.child.take() {
            // Give the LSP process 1 s to exit gracefully, then kill it.
            let timed_out = tokio::time::timeout(
                std::time::Duration::from_secs(1),
                child.wait(),
            ).await.is_err();
            if timed_out {
                let _ = child.kill().await;
            }
        }
    }
}

async fn handle_lsp_message(
    msg: Value,
    main_tx: &mpsc::Sender<LspMessage>,
    lsp_tx: &mpsc::Sender<Value>,
) {
    // Server-initiated requests (have both "id" and "method") need responses.
    if let (Some(id), Some(method)) = (
        msg.get("id").cloned(),
        msg.get("method").and_then(|m| m.as_str()),
    ) {
        match method {
            "client/registerCapability" => {
                let _ = lsp_tx.send(json!({"jsonrpc":"2.0","id":id,"result":null})).await;
            }
            "workspace/configuration" => {
                // Respond with one empty config object per requested item.
                let n = msg["params"]["items"]
                    .as_array()
                    .map(|a| a.len())
                    .unwrap_or(1);
                let result: Vec<Value> = (0..n).map(|_| json!({})).collect();
                let _ = lsp_tx.send(json!({"jsonrpc":"2.0","id":id,"result":result})).await;
            }
            _ => {
                // Unknown server request — acknowledge with null result.
                let _ = lsp_tx.send(json!({"jsonrpc":"2.0","id":id,"result":null})).await;
            }
        }
        return;
    }

    // Server notifications (no "id", has "method")
    if let Some(method) = msg.get("method").and_then(|m| m.as_str()) {
        if method == "textDocument/publishDiagnostics" {
            let diag = msg["params"]["diagnostics"]
                .as_array()
                .and_then(|a| a.first())
                .and_then(|d| d.get("message"))
                .and_then(|m| m.as_str())
                .map(|s| s.to_string());
            let _ = main_tx.send(LspMessage::Diagnostic(diag)).await;
        }
        return;
    }

    // Responses to our requests (have "id", no "method")
    if let Some(id) = msg.get("id") {
        if id == 1 {
            // initialize response — now send the required `initialized` notification
            let _ = lsp_tx.send(json!({"jsonrpc":"2.0","method":"initialized","params":{}})).await;
            let _ = main_tx.send(LspMessage::Status("ready".to_string())).await;
        } else if id == 2 {
            // completion response
            let items = msg.get("result").and_then(|r| {
                if r.is_array() { r.as_array() } else { r.get("items").and_then(|i| i.as_array()) }
            });
            let mut completions = Vec::new();
            if let Some(items) = items {
                for item in items {
                    if let Some(label) = item.get("label").and_then(|l| l.as_str()) {
                        let insert_text = item
                            .get("insertText")
                            .and_then(|it| it.as_str())
                            .unwrap_or(label)
                            .to_string();
                        completions.push(CompletionItem {
                            label: label.to_string(),
                            detail: item.get("detail").and_then(|d| d.as_str()).map(|s| s.to_string()),
                            insert_text,
                        });
                    }
                }
            }
            let _ = main_tx.send(LspMessage::Completions(completions)).await;
        }
    }
}
