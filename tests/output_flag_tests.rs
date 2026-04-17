use std::io::Write;
use std::process::{Command, Stdio};

fn run_with_input(input: &[u8], args: &[&str]) -> std::process::Output {
    let mut child = Command::new("target/debug/jqpp")
        .args(args)
        .env("JQPP_SKIP_TTY_CHECK", "1")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to spawn target/debug/jqpp - run `cargo build` first");

    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(input);
    }

    child.wait_with_output().expect("wait_with_output failed")
}

#[test]
fn print_output_outputs_current_result_on_exit() {
    let output = run_with_input(br#"{"name":"alice","age":30}"#, &["--print-output"]);
    assert!(
        output.status.success(),
        "process failed: {:?}",
        output.status
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let line = stdout.trim();
    let parsed: serde_json::Value = serde_json::from_str(line).expect("stdout was not JSON");
    assert_eq!(parsed["name"], "alice");
    assert_eq!(parsed["age"], 30);
}

#[test]
fn print_query_outputs_query_string_on_exit() {
    let output = run_with_input(br#"{"x":1}"#, &["--print-query"]);
    assert!(
        output.status.success(),
        "process failed: {:?}",
        output.status
    );
    assert_eq!(output.stdout, b"\n");
}

#[test]
fn print_flags_are_mutually_exclusive() {
    let output = run_with_input(br#"{"x":1}"#, &["--print-output", "--print-query"]);

    assert!(
        !output.status.success(),
        "expected non-zero exit with conflicting flags"
    );
    assert!(
        output.stdout.is_empty(),
        "stdout must be empty on clap usage error"
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("--print-output") && stderr.contains("--print-query"));
}
