use dwpy_core::{
    debug::{DebugCommand, DebugEvent, DebugSession},
    execute_value_with_context, DwValue, ExecutionContext, ExecutionOptions, LogEntry,
};
use serde_json::json;
use std::sync::{Arc, Mutex};
fn compare(source: &str, payload: serde_json::Value) -> bool {
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
    for _ in 0..100000 {
        if session.advance(7) == DebugEvent::Completed {
            done = true;
            break;
        }
    }
    assert!(done, "did not finish: {source}");
    let succeeded = normal.is_ok();
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
    succeeded
}
#[test]
fn every_m2_operator_matches_values_errors_and_callback_order() {
    for (op, input, body) in [
        ("groupBy", "[1,2,3]", "log($ mod 2)"),
        ("pluck", "{a:1,b:2}", "log({k:$$,v:$,i:$$$})"),
        ("mapObject", "{a:1,b:2}", "log({($$):$*2})"),
        ("filterObject", "{a:1,b:2}", "log($>1)"),
        ("flatMap", "[1,2,3]", "log(if($==2) null else [$,$])"),
        ("distinctBy", "[1,2,1]", "log($)"),
        ("orderBy", "[3,1,2]", "log(-$)"),
        ("then", "2", "log($+1)"),
        ("onNull", "null", "log($ default 4)"),
        ("takeWhile", "[1,2,3]", "log($<2)"),
        ("dropWhile", "[1,2,3]", "log($<2)"),
        ("some", "[1,2,3]", "log($==2)"),
        ("every", "[1,2,3]", "log($<2)"),
        ("countCharactersBy", "\"a🐈c\"", "log($!=\"a\")"),
        ("everyCharacter", "\"abc\"", "log($!=\"b\")"),
        ("mapString", "\"a🐈c\"", "log(upper($))"),
        ("someCharacter", "\"abc\"", "log($==\"b\")"),
        ("substringBy", "\"a,b,c\"", "log($==\",\")"),
        ("countBy", "[1,2,3]", "log($>1)"),
        ("maxBy", "[1,3,2]", "log(-$)"),
        ("minBy", "[1,3,2]", "log(-$)"),
        ("sumBy", "[1,2,3]", "log($*2)"),
        ("firstWith", "[1,2,3]", "log($>1)"),
        ("indexWhere", "[1,2,3]", "log($>1)"),
        ("partition", "[1,2,3]", "log($>1)"),
        ("splitWhere", "[1,2,3]", "log($>1)"),
        ("everyEntry", "{a:1,b:2,c:3}", "log($<2)"),
        ("someEntry", "{a:1,b:2,c:3}", "log($>1)"),
        ("readLinesWith", "\"a\\nb\"", "log(\"UTF-8\")"),
        ("writeLinesWith", "[\"a\",\"b\"]", "log(\"UTF-8\")"),
        ("scan", "\"a1b22\"", "/(\\d+)/"),
        ("mapLeafValues", "{a:[1,2],b:3}", "log($+1)"),
        ("nodeExists", "{a:[1,2],b:3}", "log($$[-1].selector==\"1\")"),
        ("filterArrayLeafs", "{a:[1,2],b:3}", "log($>1)"),
        ("filterObjectLeafs", "{a:[1,2],b:3}", "log($>1)"),
        ("failIf", "2", "log($>4)"),
        ("wait", "2", "log(4)"),
    ] {
        for value in [input, "null", "[]", "{}", "\"\""] {
            let infix = compare(&format!("{value} {op} ({body})"), json!(null));
            let prefix = compare(&format!("{op}({value}, {body})"), json!(null));
            if value == input {
                assert!(infix && prefix, "happy path {op}");
            }
        }
    }
    for source in [
        "{a:1,b:2} groupBy ((v,k,i)->log(k))",
        "\"hello\" groupBy log($)",
        "{a:1,a:2,b:1} distinctBy log($)",
        "{a:1,b:2,c:3} takeWhile log($<2)",
        "[1] mapObject log($)",
        "{a:1} mapObject log(1)",
        "[1,2] sumBy log(\"bad\")",
        "[3,1,2] orderBy ((v,i)->log(i mod 2))",
        "{a:[1,null,2],b:{c:3}} filterArrayLeafs log($ != null)",
        "null mapLeafValues log($)",
        "2 failIf log(true)",
        "outerJoin([1,2],[2,3],(x)->log(x),(x)->log(x))",
        "join([], [1],log($),log($))",
        "leftJoin([1,2],[],log($),log($))",
        "join([1,2],[2,3],log($),log($))",
        "%dw 2.0\nimport groupBy as bucket from dw::Core\n---\nbucket([1,2,3], log($ mod 2))",
        "%dw 2.0\nimport groupBy as bucket from dw::Core\n---\n[1,2,3] bucket log($ mod 2)",
        "%dw 2.0\nimport dw::core::Arrays as A\n---\nA::partition([1,2,3], log($>1))",
        "Arrays::partition([1,2,3],log($>1))",
    ] {
        compare(source, json!(null));
    }
}

#[test]
fn lazy_pipelines_compose_without_changed_short_circuit_or_fallback_effects() {
    for source in [
        "1 to 9 flatMap [$,$+1] distinctBy $ takeWhile ($<5) dropWhile ($<2) map ($*2)",
        "[1,2] flatMap log([$]) ++ [3] flatMap log([$])",
        "[1,2] default [] flatMap log([$])",
        "null default [1,2] flatMap log([$])",
        "%dw 2.0\nimport flatMap as expand from dw::core::Arrays\n---\n[1,2] expand log([$]) ++ [3] expand log([$])",
        "1 to 9 flatMap [$,$+1] filter ($>2) some ($>4)",
        "1 to 9 flatMap [$,$+1] every ($<5)",
        "1 to 9 flatMap [$,$+1] firstWith ($>4)",
        "1 to 9 flatMap [$,$+1] indexWhere ($>4)",
        "1 to 9 flatMap [$,$+1] countBy ($>4)",
        "1 to 9 flatMap [$,$+1] sumBy $",
        "1 to 9 flatMap [$,$+1] reduce ((v,a=4)->v+a)",
        "[] flatMap [$] every true",
        "null flatMap [$] every true",
        "\"ab\" flatMap [$,$] map $",
        "log([1,2,3]) flatMap [$,$+1] takeWhile ($<3) reduce ((v,a=log(0))->v+a)",
        "log(null) flatMap [$] map $",
        "log(\"ab\") flatMap [$] sumBy 1",
        "1 to 4 flatMap [$,$+1] map (1/($-3)) map (1/($-2))",
        "%dw 2.0\nvar bias=2\nfun f(x)=if(x>2) x+bias else x\n---\n[1,2,3] flatMap ((v)->[log(f(v))]) takeWhile log($<4)",
        "%dw 2.0\nfun recurse(n)=if(n<1) [] else [n] flatMap ((v)->[v] ++ recurse(n-1))\n---\nrecurse(3)",
        "{a:1,b:2} mapObject ((v,k,i)->{(k): log([v,k,i,$,$$,$$$])})",
        "[1,2] flatMap ((n,idx)->do {var f=(v)->v+n+idx\n---\n[log(f(2))]})",
        "[1,2,3] groupBy ((n,idx)->log(idx mod 2))",
        "[1,2] pluck ((x)->log(x))",
        "[1,2] flatMap ((v:Number)->log([v]))",
        "[1,2] flatMap ((v,idx=log(9))->log([v,idx]))",
        "[1,2] map ((v,idx,unused=log(9))->log(v))",
        "[1,2] filter ((v,idx,unused=log(9))->log(v>1))",
        "[1,2] reduce ((v,acc=log(0),unused=log(9))->log(v+acc))",
        "[1,2] groupBy true",
        "[1,2] flatMap ((v)->[log(v)]) filter ((v)->log(v>1))",
    ] { compare(source,json!(null)); }
}

fn stopped(session: &mut DebugSession) -> &'static str {
    for _ in 0..10000 {
        match session.advance(3) {
            DebugEvent::Stopped(reason) => return reason,
            DebugEvent::Completed => {
                panic!("completed before expected stop: {:?}", session.exception)
            }
            _ => (),
        }
    }
    panic!("did not stop")
}
#[test]
fn callback_frames_scopes_repeated_breakpoints_and_step_controls() {
    let source="%dw 2.0\n/* 🐈 */\nvar offset=10\nfun inc(v)=v+offset\n---\n{a:1,b:2} pluck ((value,key,index) ->\n inc(value) + index)";
    let logs = Arc::new(Mutex::new(vec![]));
    let mut session = DebugSession::new(
        source,
        DwValue::Null,
        ExecutionContext::default(),
        ExecutionOptions::default(),
    )
    .unwrap();
    let range = session
        .locations()
        .into_iter()
        .find(|r| &source[r.start..r.end] == "inc(value) + index")
        .unwrap();
    assert_eq!(dwpy_core::debug::position(source, range.start), (7, 2));
    session.set_breakpoints(vec![range]);
    session.command(DebugCommand::Continue).unwrap();
    for (key, value, index) in [("a", 1, 0), ("b", 2, 1)] {
        assert_eq!(stopped(&mut session), "breakpoint");
        let frame = session.frames().last().unwrap();
        for (expression, expected) in [
            ("value", json!(value)),
            ("key", json!(key)),
            ("index", json!(index)),
            ("$", json!(value)),
            ("$$", json!(key)),
            ("$$$", json!(index)),
            ("offset", json!(10)),
        ] {
            assert_eq!(
                session
                    .evaluate(expression, frame.id)
                    .unwrap()
                    .data()
                    .unwrap(),
                DwValue::from(expected)
            );
        }
        assert!(session.evaluate("[1] pluck $", frame.id).is_err());
        session.command(DebugCommand::Into).unwrap();
        stopped(&mut session);
        // Into first reaches the call expression; another Into enters inc.
        if session.frames().last().unwrap().name != "inc" {
            session.command(DebugCommand::Into).unwrap();
            stopped(&mut session);
        }
        assert_eq!(session.frames().last().unwrap().name, "inc");
        session.command(DebugCommand::Out).unwrap();
        stopped(&mut session);
        assert_eq!(session.frames().last().unwrap().name, "lambda");
        session.command(DebugCommand::Over).unwrap();
        if index == 0 {
            assert_eq!(stopped(&mut session), "breakpoint");
            // The next callback breakpoint interrupts Over at the same source span.
            let frame = session.frames().last().unwrap();
            assert_eq!(
                session.evaluate("key", frame.id).unwrap().data().unwrap(),
                DwValue::from(json!("b"))
            );
            session.command(DebugCommand::Into).unwrap();
            stopped(&mut session);
            session.command(DebugCommand::Continue).unwrap();
            while session.advance(7) != DebugEvent::Completed {}
            break;
        }
    }
    assert_eq!(session.result, Some(DwValue::from(json!([11, 13]))));
    // Keep an effectful path separate: callback inspection and watches cannot replay logs.
    let sink = logs.clone();
    let mut s = DebugSession::new(
        "[1,2,3] flatMap ((n)->[log(n)])",
        DwValue::Null,
        ExecutionContext::default()
            .with_logger(move |e: &LogEntry| sink.lock().unwrap().push(e.clone())),
        ExecutionOptions::default(),
    )
    .unwrap();
    let range = s
        .locations()
        .into_iter()
        .find(|r| &s.source()[r.start..r.end] == "log(n)")
        .unwrap();
    s.set_breakpoints(vec![range]);
    s.command(DebugCommand::Continue).unwrap();
    for n in 1..=3 {
        assert_eq!(stopped(&mut s), "breakpoint");
        let f = s.frames().last().unwrap();
        assert_eq!(
            s.evaluate("n", f.id).unwrap().data().unwrap(),
            DwValue::from(json!(n))
        );
        assert_eq!(logs.lock().unwrap().len(), (n - 1) as usize);
        s.command(DebugCommand::Continue).unwrap();
    }
    while s.advance(7) != DebugEvent::Completed {}
    assert_eq!(logs.lock().unwrap().len(), 3);
}
#[test]
fn each_cursor_yields_cancels_and_checks_materialization_budgets() {
    for source in [
        "payload groupBy $",
        "payload flatMap [$]",
        "payload orderBy $",
        "payload mapLeafValues $",
        "payload nodeExists false",
        "join(payload,payload,$,$)",
        "1 to 1000000000 flatMap [$] takeWhile ($<20) sumBy $",
    ] {
        let mut s = DebugSession::new(
            source,
            json!((0..10000).collect::<Vec<_>>()).into(),
            ExecutionContext::default(),
            ExecutionOptions::default(),
        )
        .unwrap();
        s.command(DebugCommand::Continue).unwrap();
        assert_eq!(s.advance(10), DebugEvent::Yielded, "{source}");
        s.command(DebugCommand::Pause).unwrap();
        assert_eq!(s.advance(1), DebugEvent::Stopped("pause"));
        s.command(DebugCommand::Terminate).unwrap();
        assert_eq!(s.advance(1), DebugEvent::Terminated);
    }
    let mut s = DebugSession::new(
        "1 to 1000000000 flatMap [$] takeWhile ($<20) sumBy $",
        DwValue::Null,
        ExecutionContext::default(),
        ExecutionOptions::default(),
    )
    .unwrap();
    s.command(DebugCommand::Continue).unwrap();
    for _ in 0..10000 {
        if s.advance(10) == DebugEvent::Completed {
            break;
        }
    }
    assert_eq!(s.result, Some(DwValue::from(json!(190))));
    let mut s = DebugSession::new(
        "payload flatMap [$,$]",
        json!([1, 2, 3]).into(),
        ExecutionContext::default(),
        ExecutionOptions {
            max_materialized_bytes: 64,
            ..ExecutionOptions::default()
        },
    )
    .unwrap();
    s.command(DebugCommand::Continue).unwrap();
    while s.advance(7) != DebugEvent::Completed {}
    assert!(s.exception.unwrap().contains("materialized"));
}

#[test]
fn imported_and_qualified_operators_have_concrete_successful_results() {
    let expected = Some(DwValue::from(json!({"success":[2,3],"failure":[1]})));
    for source in [
        "Arrays::partition([1,2,3],$>1)",
        "%dw 2.0\nimport partition as split from dw::core::Arrays\n---\nsplit([1,2,3],$>1)",
        "%dw 2.0\nimport partition as split from dw::core::Arrays\n---\n[1,2,3] split ($>1)",
        "%dw 2.0\nimport dw::core::Arrays as A\n---\nA::partition([1,2,3],$>1)",
        "do {import partition as split from dw::core::Arrays\n---\nsplit([1,2,3],$>1)}",
    ] {
        assert!(compare(source, json!(null)), "{source}");
        let mut s = DebugSession::new(
            source,
            DwValue::Null,
            ExecutionContext::default(),
            ExecutionOptions::default(),
        )
        .unwrap();
        s.command(DebugCommand::Continue).unwrap();
        while s.advance(7) != DebugEvent::Completed {}
        assert_eq!(s.result, expected, "{source}");
    }
}

#[test]
fn shared_m2_fixture_has_the_expected_result() {
    let transcript: serde_json::Value =
        serde_json::from_str(include_str!("../../../tests/fixtures/debugger/m2.json")).unwrap();
    let source = transcript[1]["arguments"]["source"].as_str().unwrap();
    assert!(compare(source, json!(null)));
    let expected: serde_json::Value = serde_json::from_str(include_str!(
        "../../../tests/fixtures/debugger/m2.expected.json"
    ))
    .unwrap();
    let mut s = DebugSession::new(
        source,
        DwValue::Null,
        ExecutionContext::default(),
        ExecutionOptions::default(),
    )
    .unwrap();
    s.command(DebugCommand::Continue).unwrap();
    while s.advance(7) != DebugEvent::Completed {}
    let DwValue::String(text) = s.result.unwrap() else {
        panic!("expected rendered JSON")
    };
    assert_eq!(
        serde_json::from_str::<serde_json::Value>(&text).unwrap(),
        expected
    );
}

#[test]
fn local_functions_and_parameters_shadow_callback_import_names() {
    for source in [
        "%dw 2.0\nfun flatMap(x,y)=x+y\n---\nflatMap(2,3)",
        "%dw 2.0\nimport partition as cut from dw::core::Arrays\nfun cut(x,y)=x+y\n---\ncut(2,3)",
        "%dw 2.0\nfun f(partition)=partition(2,3)\n---\nf((a,b)->a+b)",
        "do {var pluck=(a,b)->a+b\n---\npluck(2,3)}",
        "using(pluck=(a,b)->a+b) pluck(2,3)",
    ] {
        assert!(compare(source, json!(null)), "{source}");
    }
}

#[test]
fn discarded_predicates_and_repeated_keys_do_not_accumulate_cursor_memory() {
    for (source, payload, expected) in [
        (
            "payload distinctBy log($)",
            json!(vec![1; 4096]),
            json!([1]),
        ),
        (
            "payload countCharactersBy true",
            json!("🐈".repeat(4096)),
            json!(4096),
        ),
        (
            "payload mapString $",
            json!("ab".repeat(32)),
            json!("ab".repeat(32)),
        ),
    ] {
        let mut s = DebugSession::new(
            source,
            payload.into(),
            ExecutionContext::default(),
            ExecutionOptions {
                max_materialized_bytes: 128,
                ..ExecutionOptions::default()
            },
        )
        .unwrap();
        s.command(DebugCommand::Continue).unwrap();
        for _ in 0..10000 {
            if s.advance(100) == DebugEvent::Completed {
                break;
            }
        }
        assert!(s.exception.is_none(), "{source}: {:?}", s.exception);
        assert_eq!(s.result, Some(DwValue::from(expected)), "{source}");
    }
}
