use chrono::Datelike;
use dwpy_core::{
    compile, execute_json, execute_value, DwValue, ExecutionContext, ExecutionOptions,
};
use serde_json::json;

fn native(expression: &str) -> DwValue {
    execute_value(expression, DwValue::Null, false).unwrap()
}

#[test]
fn literals_and_coercions_have_distinct_native_variants() {
    for expression in ["|2026-10-26|", "\"2026-10-26\" as Date"] {
        let DwValue::Date(value) = native(expression) else {
            panic!("expected Date")
        };
        assert_eq!((value.year(), value.month(), value.day()), (2026, 10, 26));
    }
    assert!(matches!(
        native("|2026-10-26T12:34:56|"),
        DwValue::LocalDateTime(_)
    ));
    assert!(matches!(
        native("|2026-10-26T12:34:56Z|"),
        DwValue::DateTime(_)
    ));
    assert!(matches!(native("|12:34:56|"), DwValue::LocalTime(_)));
    assert!(matches!(native("|12:34:56+02:00|"), DwValue::Time(_)));
    assert!(matches!(native("|P3D|"), DwValue::Period(_)));
    assert!(matches!(
        native("\"2026-10-26T12:34:56\" as DateTime"),
        DwValue::DateTime(_)
    ));
    assert!(matches!(
        native("|2026-10-26T12:34:56Z| as LocalDateTime"),
        DwValue::LocalDateTime(_)
    ));
    assert!(matches!(
        native("|12:34:56Z| as LocalTime"),
        DwValue::LocalTime(_)
    ));
    assert!(matches!(native("now()"), DwValue::DateTime(_)));
    assert!(matches!(native("today()"), DwValue::Date(_)));
    assert!(matches!(
        native("|2026-10-26T12:34:56Z| >> \"+03:00\""),
        DwValue::DateTime(_)
    ));
}

#[test]
fn temporal_values_remain_native_through_scopes_functions_and_collections() {
    let value = native(
        r#"%dw 2.0
var start = "2026-10-26" as Date
fun advance(day) = day + |P3D|
---
{days: [start] map advance($), direct: advance(start), period: |P3D|}"#,
    );
    assert!(matches!(value["direct"], DwValue::Date(_)));
    assert!(matches!(value["days"][0], DwValue::Date(_)));
    assert!(matches!(value["period"], DwValue::Period(_)));
    assert!(!value["direct"].is_object());
    assert!(!value["direct"].is_string());
}

#[test]
fn arithmetic_uses_calendar_months_and_validates_dates() {
    for (expression, expected) in [
        ("|2024-01-31| + |P1M|", "2024-02-29"),
        ("|2024-02-29| + |P1Y|", "2025-02-28"),
        ("|2026-10-26| + |P3D|", "2026-10-29"),
        ("|2026-10-26| - |P3D|", "2026-10-23"),
        (
            "|2026-10-26T23:00:00+03:00| + |PT2H|",
            "2026-10-27T01:00:00+03:00",
        ),
    ] {
        let value = native(expression);
        assert_eq!(value.temporal_text().as_deref(), Some(expected));
    }
    for expression in ["|2026-02-30|", "|25:00:00|", "\"2026-02-30\" as Date"] {
        assert!(
            execute_value(expression, DwValue::Null, false).is_err(),
            "{expression}"
        );
    }
}

#[test]
fn nanoseconds_offsets_comparisons_and_type_checks_use_native_types() {
    let value = native(
        r#"{
        nano: |2026-10-26T12:34:56.123456789Z|.nanoseconds,
        plus: |2026-10-26T12:34:56.123456789Z| + |PT1S|,
        offset: |2026-10-26T12:34:56+03:00|.offsetSeconds,
        same: |2026-10-26T12:00:00Z| == |2026-10-26T15:00:00+03:00|,
        precision: |2026-10-26T12:00:00.000Z| == |2026-10-26T12:00:00Z|,
        later: |2026-10-26T13:00:00Z| > |2026-10-26T15:00:00+03:00|,
        dateType: |2026-10-26| is Date,
        objectType: |2026-10-26| is Object,
        stringType: "2026-10-26" is Date,
        types: [typeOf(|2026-10-26|), typeOf("2026-10-26"), typeOf(|P3D|)]
    }"#,
    );
    assert_eq!(value["nano"], json!(123456789));
    assert_eq!(
        value["plus"].temporal_text().as_deref(),
        Some("2026-10-26T12:34:57.123456789Z")
    );
    assert_eq!(value["offset"], json!(10800));
    for field in ["same", "precision", "later", "dateType"] {
        assert_eq!(value[field], json!(true));
    }
    for field in ["objectType", "stringType"] {
        assert_eq!(value[field], json!(false));
    }
    assert_eq!(value["types"], json!(["Date", "String", "Period"]));
    assert_eq!(native("(|2026-10-26T12:34:56.123456789Z| - |2026-10-26T12:34:56.123456788Z|) as Number {unit: \"nanos\"}"), json!(1));
    assert_eq!(
        native("sizeOf([|2026-10-26T12:00:00Z|, |2026-10-26T15:00:00+03:00|] distinctBy $)"),
        json!(1)
    );
}

#[test]
fn calendar_types_are_inferred() {
    for (expression, expected) in [
        ("|2026-10-26| + |P3D|", "Date"),
        ("\"2026-10-26\" as Date + |P3D|", "Date"),
        ("|P1M|", "Period"),
    ] {
        assert_eq!(
            dwpy_core::infer_type_descriptor(expression, None, None).unwrap()["kind"],
            expected
        );
    }
}

#[test]
fn ordinary_objects_with_wire_keys_never_become_temporal_values() {
    let object = json!({"__dwpy_temporal":"date","value":"2026-10-29"});
    let value = DwValue::from(object.clone());
    assert!(value.is_object());
    assert_eq!(
        execute_value("payload is Object", value.clone(), false).unwrap(),
        DwValue::Bool(true)
    );
    assert_eq!(
        execute_value("payload is Date", value.clone(), false).unwrap(),
        DwValue::Bool(false)
    );
    assert_eq!(
        execute_json("payload", object.clone(), false).unwrap(),
        object
    );
    assert_eq!(
        DwValue::from_transport_json(value.to_transport_json()).unwrap(),
        value
    );
    let from_script = native(r#"{__dwpy_temporal:"date",value:"2026-10-29"}"#);
    assert!(from_script.is_object());
}

#[test]
fn serializers_and_bridge_roundtrips_preserve_the_right_representation() {
    let native = native(
        r#"{date:|2026-10-29|,time:|12:34:56+03:00|,local:|2026-10-29T12:34:56|,period:|P1Y2M|}"#,
    );
    let wire = native.to_transport_json();
    assert_eq!(wire["date"]["__dwpy_temporal"], "date");
    assert_eq!(DwValue::from_transport_json(wire).unwrap(), native);
    let json = serde_json::to_value(&native).unwrap();
    assert_eq!(json["date"], "2026-10-29");
    assert_eq!(json["period"], "P1Y2M");
    assert!(!json.to_string().contains("__dwpy_"));
    for mime in ["text", "text/plain", "plain", "application/json"] {
        let script = format!("%dw 2.0\noutput {mime}\n---\n\"2026-10-26\" as Date + |P3D|");
        let value = execute_value(&script, DwValue::Null, true).unwrap();
        assert!(matches!(value, DwValue::String(_)));
        assert!(!value.as_str().unwrap().contains("__dwpy_"));
    }
}

#[test]
fn compiled_scripts_accept_typed_host_contexts_without_json() {
    let script = compile("payload + vars.delta").unwrap();
    let context = ExecutionContext::from_values(
        Some(DwValue::Object(
            [("delta".into(), native("|P3D|"))].into_iter().collect(),
        )),
        None,
        None,
    )
    .unwrap();
    let value = script
        .execute_value_with_context(
            native("|2026-10-26|"),
            context,
            &ExecutionOptions {
                render_output: false,
                ..Default::default()
            },
        )
        .unwrap();
    assert!(matches!(value, DwValue::Date(_)));
    assert_eq!(value.temporal_text().as_deref(), Some("2026-10-29"));

    let rendered = compile("%dw 2.0\noutput application/json\n---\npayload + |P3D|").unwrap();
    let mut bytes = Vec::new();
    rendered
        .execute_value_to_writer(
            native("|2026-10-26|"),
            None,
            &ExecutionOptions::default(),
            &mut bytes,
        )
        .unwrap();
    assert_eq!(String::from_utf8(bytes).unwrap(), "\"2026-10-29\"");
}

#[test]
fn typed_bridge_rejects_invalid_temporal_data() {
    for value in [
        json!({"__dwpy_temporal":"date","value":"2026-02-30"}),
        json!({"__dwpy_temporal":"date","value":"12:00:00"}),
        json!({"__dwpy_temporal":"unknown","value":"2026-10-26"}),
        json!({"__dwpy_temporal":"date"}),
    ] {
        assert!(DwValue::from_transport_json(value).is_err());
    }
}

#[test]
fn timezone_strings_use_z_for_utc_and_preserve_nonzero_offsets() {
    for (source, expected) in [
        ("Z", "Z"),
        ("+00:00", "Z"),
        ("-03:00", "-03:00"),
        ("+05:30", "+05:30"),
    ] {
        let value = native(&format!("|{source}|"));
        assert!(matches!(value, DwValue::TimeZone(_)));
        assert_eq!(value.temporal_text().as_deref(), Some(expected));
        assert_eq!(value.to_json(), json!(expected));
        assert_eq!(native(&format!("|{source}| as String")), json!(expected));
        let script = format!("import * from dw::util::Coercions\n---\ntoString(|{source}|)");
        assert_eq!(native(&script), json!(expected));
        let script = format!("output text\n---\n|{source}|");
        assert_eq!(
            execute_json(&script, json!(null), true).unwrap(),
            json!(expected)
        );
    }
    // Explicit numeric offsets on date-times and times retain their spelling.
    for source in ["2026-10-26T12:34:56+00:00", "12:34:56+00:00"] {
        assert_eq!(
            native(&format!("|{source}|")).temporal_text().as_deref(),
            Some(source)
        );
    }
}
