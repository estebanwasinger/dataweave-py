use dwpy_core::dap::DebugAdapter;
use serde_json::{json, Value};
fn request(a: &mut DebugAdapter, command: &str, args: Value) -> Vec<Value> {
    a.dispatch(json!({"seq":1,"type":"request","command":command,"arguments":args}))
}
fn ok(a: &mut DebugAdapter, command: &str, args: Value) -> Value {
    let m = request(a, command, args);
    assert_eq!(m[0]["success"], true, "{m:?}");
    m[0]["body"].clone()
}
fn stop(a: &mut DebugAdapter) -> Value {
    for _ in 0..1000 {
        for m in a.poll(7) {
            if m["event"] == "stopped" {
                return m;
            }
            assert_ne!(m["event"], "terminated", "terminated before stop");
        }
    }
    panic!("no stop")
}
fn launch(a: &mut DebugAdapter, source: &str) {
    ok(a, "initialize", json!({}));
    ok(
        a,
        "launch",
        json!({"source":source,"payload":[1,2,3],"stopOnEntry":true}),
    );
    ok(a, "configurationDone", json!({}));
    stop(a);
}
#[test]
fn lifecycle_and_stale_handles() {
    let mut a = DebugAdapter::new();
    launch(
        &mut a,
        "%dw 2.0\nvar x=3\n---\npayload map ((item,index) -> item+x)",
    );
    let frames = ok(&mut a, "stackTrace", json!({"threadId":1}));
    let id = frames["stackFrames"][0]["id"].clone();
    let scopes = ok(&mut a, "scopes", json!({"frameId":id}));
    let reference = scopes["scopes"][1]["variablesReference"].clone();
    ok(&mut a, "variables", json!({"variablesReference":reference}));
    assert!(!ok(
        &mut a,
        "evaluate",
        json!({"frameId":id,"expression":"payload[1]"})
    )["result"]
        .is_null());
    assert_eq!(
        request(
            &mut a,
            "evaluate",
            json!({"frameId":id,"expression":"log(1)"})
        )[0]["success"],
        false
    );
    ok(&mut a, "stepIn", json!({"threadId":1}));
    stop(&mut a);
    assert_eq!(
        request(&mut a, "scopes", json!({"frameId":id}))[0]["success"],
        false
    );
    assert_eq!(
        request(&mut a, "variables", json!({"variablesReference":reference}))[0]["success"],
        false
    );
    ok(&mut a, "continue", json!({"threadId":1}));
    let mut events = vec![];
    for _ in 0..1000 {
        events.extend(a.poll(5));
        if events.iter().any(|e| e["event"] == "terminated") {
            break;
        }
    }
    assert_eq!(
        events.iter().filter(|e| e["event"] == "terminated").count(),
        1
    );
    assert!(events.iter().any(|e| e["body"]["data"]["kind"] == "result"));
    assert!(a.poll(100).is_empty());
}
#[test]
fn breakpoints_repeat_and_columns() {
    let mut a = DebugAdapter::new();
    ok(
        &mut a,
        "initialize",
        json!({"linesStartAt1":false,"columnsStartAt1":false}),
    );
    ok(
        &mut a,
        "launch",
        json!({"source":"%dw 2.0\n---\npayload map ((item) -> item+1)","payload":[1,2],"stopOnEntry":false}),
    );
    let bp = ok(
        &mut a,
        "setBreakpoints",
        json!({"source":{"sourceReference":1},"breakpoints":[{"line":2,"column":22},{"line":1}]}),
    );
    assert_eq!(bp["breakpoints"][0]["verified"], true);
    assert_eq!(bp["breakpoints"][1]["verified"], false);
    ok(&mut a, "configurationDone", json!({}));
    for _ in 0..2 {
        assert_eq!(stop(&mut a)["body"]["reason"], "breakpoint");
        ok(&mut a, "continue", json!({"threadId":1}));
    }
    let events = a.poll(1000);
    assert!(events.iter().any(|e| e["event"] == "terminated"));
}
#[test]
fn pause_exception_and_termination() {
    let mut a = DebugAdapter::new();
    launch(&mut a, "1 to 100000 map ($+1)");
    ok(&mut a, "continue", json!({"threadId":1}));
    a.poll(3);
    ok(&mut a, "pause", json!({"threadId":1}));
    assert_eq!(stop(&mut a)["body"]["reason"], "pause");
    ok(&mut a, "terminate", json!({}));
    assert!(!a.is_running());
    let mut a = DebugAdapter::new();
    ok(&mut a, "initialize", json!({}));
    ok(
        &mut a,
        "launch",
        json!({"source":"%dw 2.0\nfun bad(x)=1/x\n---\nbad(0)","stopOnEntry":false}),
    );
    ok(
        &mut a,
        "setExceptionBreakpoints",
        json!({"filters":["uncaught"]}),
    );
    ok(&mut a, "configurationDone", json!({}));
    assert_eq!(stop(&mut a)["body"]["reason"], "exception");
    assert!(
        ok(&mut a, "stackTrace", json!({}))["totalFrames"]
            .as_u64()
            .unwrap()
            > 1
    );
    assert_eq!(
        ok(&mut a, "exceptionInfo", json!({"threadId":1}))["breakMode"],
        "unhandled"
    );
    ok(&mut a, "continue", json!({"threadId":1}));
    assert!(a.is_running(), "completion events still need pumping");
    assert!(a.poll(100).iter().any(|m| m["event"] == "terminated"));
    assert!(!a.is_running());
}
#[test]
fn output_logs_and_duplicate_variable_fields() {
    let mut a = DebugAdapter::new();
    launch(&mut a, "%dw 2.0\nvar obj={a:1,a:2}\n---\nlog(obj)");
    ok(&mut a, "stepIn", json!({"threadId":1}));
    stop(&mut a);
    ok(&mut a, "continue", json!({"threadId":1}));
    let mut events = vec![];
    for _ in 0..100 {
        events.extend(a.poll(3));
    }
    assert_eq!(
        events
            .iter()
            .filter(|e| e["body"]["data"]["kind"] == "log")
            .count(),
        1
    );
}

#[test]
fn variable_pages_preserve_duplicate_fields_and_native_types() {
    let mut a = DebugAdapter::new();
    ok(&mut a, "initialize", json!({"supportsVariablePaging":true}));
    ok(
        &mut a,
        "launch",
        json!({"source":"payload","payload":"{\"x\":1,\"x\":2}","payload_format":"application/json"}),
    );
    ok(&mut a, "configurationDone", json!({}));
    stop(&mut a);
    let frames = ok(&mut a, "stackTrace", json!({"threadId":1}));
    let frame = frames["stackFrames"][0]["id"].clone();
    let scopes = ok(&mut a, "scopes", json!({"frameId":frame}));
    let global = scopes["scopes"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["name"] == "Globals")
        .unwrap()["variablesReference"]
        .clone();
    let variables = ok(&mut a, "variables", json!({"variablesReference":global}));
    let object = variables["variables"]
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["name"] == "payload")
        .unwrap();
    assert_eq!(object["namedVariables"], 2);
    let reference = object["variablesReference"].clone();
    for (start, expected) in [(0, "1"), (1, "2")] {
        let page = ok(
            &mut a,
            "variables",
            json!({"variablesReference":reference,"start":start,"count":1,"filter":"named"}),
        );
        assert_eq!(page["variables"].as_array().unwrap().len(), 1);
        assert_eq!(page["variables"][0]["name"], "x");
        assert_eq!(page["variables"][0]["value"], expected);
        assert_eq!(page["variables"][0]["type"], "Number");
    }
    assert!(ok(
        &mut a,
        "variables",
        json!({"variablesReference":reference,"filter":"indexed"})
    )["variables"]
        .as_array()
        .unwrap()
        .is_empty());
    assert_eq!(
        ok(&mut a, "variables", json!({"variablesReference":reference}))["variables"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
}
