#![cfg(target_arch = "wasm32")]

use dwpy_wasm::{
    analyze_dataweave_request, run_dataweave_request, run_dataweave_request_with_logs,
    run_dataweave_smoke,
};
use serde_json::{json, Value};
use wasm_bindgen::JsValue;
use wasm_bindgen_test::wasm_bindgen_test;

fn execute(request: Value) -> Value {
    let output = run_dataweave_request(&request.to_string()).expect("execution should succeed");
    serde_json::from_str(&output).expect("the WASM entry point returns JSON")
}

fn analyze(request: Value) -> Value {
    let output = analyze_dataweave_request(&request.to_string()).expect("analysis should succeed");
    serde_json::from_str(&output).expect("the WASM entry point returns JSON")
}

fn error_message(error: JsValue) -> String {
    error.as_string().expect("WASM errors should be strings")
}

#[wasm_bindgen_test]
fn log_capture_preserves_result_errors_and_execution_isolation() {
    let run = |script: &str| -> Value {
        serde_json::from_str(
            &run_dataweave_request_with_logs(&json!({"script": script}).to_string()).unwrap(),
        )
        .unwrap()
    };
    let execution = run("%dw 2.0\noutput application/json\n---\n1 to 10 map log($)");
    assert_eq!(execution["result"], json!("[1,2,3,4,5,6,7,8,9,10]"));
    assert_eq!(execution["logs"].as_array().unwrap().len(), 10);
    assert_eq!(
        execution["logs"][0],
        json!({"level":"warn", "prefix": null, "message":"1"})
    );
    let failed = run("[logInfo(\"before\", 1), fail(\"boom\")]");
    assert!(failed["error"].as_str().unwrap().contains("boom"));
    assert_eq!(failed["logs"][0]["message"], "before - 1");
    assert_eq!(run("42")["logs"], json!([]));
    let bounded = run("1 to 1005 map log($)");
    assert_eq!(bounded["logs"].as_array().unwrap().len(), 1000);
    assert_eq!(bounded["logs_truncated"], true);
}

#[wasm_bindgen_test]
fn smoke_entry_point_evaluates_rust_core_inside_wasm() {
    let output = run_dataweave_smoke("{name: upper(payload.name)}", r#"{"name":"dw"}"#).unwrap();
    assert_eq!(
        serde_json::from_str::<Value>(&output).unwrap(),
        json!({"name": "DW"})
    );
}

#[wasm_bindgen_test]
fn smoke_entry_point_preserves_duplicate_json_payload_keys() {
    let payload = r#"{"test":{"test2":{"array":1},"test2":{"array":2},"test2":{"array":3}}}"#;
    let output = run_dataweave_smoke("payload.test.*test2", payload).unwrap();
    assert_eq!(
        serde_json::from_str::<Value>(&output).unwrap(),
        json!([{"array": 1}, {"array": 2}, {"array": 3}])
    );
}

#[wasm_bindgen_test]
fn request_entry_point_preserves_duplicates_before_decoding_context() {
    // Construct raw JSON: json! or a host JSON.parse would already lose duplicates.
    let request = r#"{"script":"{items: payload.test.*test2, values: vars.*a, headers: attributes.*h}","payload":{"test":{"test2":{"array":1},"test2":{"array":2},"test2":{"array":3}}},"vars":{"a":1,"a":2},"attributes":{"h":"first","h":"last"}}"#;
    let output = run_dataweave_request(request).unwrap();
    assert_eq!(
        serde_json::from_str::<Value>(&output).unwrap(),
        json!({
            "items": [{"array": 1}, {"array": 2}, {"array": 3}],
            "values": [1, 2],
            "headers": ["first", "last"]
        })
    );
}

#[wasm_bindgen_test]
fn request_entry_point_preserves_execution_context() {
    assert_eq!(
        execute(json!({
            "script": "%dw 2.0\nvar prefix = vars.prefix\n---\n{message: prefix ++ payload.name, header: attributes.headers.x, environment: p(\"env\")}",
            "payload": {"name": "Ada"},
            "vars": {"prefix": "Hello "},
            "attributes": {"headers": {"x": "yes"}},
            "properties": {"env": "test"},
            "render_output": false
        })),
        json!({"message": "Hello Ada", "header": "yes", "environment": "test"})
    );
}

#[wasm_bindgen_test]
fn rendered_output_is_distinct_from_native_values() {
    let mut request = json!({"script": "%dw 2.0\noutput application/json\n---\n{value: payload.value}", "payload": {"value": 42}});
    assert_eq!(execute(request.clone()), json!(r#"{"value":42}"#));
    request["render_output"] = json!(false);
    assert_eq!(execute(request), json!({"value": 42}));
}

#[wasm_bindgen_test]
fn coercion_binds_before_date_and_number_arithmetic() {
    assert_eq!(
        execute(json!({
            "script": r#"%dw 2.0
output application/json
---
{
  date: "2026-10-26" as Date + |P3D|,
  number: "2" as Number + 3 * 4,
  casts: "2" as Number as String as Number + 3
}"#,
            "payload": null
        })),
        json!(r#"{"date":"2026-10-29","number":14,"casts":5}"#)
    );
}

#[wasm_bindgen_test]
fn text_output_renders_date_as_text_instead_of_a_tagged_object() {
    for mime in ["text", "text/plain", "plain"] {
        assert_eq!(
            execute(json!({
                "script": format!("%dw 2.0\noutput {mime}\n---\n\"2026-10-26\" as Date + |P3D|"),
                "payload": null
            })),
            json!("2026-10-29")
        );
    }
}

#[wasm_bindgen_test]
fn native_temporals_are_encoded_only_for_explicit_typed_transport() {
    let script = r#"{date: "2026-10-26" as Date + |P3D|, nanos: |12:34:56.123456789|.nanoseconds}"#;
    let mut request = json!({"script": script, "payload": null, "render_output": false});
    assert_eq!(
        execute(request.clone()),
        json!({"date":"2026-10-29","nanos":123456789})
    );
    request["typed_transport"] = json!(true);
    let encoded = execute(request);
    assert_eq!(
        encoded["date"],
        json!({"__dwpy_temporal":"date","value":"2026-10-29"})
    );
    assert_eq!(encoded["nanos"], json!(123456789));

    let object = json!({"__dwpy_temporal":"date","value":"ordinary user data"});
    assert_eq!(
        execute(json!({"script":"payload", "payload":object, "render_output":false})),
        object
    );
}

#[wasm_bindgen_test]
fn object_selector_preserves_markdown_newlines_and_unicode() {
    assert_eq!(
        execute(json!({
            "script": "%dw 2.0\noutput text/plain\n---\n{content: \"**café** — · 😀\\n\\nnext\"}.content",
            "payload": {}
        })),
        json!("**café** — · 😀\n\nnext")
    );
}

#[wasm_bindgen_test]
fn ndjson_data_errors_preserve_record_lines_and_skip_invalid_option() {
    for format in ["ndjson", "application/x-ndjson", "application/x-ldjson"] {
        let mut request = json!({
            "script": "payload", "payload": "{\"id\":1}\ninvalid\n{\"id\":2}\n",
            "payload_format": format, "render_output": false
        });
        let error = error_message(run_dataweave_request(&request.to_string()).unwrap_err());
        assert!(
            error.starts_with("Invalid JSON value: Invalid NDJSON record on line 2:"),
            "{error}"
        );
        request["payload_format_options"] = json!({"skipInvalid": true});
        assert_eq!(execute(request), json!([{"id": 1}, {"id": 2}]));

        let request =
            json!({"script": format!("read(payload, \"{format}\")"), "payload": "{}\ninvalid"});
        let error = error_message(run_dataweave_request(&request.to_string()).unwrap_err());
        assert!(
            error.contains("Invalid NDJSON record on line 2:"),
            "{error}"
        );
        assert!(!error.contains("DataWeave parse error:"), "{error}");
    }
}

#[wasm_bindgen_test]
fn request_validation_reports_js_errors_without_trapping() {
    for (request, expected) in [
        (json!([]), "request must be a JSON object"),
        (json!({"script": 42}), "expected a 'script' string"),
        (
            json!({"script": "payload", "properties": {"env": 42}}),
            "property 'env' must be a string",
        ),
    ] {
        let error = error_message(run_dataweave_request(&request.to_string()).unwrap_err());
        assert!(error.contains(expected), "{error}");
    }
    let error = error_message(analyze_dataweave_request("{}").unwrap_err());
    assert!(error.contains("expected an 'expression' string"), "{error}");
}

#[wasm_bindgen_test]
fn infix_functions_pass_analysis_before_execution() {
    // A caller resolves references before executing. Output-only tests missed
    // the bug where joinBy was incorrectly returned as a data dependency.
    for expression in [
        r#"[1,2,3] joinBy ", ""#,
        r#"joinBy([1,2,3], ", ")"#,
        r#"[1,2,3] joinBy(", ")"#,
        "([1,2,3]\n/* separator */ joinBy \", \")",
    ] {
        let result = analyze(json!({"expression": expression, "payload": {}}));
        assert_eq!(result["references"], json!([]), "{expression}");
        assert_eq!(result["unresolvedReferences"], json!([]), "{expression}");
        assert_eq!(
            execute(json!({"script": expression, "payload": {}})),
            json!("1, 2, 3")
        );
    }
}

#[wasm_bindgen_test]
fn analysis_keeps_real_dependencies_and_fields_named_after_functions() {
    let result = analyze(json!({
        "expression": "payload.names joinBy payload.separator",
        "payload": {"names": ["a", "b"], "separator": ", "}
    }));
    assert_eq!(
        result["references"],
        json!(["payload.names", "payload.separator"])
    );
    assert_eq!(result["unresolvedReferences"], json!([]));

    let result = analyze(json!({"expression": "payload.missing joinBy \", \"", "payload": {}}));
    assert_eq!(result["unresolvedReferences"], json!(["payload.missing"]));

    let result = analyze(json!({"expression": "joinBy default []", "payload": {}}));
    assert_eq!(result["references"], json!(["joinBy"]));
    assert_eq!(result["unresolvedReferences"], json!(["joinBy"]));
}

#[wasm_bindgen_test]
fn analysis_infers_context_types() {
    let result = analyze(
        json!({"expression": "attributes.headers.x", "attributes": {"headers": {"x": "yes"}}}),
    );
    assert_eq!(result["inferredType"], json!({"kind": "String"}));
    let result = analyze(json!({"expression": "p(\"env\")"}));
    assert_eq!(
        result["inferredType"],
        json!({"kind": "Union", "options": [{"kind": "String"}, {"kind": "Null"}]})
    );
}

#[wasm_bindgen_test]
fn hmac_jwt_matches_the_documented_token() {
    assert_eq!(execute(json!({
        "script": "%dw 2.0\nimport * from dw::util::jwt::HMAC\n---\nJWT({firstName: \"Michael\", lastName: \"Jones\"}, vars.key)",
        "vars": {"key": "01234567890123456789012345678901"}
    })), json!("eyJhbGciOiAiSFMyNTYiLCJ0eXAiOiAiSldUIn0.eyJmaXJzdE5hbWUiOiAiTWljaGFlbCIsImxhc3ROYW1lIjogIkpvbmVzIn0.JaEz34HXuF4fWg5jM2U9JRnX4vu5twoeDRRg9_anWMc"));
}

#[wasm_bindgen_test]
fn rsa_jwt_uses_wasm_randomness_and_matches_openssl_signature() {
    // RS256 is deterministic; the expected signature was generated with
    // `openssl dgst -sha256 -sign` using this repository's test-only key.
    // Signing also exercises RSA blinding's getrandom JavaScript integration.
    let key = include_str!("../../../tests/fixtures/jwt/test-only-rsa-pkcs8.pem");
    let expected: Value = serde_json::from_str(include_str!("fixtures/rsa-jwt.json")).unwrap();
    assert_eq!(
        execute(json!({
            "script": "%dw 2.0\nimport * from dw::util::jwt::RSA\ninput key application/json\n---\nJWT({sub: \"test\"}, key)",
            "vars": {"key": key}
        })),
        expected["RS256"]
    );
}

#[wasm_bindgen_test]
fn debugger_shared_protocol_transcript() {
    run_debugger_transcript(
        include_str!("../../../tests/fixtures/debugger/session.json"),
        json!([2, 4, 6]),
    );
}
#[wasm_bindgen_test]
fn debugger_m1_protocol_transcript() {
    run_debugger_transcript(
        include_str!("../../../tests/fixtures/debugger/m1.json"),
        json!({"result":{"k2":"3"},"filtered":[2,3],"replaced":"a10b221","closure":5}),
    );
}
#[wasm_bindgen_test]
fn debugger_m2_protocol_transcript() {
    run_debugger_transcript(
        include_str!("../../../tests/fixtures/debugger/m2.json"),
        serde_json::from_str(include_str!(
            "../../../tests/fixtures/debugger/m2.expected.json"
        ))
        .unwrap(),
    );
}
fn run_debugger_transcript(source: &str, expected: Value) {
    let requests: Vec<Value> = serde_json::from_str(source).unwrap();
    let mut adapter = dwpy_wasm::DataWeaveDebugAdapter::new();
    for request in requests {
        let messages: Vec<Value> =
            serde_json::from_str(&adapter.dispatch(&request.to_string()).unwrap()).unwrap();
        assert_eq!(messages[0]["success"], true, "{messages:?}");
        if request["command"] == "evaluate" {
            assert_eq!(messages[0]["body"]["result"], "2");
        }
        if request["command"] == "configurationDone" || request["command"] == "stepIn" {
            let mut stopped = false;
            for _ in 0..1000 {
                let messages: Vec<Value> = serde_json::from_str(&adapter.poll(7).unwrap()).unwrap();
                if messages.iter().any(|m| m["event"] == "stopped") {
                    stopped = true;
                    break;
                }
            }
            assert!(stopped);
        }
        if request["command"] == "continue" {
            let mut result = None;
            let mut terminated = false;
            for _ in 0..1000 {
                let messages: Vec<Value> = serde_json::from_str(&adapter.poll(7).unwrap()).unwrap();
                for message in messages {
                    if message["body"]["data"]["kind"] == "result" {
                        result = Some(message["body"]["data"]["result"].clone());
                    }
                    if message["event"] == "terminated" {
                        terminated = true;
                    }
                }
                if terminated {
                    break;
                }
            }
            assert!(terminated);
            assert_eq!(
                serde_json::from_str::<Value>(result.unwrap().as_str().unwrap()).unwrap(),
                expected
            );
        }
    }
}
