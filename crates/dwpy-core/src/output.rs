use serde_json::{Map, Value};

use crate::csv::render_csv_output;
use crate::json::{json_output_options, render_json_string, render_json_value, JsonOutputOptions};
use crate::literals::evaluate_object_key;
use crate::markdown::render_markdown_output;
use crate::mime::{is_json_mime, is_markdown_mime, is_yaml_mime, output_mime};
use crate::selectors::unwrap_metadata_value;
use crate::syntax::{
    find_matching_delimiter, split_top_level, split_top_level_char, strip_wrapping_parens,
};
use crate::xml::render_xml_output;
use crate::yaml::render_yaml_output;
use crate::{evaluate_expression_scoped, DwError};

pub(crate) fn render_output_value(
    output_directive: Option<&str>,
    evaluated: Value,
    render_output: bool,
) -> Result<Value, DwError> {
    if !render_output {
        return Ok(unwrap_metadata_output_value(&evaluated));
    }
    if let Some(directive) = output_directive {
        if output_mime(directive).is_some_and(is_json_mime) {
            if directive_contains_binary_writer(directive) {
                if let Value::String(text) = &evaluated {
                    if looks_like_json_document(text) {
                        return Ok(Value::String(text.clone()));
                    }
                }
            }
            return render_json_value(&evaluated, json_output_options(directive)?)
                .map(Value::String);
        }
    }
    if let Some(directive) = output_directive {
        if output_mime(directive) == Some("application/csv") {
            return render_csv_output(&evaluated, directive).map(Value::String);
        }
        if output_mime(directive) == Some("application/xml") {
            return render_xml_output(&evaluated, directive).map(Value::String);
        }
        if is_yaml_mime(output_mime(directive).unwrap_or_default()) {
            return Ok(Value::String(render_yaml_output(&evaluated, directive)));
        }
        if output_mime(directive).is_some_and(is_markdown_mime) {
            return render_markdown_output(&evaluated, directive).map(Value::String);
        }
        if matches!(output_mime(directive), Some("text/plain" | "plain")) {
            return match evaluated {
                Value::String(text) => Ok(Value::String(text)),
                other => Err(DwError::UnsupportedFeature(format!(
                    "Plain text writer expects a string value, got {other:?}"
                ))),
            };
        }
    }
    Ok(evaluated)
}

fn directive_contains_binary_writer(directive: &str) -> bool {
    directive.contains("with binary")
}

fn looks_like_json_document(value: &str) -> bool {
    let trimmed = value.trim_start();
    trimmed.starts_with('{') || trimmed.starts_with('[')
}

fn unwrap_metadata_output_value(value: &Value) -> Value {
    if let Some(value) = unwrap_metadata_value(value) {
        return unwrap_metadata_output_value(&value);
    }
    match value {
        Value::Array(items) => {
            Value::Array(items.iter().map(unwrap_metadata_output_value).collect())
        }
        Value::Object(map) => {
            Value::Object(Map::from_iter(map.iter().map(|(key, value)| {
                (key.clone(), unwrap_metadata_output_value(value))
            })))
        }
        _ => value.clone(),
    }
}

pub(crate) fn render_json_compact_expression(
    source: &str,
    payload: &Value,
    locals: &Map<String, Value>,
    options: JsonOutputOptions,
) -> Result<String, DwError> {
    let source = strip_wrapping_parens(source.trim());
    if is_full_delimited(source, '{', '}') {
        return render_json_object_literal(source, payload, locals, options);
    }
    if is_full_delimited(source, '[', ']') {
        return render_json_array_literal(source, payload, locals, options);
    }
    let value = evaluate_expression_scoped(source, payload, locals)?;
    render_json_value(&value, options)
}

fn render_json_object_literal(
    source: &str,
    payload: &Value,
    locals: &Map<String, Value>,
    options: JsonOutputOptions,
) -> Result<String, DwError> {
    let inner = &source[1..source.len() - 1];
    let entries = split_top_level(inner, ',');
    let mut rendered = if options.sort_keys {
        Some(Vec::new())
    } else {
        None
    };
    let mut output = String::new();
    output.push('{');
    let mut first = true;
    for (index, entry) in entries.iter().enumerate() {
        let entry = entry.trim();
        if entry.is_empty() {
            if index + 1 == entries.len() {
                continue;
            }
            return Err(DwError::Parse("empty object entry".to_string()));
        }
        let Some((key_source, value_source)) = split_top_level_char(entry, ':') else {
            return Err(DwError::Parse(format!(
                "object entry missing ':' in {entry}"
            )));
        };
        if value_source.trim().is_empty() {
            return Err(DwError::Parse(format!(
                "object entry missing value in {entry}"
            )));
        }
        let key = evaluate_object_key(key_source.trim(), payload, locals)?;
        let key_json = render_json_string(&key, options.ensure_ascii)?;
        let value_json =
            render_json_compact_expression(value_source.trim(), payload, locals, options)?;
        if let Some(rendered) = rendered.as_mut() {
            rendered.push((key, format!("{key_json}:{value_json}")));
        } else {
            if !first {
                output.push(',');
            }
            first = false;
            output.push_str(&key_json);
            output.push(':');
            output.push_str(&value_json);
        }
    }
    if let Some(mut rendered) = rendered {
        rendered.sort_by(|left, right| left.0.cmp(&right.0));
        output.push_str(
            &rendered
                .into_iter()
                .map(|(_, rendered)| rendered)
                .collect::<Vec<_>>()
                .join(","),
        );
    }
    output.push('}');
    Ok(output)
}

fn render_json_array_literal(
    source: &str,
    payload: &Value,
    locals: &Map<String, Value>,
    options: JsonOutputOptions,
) -> Result<String, DwError> {
    let inner = &source[1..source.len() - 1];
    if inner.trim().is_empty() {
        return Ok("[]".to_string());
    }
    let entries = split_top_level(inner, ',');
    let mut output = String::new();
    output.push('[');
    let mut first = true;
    for (index, item) in entries.iter().enumerate() {
        let item = item.trim();
        if item.is_empty() {
            if index + 1 == entries.len() {
                continue;
            }
            return Err(DwError::Parse("empty array entry".to_string()));
        }
        if !first {
            output.push(',');
        }
        first = false;
        output.push_str(&render_json_compact_expression(
            item, payload, locals, options,
        )?);
    }
    output.push(']');
    Ok(output)
}

fn is_full_delimited(source: &str, open: char, close: char) -> bool {
    source.starts_with(open)
        && source.ends_with(close)
        && find_matching_delimiter(source, 0, open, close) == Some(source.len() - close.len_utf8())
}
