use serde_json::{Map, Value};

use crate::periods::period_literal;
use crate::syntax::{is_identifier, split_top_level};
use crate::{as_dataweave_string, evaluate_expression_scoped, DwError};

pub(crate) fn parse_literal(source: &str) -> Result<Option<Value>, DwError> {
    if is_regex_literal(source) {
        return Ok(Some(Value::String(source.to_string())));
    }
    if source.starts_with('|')
        && source.ends_with('|')
        && source.len() >= 2
        && !source[1..source.len() - 1].contains('|')
    {
        if let Some(period) = period_literal(&source[1..source.len() - 1]) {
            return Ok(Some(period));
        }
        return Ok(Some(Value::String(source[1..source.len() - 1].to_string())));
    }
    if source == "null" {
        return Ok(Some(Value::Null));
    }
    if source == "true" {
        return Ok(Some(Value::Bool(true)));
    }
    if source == "false" {
        return Ok(Some(Value::Bool(false)));
    }
    if is_number_literal_candidate(source) {
        if let Ok(value) = serde_json::from_str::<serde_json::Number>(source) {
            return Ok(Some(Value::Number(value)));
        }
    }
    if let Ok(value) = source.parse::<i64>() {
        return Ok(Some(Value::Number(value.into())));
    }
    if let Ok(value) = source.parse::<f64>() {
        if let Some(number) = serde_json::Number::from_f64(value) {
            return Ok(Some(Value::Number(number)));
        }
    }
    Ok(None)
}

fn is_regex_literal(source: &str) -> bool {
    source.starts_with('/') && source.ends_with('/') && source.len() >= 2
}

fn is_number_literal_candidate(source: &str) -> bool {
    let bytes = source.as_bytes();
    bytes.first().is_some_and(|first| {
        first.is_ascii_digit() || (*first == b'-' && bytes.get(1).is_some_and(u8::is_ascii_digit))
    })
}

pub(crate) fn evaluate_string_literal_scoped(
    source: &str,
    payload: &Value,
    locals: &Map<String, Value>,
) -> Result<Option<Value>, DwError> {
    let Some((quote, inner)) = string_literal_inner(source)? else {
        return Ok(None);
    };
    let chars = inner.chars().collect::<Vec<_>>();
    let mut output = String::new();
    let mut index = 0usize;
    let mut escaped = false;
    while index < chars.len() {
        let ch = chars[index];
        if escaped {
            output.push(unescape_char(ch));
            escaped = false;
            index += 1;
            continue;
        }
        if ch == '\\' {
            escaped = true;
            index += 1;
            continue;
        }
        if ch == '$' && chars.get(index + 1) == Some(&'(') {
            let (expression, next_index) = read_interpolation_expression(&chars, index + 2)?;
            let value = evaluate_expression_scoped(expression.trim(), payload, locals)?;
            output.push_str(&as_dataweave_string(&value));
            index = next_index;
            continue;
        }
        if ch == '$' && chars.get(index + 1) == Some(&'$') && chars.get(index + 2) == Some(&'$') {
            if let Some(value) = locals.get("$$$") {
                output.push_str(&as_dataweave_string(value));
                index += 3;
                continue;
            }
        }
        if ch == '$' && chars.get(index + 1) == Some(&'$') {
            if let Some(value) = locals.get("$$") {
                output.push_str(&as_dataweave_string(value));
                index += 2;
                continue;
            }
        }
        if ch == '$' && index + 1 == chars.len() && locals.contains_key("$") {
            output.push_str(&as_dataweave_string(&locals["$"]));
            index += 1;
            continue;
        }
        if ch == '$' && chars.get(index + 1).is_some_and(is_identifier_start) {
            let mut end = index + 2;
            while chars.get(end).is_some_and(is_identifier_part) {
                end += 1;
            }
            let name = chars[index + 1..end].iter().collect::<String>();
            if let Some(value) = locals.get(&name) {
                output.push_str(&as_dataweave_string(value));
                index = end;
                continue;
            }
        }
        output.push(ch);
        index += 1;
    }
    if escaped {
        return Err(DwError::Parse("dangling string escape".to_string()));
    }
    let _ = quote;
    Ok(Some(Value::String(output)))
}

pub(crate) fn parse_string_literal(source: &str) -> Result<Option<String>, DwError> {
    let Some((_quote, inner)) = string_literal_inner(source)? else {
        return Ok(None);
    };
    let mut output = String::new();
    let mut escaped = false;
    for ch in inner.chars() {
        if escaped {
            output.push(unescape_char(ch));
            escaped = false;
        } else if ch == '\\' {
            escaped = true;
        } else {
            output.push(ch);
        }
    }
    if escaped {
        return Err(DwError::Parse("dangling string escape".to_string()));
    }
    Ok(Some(output))
}

pub(crate) fn string_literal_inner(source: &str) -> Result<Option<(char, &str)>, DwError> {
    let bytes = source.as_bytes();
    if bytes.len() < 2 {
        return Ok(None);
    }
    let quote = bytes[0] as char;
    if quote != '"' && quote != '\'' && quote != '`' {
        return Ok(None);
    }
    if bytes[bytes.len() - 1] as char != quote {
        return Err(DwError::Parse(format!(
            "unterminated string literal {source}"
        )));
    }
    Ok(Some((quote, &source[1..source.len() - 1])))
}

fn unescape_char(ch: char) -> char {
    match ch {
        'n' => '\n',
        'r' => '\r',
        't' => '\t',
        '"' => '"',
        '\'' => '\'',
        '`' => '`',
        '\\' => '\\',
        other => other,
    }
}

fn is_identifier_start(ch: &char) -> bool {
    ch.is_ascii_alphabetic() || *ch == '_'
}

fn is_identifier_part(ch: &char) -> bool {
    ch.is_ascii_alphanumeric() || *ch == '_'
}

fn read_interpolation_expression(
    chars: &[char],
    mut index: usize,
) -> Result<(String, usize), DwError> {
    let mut depth = 1i32;
    let mut expression = String::new();
    let mut in_string: Option<char> = None;
    let mut escaped = false;
    while index < chars.len() {
        let ch = chars[index];
        if let Some(quote) = in_string {
            expression.push(ch);
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == quote {
                in_string = None;
            }
            index += 1;
            continue;
        }
        match ch {
            '"' | '\'' | '`' => {
                in_string = Some(ch);
                expression.push(ch);
            }
            '(' => {
                depth += 1;
                expression.push(ch);
            }
            ')' => {
                depth -= 1;
                if depth == 0 {
                    return Ok((expression, index + 1));
                }
                expression.push(ch);
            }
            _ => expression.push(ch),
        }
        index += 1;
    }
    Err(DwError::Parse(
        "unterminated string interpolation".to_string(),
    ))
}

pub(crate) fn parse_object_key(source: &str) -> Result<String, DwError> {
    if let Some(value) = parse_string_literal(source)? {
        return Ok(value);
    }
    if let Some((name, _attributes)) = source.split_once("@(") {
        let name = name.trim();
        if is_identifier(name) {
            return Ok(name.to_string());
        }
    }
    if is_identifier(source) {
        return Ok(source.to_string());
    }
    Err(DwError::UnsupportedFeature(format!(
        "dynamic object key {source}"
    )))
}

pub(crate) fn evaluate_object_key(
    source: &str,
    payload: &Value,
    locals: &Map<String, Value>,
) -> Result<String, DwError> {
    if source.starts_with('(') && source.ends_with(')') {
        let value = evaluate_expression_scoped(&source[1..source.len() - 1], payload, locals)?;
        return Ok(as_dataweave_string(&value));
    }
    if let Some(value) = evaluate_string_literal_scoped(source, payload, locals)? {
        if let Value::String(name) = &value {
            if matches!(name.as_str(), "$" | "$$" | "$$$") {
                if let Some(local) = locals.get(name) {
                    return Ok(as_dataweave_string(local));
                }
            }
        }
        return Ok(as_dataweave_string(&value));
    }
    parse_object_key(source)
}

pub(crate) fn parse_call_args(source: &str) -> Option<(&str, Vec<&str>)> {
    let open = source.find('(')?;
    if !source.ends_with(')') {
        return None;
    }
    let name = strip_generic_call_type_parameters(source[..open].trim());
    if !is_qualified_identifier(name) {
        return None;
    }
    let args = source[open + 1..source.len() - 1].trim();
    if args.is_empty() {
        return Some((name, Vec::new()));
    }
    Some((
        name,
        split_top_level(args, ',')
            .into_iter()
            .map(str::trim)
            .collect(),
    ))
}

fn strip_generic_call_type_parameters(source: &str) -> &str {
    let Some(open) = source.find('<') else {
        return source;
    };
    let Some(close) = source.rfind('>') else {
        return source;
    };
    if close == source.len() - 1 {
        source[..open].trim()
    } else {
        source
    }
}

fn is_qualified_identifier(source: &str) -> bool {
    source.split("::").all(is_identifier)
}
