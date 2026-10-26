use dwpy_core::{analyze_expression, execute_smoke};
use serde_json::{json, Value};

#[test]
fn range_operators_pass_dependency_analysis_before_execution() {
    for (expression, expected) in [
        ("1 to 10 map $", json!([1, 2, 3, 4, 5, 6, 7, 8, 9, 10])),
        (
            "{values: (1 to 3 map ($ * 2))}",
            json!({"values": [2, 4, 6]}),
        ),
        ("(1\n/* bounds */ to\n3) map $", json!([1, 2, 3])),
        ("(3 to -1) map $", json!([3, 2, 1, 0, -1])),
        ("[1, 2, 3][0 to 1]", json!([1, 2])),
        ("7 mod 4", json!(3)),
        ("2 pow 3", json!(8)),
    ] {
        let analysis = analyze_expression(expression, Some(json!({})), None).unwrap();
        assert_eq!(analysis["references"], json!([]), "{expression}");
        assert_eq!(analysis["unresolvedReferences"], json!([]), "{expression}");
        assert_eq!(
            execute_smoke(expression, Value::Null).expect(expression),
            expected,
            "{expression}"
        );
    }
}

#[test]
fn range_analysis_preserves_actual_bounds_and_operator_named_fields() {
    let analysis = analyze_expression(
        "record.start to record.end map $",
        Some(json!({"record": {"start": 1, "end": 3}})),
        None,
    )
    .unwrap();
    assert_eq!(
        analysis["references"],
        json!(["record.end", "record.start"])
    );
    assert_eq!(analysis["unresolvedReferences"], json!([]));

    for expression in [
        "to default 3",
        "[to, mod, pow]",
        "if (true) to else 3",
        "1 to missing",
        "payload.to",
    ] {
        let analysis = analyze_expression(expression, Some(json!({})), None).unwrap();
        let expected = match expression {
            "[to, mod, pow]" => json!(["mod", "pow", "to"]),
            "1 to missing" => json!(["missing"]),
            "payload.to" => json!(["payload.to"]),
            _ => json!(["to"]),
        };
        assert_eq!(analysis["references"], expected, "{expression}");
        assert_eq!(analysis["unresolvedReferences"], expected, "{expression}");
    }
}
