use serde_json::{json, Value};
use std::fs::OpenOptions;
use std::io::{self, Read, Write};

fn log(msg: &str) {
    let log_path = std::env::var("MOCK_LSP_LOG").unwrap_or_else(|_| "mock_lsp.log".to_string());
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(log_path) {
        let _ = writeln!(file, "{}", msg);
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    log("Mock LSP started");
    let mut stdin = io::stdin();
    let mut stdout = io::stdout();

    loop {
        let mut header = String::new();
        loop {
            let mut buf = [0u8; 1];
            if stdin.read_exact(&mut buf).is_err() {
                log("Stdin closed");
                return Ok(());
            }
            header.push(buf[0] as char);
            if header.ends_with("\r\n\r\n") {
                break;
            }
        }

        if let Some(pos) = header.find("Content-Length: ") {
            let len_part = &header[pos + 16..];
            let end_pos = len_part.find("\r\n").unwrap();
            let len: usize = len_part[..end_pos].parse()?;

            let mut body = vec![0u8; len];
            stdin.read_exact(&mut body)?;
            let msg: Value = serde_json::from_slice(&body)?;
            log(&format!("Received: {}", msg));

            if let Some(id) = msg.get("id") {
                let method = msg.get("method").and_then(|m| m.as_str());
                if method == Some("initialize") {
                    log("Responding to initialize");
                    send(
                        &mut stdout,
                        json!({
                            "jsonrpc": "2.0",
                            "id": id,
                            "result": { "capabilities": {} }
                        }),
                    )?;
                } else if method == Some("textDocument/completion") {
                    log("Responding to completion");
                    send(
                        &mut stdout,
                        json!({
                            "jsonrpc": "2.0",
                            "id": id,
                            "result": [
                                { "label": "mock_field", "detail": "mock" }
                            ]
                        }),
                    )?;
                } else if method == Some("shutdown") {
                    log("Responding to shutdown");
                    send(
                        &mut stdout,
                        json!({
                            "jsonrpc": "2.0",
                            "id": id,
                            "result": null
                        }),
                    )?;
                }
            } else {
                let method = msg.get("method").and_then(|m| m.as_str());
                if method == Some("exit") {
                    log("Exiting");
                    return Ok(());
                }
            }
        }
    }
}

fn send(stdout: &mut io::Stdout, msg: Value) -> io::Result<()> {
    let body = serde_json::to_string(&msg).unwrap();
    log(&format!("Sending: {}", body));
    let out = format!("Content-Length: {}\r\n\r\n{}", body.len(), body);
    stdout.write_all(out.as_bytes())?;
    stdout.flush()
}
