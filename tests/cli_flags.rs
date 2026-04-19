use std::fs;
use std::process::Command;
use tempfile::tempdir;

#[test]
fn test_query_flag_sets_initial_query_in_headless_mode() {
    let dir = tempdir().unwrap();
    let input_path = dir.path().join("test.json");
    fs::write(&input_path, r#"{"name": "alice"}"#).unwrap();

    let output = Command::new("target/debug/jqpp")
        .arg(input_path)
        .arg("--query")
        .arg(".name")
        .arg("--print-query")
        .env("JQPP_SKIP_TTY_CHECK", "1")
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), ".name");
}

#[test]
fn test_query_flag_with_print_output_evaluates_query() {
    let dir = tempdir().unwrap();
    let input_path = dir.path().join("test.json");
    fs::write(&input_path, r#"{"name": "alice"}"#).unwrap();

    let output = Command::new("target/debug/jqpp")
        .arg(input_path)
        .arg("--query")
        .arg(".name")
        .arg("--print-output")
        .env("JQPP_SKIP_TTY_CHECK", "1")
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "\"alice\"");
}

#[test]
fn test_empty_query_flag_produces_empty_print_query() {
    let dir = tempdir().unwrap();
    let input_path = dir.path().join("test.json");
    fs::write(&input_path, r#"{"name": "alice"}"#).unwrap();

    let output = Command::new("target/debug/jqpp")
        .arg(input_path)
        .arg("--query")
        .arg("")
        .arg("--print-query")
        .env("JQPP_SKIP_TTY_CHECK", "1")
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "");
}

#[test]
fn test_no_query_flag_leaves_bar_empty() {
    let dir = tempdir().unwrap();
    let input_path = dir.path().join("test.json");
    fs::write(&input_path, r#"{"name": "alice"}"#).unwrap();

    let output = Command::new("target/debug/jqpp")
        .arg(input_path)
        .arg("--print-query")
        .env("JQPP_SKIP_TTY_CHECK", "1")
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "");
}

#[test]
fn test_two_files_produce_merged_array() {
    let dir = tempdir().unwrap();
    let f1 = dir.path().join("1.json");
    let f2 = dir.path().join("2.json");
    fs::write(&f1, r#"{"a": 1}"#).unwrap();
    fs::write(&f2, r#"{"b": 2}"#).unwrap();

    let output = Command::new("target/debug/jqpp")
        .arg(f1)
        .arg(f2)
        .arg("--print-input")
        .env("JQPP_SKIP_TTY_CHECK", "1")
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());
    let val: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(val, serde_json::json!([{"a": 1}, {"b": 2}]));
}

#[test]
fn test_two_files_default_query_is_dot_slice() {
    let dir = tempdir().unwrap();
    let f1 = dir.path().join("1.json");
    let f2 = dir.path().join("2.json");
    fs::write(&f1, r#"1"#).unwrap();
    fs::write(&f2, r#"2"#).unwrap();

    let output = Command::new("target/debug/jqpp")
        .arg(f1)
        .arg(f2)
        .arg("--print-query")
        .env("JQPP_SKIP_TTY_CHECK", "1")
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), ".[]");
}

#[test]
fn test_two_files_explicit_query_overrides_default() {
    let dir = tempdir().unwrap();
    let f1 = dir.path().join("1.json");
    let f2 = dir.path().join("2.json");
    fs::write(&f1, r#"1"#).unwrap();
    fs::write(&f2, r#"2"#).unwrap();

    let output = Command::new("target/debug/jqpp")
        .arg(f1)
        .arg(f2)
        .arg("--query")
        .arg(".[] | .id")
        .arg("--print-query")
        .env("JQPP_SKIP_TTY_CHECK", "1")
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), ".[] | .id");
}

#[test]
fn test_single_file_not_affected() {
    let dir = tempdir().unwrap();
    let f1 = dir.path().join("1.json");
    fs::write(&f1, r#"{"a": 1}"#).unwrap();

    let output = Command::new("target/debug/jqpp")
        .arg(f1)
        .arg("--print-input")
        .env("JQPP_SKIP_TTY_CHECK", "1")
        .output()
        .expect("failed to execute process");

    assert!(output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).trim(),
        r#"{"a": 1}"#
    );
}

#[test]
fn test_negative_cursor_resolves_from_end() {
    let dir = tempdir().unwrap();
    let f1 = dir.path().join("1.json");
    fs::write(&f1, r#"{"price": 10}"#).unwrap();

    let output = Command::new("target/debug/jqpp")
        .arg(f1)
        .arg("--query")
        .arg("sort_by(.price)")
        .arg("--cursor")
        .arg("-7")
        .arg("--print-query")
        .env("JQPP_SKIP_TTY_CHECK", "1")
        .output()
        .expect("failed to execute process");

    // We just want to check it doesn't fail due to clap treating -7 as a flag
    assert!(output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).trim(),
        "sort_by(.price)"
    );
}

#[test]
fn test_missing_file_in_multi_file_list_exits_nonzero() {
    let dir = tempdir().unwrap();
    let f1 = dir.path().join("1.json");
    fs::write(&f1, r#"1"#).unwrap();
    let f2 = dir.path().join("missing.json");

    let output = Command::new("target/debug/jqpp")
        .arg(f1)
        .arg(f2)
        .env("JQPP_SKIP_TTY_CHECK", "1")
        .output()
        .expect("failed to execute process");

    assert!(!output.status.success());
}
