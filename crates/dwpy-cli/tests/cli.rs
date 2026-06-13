use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn temp_file(name: &str, suffix: &str, contents: &str) -> PathBuf {
    let mut path = env::temp_dir();
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time before unix epoch")
        .as_nanos();
    path.push(format!("dwpy-cli-{name}-{nanos}.{suffix}"));
    fs::write(&path, contents).expect("failed to write temp file");
    path
}

#[test]
fn runs_inline_script_and_pretty_prints_native_values() {
    let output = Command::new(env!("CARGO_BIN_EXE_dw"))
        .args([
            "run",
            "%dw 2.0\noutput application/python\n---\n{message: upper(payload.name)}",
            "--payload",
            r#"{"name":"dw"}"#,
        ])
        .output()
        .expect("failed to run dw binary");

    assert!(output.status.success(), "stderr: {}", String::from_utf8_lossy(&output.stderr));
    assert_eq!(
        String::from_utf8_lossy(&output.stdout),
        "{\n  \"message\": \"DW\"\n}\n"
    );
    assert!(output.stderr.is_empty());
}

#[test]
fn runs_script_file_with_inferred_json_payload_file_format() {
    let script_path = temp_file(
        "script",
        "dwl",
        "%dw 2.0\noutput application/json\n---\npayload.user",
    );
    let payload_path = temp_file("payload", "json", r#"{"user":{"name":"Ana"}}"#);

    let output = Command::new(env!("CARGO_BIN_EXE_dw"))
        .args([
            "run",
            "--file",
            script_path.to_str().expect("non-utf8 path"),
            "--payload-file",
            payload_path.to_str().expect("non-utf8 path"),
        ])
        .output()
        .expect("failed to run dw binary");

    assert!(output.status.success(), "stderr: {}", String::from_utf8_lossy(&output.stderr));
    assert_eq!(String::from_utf8_lossy(&output.stdout), "{\"name\":\"Ana\"}");
    assert!(output.stderr.is_empty());

    let _ = fs::remove_file(script_path);
    let _ = fs::remove_file(payload_path);
}

#[test]
fn prints_rendered_text_output_as_is() {
    let output = Command::new(env!("CARGO_BIN_EXE_dw"))
        .args([
            "run",
            "%dw 2.0\noutput text/plain\n---\nupper(payload.name)",
            "--payload",
            r#"{"name":"dw"}"#,
        ])
        .output()
        .expect("failed to run dw binary");

    assert!(output.status.success(), "stderr: {}", String::from_utf8_lossy(&output.stderr));
    assert_eq!(String::from_utf8_lossy(&output.stdout), "DW");
    assert!(output.stderr.is_empty());
}

#[test]
fn rejects_invalid_argument_combinations() {
    let output = Command::new(env!("CARGO_BIN_EXE_dw"))
        .args(["run", "payload", "--file", "script.dwl"])
        .output()
        .expect("failed to run dw binary");

    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("provide either an inline script"));
}

#[test]
fn rejects_unsupported_features_in_strict_rust_mode() {
    let output = Command::new(env!("CARGO_BIN_EXE_dw"))
        .args([
            "run",
            "%dw 2.0\noutput application/json\n---\npayload",
            "--payload",
            r#"{"name":"dw"}"#,
            "--payload-format",
            "application/avro",
        ])
        .output()
        .expect("failed to run dw binary");

    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("does not support") || stderr.contains("payload format"));
}
