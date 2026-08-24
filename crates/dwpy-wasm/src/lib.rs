use std::collections::BTreeMap;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn run_dataweave_smoke(script_source: &str, payload_json: &str) -> Result<String, JsValue> {
    let payload: serde_json::Value =
        serde_json::from_str(payload_json).map_err(|err| JsValue::from_str(&err.to_string()))?;
    let result = dwpy_core::execute_smoke(script_source, payload)
        .map_err(|err| JsValue::from_str(&err.to_string()))?;
    serde_json::to_string(&result).map_err(|err| JsValue::from_str(&err.to_string()))
}

#[wasm_bindgen]
pub fn run_dataweave_request(request_json: &str) -> Result<String, JsValue> {
    let request: serde_json::Value =
        serde_json::from_str(request_json).map_err(|err| JsValue::from_str(&err.to_string()))?;
    let request = request
        .as_object()
        .ok_or_else(|| JsValue::from_str("DataWeave request must be a JSON object"))?;

    let script_source = request
        .get("script")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| JsValue::from_str("Invalid message: expected a 'script' string"))?;
    let render_output = request
        .get("render_output")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(true);
    let payload_format = request
        .get("payload_format")
        .and_then(serde_json::Value::as_str);
    let payload_format_options = request.get("payload_format_options");

    let payload = request
        .get("payload")
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    let payload = dwpy_core::parse_payload_format_with_options(
        payload,
        payload_format,
        payload_format_options,
    )
    .map_err(|err| JsValue::from_str(&wasm_error_message(err, script_source)))?;

    let vars = request
        .get("vars")
        .cloned()
        .unwrap_or_else(|| serde_json::Value::Object(serde_json::Map::new()));
    let attributes = request
        .get("attributes")
        .cloned()
        .unwrap_or_else(|| serde_json::Value::Object(serde_json::Map::new()));
    let properties = request
        .get("properties")
        .map(properties_to_map)
        .transpose()
        .map_err(|err| JsValue::from_str(&err))?;
    let context = dwpy_core::ExecutionContext::new(Some(vars), Some(attributes), properties)
        .map_err(|err| JsValue::from_str(&wasm_error_message(err, script_source)))?;
    let result =
        dwpy_core::execute_json_with_context(script_source, payload, context, render_output)
            .map_err(|err| JsValue::from_str(&wasm_error_message(err, script_source)))?;

    serde_json::to_string(&result).map_err(|err| JsValue::from_str(&err.to_string()))
}

#[wasm_bindgen]
pub fn analyze_dataweave_request(request_json: &str) -> Result<String, JsValue> {
    let request: serde_json::Value =
        serde_json::from_str(request_json).map_err(|err| JsValue::from_str(&err.to_string()))?;
    let request = request
        .as_object()
        .ok_or_else(|| JsValue::from_str("DataWeave analysis request must be a JSON object"))?;
    let expression = request
        .get("expression")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| JsValue::from_str("Invalid message: expected an 'expression' string"))?;
    let payload = request.get("payload").cloned();
    let vars = request.get("vars").cloned();
    let attributes = request.get("attributes").cloned();

    let result = dwpy_core::analyze_expression_with_context(expression, payload, vars, attributes)
        .map_err(|err| JsValue::from_str(&wasm_error_message(err, expression)))?;
    serde_json::to_string(&result).map_err(|err| JsValue::from_str(&err.to_string()))
}

fn properties_to_map(value: &serde_json::Value) -> Result<BTreeMap<String, String>, String> {
    let Some(object) = value.as_object() else {
        return Err("properties must be a JSON object with string values".to_string());
    };
    object
        .iter()
        .map(|(key, value)| {
            value
                .as_str()
                .map(|value| (key.clone(), value.to_string()))
                .ok_or_else(|| format!("property '{key}' must be a string value"))
        })
        .collect()
}

fn wasm_error_message(err: dwpy_core::DwError, script_source: &str) -> String {
    match err {
        dwpy_core::DwError::UnsupportedFeature(message) => {
            normalized_script_error_message(&message, script_source)
        }
        dwpy_core::DwError::Parse(message) => {
            format!("DataWeave parse error: {message}")
        }
        dwpy_core::DwError::InvalidJson(message) => format!("Invalid JSON value: {message}"),
        dwpy_core::DwError::ResourceLimit { .. } | dwpy_core::DwError::Output(_) => err.to_string(),
    }
}

fn normalized_script_error_message(message: &str, script_source: &str) -> String {
    if message.contains("expected number, got")
        || (message.contains("cannot coerce string") && message.contains("to Number"))
    {
        let operator = numeric_operator_from_script(script_source).unwrap_or("+");
        return format!("You called the function '{operator}' with these arguments, but it expects one of these combinations:\n(Number, Number)\n\nLocation:\nmain (line: 1, column: 1)");
    }
    if let Some((line, column, width, line_text)) =
        missing_expression_location(message, script_source)
    {
        return format!(
            "Missing Expression\n\n{line}| {line_text}\n{}\nLocation:\nmain (line: {line}, column:{column})",
            underline(line, column, width)
        );
    }
    if let Some((name, line, column, line_text)) =
        unresolved_identifier_location(message, script_source)
    {
        return format!(
            "Unable to resolve reference of: `{name}`.\n\n{line}| {line_text}\n{}\nLocation:\nmain (line: {line}, column:{column})",
            underline(line, column, 1)
        );
    }
    if let Some((name, line, column, line_text)) = unresolved_infix_location(script_source) {
        return format!(
            "Unable to resolve reference of `{name}`.\n\n{line}| {line_text}\n\nLocation:\nmain (line: {line}, column: {column})"
        );
    }
    message.to_string()
}

const COLLECTION_OPERATOR_KEYWORDS: &[&str] = &[
    "groupBy",
    "map",
    "pluck",
    "mapObject",
    "filter",
    "filterObject",
    "flatMap",
    "distinctBy",
    "orderBy",
    "reduce",
    "then",
    "onNull",
    "takeWhile",
    "dropWhile",
    "some",
    "every",
    "countCharactersBy",
    "everyCharacter",
    "mapString",
    "someCharacter",
    "substringBy",
    "countBy",
    "maxBy",
    "minBy",
    "sumBy",
    "firstWith",
    "indexWhere",
    "partition",
    "splitWhere",
    "everyEntry",
    "someEntry",
    "readLinesWith",
    "writeLinesWith",
    "scan",
    "mapLeafValues",
    "nodeExists",
];

fn missing_expression_location(
    message: &str,
    script_source: &str,
) -> Option<(usize, usize, usize, String)> {
    let body = script_body(script_source);
    if message.trim() != body.trim() {
        return None;
    }

    script_body_lines(script_source)
        .into_iter()
        .find_map(|(line_number, line)| {
            let trimmed_end = line.trim_end();
            let (operator_start, operator) = dangling_keyword(trimmed_end)?;
            let after_operator = &line[operator_start + operator.len()..];
            let width = after_operator.chars().count().max(1);
            Some((
                line_number,
                operator_start + operator.len() + 1,
                width,
                line.to_string(),
            ))
        })
}

fn dangling_keyword<'a>(line: &'a str) -> Option<(usize, &'static str)> {
    let mut match_value = None;
    for operator in COLLECTION_OPERATOR_KEYWORDS {
        for (index, _) in line.match_indices(operator) {
            if !is_top_level_index(line, index) {
                continue;
            }
            let before_ok = line[..index].chars().last().is_none_or(char::is_whitespace);
            let after_index = index + operator.len();
            let after_ok = line[after_index..].chars().all(char::is_whitespace);
            if before_ok && after_ok {
                match_value = Some((index, *operator));
            }
        }
    }
    match_value
}

fn unresolved_identifier_location(
    message: &str,
    script_source: &str,
) -> Option<(String, usize, usize, String)> {
    if !is_identifier(message) {
        return None;
    }

    let name = message.trim();
    script_body_lines(script_source)
        .into_iter()
        .find_map(|(line_number, line)| {
            identifier_column(line, name)
                .map(|column| (name.to_string(), line_number, column, line.to_string()))
        })
}

fn identifier_column(line: &str, name: &str) -> Option<usize> {
    let mut quote: Option<char> = None;
    let mut escaped = false;
    for (index, ch) in line.char_indices() {
        if let Some(quote_ch) = quote {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == quote_ch {
                quote = None;
            }
            continue;
        }

        if matches!(ch, '"' | '\'' | '`') {
            quote = Some(ch);
            continue;
        }

        if !line[index..].starts_with(name) {
            continue;
        }

        let before_ok = line[..index]
            .chars()
            .last()
            .is_none_or(|before| !is_identifier_part(before));
        let after_ok = line[index + name.len()..]
            .chars()
            .next()
            .is_none_or(|after| !is_identifier_part(after));
        if before_ok && after_ok {
            return Some(index + 1);
        }
    }
    None
}

fn unresolved_infix_location(script_source: &str) -> Option<(String, usize, usize, String)> {
    for (line_index, line) in script_source.lines().enumerate() {
        let Some(close_brace) = line.find('}') else {
            continue;
        };
        let after_brace = &line[close_brace + 1..];
        let trimmed = after_brace.trim_start();
        let leading_spaces = after_brace.len() - trimmed.len();
        let name = trimmed
            .chars()
            .take_while(|ch| ch.is_ascii_alphanumeric() || *ch == '_')
            .collect::<String>();
        if name.is_empty() {
            continue;
        }
        let after_name = trimmed[name.len()..].trim_start();
        if !after_name.starts_with('{') {
            continue;
        }
        return Some((
            name,
            line_index + 1,
            close_brace + 1 + leading_spaces + 1,
            line.to_string(),
        ));
    }
    None
}

fn underline(line: usize, column: usize, width: usize) -> String {
    format!(
        "{}{}",
        " ".repeat(line.to_string().len() + 2 + column.saturating_sub(1)),
        "^".repeat(width)
    )
}

fn script_body(script_source: &str) -> String {
    script_body_lines(script_source)
        .into_iter()
        .map(|(_, line)| line)
        .collect::<Vec<_>>()
        .join("\n")
}

fn script_body_lines(script_source: &str) -> Vec<(usize, &str)> {
    let start_line = dwpy_core::parse_script_boundary(script_source)
        .map(|boundary_line| boundary_line + 1)
        .unwrap_or(0);
    script_source
        .lines()
        .enumerate()
        .skip(start_line)
        .map(|(index, line)| (index + 1, line))
        .collect()
}

fn is_identifier(source: &str) -> bool {
    let mut chars = source.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    (first.is_ascii_alphabetic() || first == '_')
        && chars.all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
}

fn is_identifier_part(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '_'
}

fn is_top_level_index(source: &str, target: usize) -> bool {
    let mut depth = 0i32;
    let mut quote: Option<char> = None;
    let mut escaped = false;
    for (index, ch) in source.char_indices() {
        if index >= target {
            return depth == 0 && quote.is_none();
        }
        if let Some(quote_ch) = quote {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == quote_ch {
                quote = None;
            }
            continue;
        }
        match ch {
            '"' | '\'' | '`' => quote = Some(ch),
            '{' | '[' | '(' => depth += 1,
            '}' | ']' | ')' => depth -= 1,
            _ => {}
        }
    }
    depth == 0 && quote.is_none()
}

fn numeric_operator_from_script(script_source: &str) -> Option<&'static str> {
    if let Some(boundary_line) = dwpy_core::parse_script_boundary(script_source) {
        let body = script_source
            .lines()
            .skip(boundary_line + 1)
            .collect::<Vec<_>>()
            .join("\n");
        return find_top_level_operator(&body, &["*", "/", "+", "-"]);
    };
    find_top_level_operator(script_source, &["*", "/", "+", "-"])
}

fn find_top_level_operator(source: &str, operators: &[&'static str]) -> Option<&'static str> {
    let mut paren_depth = 0i64;
    let mut bracket_depth = 0i64;
    let mut brace_depth = 0i64;
    let mut quote: Option<char> = None;
    let mut previous = '\0';

    for (index, ch) in source.char_indices() {
        if let Some(quote_ch) = quote {
            if ch == quote_ch && previous != '\\' {
                quote = None;
            }
            previous = ch;
            continue;
        }

        match ch {
            '"' | '\'' | '`' => quote = Some(ch),
            '(' => paren_depth += 1,
            ')' => paren_depth -= 1,
            '[' => bracket_depth += 1,
            ']' => bracket_depth -= 1,
            '{' => brace_depth += 1,
            '}' => brace_depth -= 1,
            _ => {
                if paren_depth == 0 && bracket_depth == 0 && brace_depth == 0 {
                    let rest = &source[index..];
                    if let Some(operator) = operators
                        .iter()
                        .find(|operator| rest.starts_with(**operator) && !rest.starts_with("***"))
                    {
                        return Some(*operator);
                    }
                }
            }
        }
        previous = ch;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::{normalized_script_error_message, numeric_operator_from_script};

    #[test]
    fn hides_rust_backend_prefix_for_numeric_type_errors() {
        let message = normalized_script_error_message(
            "expected number, got Array [Number(1), Number(2)]",
            "%dw 2.0\noutput application/json\n---\n(1 to 10) * 2",
        );

        assert!(!message.contains("Rust evaluator"));
        assert!(message.contains("You called the function '*'"));
    }

    #[test]
    fn finds_numeric_operator_from_script_body() {
        assert_eq!(
            numeric_operator_from_script("%dw 2.0\noutput application/json\n---\n(1 to 10) * 2"),
            Some("*")
        );
        assert_eq!(
            numeric_operator_from_script("%dw 2.0\noutput application/json\n---\n{ value: 1 + 2 }"),
            None
        );
    }

    #[test]
    fn hides_rust_backend_prefix_for_unresolved_infix_errors() {
        let message = normalized_script_error_message(
            "some internal unresolved expression",
            "%dw 2.0\n---\n{a: 1} unknownInfix {b: 2}",
        );

        assert_eq!(
            message,
            "Unable to resolve reference of `unknownInfix`.\n\n3| {a: 1} unknownInfix {b: 2}\n\nLocation:\nmain (line: 3, column: 8)"
        );
    }

    #[test]
    fn formats_missing_expression_for_dangling_collection_operator() {
        let message = normalized_script_error_message(
            "(1 to 10) reduce",
            "%dw 2.0\noutput application/json\n---\n(1 to 10) reduce  ",
        );

        assert_eq!(
            message,
            "Missing Expression\n\n4| (1 to 10) reduce  \n                   ^^\nLocation:\nmain (line: 4, column:17)"
        );
    }

    #[test]
    fn formats_unresolved_identifier_with_source_location() {
        let message = normalized_script_error_message(
            "b",
            "%dw 2.0\noutput application/json\n---\n\"a\" + b",
        );

        assert_eq!(
            message,
            "Unable to resolve reference of: `b`.\n\n4| \"a\" + b\n         ^\nLocation:\nmain (line: 4, column:7)"
        );
    }
}
