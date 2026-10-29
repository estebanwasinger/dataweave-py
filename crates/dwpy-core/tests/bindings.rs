use dwpy_core::{compile, DwValue, ExecutionContext, ExecutionOptions};
use serde_json::json;

#[test]
fn bindings_work_in_execution_and_streaming_writer() {
    let script = compile("%dw 2.0\noutput application/json\n---\n{rent: record.rent, tenant: tenant.name, value: vars.x}").unwrap();
    let context = ExecutionContext::new(Some(json!({"x": 2})), None, None)
        .unwrap()
        .with_bindings(DwValue::from(
            json!({"record": {"rent": 5}, "tenant": {"name": "Ada"}}),
        ))
        .unwrap();
    let options = ExecutionOptions::default();
    let result = script
        .execute_with_context(
            json!({}),
            context.clone(),
            &ExecutionOptions {
                render_output: false,
                ..options
            },
        )
        .unwrap();
    assert_eq!(result, json!({"rent": 5, "tenant": "Ada", "value": 2}));
    let mut output = Vec::new();
    script
        .execute_to_writer_with_context(json!({}), context, &options, &mut output)
        .unwrap();
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&output).unwrap(),
        result
    );
}

#[test]
fn rejects_invalid_bindings_and_allows_header_shadowing() {
    for name in [
        "payload",
        "vars",
        "attributes",
        "invalid-name",
        "__dwpy_properties",
    ] {
        let bindings = DwValue::from(json!({name: 1}));
        assert!(
            ExecutionContext::default().with_bindings(bindings).is_err(),
            "{name}"
        );
    }
    assert!(ExecutionContext::default()
        .with_bindings(DwValue::from(json!([])))
        .is_err());
    let context = ExecutionContext::default()
        .with_bindings(DwValue::from(json!({"record": 1, "properties": 4})))
        .unwrap();
    let result = compile("var record = 7\n---\nrecord + properties")
        .unwrap()
        .execute_with_context(
            json!({}),
            context,
            &ExecutionOptions {
                render_output: false,
                ..ExecutionOptions::default()
            },
        )
        .unwrap();
    assert_eq!(result, json!(11));
}
