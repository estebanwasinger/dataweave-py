use dwpy_core::completion::{complete, complete_fields, CompletionItem};
use dwpy_core::{parse_native_payload_format_with_options, DwValue};

fn fields(
    expression: &str,
    input: &str,
    format: &str,
    options: serde_json::Value,
) -> Vec<CompletionItem> {
    complete_fields(expression, 0, expression.encode_utf16().count(), |_| {
        parse_native_payload_format_with_options(
            DwValue::String(input.into()),
            Some(format),
            Some(&options.into()),
        )
        .ok()
    })
    .unwrap()
}

fn at(source: &str) -> Vec<CompletionItem> {
    let offset = source.find("<cursor>").unwrap();
    let line = source[..offset].bytes().filter(|b| *b == b'\n').count();
    let column = source[..offset]
        .rsplit('\n')
        .next()
        .unwrap()
        .encode_utf16()
        .count();
    complete(&source.replace("<cursor>", ""), line, column)
}

fn declared_at(source: &str) -> Vec<CompletionItem> {
    at(source)
        .into_iter()
        .filter(|i| !i.sort_text.starts_with("4_") && !i.sort_text.starts_with("3_"))
        .collect()
}

#[test]
fn overloaded_header_functions_survive_an_incomplete_body() {
    let items = declared_at("%dw 2.0\noutput application/json\nfun multiply(number, multiplier : Number) = number * multiplier * 2\nfun multiply(number, multiplier : String) = number * multiplier as Number\nfun multiply(number) = multiply(number,number)\n---\n{ result: m<cursor>");
    assert_eq!(items.len(), 3);
    assert!(items
        .iter()
        .all(|i| i.label == "multiply" && i.kind == "function"));
    assert_eq!(items[0].insert_text, "multiply(${1:number})");
    assert!(items
        .iter()
        .any(|i| i.detail.contains("multiplier : Number")
            && i.insert_text == "multiply(${1:number}, ${2:multiplier})"));
    assert!(items
        .iter()
        .any(|i| i.detail.contains("multiplier : String")));
}

#[test]
fn comments_multiline_generics_and_nested_defaults_use_lexer_boundaries() {
    let items = declared_at("%dw 2.0\n/*\n---\nfun imaginary() = 0\n*/\nvar label = \"---\"\nfun identity<T>(\n first: Array<T>, /* comma , and ) */\n second = { items: [1,2], text: \"a,b\" }\n): Array<T> = first\n---\ni<cursor>");
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].label, "identity");
    assert_eq!(items[0].insert_text, "identity(${1:first}, ${2:second})");
    assert!(items[0].detail.ends_with(": Array<T>"));
}

#[test]
fn suggestions_do_not_execute_initializers_or_parse_the_body() {
    let items = declared_at("%dw 2.0\nvar message: String = fail(\"must not execute\")\nfun make(value) = log(value)\n---\nm<cursor> + \"unfinished");
    assert_eq!(items.len(), 2);
    assert!(items
        .iter()
        .any(|i| i.label == "message" && i.kind == "variable"));
}

#[test]
fn incomplete_function_body_keeps_its_signature() {
    let items = declared_at("%dw 2.0\nfun multiply(value) = m<cursor> + \"unfinished");
    assert_eq!(items[0].label, "multiply");
}

#[test]
fn parameters_are_scoped_to_their_function_body_and_shadow_globals() {
    let source = "%dw 2.0\nvar number = 0\nfun first(number) = n<cursor>\nfun second(other) = other\n---\nnull";
    let items = declared_at(source);
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].detail, "Function parameter");
    let body_items = declared_at(&source.replace("<cursor>", "").replace("null", "n<cursor>"));
    assert_eq!(body_items.len(), 1);
    assert_eq!(body_items[0].detail, "Script variable");
    assert!(declared_at(
        "%dw 2.0\nfun first(number) = number\nfun second(other) = n<cursor>\n---\nnull"
    )
    .is_empty());
}

#[test]
fn completion_in_existing_call_does_not_duplicate_arguments() {
    let items = declared_at("%dw 2.0\nfun multiply(number) = number\n---\nmu<cursor>ltiply(4)");
    assert_eq!(items[0].insert_text, "multiply");
    assert_eq!(items[0].insert_text_format, "plain");
}

#[test]
fn utf16_and_crlf_positions_match_monaco() {
    let items = declared_at(
        "%dw 2.0\r\nfun multiply(number) = number\r\n---\r\n{ emoji: \"😀\", result: m<cursor> }",
    );
    assert_eq!(items[0].label, "multiply");
    // Oversized columns clamp at the end of their own line.
    let source = "%dw 2.0\nfun multiply(n) = n\n---\nm\nother";
    assert_eq!(complete(source, 3, 100)[0].label, "multiply");
}

#[test]
fn selectors_do_not_offer_global_functions() {
    assert!(at("%dw 2.0\nfun multiply(n) = n\n---\npayload.m<cursor>").is_empty());
    assert!(at("%dw 2.0\nfun multiply(n) = n\n---\nMath::m<cursor>").is_empty());
}

#[test]
fn context_roots_are_available_without_declarations_or_input_values() {
    let items = declared_at("%dw 2.0\n---\n<cursor>");
    let labels: std::collections::BTreeSet<_> = items.iter().map(|i| i.label.as_str()).collect();
    assert_eq!(
        labels,
        ["attributes", "payload", "vars"].into_iter().collect()
    );
    assert!(items.iter().all(|i| i.kind == "variable"
        && i.detail == "Runtime context"
        && i.insert_text == i.label
        && i.insert_text_format == "plain"));
    for (prefix, name) in [("pa", "payload"), ("at", "attributes"), ("va", "vars")] {
        let items = declared_at(&format!("%dw 2.0\n---\n{prefix}<cursor>"));
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].label, name);
    }
}

#[test]
fn context_roots_respect_shadowing_and_stay_out_of_field_completions() {
    let items = declared_at("%dw 2.0\nfun f(payload) = pa<cursor>\n---\nnull");
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].label, "payload");
    assert_eq!(items[0].detail, "Function parameter");
    let items = declared_at("%dw 2.0\ninput payload application/json\n---\npa<cursor>");
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].detail, "Script variable");
    assert!(at("payload.<cursor>").is_empty());
    assert!(at("attributes.a<cursor>").is_empty());
}

#[test]
fn csv_fields_support_array_projection_indexing_options_and_quoted_names() {
    let input = "name;full name;city\nAda;Ada Lovelace;London\nGrace;Grace Hopper;New York";
    let options = serde_json::json!({"separator": ";"});
    for expression in ["payload.", "payload[0].", "payload[-1]."] {
        let items = fields(expression, input, "application/csv", options.clone());
        assert_eq!(
            items.iter().map(|i| i.label.as_str()).collect::<Vec<_>>(),
            ["name", "full name", "city"]
        );
        assert_eq!(items[1].insert_text, "\"full name\"");
    }
    assert_eq!(
        fields("payload[1].ci", input, "text/csv", options)
            .iter()
            .map(|i| i.label.as_str())
            .collect::<Vec<_>>(),
        ["city"]
    );
    assert!(fields(
        "payload.",
        "Ada,London",
        "application/csv",
        serde_json::json!({"header": false})
    )
    .is_empty());
}

#[test]
fn structured_formats_share_runtime_readers_and_selector_semantics() {
    for (input, format, path, names) in [
        (
            r#"[{"name":"Ada"},{"city":"London"}]"#,
            "application/json",
            "payload.",
            vec!["name", "city"],
        ),
        (
            "- name: Ada\n- city: London",
            "application/yaml",
            "payload.",
            vec!["name", "city"],
        ),
        (
            "{\"name\":\"Ada\"}\n{\"city\":\"London\"}",
            "application/x-ndjson",
            "payload.",
            vec!["name", "city"],
        ),
        (
            "<root><row><name>Ada</name></row><row><city>London</city></row></root>",
            "application/xml",
            "payload.root.row.",
            vec!["name", "city"],
        ),
        (
            "| name | city |\n| --- | --- |\n| Ada | London |",
            "text/markdown",
            "payload.",
            vec!["name", "city"],
        ),
        (
            r#"{"row":{"name":"Ada"},"row":{"city":"London"}}"#,
            "application/json",
            "payload.*row.",
            vec!["name", "city"],
        ),
    ] {
        let items = fields(path, input, format, serde_json::json!({}));
        assert_eq!(
            items.iter().map(|i| i.label.as_str()).collect::<Vec<_>>(),
            names,
            "{format}"
        );
        assert!(items.iter().all(|i| i.kind == "field"));
    }
}

#[test]
fn malformed_or_scalar_inputs_and_effectful_selector_expressions_have_no_fields() {
    assert!(fields(
        "payload.",
        "{bad",
        "application/json",
        serde_json::json!({})
    )
    .is_empty());
    assert!(fields("payload.", "hello", "text/plain", serde_json::json!({})).is_empty());
    assert!(fields("payload.", "42", "application/json", serde_json::json!({})).is_empty());
    let items = complete_fields("payload[log(1)].", 0, 16, |_| {
        panic!("must not inspect input for an expression requiring evaluation")
    });
    assert!(items.is_none());
}

#[test]
fn callback_references_complete_input_fields_without_running_mappers() {
    let input = r#"[{"name":"Esteban","lastName":"Wasinger","address":{"city":"BA"}}, {"onlySecond":true}]"#;
    for expression in [
        "payload map $.",
        "payload filter $.",
        "payload map ($.)",
        "payload map ((item, index) -> item.)",
        "payload map ((item) -> $.)",
        "map(payload, $.)",
        "map(payload, (item) -> item.)",
        "payload filter ($.name != null) map $.",
        "%dw 2.0\n---\npayload map /* $.notReal */ { value: $. }",
    ] {
        let dot = expression.rfind('.').unwrap() + 1;
        let line = expression[..dot].bytes().filter(|c| *c == b'\n').count();
        let column = expression[..dot]
            .rsplit('\n')
            .next()
            .unwrap()
            .encode_utf16()
            .count();
        let items = complete_fields(expression, line, column, |_| {
            parse_native_payload_format_with_options(
                DwValue::String(input.into()),
                Some("application/json"),
                None,
            )
            .ok()
        })
        .unwrap_or_default();
        assert_eq!(
            items.iter().map(|i| i.label.as_str()).collect::<Vec<_>>(),
            ["name", "lastName", "address", "onlySecond"],
            "{expression}"
        );
    }
    assert_eq!(
        fields(
            "payload map $.address.c",
            input,
            "application/json",
            serde_json::json!({})
        )[0]
        .label,
        "city"
    );
    let csv = fields(
        "payload map $.",
        "name,lastName\nEsteban,Wasinger",
        "application/csv",
        serde_json::json!({}),
    );
    assert_eq!(
        csv.iter().map(|i| i.label.as_str()).collect::<Vec<_>>(),
        ["name", "lastName"]
    );
}

#[test]
fn callback_references_respect_nested_scopes_and_sibling_boundaries() {
    let input = r#"[{"parentOnly":1,"children":[{"childOnly":true},{"otherChild":true}]}]"#;
    for expression in [
        "payload map ($.children map $.)",
        "payload map ((parent) -> parent.children map $.)",
        "payload map ((item) -> item.children map ((item) -> item.))",
        "payload map $.children map $.",
    ] {
        let dot = expression.rfind('.').unwrap() + 1;
        let items = complete_fields(expression, 0, dot, |_| {
            serde_json::from_str::<serde_json::Value>(input)
                .ok()
                .map(Into::into)
        })
        .unwrap_or_default();
        let labels: Vec<_> = items.iter().map(|i| i.label.as_str()).collect();
        // Mapping children produces arrays of children; the direct nested map
        // receives child objects, while projecting arrays preserves their fields.
        assert_eq!(labels, ["childOnly", "otherChild"], "{expression}");
    }
    for expression in [
        "$.",
        "[payload map $.name, $.]",
        "[payload map ((item) -> item.name), item.]",
        "payload map ($$.)",
        "unknown map $.",
        "fail(\"never run\") map $.",
        "payload map log($) map $.",
        "%dw 2.0\nvar names = payload map $.name\n---\n$.",
        "payload map ((item) -> custom((item) -> item.))",
    ] {
        let dot = expression.rfind('.').unwrap() + 1;
        let line = expression[..dot].bytes().filter(|c| *c == b'\n').count();
        let column = expression[..dot].rsplit('\n').next().unwrap().len();
        let items = complete_fields(expression, line, column, |_| {
            serde_json::from_str::<serde_json::Value>(input)
                .ok()
                .map(Into::into)
        })
        .unwrap_or_default();
        assert!(items.is_empty(), "{expression}: {items:?}");
    }
}

#[test]
fn callback_fields_preserve_native_shapes_and_lexical_shadowing() {
    let items = fields(
        "payload mapObject $.",
        r#"{"a":{"name":"E"},"b":{"lastName":"W"}}"#,
        "application/json",
        serde_json::json!({}),
    );
    assert_eq!(
        items.iter().map(|i| i.label.as_str()).collect::<Vec<_>>(),
        ["name", "lastName"]
    );
    let items = fields(
        "payload.root.row map $.",
        "<root><row><name>E</name><lastName>W</lastName></row><row><name>E</name></row></root>",
        "application/xml",
        serde_json::json!({}),
    );
    assert_eq!(
        items.iter().map(|i| i.label.as_str()).collect::<Vec<_>>(),
        ["name", "lastName"]
    );
    let source = "%dw 2.0\r\n---\r\n{emoji: \"😀\", names: payload map (( /* argument */ item: Object) -> item.la";
    let column = source.rsplit('\n').next().unwrap().encode_utf16().count();
    let items = complete_fields(source, 2, column, |_| {
        Some(serde_json::json!([{"lastName":"W"}]).into())
    })
    .unwrap();
    assert_eq!(items[0].label, "lastName");
    for source in [
        "%dw 2.0\nfun map(value, callback) = null\n---\npayload map $.",
        "payload map ((item) -> do { var item = 42\n---\nitem.",
        "payload map ((item) -> do { fun inner(item) = item.\n---\ninner(null) })",
    ] {
        let cursor = source.rfind('.').unwrap() + 1;
        let line = source[..cursor].bytes().filter(|b| *b == b'\n').count();
        let column = source[..cursor].rsplit('\n').next().unwrap().len();
        let items = complete_fields(source, line, column, |_| {
            panic!("shadowed or unknown values must not inspect payload")
        })
        .unwrap_or_default();
        assert!(items.is_empty(), "{source}");
    }
}

#[test]
fn inline_literal_selectors_complete_static_structure_without_input_or_execution() {
    for (source, expected) in [
        ("{\n    \"name\" : \"name\"\n}.", vec!["name"]),
        (
            "({name: fail(\"must not run\"), lastName: log(1)}).",
            vec!["name", "lastName"],
        ),
        ("{'full name': \"name\", bareKey: 42}.f", vec!["full name"]),
        (
            "{person: {name: \"E\", address: {city: \"BA\"}}}.person.",
            vec!["name", "address"],
        ),
        (
            "(({person: {address: {city: \"BA\"}}})).person.address.",
            vec!["city"],
        ),
        (
            "[{name: \"E\"}, {lastName: \"W\"}].",
            vec!["name", "lastName"],
        ),
        ("[{name: \"E\"}, {lastName: \"W\"}][0].", vec!["name"]),
        ("[{name: \"E\"}] map $.", vec!["name"]),
        (
            "{ rows: [{name: \"E\"}, {lastName: \"W\"}] }.rows.",
            vec!["name", "lastName"],
        ),
        (
            "{ entry: {first: 1}, entry: {second: 2} }.*entry.",
            vec!["first", "second"],
        ),
        (
            "{/* misleading: {extra: 1} */ name: \"a:b,c\"}.",
            vec!["name"],
        ),
        (
            r#"{"$(fail('never'))": 1, known: fail('never')}."#,
            vec!["known"],
        ),
    ] {
        let line = source.bytes().filter(|b| *b == b'\n').count();
        let column = source.rsplit('\n').next().unwrap().encode_utf16().count();
        let items = complete_fields(source, line, column, |_| {
            panic!("literal completion must not request inputs")
        })
        .unwrap_or_default();
        assert_eq!(
            items.iter().map(|i| i.label.as_str()).collect::<Vec<_>>(),
            expected,
            "{source}"
        );
        if let Some(item) = items.iter().find(|i| i.label == "full name") {
            assert_eq!(item.insert_text, "\"full name\"");
        }
    }
    for source in [
        "fail({secret: 1}).",
        "payload[{secret: 1}].",
        "{name: fail({secret: 1})}.name.",
    ] {
        let items = complete_fields(source, 0, source.len(), |_| None).unwrap_or_default();
        assert!(items.is_empty(), "{source}: {items:?}");
    }
}

#[test]
fn then_binds_the_whole_input_and_preserves_projected_result_shapes() {
    for (source, expected) in [
        ("%dw 2.0\nimport hashWith from dw::Crypto\noutput application/json\n---\n{\n    \"name\" : \"name\"\n} then $.", vec!["name"]),
        ("{name: \"E\", nested: {city: \"BA\"}} then $.", vec!["name", "nested"]),
        ("{name: \"E\"} then ((value) -> value.)", vec!["name"]),
        ("then({name: \"E\"}, $.)", vec!["name"]),
        ("{person: {name: \"E\"}} then $.person then $.", vec!["name"]),
        ("{name: \"E\"} then {renamed: fail(\"never\")} then $.", vec!["renamed"]),
        ("[{name: \"E\"}] map ($ then $.)", vec!["name"]),
    ] {
        let cursor = source.rfind('.').unwrap() + 1;
        let line = source[..cursor].bytes().filter(|b| *b == b'\n').count();
        let column = source[..cursor].rsplit('\n').next().unwrap().encode_utf16().count();
        let items = complete_fields(source, line, column, |_| panic!("static then completion must not inspect inputs")).unwrap_or_default();
        assert_eq!(items.iter().map(|i| i.label.as_str()).collect::<Vec<_>>(), expected, "{source}");
    }
    let items = fields(
        "payload then $.",
        r#"{"name":"E","nested":{"city":"BA"}}"#,
        "application/json",
        serde_json::json!({}),
    );
    assert_eq!(
        items.iter().map(|i| i.label.as_str()).collect::<Vec<_>>(),
        ["name", "nested"]
    );
    for source in [
        "[{name: \"E\"} then $.name, $.]",
        "{name: \"E\"} then $$.",
        "fail(\"never\") then $.",
    ] {
        let column = source.rfind('.').unwrap() + 1;
        assert!(
            complete_fields(source, 0, column, |_| None)
                .unwrap_or_default()
                .is_empty(),
            "{source}"
        );
    }
}
