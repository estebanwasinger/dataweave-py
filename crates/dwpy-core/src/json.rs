use serde::Serialize;
use serde_json::Map;
use serde_json::Value;

use crate::builtins::{binary_bytes, DW_BINARY_MARKER, DW_NONFINITE_MARKER};
use crate::mime::{is_json_mime, output_mime};
use crate::periods::special_string_value;
use crate::selectors::{
    collapse_xml_like_value, duplicate_object_pairs, unwrap_metadata_value, DW_METADATA_MARKER,
    DW_METADATA_VALUE_MARKER,
};
use crate::{xml_list_items, DwError};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct JsonOutputOptions {
    pub(crate) indent: Option<usize>,
    pub(crate) ensure_ascii: bool,
    pub(crate) sort_keys: bool,
    pub(crate) duplicate_key_as_array: bool,
}

pub(crate) fn render_json_value(
    value: &Value,
    options: JsonOutputOptions,
) -> Result<String, DwError> {
    if options.indent.is_some() {
        return render_json_value_pretty(value, options);
    }
    render_json_value_compact(value, options)
}

fn render_json_value_compact(value: &Value, options: JsonOutputOptions) -> Result<String, DwError> {
    let mut output = String::new();
    write_json_value_compact(&mut output, value, options)?;
    Ok(output)
}

fn write_json_value_compact(
    output: &mut String,
    value: &Value,
    options: JsonOutputOptions,
) -> Result<(), DwError> {
    if let Some(text) = special_string_value(value) {
        output.push_str(&render_json_string(&text, options.ensure_ascii)?);
        return Ok(());
    }
    if let Some(value) = unwrap_metadata_value(value) {
        return write_json_value_compact(output, &value, options);
    }
    if is_binary_marker_value(value) {
        let text = String::from_utf8_lossy(&binary_bytes(value)?).into_owned();
        output.push_str(&render_json_string(&text, options.ensure_ascii)?);
        return Ok(());
    }
    if is_nonfinite_value(value) {
        output.push_str("null");
        return Ok(());
    }
    if let Some(items) = xml_list_items(value) {
        output.push('[');
        for (index, item) in items.iter().enumerate() {
            if index > 0 {
                output.push(',');
            }
            write_json_value_compact(output, &collapse_xml_like_value(item), options)?;
        }
        output.push(']');
        return Ok(());
    }
    if let Some(pairs) = duplicate_object_pairs(value) {
        output.push('{');
        let mut first = true;
        for (key, value) in pairs {
            if !first {
                output.push(',');
            }
            first = false;
            output.push_str(&render_json_string(&key, options.ensure_ascii)?);
            output.push(':');
            write_json_value_compact(output, &value, options)?;
        }
        output.push('}');
        return Ok(());
    }
    match value {
        Value::Object(map) => {
            let mut object_entries = map.iter().collect::<Vec<_>>();
            if options.sort_keys {
                object_entries.sort_by(|left, right| left.0.cmp(right.0));
            }
            output.push('{');
            let mut first = true;
            for (key, value) in object_entries {
                if key == DW_METADATA_MARKER || key == DW_METADATA_VALUE_MARKER {
                    continue;
                }
                let key_json = render_json_string(key, options.ensure_ascii)?;
                if let Some(items) = xml_list_items(value) {
                    if options.duplicate_key_as_array {
                        if !first {
                            output.push(',');
                        }
                        first = false;
                        output.push_str(&key_json);
                        output.push(':');
                        write_json_value_compact(
                            output,
                            &Value::Array(items.iter().map(collapse_xml_like_value).collect()),
                            options,
                        )?;
                    } else {
                        for item in items {
                            if !first {
                                output.push(',');
                            }
                            first = false;
                            output.push_str(&key_json);
                            output.push(':');
                            write_json_value_compact(
                                output,
                                &collapse_xml_like_value(item),
                                options,
                            )?;
                        }
                    }
                } else {
                    if !first {
                        output.push(',');
                    }
                    first = false;
                    output.push_str(&key_json);
                    output.push(':');
                    write_json_value_compact(output, value, options)?;
                }
            }
            output.push('}');
            Ok(())
        }
        Value::Array(items) => {
            output.push('[');
            for (index, item) in items.iter().enumerate() {
                if index > 0 {
                    output.push(',');
                }
                write_json_value_compact(output, item, options)?;
            }
            output.push(']');
            Ok(())
        }
        Value::String(text) => {
            output.push_str(&render_json_string(text, options.ensure_ascii)?);
            Ok(())
        }
        _ => {
            let rendered = serde_json::to_string(value)
                .map_err(|err| DwError::InvalidJson(err.to_string()))?;
            if options.ensure_ascii {
                output.push_str(&escape_non_ascii(&rendered));
            } else {
                output.push_str(&rendered);
            }
            Ok(())
        }
    }
}

fn render_json_value_pretty(value: &Value, options: JsonOutputOptions) -> Result<String, DwError> {
    let value = normalize_special_json_values(value);
    let value = if options.sort_keys {
        sort_json_object_keys(&value)
    } else {
        value
    };
    let indent = options.indent.unwrap_or(2);
    let indent_bytes = vec![b' '; indent];
    let formatter = serde_json::ser::PrettyFormatter::with_indent(&indent_bytes);
    let mut output = Vec::new();
    let mut serializer = serde_json::Serializer::with_formatter(&mut output, formatter);
    value
        .serialize(&mut serializer)
        .map_err(|err| DwError::InvalidJson(err.to_string()))?;
    let rendered =
        String::from_utf8(output).map_err(|err| DwError::InvalidJson(err.to_string()))?;
    Ok(if options.ensure_ascii {
        escape_non_ascii(&rendered)
    } else {
        rendered
    })
}

fn sort_json_object_keys(value: &Value) -> Value {
    match value {
        Value::Object(map) => {
            let mut entries = map.iter().collect::<Vec<_>>();
            entries.sort_by(|left, right| left.0.cmp(right.0));
            Value::Object(Map::from_iter(
                entries
                    .into_iter()
                    .map(|(key, value)| (key.clone(), sort_json_object_keys(value))),
            ))
        }
        Value::Array(items) => Value::Array(items.iter().map(sort_json_object_keys).collect()),
        _ => value.clone(),
    }
}

fn normalize_special_json_values(value: &Value) -> Value {
    if let Some(text) = special_string_value(value) {
        return Value::String(text);
    }
    if let Some(value) = unwrap_metadata_value(value) {
        return normalize_special_json_values(&value);
    }
    if is_binary_marker_value(value) {
        return Value::String(
            String::from_utf8_lossy(&binary_bytes(value).unwrap_or_default()).into_owned(),
        );
    }
    if is_nonfinite_value(value) {
        return Value::Null;
    }
    match value {
        value if duplicate_object_pairs(value).is_some() => {
            let Some(pairs) = duplicate_object_pairs(value) else {
                return value.clone();
            };
            Value::Object(Map::from_iter(
                pairs
                    .into_iter()
                    .map(|(key, value)| (key, normalize_special_json_values(&value))),
            ))
        }
        Value::Array(items) => {
            Value::Array(items.iter().map(normalize_special_json_values).collect())
        }
        Value::Object(map) => Value::Object(Map::from_iter(
            map.iter()
                .filter(|(key, _)| {
                    key.as_str() != DW_METADATA_MARKER && key.as_str() != DW_METADATA_VALUE_MARKER
                })
                .map(|(key, value)| (key.clone(), normalize_special_json_values(value))),
        )),
        _ => value.clone(),
    }
}

fn is_nonfinite_value(value: &Value) -> bool {
    matches!(value, Value::Object(map) if map.len() == 1 && map.contains_key(DW_NONFINITE_MARKER))
}

fn is_binary_marker_value(value: &Value) -> bool {
    matches!(value, Value::Object(map) if map.len() == 1 && map.contains_key(DW_BINARY_MARKER))
}

pub(crate) fn render_json_string(value: &str, ensure_ascii: bool) -> Result<String, DwError> {
    let rendered =
        serde_json::to_string(value).map_err(|err| DwError::InvalidJson(err.to_string()))?;
    Ok(if ensure_ascii {
        escape_non_ascii(&rendered)
    } else {
        rendered
    })
}

fn escape_non_ascii(value: &str) -> String {
    let mut escaped = String::new();
    for ch in value.chars() {
        if ch.is_ascii() {
            escaped.push(ch);
        } else {
            for unit in ch.encode_utf16(&mut [0; 2]) {
                escaped.push_str(&format!("\\u{unit:04x}"));
            }
        }
    }
    escaped
}

pub(crate) fn json_output_options(directive: &str) -> Result<JsonOutputOptions, DwError> {
    let mut options = JsonOutputOptions {
        indent: None,
        ensure_ascii: true,
        sort_keys: false,
        duplicate_key_as_array: false,
    };
    if !output_mime(directive).is_some_and(is_json_mime) {
        return Ok(options);
    }
    for (key, value) in parse_json_option_tokens(directive)? {
        let value = value.trim_matches('"');
        match key.as_str() {
            "indent" => {
                if value == "false" {
                    options.indent = None;
                } else {
                    options.indent = Some(value.parse::<usize>().map_err(|_| {
                        DwError::UnsupportedFeature(format!("JSON indent {value}"))
                    })?);
                }
            }
            "ensure_ascii" => {
                options.ensure_ascii = parse_bool_option(value, "ensure_ascii")?;
            }
            "sort_keys" => {
                options.sort_keys = parse_bool_option(value, "sort_keys")?;
            }
            "duplicateKeyAsArray" => {
                options.duplicate_key_as_array = parse_bool_option(value, "duplicateKeyAsArray")?;
            }
            _ => {
                return Err(DwError::UnsupportedFeature(format!(
                    "JSON output option {key}"
                )));
            }
        }
    }
    Ok(options)
}

fn parse_json_option_tokens(directive: &str) -> Result<Vec<(String, String)>, DwError> {
    let tokens = directive.split_whitespace().skip(1).collect::<Vec<_>>();
    let mut options = Vec::new();
    let mut index = 0usize;
    while index < tokens.len() {
        let token = tokens[index];
        if token == "with" && matches!(tokens.get(index + 1), Some(&"json" | &"binary")) {
            index += 2;
            continue;
        }
        if let Some((key, value)) = token.split_once('=') {
            if value.is_empty() {
                let Some(next) = tokens.get(index + 1) else {
                    return Err(DwError::UnsupportedFeature(format!(
                        "JSON output option {token}"
                    )));
                };
                options.push((key.to_string(), (*next).to_string()));
                index += 2;
            } else {
                options.push((key.to_string(), value.to_string()));
                index += 1;
            }
            continue;
        }
        if tokens.get(index + 1) == Some(&"=") {
            let Some(value) = tokens.get(index + 2) else {
                return Err(DwError::UnsupportedFeature(format!(
                    "JSON output option {token}"
                )));
            };
            options.push((token.to_string(), (*value).to_string()));
            index += 3;
            continue;
        }
        return Err(DwError::UnsupportedFeature(format!(
            "JSON output option {token}"
        )));
    }
    Ok(options)
}

fn parse_bool_option(value: &str, name: &str) -> Result<bool, DwError> {
    match value {
        "true" | "True" => Ok(true),
        "false" | "False" => Ok(false),
        _ => Err(DwError::UnsupportedFeature(format!(
            "Boolean output option {name}={value}"
        ))),
    }
}
