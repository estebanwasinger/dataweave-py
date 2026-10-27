use dwpy_core::{
    debug::{DebugCommand, DebugEvent, DebugSession, DebugState},
    execute_value, DwValue, ExecutionContext, ExecutionOptions,
};
use serde_json::json;
fn run(source: &str, payload: serde_json::Value) -> DwValue {
    let mut session = DebugSession::new(
        source,
        payload.into(),
        ExecutionContext::default(),
        ExecutionOptions::default(),
    )
    .unwrap();
    session.command(DebugCommand::Continue).unwrap();
    for _ in 0..10000 {
        if session.advance(17) == DebugEvent::Completed {
            assert!(session.exception.is_none(), "{:?}", session.exception);
            return session.result.unwrap();
        }
    }
    panic!("did not finish")
}
#[test]
fn differential_core_transformations() {
    for source in [
        "%dw 2.0\nfun hi(a:Number,b:Number)=a ++ b\n---\nhi(2,\"3\")",
        "%dw 2.0\nfun f(x:Number)=\"number\"\nfun f(x:String)=\"string\"\n---\n[f(\"3\"),f(3)]",
        "%dw 2.0\noutput application/json\n---\npayload map ((item,index) -> {index:index, value:item*2})",
        "1 to 8 map ($ * 3) filter ($ > 9) reduce ((item, acc = 0) -> item + acc)",
        "%dw 2.0\nvar start = 2\nfun add(x) = x + start\n---\npayload map ((x) -> add(x))",
        "do { var x = 4\n---\n{x:x, y: if (x > 3) upper(\"yes\") else \"no\"} }",
        "[] reduce ((x,acc=42) -> x + acc)",
        "[1,2] map ((x) -> do { var y=x+2\n---\ny*y })",
        "false and log(\"never\")",
        "%dw 2.0\nfun f(payload)=payload\n---\nf(2)",
        "sizeOf(1 to 1000000000)",
        "sizeOf(1 to 4 map log($))",
        "sizeOf(log(3 as Number))",
        "not false or true", "-1+2", "payload[0 to 1]", "[1,2,3] filter (($ mod 2) == 1)",
        "log([1,2]) reduce ((x,acc=log(0))->x+acc)",
        "%dw 2.0\noutput application/json\n---\nlog(null) map ($*2)", "true or log(\"never\")", "4 default log(\"never\")",
        "[payload[0], (payload map ($+1))[1], sizeOf(payload), 1 as String]",
        "%dw 2.0\nfun fact(n) = if (n <= 1) 1 else n * fact(n-1)\n---\nfact(5)",
        "%dw 2.0\nvar make = (x) -> (y) -> x + y\nvar add = make(3)\n---\nadd(4)",
        "{a: 1, a: 2}", "1 to 5 map ($*2) filter ($>2)", "payload filter ($ > 1) map ($*2)",
    ] {
        let payload=json!([1,2,3]);
        assert_eq!(run(source,payload.clone()),execute_value(source,payload.into(),true).unwrap(),"{source}");
    }
}
#[test]
fn stops_watches_and_scope() {
    let source = "%dw 2.0\nvar x = 3\n---\npayload map ((item,index) -> item+x)";
    let mut session = DebugSession::new(
        source,
        json!([1, 2]).into(),
        ExecutionContext::default(),
        ExecutionOptions::default(),
    )
    .unwrap();
    assert_eq!(session.advance(100), DebugEvent::Stopped("entry"));
    assert_eq!(session.state, DebugState::Stopped);
    session.command(DebugCommand::Into).unwrap();
    let mut iterations = vec![];
    for _ in 0..50 {
        match session.advance(100) {
            DebugEvent::Stopped(_) => {
                let frame = session.frames().last().unwrap();
                if frame.name == "lambda" {
                    iterations.push(session.evaluate("item", frame.id).unwrap().data().unwrap());
                    assert_eq!(
                        session
                            .evaluate("upper(\"yes\")", frame.id)
                            .unwrap()
                            .data()
                            .unwrap(),
                        DwValue::String("YES".into())
                    );
                    for watch in [
                        "log(item)",
                        "now()",
                        "((x)->x)(1)",
                        "do {var x=log(1)\n---\nx}",
                    ] {
                        assert!(session.evaluate(watch, frame.id).is_err(), "{watch}");
                    }
                }
                session.command(DebugCommand::Into).unwrap();
            }
            DebugEvent::Completed => break,
            _ => (),
        }
    }
    assert_eq!(
        iterations,
        vec![DwValue::from(json!(1)), DwValue::from(json!(2))]
    );
}
#[test]
fn locations_and_preflight() {
    let source="%dw 2.0\n/* commented --- 🐈 */\nvar x =\n  1 + 2\n---\n{x: x, nested: do {var y=3\n---\ny+x}}";
    let session = DebugSession::new(
        source,
        DwValue::Null,
        ExecutionContext::default(),
        ExecutionOptions::default(),
    )
    .unwrap();
    let locations = session.locations();
    assert!(locations.iter().any(|r| &source[r.start..r.end] == "1 + 2"));
    assert!(locations.iter().any(|r| &source[r.start..r.end] == "y+x"));
    for source in [
        "try(() -> payload)",
        "payload update {case x -> 1}",
        "{x: try(() -> payload)}",
    ] {
        assert!(DebugSession::new(
            source,
            DwValue::Null,
            ExecutionContext::default(),
            ExecutionOptions::default()
        )
        .is_err());
    }
}
#[test]
fn differential_callbacks_formats_and_edges() {
    use std::sync::{Arc, Mutex};
    for source in [
        "1 to 4 map log($) filter log($>1)",
        "%dw 2.0\nfun f(x:Number,y=log(10))=log(x+y)\nfun f(x:String,y=log(20))=log(x++(y as String))\n---\n[f(log(2)),f(log(\"a\"))]",
        "log({a:2}) update {case .a if(log($>1)) -> log($*2)\ncase .missing if(log(false)) -> log(99)\ncase .a -> log($+1)}",
        "log({a:2}) update log(\"a\") with log(4)",
        "{a:2} default log({a:5}) update {case .a -> log(9)}",
        "1 to 4 map ($*2) filter ($>3) reduce ((x,acc=log(0))->x+acc)",
        "1 to 4 map ($/($-2)) filter ($>10)",
        "\"abc\" reduce ((ch,acc=\"\") -> acc ++ ch)",
        "42 map $", "null filter $", "null reduce ((x,acc=log(4))->x+acc)",
        "%dw 2.0\noutput application/csv separator=\";\"\n---\npayload map ((item)-> {value:item*2})",
        "%dw 2.0\nvar x=log(1)\nfun f(a,b=log(a+1))=a+b\n---\nf(x)",
        "do {var x=log(1)\n---\n{x:log(x), y:log(2)}}",
        "[1,2] map ((x) -> ((y)->x+y)(1))",
    ] {
        let normal_logs=Arc::new(Mutex::new(vec![]));let sink=normal_logs.clone();
        let normal_context=ExecutionContext::default().with_logger(move|e:&dwpy_core::LogEntry|sink.lock().unwrap().push(e.clone()));
        let expected=dwpy_core::execute_value_with_context(source,json!([1,2]).into(),normal_context,true);
        let debug_logs=Arc::new(Mutex::new(vec![]));let sink=debug_logs.clone();
        let context=ExecutionContext::default().with_logger(move|e:&dwpy_core::LogEntry|sink.lock().unwrap().push(e.clone()));
        let mut session=match DebugSession::new(source,json!([1,2]).into(),context,ExecutionOptions::default()) {Ok(s)=>s,Err(e) if source.contains("((y)->x+y)(1)")=>{assert!(e.to_string().contains("unsupported"));continue;},Err(e)=>panic!("{source}: {e}")};
        session.command(DebugCommand::Continue).unwrap();
        for _ in 0..10000 {if session.advance(11)==DebugEvent::Completed {break;}}
        match expected {Ok(value)=>{assert!(session.exception.is_none(),"{source}: {:?}",session.exception);assert_eq!(session.result,Some(value),"{source}");},Err(e)=>assert_eq!(session.exception,Some(e.to_string()),"{source}")}
        assert_eq!(*debug_logs.lock().unwrap(),*normal_logs.lock().unwrap(),"{source}");
    }
}
#[test]
fn step_over_and_out_keep_the_caller() {
    let source = "%dw 2.0\nfun add(x)=x+1\n---\n{a: add(1), b: add(2)}";
    let mut s = DebugSession::new(
        source,
        DwValue::Null,
        ExecutionContext::default(),
        ExecutionOptions::default(),
    )
    .unwrap();
    s.advance(100);
    s.command(DebugCommand::Into).unwrap();
    s.advance(100);
    assert_eq!(
        &source[s.frames().last().unwrap().range.start..s.frames().last().unwrap().range.end],
        "add(1)"
    );
    s.command(DebugCommand::Into).unwrap();
    s.advance(100);
    assert_eq!(s.frames().len(), 2);
    s.command(DebugCommand::Out).unwrap();
    assert_eq!(s.advance(100), DebugEvent::Stopped("step"));
    assert_eq!(s.frames().len(), 1);
    let r = s.frames()[0].range;
    assert_eq!(&source[r.start..r.end], "add(1)");
    assert_eq!(
        s.frames()[0].last.as_ref().unwrap().data().unwrap(),
        DwValue::from(json!(2))
    );
    s.command(DebugCommand::Into).unwrap();
    s.advance(100);
    s.command(DebugCommand::Over).unwrap();
    assert_eq!(s.advance(1000), DebugEvent::Completed);
}

#[test]
fn shadowed_parameters_and_globals_are_visible_in_their_scopes() {
    let source = "%dw 2.0\nvar x=1\nfun f(arg)=do {var x=9\nvar arg=7\n---\nx+arg}\n---\nf(3)";
    let mut s = DebugSession::new(
        source,
        DwValue::Null,
        ExecutionContext::default(),
        ExecutionOptions::default(),
    )
    .unwrap();
    for _ in 0..100 {
        match s.advance(100) {
            DebugEvent::Stopped(_) => {
                let f = s.frames().last().unwrap();
                let range = f.range;
                if &source[range.start..range.end] == "x+arg" {
                    let scopes = s.scopes(f.id).unwrap();
                    let find = |scope: &str, key: &str| {
                        scopes
                            .iter()
                            .find(|(n, _)| n == scope)
                            .unwrap()
                            .1
                            .iter()
                            .find(|(k, _)| k == key)
                            .unwrap()
                            .1
                            .data()
                            .unwrap()
                    };
                    assert_eq!(find("Globals", "x"), DwValue::from(json!(1)));
                    assert_eq!(find("Locals", "x"), DwValue::from(json!(9)));
                    assert_eq!(find("Parameters", "arg"), DwValue::from(json!(3)));
                    assert_eq!(find("Locals", "arg"), DwValue::from(json!(7)));
                    return;
                }
                s.command(DebugCommand::Into).unwrap();
            }
            DebugEvent::Completed => panic!("missed scope"),
            _ => (),
        }
    }
    panic!("did not stop");
}
#[test]
fn eager_ranges_obey_materialization_limits() {
    let mut s = DebugSession::new(
        "[1 to 1000000000]",
        DwValue::Null,
        ExecutionContext::default(),
        ExecutionOptions {
            max_materialized_bytes: 4096,
            ..ExecutionOptions::default()
        },
    )
    .unwrap();
    s.command(DebugCommand::Continue).unwrap();
    assert_eq!(s.advance(100), DebugEvent::Completed);
    assert!(s.exception.unwrap().contains("range limit"));
}

#[test]
fn differential_updates() {
    for source in [
        "%dw 2.0\noutput application/json\nfun multiply(number, multiplier) = number * multiplier * 2\n---\n{numA: 2} update {case .numA -> $ * 2}",
        "{user:{age:30}, items:[1,2]} update {case age at .user.age -> age+1\ncase v at .items[-1] -> v*3}",
        "{a:2} update {case .a -> $+1\ncase .a -> $*2}",
        "[{name: \"Ken\"}, {}] map ((user) -> user update {case name at .name! -> if (name == null) \"JOHN\" else upper(name)})",
        "[1,2,3] update {case v at [1] if (v>1) -> v*2}",
        "{a:2} update {case .a if ($>5) -> 0\ncase .a if ($==2) -> 4}",
        "do {var path=\"a\"\n---\n{a:2} update {case value at .\"$(path)\" -> value*2}}",
        "{a:2} update {case .a -> ({b:$} update {case .b -> $*2})}",
        "{a:2} update {}", "{a:2} update {case .missing -> 3}",
        "null update {case .a -> 3}",
        "{a:2} default ({a:5} update {case .a -> 9})",
        "{a:2} default {a:5} update {case .a -> 9}",
        "null default {a:5} update {case .a -> 9}",
        "{a:2} update \"a\" with 4", "[1,2] update 1 with 4",
        "{user:{age:2}} update [\"user\",\"age\"] with 4",
        "%dw 2.0\nvar x=5\nfun twice(n)=n*2\n---\n{a:2} update {case x at .a -> twice(x)}",
        "%dw 2.0\n/* --- 🐈 */\n---\n{a:2} update {\n// case ignored at .x -> 3\ncase .a ->\n  $ * 2\n}",
    ] {
        assert_eq!(run(source, json!(null)), execute_value(source, DwValue::Null, true).unwrap(), "{source}");
    }
}

#[test]
fn updates_step_into_replacements_and_restore_bindings() {
    let source = "%dw 2.0\nvar n=99\nfun twice(x)=x*2\n---\n{result: {numA:2} update {\ncase .numA -> $ * 2\ncase n at .numA if(n==4) -> twice(n)\n}, outer: n}";
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
    let locations = session.locations();
    assert!(locations.iter().any(|r| &source[r.start..r.end] == "$ * 2"));
    let mut replacements = vec![];
    for _ in 0..1000 {
        match session.advance(1) {
            DebugEvent::Stopped(_) => {
                let frame = session.frames().last().unwrap();
                let expression = &source[frame.range.start..frame.range.end];
                if expression == "$ * 2" {
                    assert_eq!(
                        session.evaluate("$", frame.id).unwrap().data().unwrap(),
                        DwValue::from(json!(2))
                    );
                    replacements.push(expression);
                }
                if expression == "twice(n)" {
                    assert_eq!(
                        session.evaluate("n", frame.id).unwrap().data().unwrap(),
                        DwValue::from(json!(4))
                    );
                    let scopes = session.scopes(frame.id).unwrap();
                    assert!(scopes.iter().any(|(name, values)| name == "Locals"
                        && values.iter().any(|(key, _)| key == "n")));
                    replacements.push(expression);
                }
                assert!(session
                    .evaluate("{a:1} update {case .a -> log($)}", frame.id)
                    .is_err());
                session.command(DebugCommand::Into).unwrap();
            }
            DebugEvent::Completed => break,
            _ => (),
        }
    }
    assert_eq!(replacements, ["$ * 2", "twice(n)"]);
    assert!(session.exception.is_none(), "{:?}", session.exception);
    assert_eq!(
        session.result,
        Some(DwValue::from(json!({"result":{"numA":8},"outer":99})))
    );
}

#[test]
fn update_breakpoints_repeat_per_callback_without_replaying() {
    let source =
        "%dw 2.0\nfun twice(x)=x*2\n---\n[1,2] map ((n) -> {a:n} update {case .a -> twice($)})";
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
        .find(|r| &source[r.start..r.end] == "twice($)")
        .unwrap();
    session.set_breakpoints(vec![range]);
    session.command(DebugCommand::Continue).unwrap();
    let mut selected = vec![];
    for _ in 0..1000 {
        match session.advance(3) {
            DebugEvent::Stopped("breakpoint") => {
                let frame = session.frames().last().unwrap();
                selected.push(session.evaluate("$", frame.id).unwrap().data().unwrap());
                session.command(DebugCommand::Over).unwrap();
            }
            DebugEvent::Stopped(_) => {
                session.command(DebugCommand::Continue).unwrap();
            }
            DebugEvent::Completed => break,
            _ => (),
        }
    }
    assert_eq!(
        selected,
        vec![DwValue::from(json!(1)), DwValue::from(json!(2))]
    );
    assert_eq!(
        session.result,
        Some(DwValue::from(json!([{"a":2},{"a":4}])))
    );
}

#[test]
fn differential_overloads_and_recursive_dispatch() {
    for source in [
        "%dw 2.0\nfun f(x:Number)=x*2\nfun f(x:String)=upper(x)\n---\n[f(3),f(\"ok\")]",
        "%dw 2.0\nfun multiply(n,m:Number)=n*m*2\nfun multiply(n,m:String)=n*m as Number\nfun multiply(n)=multiply(n,n)\n---\n[multiply(2,4),multiply(2,\"4\"),multiply(2)]",
        "%dw 2.0\nfun f(x:Number|Null)=1\nfun f(x:String)=2\n---\n[f(null),f(1),f(\"a\")]",
        "%dw 2.0\nfun f(x)=1\nfun f(x:Number)=2\n---\nf(3)",
        "%dw 2.0\nfun f(x:String)=f(x as Number)\nfun f(x:Number)=x*2\n---\nf(\"3\")",
        "%dw 2.0\nfun f(n)=if(n==0) 0 else f(n-1,10)\nfun f(n,acc)=if(n==0) acc else f(n-1)+acc\n---\nf(4)",
        "%dw 2.0\nfun f(x:Number)=x+1\n---\ndo {fun f(x:String)=upper(x)\n---\n[f(2),f(\"a\")]}",
        "%dw 2.0\nfun f(x:Number)=x+1\nvar chosen=f\nfun f(x:String)=upper(x)\n---\nchosen(\"a\")",
        "%dw 2.0\nfun f(x:Array)=sizeOf(x)\nfun f(x:Object)=keysOf(x)\n---\n[f([1,2]),f({a:1})]",
    ] {
        assert_eq!(run(source, json!(null)), execute_value(source, DwValue::Null, true).unwrap(), "{source}");
    }
}

#[test]
fn overload_errors_occur_in_the_selected_body_and_match_normal_execution() {
    let source = "%dw 2.0\noutput application/json\nfun multiply(number, multiplier : Number) = number * multiplier * 2\nfun multiply(number, multiplier : String) = number * multiplier as Number\nfun multiply(number) = multiply(number,number)\n---\n{numA:2} update {case .numA -> multiply($,\"4s\")}";
    let expected = execute_value(source, DwValue::Null, true)
        .unwrap_err()
        .to_string();
    let mut session = DebugSession::new(
        source,
        DwValue::Null,
        ExecutionContext::default(),
        ExecutionOptions::default(),
    )
    .unwrap();
    session.stop_on_exception = true;
    session.command(DebugCommand::Continue).unwrap();
    for _ in 0..1000 {
        if session.advance(5) == DebugEvent::Stopped("exception") {
            break;
        }
    }
    assert_eq!(session.exception.as_deref(), Some(expected.as_str()));
    let frame = session.frames().last().unwrap();
    assert_eq!(frame.name, "multiply");
    assert_eq!(
        session
            .evaluate("multiplier", frame.id)
            .unwrap()
            .data()
            .unwrap(),
        DwValue::String("4s".into())
    );
    assert_eq!(dwpy_core::debug::position(source, frame.range.start).0, 4);
    let mismatch = "%dw 2.0\nfun f(x:Number)=1\nfun f(x:String)=2\n---\nf(log(true))";
    let mut session = DebugSession::new(
        mismatch,
        DwValue::Null,
        ExecutionContext::default(),
        ExecutionOptions::default(),
    )
    .unwrap();
    session.command(DebugCommand::Continue).unwrap();
    while session.advance(20) != DebugEvent::Completed {}
    assert_eq!(
        session.exception,
        Some(
            execute_value(mismatch, DwValue::Null, true)
                .unwrap_err()
                .to_string()
        )
    );
}
