use serde_json::{Map, Value};

use regex::Regex;

use crate::as_dataweave_string;
use crate::operators::simple_full_match;
use crate::strings::regex_literal_inner;
use crate::syntax::{
    find_matching_delimiter, is_identifier, split_top_level_arrow, split_top_level_char,
    split_top_level_keyword,
};
use crate::{evaluate_expression_scoped, evaluate_type_check, is_truthy, DwError};

pub(crate) fn evaluate_match_expression(
    value_source: &str,
    cases_source: &str,
    payload: &Value,
    locals: &Map<String, Value>,
) -> Result<Value, DwError> {
    let value = evaluate_expression_scoped(value_source, payload, locals)?;
    for case_source in split_match_cases(cases_source) {
        let case_source = case_source.trim();
        if case_source.is_empty() {
            continue;
        }
        let Some((pattern_source, result_source)) = split_top_level_arrow(case_source) else {
            return Err(DwError::Parse(format!("invalid match case {case_source}")));
        };
        let pattern_source = pattern_source.trim();
        let result_source = result_source.trim();
        if pattern_source == "else" {
            return evaluate_expression_scoped(result_source, payload, locals);
        }
        let Some(pattern_source) = pattern_source.strip_prefix("case ").map(str::trim) else {
            return Err(DwError::Parse(format!("invalid match case {case_source}")));
        };

        let (pattern_without_guard, guard_source) = split_top_level_keyword(pattern_source, "when")
            .or_else(|| split_top_level_keyword(pattern_source, "if"))
            .map(|(pattern, guard)| (pattern.trim(), Some(guard.trim())))
            .unwrap_or((pattern_source, None));

        let mut case_locals = locals.clone();
        let matches = match_pattern(
            &value,
            pattern_without_guard,
            payload,
            locals,
            &mut case_locals,
        )?;

        if !matches {
            continue;
        }
        if let Some(guard_source) = guard_source {
            let guard = evaluate_expression_scoped(guard_source, payload, &case_locals)?;
            if !is_truthy(&guard) {
                continue;
            }
        }
        return evaluate_expression_scoped(result_source, payload, &case_locals);
    }
    Ok(Value::Null)
}

fn match_values(value: &Value, expected: &Value) -> bool {
    value == expected
}

fn match_pattern(
    value: &Value,
    pattern_source: &str,
    payload: &Value,
    locals: &Map<String, Value>,
    case_locals: &mut Map<String, Value>,
) -> Result<bool, DwError> {
    if let Some(binding) = pattern_source.strip_prefix("var ").map(str::trim) {
        bind_match_value(binding, value, case_locals)?;
        return Ok(true);
    }
    if let Some(type_source) = pattern_source.strip_prefix("is ").map(str::trim) {
        return Ok(evaluate_type_check(value, type_source, locals));
    }
    if let Some((binding, pattern_source)) = split_top_level_keyword(pattern_source, "matches") {
        if is_identifier(binding) {
            let pattern = evaluate_expression_scoped(pattern_source, payload, locals)?;
            if let Some(captures) = regex_match_captures(value, &pattern)? {
                case_locals.insert(binding.to_string(), Value::Array(captures));
                return Ok(true);
            }
            return Ok(false);
        }
    }
    if let Some((binding, type_source)) = split_top_level_keyword(pattern_source, "is") {
        if is_identifier(binding) {
            bind_match_value(binding, value, case_locals)?;
            return Ok(evaluate_type_check(value, type_source, locals));
        }
    }
    if let Some((binding, expected_source)) = split_top_level_char(pattern_source, ':') {
        if is_identifier(binding) {
            let expected = evaluate_expression_scoped(expected_source, payload, locals)?;
            if match_values(value, &expected) {
                bind_match_value(binding, value, case_locals)?;
                return Ok(true);
            }
            return Ok(false);
        }
    }
    if is_binding_identifier(pattern_source) {
        bind_match_value(pattern_source, value, case_locals)?;
        return Ok(true);
    }
    let expected = evaluate_expression_scoped(pattern_source, payload, locals)?;
    Ok(match_values(value, &expected))
}

fn regex_match_captures(value: &Value, pattern: &Value) -> Result<Option<Vec<Value>>, DwError> {
    if value.is_null() || pattern.is_null() {
        return Ok(None);
    }
    let text = as_dataweave_string(value);
    let pattern_text = as_dataweave_string(pattern);
    let pattern = regex_literal_inner(&pattern_text).unwrap_or(&pattern_text);
    if !simple_full_match(&text, pattern)? {
        return Ok(None);
    }
    let regex = Regex::new(pattern)
        .map_err(|err| DwError::UnsupportedFeature(format!("regex pattern /{pattern}/: {err}")))?;
    let Some(captures) = regex.captures(&text) else {
        return Ok(None);
    };
    Ok(Some(
        (0..captures.len())
            .map(|index| {
                captures
                    .get(index)
                    .map(|matched| Value::String(matched.as_str().to_string()))
                    .unwrap_or(Value::Null)
            })
            .collect(),
    ))
}

fn is_binding_identifier(source: &str) -> bool {
    is_identifier(source) && !matches!(source, "true" | "false" | "null")
}

fn bind_match_value(
    binding: &str,
    value: &Value,
    case_locals: &mut Map<String, Value>,
) -> Result<(), DwError> {
    if !is_identifier(binding) {
        return Err(DwError::Parse(format!("invalid match binding {binding}")));
    }
    case_locals.insert(binding.to_string(), value.clone());
    Ok(())
}

fn split_match_cases(source: &str) -> Vec<&str> {
    let mut cases = Vec::new();
    let mut start = None;
    let mut depth = 0i32;
    let mut in_string = None;
    let mut escaped = false;
    for (index, ch) in source.char_indices() {
        if let Some(quote) = in_string {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == quote {
                in_string = None;
            }
            continue;
        }
        match ch {
            '"' | '\'' | '`' | '|' => in_string = Some(ch),
            '{' | '[' | '(' => depth += 1,
            '}' | ']' | ')' => depth -= 1,
            ',' if depth == 0 => {
                if let Some(case_start) = start.take() {
                    push_match_case(&mut cases, &source[case_start..index]);
                }
            }
            _ => {
                if depth == 0 && starts_match_case(source, index) {
                    if let Some(case_start) = start.replace(index) {
                        push_match_case(&mut cases, &source[case_start..index]);
                    }
                }
            }
        }
    }
    if let Some(case_start) = start {
        push_match_case(&mut cases, &source[case_start..]);
    }
    cases
}

fn starts_match_case(source: &str, index: usize) -> bool {
    let rest = &source[index..];
    let starts = rest.starts_with("case ") || rest.starts_with("else");
    if !starts {
        return false;
    }
    source[..index]
        .chars()
        .last()
        .is_none_or(|ch| ch.is_whitespace() || ch == ',')
}

fn push_match_case<'a>(cases: &mut Vec<&'a str>, source: &'a str) {
    let case = source.trim().trim_end_matches(',').trim();
    if !case.is_empty() {
        cases.push(case);
    }
}

pub(crate) fn parse_match_expression_source(source: &str) -> Option<(&str, &str)> {
    let (value_source, cases_source) = split_top_level_keyword(source, "match")?;
    if !cases_source.starts_with('{') || !cases_source.ends_with('}') {
        return None;
    }
    let close = find_matching_delimiter(cases_source, 0, '{', '}')?;
    if close + 1 != cases_source.len() {
        return None;
    }
    Some((value_source, cases_source[1..close].trim()))
}
