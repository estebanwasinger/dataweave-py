use serde_json::{Map, Value};

use crate::builtins::json_write_value_with_options;
use crate::json::{render_json_value, JsonOutputOptions};
use crate::{as_dataweave_string, DwError};

pub(crate) fn read_ndjson(text: &str, options: &Value) -> Result<Value, DwError> {
    let ignore_empty_line = bool_option(options, "ignoreEmptyLine", true);
    let skip_invalid = bool_option(options, "skipInvalid", false);
    let mut records = Vec::new();

    for (index, line) in text.lines().enumerate() {
        let line_number = index + 1;
        if line.trim().is_empty() && ignore_empty_line {
            continue;
        }
        match serde_json::from_str::<Value>(line) {
            Ok(record) => records.push(record),
            Err(_error) if skip_invalid => continue,
            Err(error) => {
                return Err(DwError::Parse(format!(
                    "Invalid NDJSON record on line {line_number}: {error}"
                )))
            }
        }
    }

    Ok(Value::Array(records))
}

pub(crate) fn render_ndjson_value(value: &Value, options: &Value) -> Result<String, DwError> {
    validate_options(options)?;
    let values = match value {
        Value::Array(items) => items.clone(),
        other => vec![other.clone()],
    };
    let prepared = json_write_value_with_options(&Value::Array(values), options);
    let Value::Array(records) = prepared else {
        return Err(DwError::Parse(
            "NDJSON writer failed to prepare records".to_string(),
        ));
    };
    let output_options = JsonOutputOptions {
        indent: None,
        ensure_ascii: bool_option(options, "ensure_ascii", true),
        sort_keys: false,
        duplicate_key_as_array: false,
    };
    let mut output = String::new();
    for record in records {
        output.push_str(&render_json_value(&record, output_options)?);
        output.push('\n');
    }
    Ok(output)
}

pub(crate) fn render_ndjson_output(value: &Value, directive: &str) -> Result<String, DwError> {
    render_ndjson_value(value, &directive_options(directive))
}

fn directive_options(directive: &str) -> Value {
    let mut options = Map::new();
    let tokens = directive.split_whitespace().skip(1).collect::<Vec<_>>();
    let mut index = 0usize;
    while index < tokens.len() {
        let token = tokens[index];
        let (key, raw_value, consumed) = if let Some((key, value)) = token.split_once('=') {
            (key, value, 1)
        } else if tokens.get(index + 1) == Some(&"=") {
            let value = tokens.get(index + 2).copied().unwrap_or_default();
            (token, value, 3)
        } else {
            index += 1;
            continue;
        };
        let value = raw_value.trim_matches('"');
        match key {
            "skipNullOn" | "encoding" => {
                options.insert(key.to_string(), Value::String(value.to_string()));
            }
            "writeAttributes" | "ensure_ascii" => {
                options.insert(key.to_string(), Value::Bool(parse_bool(value)));
            }
            _ => {}
        }
        index += consumed;
    }
    Value::Object(options)
}

fn bool_option(options: &Value, name: &str, default: bool) -> bool {
    let Some(value) = options.as_object().and_then(|map| map.get(name)) else {
        return default;
    };
    match value {
        Value::Bool(value) => *value,
        other => matches!(
            as_dataweave_string(other).as_str(),
            "true" | "True" | "1" | "yes"
        ),
    }
}

fn validate_options(options: &Value) -> Result<(), DwError> {
    let skip_null_on = options
        .as_object()
        .and_then(|map| map.get("skipNullOn"))
        .map(as_dataweave_string)
        .unwrap_or_default()
        .to_ascii_lowercase();
    if !skip_null_on.is_empty()
        && !matches!(skip_null_on.as_str(), "arrays" | "objects" | "everywhere")
    {
        return Err(DwError::UnsupportedFeature(
            "NDJSON skipNullOn must be 'arrays', 'objects', or 'everywhere'".to_string(),
        ));
    }
    Ok(())
}

fn parse_bool(value: &str) -> bool {
    matches!(value, "true" | "True" | "1" | "yes")
}
