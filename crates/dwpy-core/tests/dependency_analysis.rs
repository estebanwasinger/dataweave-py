use dwpy_core::{analyze_expression, execute_smoke};
use serde_json::{json, Value};

#[test]
fn audited_executable_expressions_have_exact_dependencies() {
    let cases: Value = serde_json::from_str(include_str!(
        "../../../tests/fixtures/dependency-analysis/gaps.json"
    ))
    .unwrap();
    for case in cases.as_array().unwrap() {
        let id = case["id"].as_str().unwrap();
        let expression = case["request"]["expression"].as_str().unwrap();
        let payload = case["request"]["payload"].clone();
        assert_eq!(
            execute_smoke(expression, payload.clone()).expect(id),
            case["expectedResult"],
            "{id}"
        );
        let analysis = analyze_expression(expression, Some(payload), None).expect(id);
        assert_eq!(analysis["references"], case["expectedReferences"], "{id}");
        assert_eq!(
            analysis["unresolvedReferences"], case["expectedUnresolvedReferences"],
            "{id}"
        );
    }
}

#[test]
fn lexical_bindings_do_not_hide_references_in_sibling_expressions_or_defaults() {
    for (expression, references) in [
        (
            "[payload.items map ((item) -> item.price), item.price]",
            json!(["item.price", "payload.items"]),
        ),
        ("[using (x = record.a) x, x]", json!(["record.a", "x"])),
        (
            "[record.status match { case var n when n > 0 -> n else -> 0 }, n]",
            json!(["n", "record.status"]),
        ),
        (
            "[record update { case n at .name -> n ++ record.suffix }, n]",
            json!(["n", "record", "record.suffix"]),
        ),
        (
            "%dw 2.0\nfun f(x = payload.default) = x\n---\nf()",
            json!(["payload.default"]),
        ),
        (
            "[1] map ((x = record.default) -> x + record.offset)",
            json!(["record.default", "record.offset"]),
        ),
        (
            "[update, mask, replace, matches, with, String]",
            json!(["String", "mask", "matches", "replace", "update", "with"]),
        ),
        (
            "[payload.String, payload.update, payload.mask]",
            json!(["payload.String", "payload.mask", "payload.update"]),
        ),
    ] {
        let analysis = analyze_expression(expression, Some(json!({})), None).expect(expression);
        assert_eq!(analysis["references"], references, "{expression}");
    }
}

#[test]
fn interpolation_and_dynamic_selectors_keep_external_inputs() {
    for (expression, references, wildcards) in [
        (
            r#""literal payload.fake $(record.price * record.quantity)""#,
            json!(["record.price", "record.quantity"]),
            json!([]),
        ),
        (
            r#""escaped \$(record.fake) $(record.real)""#,
            json!(["record.real"]),
            json!([]),
        ),
        (
            r#"record[record.selector] ++ "record[unused]""#,
            json!(["record", "record.selector"]),
            json!(["record"]),
        ),
        (
            r#"record.value ++ "record.value[unused]""#,
            json!(["record.value"]),
            json!([]),
        ),
        ("payload.*name", json!(["payload.name"]), json!([])),
        ("payload..name", json!(["payload"]), json!(["payload"])),
        (
            r#"payload contains /[a-z]+/"#,
            json!(["payload"]),
            json!([]),
        ),
        (
            "payload / record.divisor / 2",
            json!(["payload", "record.divisor"]),
            json!([]),
        ),
    ] {
        let analysis = analyze_expression(expression, Some(json!({})), None).expect(expression);
        assert_eq!(analysis["references"], references, "{expression}");
        assert_eq!(analysis["wildcardReferences"], wildcards, "{expression}");
    }
}

#[test]
fn type_inference_handles_complete_coercion_and_spread_expressions() {
    for (expression, kind) in [
        (r#""2" as Number + 3"#, "Number"),
        (r#"("12" as Number) / 3"#, "Number"),
        ("{(payload)}", "Object"),
        ("{a: payload.value}", "Object"),
    ] {
        let result =
            analyze_expression(expression, Some(json!({"value": 1})), None).expect(expression);
        // Unsupported inference remains an explicit warning; it is not a parse error.
        assert!(
            result["inferredType"]["kind"] == kind
                || !result["diagnostics"].as_array().unwrap().is_empty(),
            "{expression}: {result}"
        );
    }
}

#[test]
fn addition_and_concatenation_have_distinct_inferred_types() {
    for (expression, kind) in [
        (r#""2" as Number + 3"#, "Number"),
        ("1 + 2", "Number"),
        ("1 ++ 2", "String"),
        (r#""a" ++ "b""#, "String"),
        ("[1] + 2", "Array"),
        ("[1] ++ [2]", "Array"),
    ] {
        let result = analyze_expression(expression, None, None).expect(expression);
        assert_eq!(
            result["inferredType"]["kind"], kind,
            "{expression}: {result}"
        );
        assert_eq!(result["diagnostics"], json!([]), "{expression}: {result}");
    }
}

#[test]
fn malformed_source_is_rejected_without_evaluating_valid_headers() {
    for expression in [r#""unfinished"#, "{a 1}", "payload +", "payload["] {
        assert!(
            analyze_expression(expression, Some(json!({})), None).is_err(),
            "{expression}"
        );
    }
    let result = analyze_expression(
        "%dw 2.0\nvar unused = fail(\"must not run\")\n---\npayload.value",
        Some(json!({"value": 1})),
        None,
    )
    .unwrap();
    assert_eq!(result["references"], json!(["payload.value"]));
}

#[test]
fn context_roots_are_resolved_independently_and_null_fields_exist() {
    let result = dwpy_core::analyze_expression_with_context(
        "[vars, vars.present, vars.missing, attributes, attributes.missing]",
        None,
        Some(json!({"present": null})),
        Some(json!({})),
    )
    .unwrap();
    assert_eq!(
        result["unresolvedReferences"],
        json!(["attributes.missing", "vars.missing"])
    );
}

#[test]
fn guards_defaults_and_type_syntax_keep_their_lexical_boundaries() {
    for (expression, references) in [
        ("payload update {case name at .name if(record.flag) -> if (name == null) record.default else name}", json!(["payload", "record.default", "record.flag"])),
        ("using (x=record.a, y=x+record.b) {x:x,y:y}", json!(["record.a", "record.b"])),
        ("%dw 2.0\nfun f(a,b=log(a+record.delta))=a+b\n---\nf(record.base)", json!(["record.base", "record.delta"])),
        ("null match {case is Number | Null -> 3 else -> 4}", json!([])),
        ("%dw 2.0\nfun count<T>(n:T)=if(n<=0) 0 else 1+count<T>(n-1)\n---\ncount(4)", json!([])),
        ("output application/json\npayload.value", json!(["payload.value"])),
        ("{name @(mail:record.email): record.value}", json!(["record.email", "record.value"])),
        ("[payload.., payload.*]", json!(["payload"])),
    ] {
        let result = analyze_expression(expression, Some(json!({})), None).expect(expression);
        assert_eq!(result["references"], references, "{expression}");
    }
}

#[test]
fn function_values_and_xml_selectors_resolve_against_the_right_context() {
    let result = analyze_expression("location(sqrt)", Some(json!({})), None).unwrap();
    assert_eq!(result["references"], json!([]));
    let result = analyze_expression("location(sqrt)", Some(json!({"sqrt": 1})), None).unwrap();
    assert_eq!(result["references"], json!(["sqrt"]));
    let result = analyze_expression(
        "payload.root.name.@a",
        Some(json!({"root": {"name": {"__dwpy_xml_list": [{"@a": "b"}, "John"]}}})),
        None,
    )
    .unwrap();
    assert_eq!(result["references"], json!(["payload.root.name.@a"]));
    assert_eq!(result["unresolvedReferences"], json!([]));
}

#[test]
fn keyword_named_fields_inside_guards_remain_dependencies() {
    let result = analyze_expression(
        "payload update {case x at .name if (at > 1) -> x ++ with}",
        Some(json!({})),
        None,
    )
    .unwrap();
    assert_eq!(result["references"], json!(["at", "payload", "with"]));
    let result = analyze_expression(
        "record match {case var x when when > 0 -> x else -> 0}",
        Some(json!({})),
        None,
    )
    .unwrap();
    assert_eq!(result["references"], json!(["record", "when"]));
}

#[test]
fn input_declarations_preserve_their_underlying_context_dependencies() {
    let result = analyze_expression(
        "%dw 2.0\ninput payload application/json\n---\npayload.name",
        Some(json!({"name": "ana"})),
        None,
    )
    .unwrap();
    assert_eq!(result["references"], json!(["payload.name"]));
    assert_eq!(result["unresolvedReferences"], json!([]));
    let result = analyze_expression(
        "%dw 2.0\ninput data application/json\n---\ndata.name",
        Some(json!({})),
        Some(json!({"data": {"name": "ana"}})),
    )
    .unwrap();
    assert_eq!(result["references"], json!(["vars.data"]));
    assert_eq!(result["unresolvedReferences"], json!([]));
}

#[test]
fn builtin_function_aliases_are_symbols_unless_input_data_shadows_them() {
    for expression in [
        "%dw 2.0\nvar mapper = upper\n---\npayload.names map mapper",
        "using (mapper=upper) payload.names map mapper",
        "do {var mapper=upper\n---\npayload.names map mapper}",
    ] {
        let result =
            analyze_expression(expression, Some(json!({"names": ["a"]})), None).expect(expression);
        assert_eq!(result["references"], json!(["payload.names"]));
    }
    let result =
        analyze_expression("using (value=upper) value", Some(json!({"upper": 1})), None).unwrap();
    assert_eq!(result["references"], json!(["upper"]));
}
