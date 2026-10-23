use crate::value::DwMap as Map;
use crate::value::DwValue as Value;
use serde::de::{MapAccess, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize};

use crate::builtins::{binary_bytes, DW_BINARY_MARKER, DW_NONFINITE_MARKER};
use crate::mime::{is_json_mime, output_mime};
use crate::periods::special_string_value;
use crate::selectors::{
    collapse_xml_like_value, duplicate_object_pairs, duplicate_object_value, unwrap_metadata_value,
    DW_METADATA_MARKER, DW_METADATA_VALUE_MARKER,
};
use crate::{xml_list_items, DwError};

/// Decode JSON objects before a map can discard repeated keys. Use the same
/// ordered-pair representation as DataWeave object literals, including nested objects.
pub(crate) fn read_json_value(text: &str) -> Result<Value, serde_json::Error> {
    serde_json::from_str::<JsonInput>(text).map(|input| input.0)
}

struct JsonInput(Value);

impl<'de> Deserialize<'de> for JsonInput {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct JsonVisitor;

        impl<'de> Visitor<'de> for JsonVisitor {
            type Value = JsonInput;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a JSON value")
            }

            fn visit_unit<E: serde::de::Error>(self) -> Result<JsonInput, E> {
                Ok(JsonInput(Value::Null))
            }

            fn visit_bool<E: serde::de::Error>(self, value: bool) -> Result<JsonInput, E> {
                Ok(JsonInput(Value::Bool(value)))
            }

            fn visit_i64<E: serde::de::Error>(self, value: i64) -> Result<JsonInput, E> {
                Ok(JsonInput(Value::Number(value.into())))
            }

            fn visit_u64<E: serde::de::Error>(self, value: u64) -> Result<JsonInput, E> {
                Ok(JsonInput(Value::Number(value.into())))
            }

            fn visit_f64<E: serde::de::Error>(self, value: f64) -> Result<JsonInput, E> {
                serde_json::Number::from_f64(value)
                    .map(|number| JsonInput(Value::Number(number)))
                    .ok_or_else(|| E::custom("number out of range"))
            }

            fn visit_str<E: serde::de::Error>(self, value: &str) -> Result<JsonInput, E> {
                Ok(JsonInput(Value::String(value.to_string())))
            }

            fn visit_string<E: serde::de::Error>(self, value: String) -> Result<JsonInput, E> {
                Ok(JsonInput(Value::String(value)))
            }

            fn visit_seq<A: SeqAccess<'de>>(self, mut sequence: A) -> Result<JsonInput, A::Error> {
                let mut items = Vec::new();
                while let Some(JsonInput(value)) = sequence.next_element()? {
                    items.push(value);
                }
                Ok(JsonInput(Value::Array(items)))
            }

            fn visit_map<A: MapAccess<'de>>(self, mut object: A) -> Result<JsonInput, A::Error> {
                let mut pairs = Vec::new();
                let mut keys = std::collections::HashSet::new();
                let mut has_duplicates = false;
                while let Some(key) = object.next_key::<String>()? {
                    // With arbitrary_precision, serde_json sends numbers through
                    // MapAccess using this token instead of rounding through f64.
                    if pairs.is_empty() && key == "$serde_json::private::Number" {
                        let text = object.next_value::<String>()?;
                        let number = text.parse().map_err(serde::de::Error::custom)?;
                        return Ok(JsonInput(Value::Number(number)));
                    }
                    let JsonInput(value) = object.next_value::<JsonInput>()?;
                    has_duplicates |= !keys.insert(key.clone());
                    pairs.push((key, value));
                }
                Ok(JsonInput(if has_duplicates {
                    duplicate_object_value(pairs)
                } else {
                    Value::Object(pairs.into_iter().collect())
                }))
            }
        }

        deserializer.deserialize_any(JsonVisitor)
    }
}

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
    JsonOutputValue(&value)
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
    if let Some(mut pairs) = duplicate_object_pairs(value) {
        pairs.sort_by(|left, right| left.0.cmp(&right.0));
        return duplicate_object_value(
            pairs
                .into_iter()
                .map(|(key, value)| (key, sort_json_object_keys(&value)))
                .collect(),
        );
    }
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
            duplicate_object_value(
                pairs
                    .into_iter()
                    .map(|(key, value)| (key, normalize_special_json_values(&value)))
                    .collect(),
            )
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

/// Serialize DataWeave object pairs directly into the formatter rather than
/// collecting them into a unique-key JSON map first.
struct JsonOutputValue<'a>(&'a Value);

impl Serialize for JsonOutputValue<'_> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        if let Some(pairs) = duplicate_object_pairs(self.0) {
            return serializer.collect_map(
                pairs
                    .iter()
                    .map(|(key, value)| (key, JsonOutputValue(value))),
            );
        }
        match self.0 {
            Value::Array(items) => serializer.collect_seq(items.iter().map(JsonOutputValue)),
            Value::Object(map) => {
                serializer.collect_map(map.iter().map(|(key, value)| (key, JsonOutputValue(value))))
            }
            value => value.serialize(serializer),
        }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pretty_writer_preserves_nested_duplicate_object_entries() {
        let value = read_json_value(r#"[{"a":null,"a":[1,2],"a":{"x":"café","x":3}}]"#).unwrap();
        for indent in [0, 2, 4] {
            let rendered = render_json_value(
                &value,
                JsonOutputOptions {
                    indent: Some(indent),
                    ensure_ascii: true,
                    sort_keys: false,
                    duplicate_key_as_array: false,
                },
            )
            .unwrap();
            assert_eq!(read_json_value(&rendered).unwrap(), value);
            assert!(rendered.contains("caf\\u00e9"));
        }
    }

    #[test]
    fn pretty_writer_sorts_keys_without_reordering_equal_key_entries() {
        let value = read_json_value(r#"{"z":0,"a":1,"z":2,"a":3}"#).unwrap();
        let rendered = render_json_value(
            &value,
            JsonOutputOptions {
                indent: Some(2),
                ensure_ascii: false,
                sort_keys: true,
                duplicate_key_as_array: false,
            },
        )
        .unwrap();
        let pairs = duplicate_object_pairs(&read_json_value(&rendered).unwrap()).unwrap();
        let keys = pairs
            .iter()
            .map(|(key, _)| key.as_str())
            .collect::<Vec<_>>();
        assert_eq!(keys, vec!["a", "a", "z", "z"]);
        assert_eq!(pairs[0].1.as_i64(), Some(1));
        assert_eq!(pairs[1].1.as_i64(), Some(3));
    }

    #[test]
    fn json_reader_preserves_repeated_keys_at_every_depth() {
        let payload = read_json_value(
            r#"{"test":{"test2":{"array":1},"test2":{"array":2},"test2":{"array":3}}}"#,
        )
        .unwrap();
        let result = crate::execute_value("payload.test.*test2", payload, false).unwrap();
        assert_eq!(
            result.to_json(),
            serde_json::json!([
                {"array": 1}, {"array": 2}, {"array": 3}
            ])
        );
    }

    #[test]
    fn json_reader_matches_standard_decoder_for_unique_keys_and_scalars() {
        for input in [
            r#"{"values":[null,true,false,-1,18446744073709551615,1.25,1e30,"caf\u00e9"],"empty":{}}"#,
            "[]",
            "null",
            "42",
            "-1.5",
            "true",
            r#""hello""#,
            "1.234567890123456789",
            "18446744073709551616",
        ] {
            assert_eq!(
                read_json_value(input).unwrap().to_json(),
                serde_json::from_str::<serde_json::Value>(input).unwrap()
            );
        }
    }

    #[test]
    fn json_reader_rejects_invalid_syntax_and_trailing_content() {
        for input in [r#"{"a":1,"a":}"#, r#"{"a":1,"a":2} trailing"#, "[1,]"] {
            assert!(read_json_value(input).is_err());
        }
    }
}
