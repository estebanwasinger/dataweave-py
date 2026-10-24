use serde_json::Value;
use std::{
    io::{BufRead, BufReader, Write},
    process::{Command, Stdio},
};
fn read(reader: &mut impl BufRead) -> Value {
    let mut length = 0;
    loop {
        let mut line = String::new();
        assert!(
            reader.read_line(&mut line).unwrap() > 0,
            "adapter closed stdout"
        );
        if line == "\r\n" {
            break;
        }
        if let Some(l) = line.strip_prefix("Content-Length: ") {
            length = l.trim().parse().unwrap();
        }
    }
    let mut bytes = vec![0; length];
    reader.read_exact(&mut bytes).unwrap();
    serde_json::from_slice(&bytes).unwrap()
}
#[test]
fn shared_dap_transcript_over_stdio() {
    run_transcript(
        include_str!("../../../tests/fixtures/debugger/session.json"),
        serde_json::json!([2, 4, 6]),
    );
}
#[test]
fn m1_grammar_transcript_over_stdio() {
    run_transcript(
        include_str!("../../../tests/fixtures/debugger/m1.json"),
        serde_json::json!({"result":{"k2":"3"},"filtered":[2,3],"replaced":"a10b221","closure":5}),
    );
}
#[test]
fn m2_callbacks_transcript_over_stdio() {
    run_transcript(
        include_str!("../../../tests/fixtures/debugger/m2.json"),
        serde_json::from_str(include_str!(
            "../../../tests/fixtures/debugger/m2.expected.json"
        ))
        .unwrap(),
    );
}
fn run_transcript(source: &str, expected: Value) {
    let requests: Vec<Value> = serde_json::from_str(source).unwrap();
    let mut child = Command::new(env!("CARGO_BIN_EXE_dw"))
        .arg("dap")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .unwrap();
    let mut stdin = child.stdin.take().unwrap();
    let mut stdout = BufReader::new(child.stdout.take().unwrap());
    for request in requests {
        let bytes = serde_json::to_vec(&request).unwrap();
        write!(stdin, "Content-Length: {}\r\n\r\n", bytes.len()).unwrap();
        stdin.write_all(&bytes).unwrap();
        stdin.flush().unwrap();
        loop {
            let message = read(&mut stdout);
            if message["type"] == "response" && message["request_seq"] == request["seq"] {
                assert_eq!(message["success"], true, "{message}");
                if request["command"] == "evaluate" {
                    assert_eq!(message["body"]["result"], "2");
                }
                break;
            }
        }
        if request["command"] == "configurationDone" || request["command"] == "stepIn" {
            loop {
                let message = read(&mut stdout);
                if message["event"] == "stopped" {
                    break;
                }
                assert_ne!(message["event"], "terminated");
            }
        }
        if request["command"] == "continue" {
            let mut result = None;
            loop {
                let message = read(&mut stdout);
                if message["body"]["data"]["kind"] == "result" {
                    result = Some(message["body"]["data"]["result"].clone());
                }
                if message["event"] == "terminated" {
                    break;
                }
            }
            assert_eq!(
                serde_json::from_str::<Value>(result.unwrap().as_str().unwrap()).unwrap(),
                expected
            );
        }
    }
    drop(stdin);
    assert!(child.wait().unwrap().success());
}
