use serde_json::Map;
use serde_json::Value;

use crate::{as_dataweave_string, DwError};

pub(crate) fn output_mime(directive: &str) -> Option<&str> {
    directive.split_whitespace().next()
}

pub(crate) fn is_json_mime(value: &str) -> bool {
    matches!(value, "json" | "application/json")
}

pub(crate) fn is_xml_mime(value: &str) -> bool {
    matches!(value, "xml" | "application/xml" | "text/xml")
}

pub(crate) fn is_yaml_mime(value: &str) -> bool {
    matches!(
        value,
        "yaml" | "yml" | "application/yaml" | "application/x-yaml" | "text/yaml" | "text/x-yaml"
    )
}

pub(crate) fn is_csv_mime(value: &str) -> bool {
    matches!(value, "csv" | "application/csv" | "text/csv")
}

pub(crate) fn is_ndjson_mime(value: &str) -> bool {
    matches!(
        value,
        "ndjson" | "application/x-ndjson" | "application/x-ldjson"
    )
}

pub(crate) fn is_markdown_mime(value: &str) -> bool {
    matches!(
        value,
        "markdown" | "md" | "text/markdown" | "text/x-markdown"
    )
}

pub(crate) fn mime_from_string(value: &Value) -> Result<Value, DwError> {
    let text = as_dataweave_string(value);
    let (mime, parameters_source) = text.split_once(';').unwrap_or((&text, ""));
    let Some((major, subtype)) = mime.trim().split_once('/') else {
        return Ok(Value::Object(Map::from_iter([
            ("success".to_string(), Value::Bool(false)),
            (
                "error".to_string(),
                Value::Object(Map::from_iter([(
                    "message".to_string(),
                    Value::String(format!("Unable to find a sub type in `{text}`.")),
                )])),
            ),
        ])));
    };
    let mut parameters = Map::new();
    for parameter in parameters_source.split(';').map(str::trim) {
        if parameter.is_empty() {
            continue;
        }
        if let Some((key, value)) = parameter.split_once('=') {
            parameters.insert(
                key.trim().to_string(),
                Value::String(value.trim_matches('"').to_string()),
            );
        }
    }
    Ok(Value::Object(Map::from_iter([
        ("success".to_string(), Value::Bool(true)),
        (
            "result".to_string(),
            Value::Object(Map::from_iter([
                ("type".to_string(), Value::String(major.trim().to_string())),
                (
                    "subtype".to_string(),
                    Value::String(subtype.trim().to_string()),
                ),
                ("parameters".to_string(), Value::Object(parameters)),
            ])),
        ),
    ])))
}

pub(crate) fn mime_to_string(value: &Value) -> Result<Value, DwError> {
    let Value::Object(map) = value else {
        return Ok(Value::String(as_dataweave_string(value)));
    };
    let major = map.get("type").map(as_dataweave_string).unwrap_or_default();
    let subtype = map
        .get("subtype")
        .map(as_dataweave_string)
        .unwrap_or_default();
    if major.is_empty() || subtype.is_empty() {
        return Ok(Value::String(as_dataweave_string(value)));
    }
    let mut text = format!("{major}/{subtype}");
    if let Some(Value::Object(parameters)) = map.get("parameters") {
        for (key, value) in parameters {
            text.push(';');
            text.push_str(key);
            text.push('=');
            text.push_str(&as_dataweave_string(value));
        }
    }
    Ok(Value::String(text))
}

pub(crate) fn mime_is_handled_by(handler: &Value, requested: &Value) -> Result<Value, DwError> {
    let (Value::Object(handler), Value::Object(requested)) = (handler, requested) else {
        return Ok(Value::Bool(false));
    };
    let handler_type = handler
        .get("type")
        .map(as_dataweave_string)
        .unwrap_or_default();
    let handler_subtype = handler
        .get("subtype")
        .map(as_dataweave_string)
        .unwrap_or_default();
    let requested_type = requested
        .get("type")
        .map(as_dataweave_string)
        .unwrap_or_default();
    let requested_subtype = requested
        .get("subtype")
        .map(as_dataweave_string)
        .unwrap_or_default();
    Ok(Value::Bool(
        (handler_type == "*" || handler_type == requested_type)
            && subtype_matches(&handler_subtype, &requested_subtype),
    ))
}

fn subtype_matches(handler: &str, requested: &str) -> bool {
    if handler == "*" || handler == requested {
        return true;
    }
    handler
        .strip_prefix("*+")
        .is_some_and(|suffix| requested.ends_with(&format!("+{suffix}")))
}
