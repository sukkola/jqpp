use std::env;
use std::fs;
use std::io::{Read, Write};
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

// ── helper: JSON-RPC framing ──────────────────────────────────────────────────

fn send_rpc(stdin: &mut impl Write, msg: &serde_json::Value) {
    let body = serde_json::to_string(msg).unwrap();
    write!(stdin, "Content-Length: {}\r\n\r\n{}", body.len(), body).unwrap();
    stdin.flush().unwrap();
}

fn recv_rpc(stdout: &mut impl Read) -> serde_json::Value {
    // Read header byte-by-byte until "\r\n\r\n"
    let mut header = String::new();
    let mut buf = [0u8; 1];
    loop {
        stdout.read_exact(&mut buf).expect("EOF reading LSP header");
        header.push(buf[0] as char);
        if header.ends_with("\r\n\r\n") {
            break;
        }
    }
    let len: usize = header
        .lines()
        .find(|l| l.starts_with("Content-Length:"))
        .expect("No Content-Length header")
        .split(':')
        .nth(1)
        .unwrap()
        .trim()
        .parse()
        .unwrap();
    let mut body = vec![0u8; len];
    stdout.read_exact(&mut body).expect("EOF reading LSP body");
    serde_json::from_slice(&body).expect("Invalid JSON in LSP response")
}

// ── test: initialize handshake via jqpp process ───────────────────────────────

#[test]
fn test_lsp_integration() {
    // 1. Build everything
    let _ = Command::new("cargo")
        .args(["build", "--bin", "jqpp", "--bin", "mock_lsp"])
        .status();

    let root = env::current_dir().unwrap();
    let mock_lsp_path = root.join("target/debug/mock_lsp");
    let log_path = root.join("mock_lsp.log");

    if log_path.exists() {
        let _ = fs::remove_file(&log_path);
    }

    // 2. Run jqpp with mock_lsp
    let mut child = Command::new(root.join("target/debug/jqpp"))
        .env("JQPP_LSP_BIN", &mock_lsp_path)
        .env("JQPP_SKIP_TTY_CHECK", "1")
        .env("MOCK_LSP_LOG", &log_path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to spawn jqpp");

    let mut stdin = child.stdin.take().expect("Failed to open stdin");
    stdin.write_all(b"{}").expect("Failed to write to stdin");
    drop(stdin);

    // Wait for LSP handshake
    thread::sleep(Duration::from_millis(2000));

    // Kill jqpp
    let _ = child.kill();
    let output = child.wait_with_output().expect("Failed to wait for jqpp");

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    // 3. Check logs
    if let Ok(log_content) = fs::read_to_string(&log_path) {
        assert!(
            log_content.contains("Mock LSP started"),
            "Log should contain 'Mock LSP started'. Content: {}",
            log_content
        );
        assert!(
            log_content.contains("Responding to initialize"),
            "Log should contain 'Responding to initialize'. Content: {}",
            log_content
        );
    } else {
        panic!(
            "mock_lsp.log was not created at {:?}.\nSTDERR: {}\nSTDOUT: {}",
            log_path, stderr, stdout
        );
    }
}

// ── test: completion round-trip directly with mock_lsp ────────────────────────

/// Spawn `mock_lsp` directly, send an initialize + completion request over
/// JSON-RPC pipes, and assert we receive a valid completion response containing
/// the expected item label.  This exercises the full protocol framing without
/// needing a real TTY.
#[test]
fn test_lsp_completion_round_trip() {
    let _ = Command::new("cargo")
        .args(["build", "--bin", "mock_lsp"])
        .status();

    let root = env::current_dir().unwrap();
    let mock_lsp_path = root.join("target/debug/mock_lsp");

    let log_path = root.join("mock_lsp_completion_test.log");
    if log_path.exists() {
        let _ = fs::remove_file(&log_path);
    }

    let mut child = Command::new(&mock_lsp_path)
        .env("MOCK_LSP_LOG", &log_path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("Failed to spawn mock_lsp");

    let mut stdin = child.stdin.take().expect("stdin");
    let mut stdout = child.stdout.take().expect("stdout");

    // ── 1. initialize ─────────────────────────────────────────────────────────
    send_rpc(
        &mut stdin,
        &serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "processId": null,
                "rootUri": null,
                "capabilities": {}
            }
        }),
    );

    let init_resp = recv_rpc(&mut stdout);
    assert_eq!(
        init_resp["id"], 1,
        "initialize response id mismatch: {init_resp}"
    );
    assert!(
        init_resp.get("result").is_some(),
        "initialize response missing 'result': {init_resp}"
    );

    // ── 2. textDocument/completion ────────────────────────────────────────────
    send_rpc(
        &mut stdin,
        &serde_json::json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "textDocument/completion",
            "params": {
                "textDocument": { "uri": "file:///query.jq" },
                "position": { "line": 0, "character": 1 }
            }
        }),
    );

    let comp_resp = recv_rpc(&mut stdout);
    assert_eq!(
        comp_resp["id"], 2,
        "completion response id mismatch: {comp_resp}"
    );

    // The result is an array of completion items (or an object with 'items').
    let items = comp_resp["result"]
        .as_array()
        .or_else(|| comp_resp["result"]["items"].as_array())
        .expect("completion result should be an array or object with 'items'");

    assert!(
        !items.is_empty(),
        "Expected at least one completion item, got none"
    );

    let first_label = items[0]["label"]
        .as_str()
        .expect("first completion item missing 'label'");
    assert_eq!(
        first_label, "mock_field",
        "Unexpected completion label: {first_label}"
    );

    // ── 3. shutdown / exit ────────────────────────────────────────────────────
    send_rpc(
        &mut stdin,
        &serde_json::json!({
            "jsonrpc": "2.0",
            "id": 3,
            "method": "shutdown"
        }),
    );

    let shutdown_resp = recv_rpc(&mut stdout);
    assert_eq!(shutdown_resp["id"], 3, "shutdown response id mismatch");

    send_rpc(
        &mut stdin,
        &serde_json::json!({
            "jsonrpc": "2.0",
            "method": "exit"
        }),
    );

    drop(stdin);
    let status = child.wait().expect("Failed to wait for mock_lsp");
    assert!(status.success(), "mock_lsp exited with non-zero status");

    // Verify the log recorded all expected interactions
    let log = fs::read_to_string(&log_path).unwrap_or_default();
    assert!(
        log.contains("Responding to completion"),
        "Log should contain 'Responding to completion'. Content: {log}"
    );
    assert!(
        log.contains("Exiting"),
        "Log should contain 'Exiting'. Content: {log}"
    );
}
