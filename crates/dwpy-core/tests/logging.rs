use std::sync::{Arc, Mutex};

use dwpy_core::{compile, execute_json, ExecutionContext, ExecutionOptions, LogEntry, LogLevel};
use serde_json::{json, Value};

fn capture() -> (ExecutionContext, Arc<Mutex<Vec<LogEntry>>>) {
    let entries = Arc::new(Mutex::new(Vec::new()));
    let sink = entries.clone();
    let context = ExecutionContext::new(None, None, None)
        .unwrap()
        .with_logger(move |entry: &LogEntry| sink.lock().unwrap().push(entry.clone()));
    (context, entries)
}

#[test]
fn map_returns_values_and_fans_out_logs_in_order() {
    let (context, first) = capture();
    let second = Arc::new(Mutex::new(Vec::new()));
    let sink = second.clone();
    let context =
        context.with_logger(move |entry: &LogEntry| sink.lock().unwrap().push(entry.clone()));
    let script = compile("%dw 2.0\noutput application/json\n---\n1 to 10 map log($)").unwrap();
    let result = script
        .execute_with_context(Value::Null, context, &ExecutionOptions::default())
        .unwrap();
    assert_eq!(result, json!("[1,2,3,4,5,6,7,8,9,10]"));
    let entries = first.lock().unwrap();
    assert_eq!(entries.len(), 10);
    assert_eq!(*entries, *second.lock().unwrap());
    assert_eq!(
        entries
            .iter()
            .map(|entry| entry.message.clone())
            .collect::<Vec<_>>(),
        (1..=10).map(|n| n.to_string()).collect::<Vec<_>>()
    );
    assert!(entries
        .iter()
        .all(|entry| entry.level == LogLevel::Warn && entry.prefix.is_none()));
}

#[test]
fn nested_logs_evaluate_prefix_and_value_once_with_all_levels() {
    let (context, entries) = capture();
    let script = compile(
        r#"%dw 2.0
var initial = logDebug("header", 1)
fun inspect(value) = do {
    var seen = logInfo(/* a --- boundary */ "function", value)
    ---
    logError(log("prefix", "outer"), logWarn("inner", seen))
}
---
[inspect(initial), logWith("DEBUG", "custom", null)]"#,
    )
    .unwrap();
    assert_eq!(
        script
            .execute_with_context(
                Value::Null,
                context,
                &ExecutionOptions {
                    render_output: false,
                    ..Default::default()
                }
            )
            .unwrap(),
        json!([1, null])
    );
    let entries = entries.lock().unwrap();
    assert_eq!(
        entries
            .iter()
            .map(|entry| (entry.level, entry.message.as_str()))
            .collect::<Vec<_>>(),
        vec![
            (LogLevel::Debug, "header - 1"),
            (LogLevel::Info, "function - 1"),
            (LogLevel::Warn, "prefix - outer"),
            (LogLevel::Warn, "inner - 1"),
            (LogLevel::Error, "outer - 1"),
            (LogLevel::Debug, "custom - null"),
        ]
    );
}

#[test]
fn writer_and_eager_execution_emit_each_message_once() {
    for lazy_sequences in [true, false] {
        let (context, entries) = capture();
        let mut writer = Vec::new();
        compile("%dw 2.0\noutput application/json\n---\n1 to 3 map log($)")
            .unwrap()
            .execute_to_writer_with_context(
                Value::Null,
                context,
                &ExecutionOptions {
                    lazy_sequences,
                    ..Default::default()
                },
                &mut writer,
            )
            .unwrap();
        assert_eq!(String::from_utf8(writer).unwrap(), "[1,2,3]");
        assert_eq!(entries.lock().unwrap().len(), 3);
    }
}

#[test]
fn scopes_restore_after_errors_and_reentrant_execution() {
    let (inner_context, inner) = capture();
    let (outer_context, outer) = capture();
    let context = outer_context.with_logger(move |_: &LogEntry| {
        compile("logInfo(\"inner\")")
            .unwrap()
            .execute_with_context(
                Value::Null,
                inner_context.clone(),
                &ExecutionOptions::default(),
            )
            .unwrap();
        // A nested execution with no sinks must not inherit the outer sinks.
        execute_json("log(99)", Value::Null, false).unwrap();
    });
    assert!(compile("[log(1), fail(\"boom\")]")
        .unwrap()
        .execute_with_context(Value::Null, context, &ExecutionOptions::default())
        .is_err());
    assert_eq!(outer.lock().unwrap().len(), 1);
    assert_eq!(inner.lock().unwrap().len(), 1);
    execute_json("log(2)", Value::Null, false).unwrap();
    assert_eq!(outer.lock().unwrap().len(), 1);
}

#[test]
fn simultaneous_executions_have_separate_sinks() {
    let threads = (0..4)
        .map(|id| {
            std::thread::spawn(move || {
                let (context, entries) = capture();
                compile("1 to 5 map log($)")
                    .unwrap()
                    .execute_with_context(json!(id), context, &ExecutionOptions::default())
                    .unwrap();
                assert_eq!(entries.lock().unwrap().len(), 5);
            })
        })
        .collect::<Vec<_>>();
    for thread in threads {
        thread.join().unwrap();
    }
}

#[test]
fn logging_preserves_return_type_and_reports_invalid_calls() {
    assert_eq!(
        dwpy_core::infer_type_descriptor("logInfo(\"label\", payload)", Some(json!(42)), None)
            .unwrap()["kind"],
        "Number"
    );
    let (context, entries) = capture();
    assert!(compile("log()")
        .unwrap()
        .execute_with_context(Value::Null, context, &ExecutionOptions::default())
        .is_err());
    assert!(entries.lock().unwrap().is_empty());
}

#[test]
fn compact_json_rendering_never_replays_logs_on_failure_or_fallback() {
    for body in [
        "[logInfo(\"before\", 1), fail(\"boom\")]",
        "{first: log(1), second: fail(\"boom\")}",
        "{first: log(1), (payload)}",
        "{first: log(1), nested: {(payload)}}",
        "({first: log(1), (payload)})",
        "([{first: log(1), (payload)}])",
        "[log(1), (2) if false]",
    ] {
        let (context, entries) = capture();
        let script = compile(&format!("%dw 2.0\noutput application/json\n---\n{body}")).unwrap();
        let result =
            script.execute_with_context(json!({"other": 2}), context, &ExecutionOptions::default());
        if body.contains("fail(") {
            assert!(result.is_err());
        } else {
            assert!(result.is_ok(), "{body}: {result:?}");
        }
        assert_eq!(entries.lock().unwrap().len(), 1, "{body}");
    }
    let (context, entries) = capture();
    let script = compile("%dw 2.0\noutput application/json\n---\n{a: log(1)\n b: log(2)}").unwrap();
    assert_eq!(
        script
            .execute_with_context(Value::Null, context, &ExecutionOptions::default())
            .unwrap(),
        json!("{\"a\":1,\"b\":2}")
    );
    assert_eq!(entries.lock().unwrap().len(), 2);
}
