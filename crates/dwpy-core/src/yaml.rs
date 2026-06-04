use serde_json::Map;
use serde_json::Value;
use yaml_serde::{Mapping as YamlMapping, Value as YamlValue};

use crate::{number_result, output_bool_option, output_option, DwError};

pub(crate) fn read_simple_yaml(text: &str) -> Result<Value, DwError> {
    let lines = text
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| YamlLine {
            indent: line.chars().take_while(|ch| *ch == ' ').count(),
            text: line.trim().to_string(),
        })
        .collect::<Vec<_>>();
    if lines.is_empty() {
        return Ok(Value::Null);
    }
    let (value, index) = parse_yaml_block(&lines, 0, lines[0].indent)?;
    if index != lines.len() {
        return Err(DwError::Parse("invalid YAML indentation".to_string()));
    }
    Ok(value)
}

#[derive(Debug)]
struct YamlLine {
    indent: usize,
    text: String,
}

fn parse_yaml_block(
    lines: &[YamlLine],
    mut index: usize,
    indent: usize,
) -> Result<(Value, usize), DwError> {
    if lines
        .get(index)
        .is_some_and(|line| line.indent == indent && line.text.starts_with("- "))
    {
        parse_yaml_sequence(lines, index, indent)
    } else {
        parse_yaml_mapping(lines, &mut index, indent).map(|value| (Value::Object(value), index))
    }
}

fn parse_yaml_mapping(
    lines: &[YamlLine],
    index: &mut usize,
    indent: usize,
) -> Result<Map<String, Value>, DwError> {
    let mut map = Map::new();
    while let Some(line) = lines.get(*index) {
        if line.indent < indent {
            break;
        }
        if line.indent > indent {
            return Err(DwError::Parse("invalid YAML indentation".to_string()));
        }
        if line.text.starts_with("- ") {
            break;
        }
        let Some((key, value_source)) = line.text.split_once(':') else {
            return Err(DwError::Parse("invalid YAML mapping".to_string()));
        };
        let key = key.trim();
        if key.is_empty() {
            return Err(DwError::Parse("invalid YAML key".to_string()));
        }
        let value_source = value_source.trim();
        *index += 1;
        let value = if value_source.is_empty() {
            if lines.get(*index).is_some_and(|next| next.indent > indent) {
                let child_indent = lines[*index].indent;
                let (child, next_index) = parse_yaml_block(lines, *index, child_indent)?;
                *index = next_index;
                child
            } else {
                Value::Null
            }
        } else {
            parse_simple_yaml_scalar_checked(value_source)?
        };
        map.insert(key.to_string(), value);
    }
    Ok(map)
}

fn parse_yaml_sequence(
    lines: &[YamlLine],
    mut index: usize,
    indent: usize,
) -> Result<(Value, usize), DwError> {
    let mut items = Vec::new();
    while let Some(line) = lines.get(index) {
        if line.indent < indent {
            break;
        }
        if line.indent > indent || !line.text.starts_with("- ") {
            return Err(DwError::Parse("invalid YAML sequence".to_string()));
        }
        let item_source = line.text.trim_start_matches("- ").trim();
        index += 1;
        let item = if item_source.is_empty() {
            if lines.get(index).is_some_and(|next| next.indent > indent) {
                let (child, next_index) = parse_yaml_block(lines, index, lines[index].indent)?;
                index = next_index;
                child
            } else {
                Value::Null
            }
        } else if let Some((key, value_source)) = item_source.split_once(':') {
            let mut map = Map::new();
            map.insert(
                key.trim().to_string(),
                parse_simple_yaml_scalar_checked(value_source.trim())?,
            );
            if lines.get(index).is_some_and(|next| next.indent > indent) {
                let child_indent = lines[index].indent;
                let mut child_map = parse_yaml_mapping(lines, &mut index, child_indent)?;
                map.append(&mut child_map);
            }
            Value::Object(map)
        } else {
            parse_simple_yaml_scalar_checked(item_source)?
        };
        items.push(item);
    }
    Ok((Value::Array(items), index))
}

fn parse_simple_yaml_scalar_checked(source: &str) -> Result<Value, DwError> {
    let source = source.trim();
    if source.starts_with('[') && !source.ends_with(']') {
        return Err(DwError::Parse("invalid YAML sequence".to_string()));
    }
    Ok(parse_simple_yaml_scalar(source))
}

fn parse_simple_yaml_scalar(source: &str) -> Value {
    let source = source.trim();
    if source == "null" || source == "~" {
        Value::Null
    } else if source == "true" {
        Value::Bool(true)
    } else if source == "false" {
        Value::Bool(false)
    } else if let Ok(value) = source.parse::<i64>() {
        Value::Number(value.into())
    } else if let Ok(value) = source.parse::<f64>() {
        number_result(value).unwrap_or_else(|_| Value::String(source.to_string()))
    } else {
        Value::String(source.trim_matches('"').trim_matches('\'').to_string())
    }
}

pub(crate) fn write_simple_yaml(value: &Value) -> Result<String, DwError> {
    let yaml_value = to_yaml_value(value);
    yaml_serde::to_string(&yaml_value)
        .map(|mut rendered| {
            rendered = quote_yaml_1_1_ambiguous_scalars(&rendered);
            if !rendered.ends_with('\n') {
                rendered.push('\n');
            }
            rendered
        })
        .map_err(|err| DwError::Parse(format!("Failed to render output as yaml: {err}")))
}

pub(crate) fn render_yaml_output(value: &Value, directive: &str) -> Result<String, DwError> {
    let filtered = output_option(directive, "skipNullOn")
        .map(|mode| filter_yaml_nulls(value, mode))
        .unwrap_or_else(|| value.clone());
    let mut output = String::new();
    if output_bool_option(directive, "writeDeclaration", false) {
        output.push_str("---\n");
    }
    output.push_str(&write_simple_yaml(&filtered)?);
    Ok(output)
}

fn to_yaml_value(value: &Value) -> YamlValue {
    match value {
        Value::Null => YamlValue::Null,
        Value::Bool(value) => YamlValue::Bool(*value),
        Value::Number(value) => {
            if let Some(value) = value.as_i64() {
                return YamlValue::Number(value.into());
            }
            if let Some(value) = value.as_u64() {
                return YamlValue::Number(value.into());
            }
            if let Some(value) = value.as_f64() {
                return YamlValue::Number(value.into());
            }
            YamlValue::String(value.to_string())
        }
        Value::String(value) => YamlValue::String(value.clone()),
        Value::Array(items) => YamlValue::Sequence(items.iter().map(to_yaml_value).collect()),
        Value::Object(map) => {
            let mut yaml_map = YamlMapping::new();
            for (key, value) in map {
                yaml_map.insert(YamlValue::String(key.clone()), to_yaml_value(value));
            }
            YamlValue::Mapping(yaml_map)
        }
    }
}

fn quote_yaml_1_1_ambiguous_scalars(rendered: &str) -> String {
    let mut output = rendered
        .lines()
        .map(quote_yaml_1_1_ambiguous_scalar_line)
        .collect::<Vec<_>>()
        .join("\n");
    if rendered.ends_with('\n') {
        output.push('\n');
    }
    output
}

fn quote_yaml_1_1_ambiguous_scalar_line(line: &str) -> String {
    let indent_len = line.len() - line.trim_start_matches(' ').len();
    let (indent, rest) = line.split_at(indent_len);
    if let Some(value) = rest.strip_prefix("- ") {
        return format!("{indent}- {}", quote_yaml_1_1_ambiguous_scalar(value));
    }
    if let Some((key, value)) = split_yaml_mapping_line(rest) {
        return format!("{indent}{key}: {}", quote_yaml_1_1_ambiguous_scalar(value));
    }
    if needs_yaml_1_1_string_quote(rest) {
        return format!("{indent}{}", quote_yaml_1_1_ambiguous_scalar(rest));
    }
    line.to_string()
}

fn split_yaml_mapping_line(line: &str) -> Option<(&str, &str)> {
    let mut single_quoted = false;
    let mut double_quoted = false;
    for (index, ch) in line.char_indices() {
        match ch {
            '\'' if !double_quoted => single_quoted = !single_quoted,
            '"' if !single_quoted => double_quoted = !double_quoted,
            ':' if !single_quoted && !double_quoted => {
                let value_start = index + ch.len_utf8();
                if line[value_start..].starts_with(' ') {
                    return Some((&line[..index], line[value_start + 1..].trim_start()));
                }
            }
            _ => {}
        }
    }
    None
}

fn quote_yaml_1_1_ambiguous_scalar(value: &str) -> String {
    if needs_yaml_1_1_string_quote(value) {
        format!("'{}'", value.replace('\'', "''"))
    } else {
        value.to_string()
    }
}

fn needs_yaml_1_1_string_quote(value: &str) -> bool {
    matches!(
        value,
        "y" | "Y"
            | "yes"
            | "Yes"
            | "YES"
            | "n"
            | "N"
            | "no"
            | "No"
            | "NO"
            | "on"
            | "On"
            | "ON"
            | "off"
            | "Off"
            | "OFF"
    )
}

fn filter_yaml_nulls(value: &Value, mode: &str) -> Value {
    match value {
        Value::Object(map) => Value::Object(
            map.iter()
                .filter_map(|(key, value)| {
                    let filtered = filter_yaml_nulls(value, mode);
                    if matches!(mode, "objects" | "everywhere") && filtered.is_null() {
                        None
                    } else {
                        Some((key.clone(), filtered))
                    }
                })
                .collect(),
        ),
        Value::Array(items) => Value::Array(
            items
                .iter()
                .filter_map(|value| {
                    let filtered = filter_yaml_nulls(value, mode);
                    if matches!(mode, "arrays" | "everywhere") && filtered.is_null() {
                        None
                    } else {
                        Some(filtered)
                    }
                })
                .collect(),
        ),
        other => other.clone(),
    }
}
