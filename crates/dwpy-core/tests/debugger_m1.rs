use dwpy_core::{
    debug::{DebugCommand, DebugEvent, DebugSession},
    execute_value_with_context, DwValue, ExecutionContext, ExecutionOptions, LogEntry,
};
use serde_json::json;
use std::sync::{Arc, Mutex};
fn compare(source: &str, payload: serde_json::Value) {
    let normal_logs = Arc::new(Mutex::new(vec![]));
    let sink = normal_logs.clone();
    let normal = execute_value_with_context(
        source,
        payload.clone().into(),
        ExecutionContext::default()
            .with_logger(move |e: &LogEntry| sink.lock().unwrap().push(e.clone())),
        true,
    );
    let logs = Arc::new(Mutex::new(vec![]));
    let sink = logs.clone();
    let mut session = DebugSession::new(
        source,
        payload.into(),
        ExecutionContext::default()
            .with_logger(move |e: &LogEntry| sink.lock().unwrap().push(e.clone())),
        ExecutionOptions::default(),
    )
    .unwrap_or_else(|e| panic!("preflight {source}: {e}"));
    session.command(DebugCommand::Continue).unwrap();
    let mut done = false;
    for _ in 0..10000 {
        if session.advance(7) == DebugEvent::Completed {
            done = true;
            break;
        }
    }
    assert!(done, "did not finish: {source}");
    match normal {
        Ok(value) => {
            assert!(
                session.exception.is_none(),
                "{source}: {:?}",
                session.exception
            );
            assert_eq!(session.result, Some(value), "{source}");
        }
        Err(e) => assert_eq!(session.exception, Some(e.to_string()), "{source}"),
    }
    assert_eq!(
        *logs.lock().unwrap(),
        *normal_logs.lock().unwrap(),
        "effect order: {source}"
    );
}
#[test]
fn grammar_parity_matrix() {
    let sources=[
        r#""outer $("inner $(upper("x)"))")""#,
        r#"%dw 2.0
var text="outer $("inner $(upper("x),y//"))")"
var next=2
---
{value:text, next:next}"#,
        r#"{a:"outer $("inner $(upper("x),y//"))")", b:2}"#,
        "%dw 2.0\ntype Moment=Null | Time\n---\nnull is Moment",
        "\"b\" match {case captures matches /a|b/ -> captures[0] else -> \"no\"}",
        "null match {case is Number | Null -> 3 else -> 4}",
        "%dw 2.0\nfun label(m)=upper(m[0])\n---\n\"a1b2\" replace /(\\d+)/ with label",
        "((n:Number)->n)(\"bad\")",
        "((n)->n)(1,2)",
        "((n)->n)()",
        "2 match {case 1 -> 10 case x:2 if (x==2) -> x+3 else -> 0}",
        "3 match {case n is String -> 0 case n is Number when n>1 -> n*2 else -> 8}",
        "null match {case is Null -> 1 else -> 0}",
        "using(n=2, limit=4) n match {case v if(v<limit) -> v+1 else -> 0}",
        "using(n=2, limit=4) n match {case v if(v<limit and v>0) -> v+1 else -> 0}",
        "{a:2} match {case var obj -> obj.a}",
        "4 match {case 2 -> 1}",
        "\"abc-12\" match {case groups matches /([a-z]+)-(\\d+)/ -> groups[1] ++ groups[2] else -> \"no\"}",
        "%dw 2.0\nvar n=9\nfun twice(x)=x*2\n---\n[1,2] map ((item)->item match {case n if(log(n>1)) -> twice(n) else -> n})",
        "using (x=log(1), y=log(x+1)) {value:y, nested:using(x=5) x, outer:x}",
        "{(log(\"a\")):log(2), (log(\"b\")):log(3)}",
        "{((log(\"key\")):log(1)) if(log(false)), a:log(2)}",
        "{({a:1,a:2}), a:3, ([{b:4},{b:5}])}",
        "{(log({a:1})) if(log(true)), b:2}",
        "%dw 2.0\nvar key=\"a\"\n---\n{(key):2, (\"$(upper(key))\"):3}",
        "\"🐈 $(log(2+3)) / $(upper(\"yes\"))\"",
        "\"a\" ++ \"b\"",
        "%dw 2.0\nvar key=2\n---\n\"$(using(x=key) x+1) $(if(true) \"yes\" else \"no\")\"",
        "\"value $(1 /* 🐈 */ + 2)\"",
        "%dw 2.0\nfun identity<T>(value:T)=value\n---\n[identity<Number>(3),identity<String>(\"x\")]",
        "%dw 2.0\nfun count<T>(n:T)=if(n<=0) 0 else 1+count<T>(n-1)\n---\ncount(4)",
        "%dw 2.0\ntype Both=String | Number\n---\n[1 is Both, null is Both, [] is Array<Number>, {a:1} is Object {a:Number}]",
        "[1,2] as Array<Number>",
        "12.3 as String {format: \"0.00\"}",
        "\"2020-01-02\" as Date {format: \"yyyy-MM-dd\"} as String",
        "{x:1} as String {class: \"custom\"}",
        "[1,2,3][?($>1)]",
        "{a:1,b:2}[?($>1)]",
        "3[?($>1)]",
        "null[?(log(true))]",
        "[][?(log(true))]",
        "[1,2,3][?(log($>5))]",
        "%dw 2.0\nvar key=\"name\"\n---\n[payload.users[*(key)],payload.users[&(key)]]",
        "read(\"<root><name>a</name><name>b</name></root>\",\"application/xml\").root[?($==\"b\")]",
        "((x)->x+1)(3)",
        "%dw 2.0\nvar make=(x)->(y)->x+y\n---\nmake(log(3))(log(4))",
        "[1,2] map ((x)->((y)->x+y)(3))",
        "\"abc123abc\" replace \"abc\" with(log(\"X\"))",
        "replace(\"a1b22\", /(\\d+)/) with ((groups,index)->log(groups[1] ++ (index as String)))",
        "\"abc abc\" replace \"abc\" with ((m)->upper(m[0]))",
        "null replace /x/ with ((m)->log(m[0]))",
        "\"abc\" replace /x/ with ((m)->log(m[0]))",
        "{a: (using (n=2) n match {case v -> \"x$(v)\"}), b: ((x)->{(x):2})(\"k\")}",
    ];
    for source in sources {
        compare(source, json!({"users":[{"name":"a"},{"name":"b"}]}));
    }
}

#[test]
fn nested_grammar_steps_into_calls_restores_scope_and_keeps_original_ranges() {
    let source="%dw 2.0\n/* 🐈 --- */\nvar x=99\nfun inc(n)=n+1\n---\n{a:using(x=2) x match {case n if(n>1) -> \"🐈 $(inc(n))\" else -> \"no\"}, b:x}";
    let mut session = DebugSession::new(
        source,
        DwValue::Null,
        ExecutionContext::default(),
        ExecutionOptions {
            render_output: false,
            ..ExecutionOptions::default()
        },
    )
    .unwrap();
    let range = session
        .locations()
        .into_iter()
        .find(|r| &source[r.start..r.end] == "inc(n)")
        .unwrap();
    assert_eq!(dwpy_core::debug::position(source, range.start).0, 6);
    session.set_breakpoints(vec![range]);
    session.command(DebugCommand::Continue).unwrap();
    loop {
        if session.advance(3) == DebugEvent::Stopped("breakpoint") {
            break;
        }
    }
    let frame = session.frames().last().unwrap();
    assert_eq!(
        session.evaluate("n+x", frame.id).unwrap().data().unwrap(),
        DwValue::from(json!(4))
    );
    assert!(session.evaluate("\"$(log(n))\"", frame.id).is_err());
    assert!(session.evaluate("[n][?($>1)]", frame.id).is_err());
    session.command(DebugCommand::Into).unwrap();
    loop {
        if matches!(session.advance(3), DebugEvent::Stopped(_)) {
            break;
        }
    }
    assert_eq!(session.frames().last().unwrap().name, "inc");
    session.command(DebugCommand::Out).unwrap();
    loop {
        if matches!(session.advance(3), DebugEvent::Stopped(_)) {
            break;
        }
    }
    assert_eq!(session.frames().last().unwrap().name, "script");
    assert_eq!(
        session
            .frames()
            .last()
            .unwrap()
            .last
            .as_ref()
            .unwrap()
            .data()
            .unwrap(),
        DwValue::from(json!(3))
    );
    session.command(DebugCommand::Continue).unwrap();
    while session.advance(5) != DebugEvent::Completed {}
    assert_eq!(
        session.result,
        Some(DwValue::from(json!({"a":"🐈 3","b":99})))
    );
}

#[test]
fn filter_selector_breakpoints_repeat_without_replay_and_pause_is_cooperative() {
    let source = "payload[?(log($ > 1))]";
    let logs = Arc::new(Mutex::new(vec![]));
    let sink = logs.clone();
    let mut session = DebugSession::new(
        source,
        json!([1, 2, 3]).into(),
        ExecutionContext::default()
            .with_logger(move |e: &LogEntry| sink.lock().unwrap().push(e.clone())),
        ExecutionOptions::default(),
    )
    .unwrap();
    let range = session
        .locations()
        .into_iter()
        .find(|r| &source[r.start..r.end] == "log($ > 1)")
        .unwrap();
    session.set_breakpoints(vec![range]);
    session.command(DebugCommand::Continue).unwrap();
    let mut values = vec![];
    for _ in 0..1000 {
        match session.advance(1) {
            DebugEvent::Stopped("breakpoint") => {
                let f = session.frames().last().unwrap();
                values.push(session.evaluate("$", f.id).unwrap().data().unwrap());
                session.command(DebugCommand::Continue).unwrap();
            }
            DebugEvent::Completed => break,
            _ => (),
        }
    }
    assert_eq!(
        values,
        vec![1, 2, 3]
            .into_iter()
            .map(|n| DwValue::from(json!(n)))
            .collect::<Vec<_>>()
    );
    assert_eq!(logs.lock().unwrap().len(), 3);
    assert_eq!(session.result, Some(json!([2, 3]).into()));
    let mut session = DebugSession::new(
        "payload[?($ > 1)]",
        json!((0..10000).collect::<Vec<_>>()).into(),
        ExecutionContext::default(),
        ExecutionOptions::default(),
    )
    .unwrap();
    session.command(DebugCommand::Continue).unwrap();
    assert_eq!(session.advance(20), DebugEvent::Yielded);
    session.command(DebugCommand::Pause).unwrap();
    assert_eq!(session.advance(1), DebugEvent::Stopped("pause"));
    session.command(DebugCommand::Terminate).unwrap();
    assert_eq!(session.advance(1), DebugEvent::Terminated);
}

#[test]
fn unsupported_nested_features_and_engine_only_update_paths_fail_before_debug_effects() {
    for body in [
        "using(x=2) try(() -> payload)",
        "payload match {case x -> try(() -> x)}",
        "\"$(try(() -> payload))\"",
        "{(\"a\"):try(() -> payload)}",
        "((x)->x)(try(() -> payload))",
        "payload[?(try(() -> payload))]",
        "payload update {case ..a -> 1}",
        "payload update {case .^class -> 1}",
        "payload update {case .&a -> 1}",
    ] {
        let source = format!("%dw 2.0\nvar before=log(1)\n---\n{body}");
        let logs = Arc::new(Mutex::new(vec![]));
        let sink = logs.clone();
        let error = match DebugSession::new(
            &source,
            json!({"a":2}).into(),
            ExecutionContext::default()
                .with_logger(move |e: &LogEntry| sink.lock().unwrap().push(e.clone())),
            ExecutionOptions::default(),
        ) {
            Ok(_) => panic!("accepted {body}"),
            Err(e) => e.to_string(),
        };
        assert!(error.contains("at 4:"), "{body}: {error}");
        assert!(logs.lock().unwrap().is_empty());
    }
    for source in [
        "{a:2} update {case ..a -> 3}",
        "{a:2} update {case .^class -> 3}",
        "{a:2} update {case .&a -> 3}",
    ] {
        assert!(execute_value_with_context(
            source,
            DwValue::Null,
            ExecutionContext::default(),
            true
        )
        .unwrap_err()
        .to_string()
        .contains("only property and index paths"));
    }
}
