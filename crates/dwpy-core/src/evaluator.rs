use serde_json::{Map, Value};

use crate::functions::evaluate_header_declarations;
use crate::literals::{evaluate_object_key, parse_literal, parse_string_literal};
use crate::script::parse_script_boundary_span;
use crate::selectors::{
    collapse_xml_like_value, duplicate_object_pairs, duplicate_object_value,
    evaluate_local_path_with_collapse, evaluate_payload_path_with_collapse,
    select_dynamic_multi_property, select_key_value_pairs,
};
use crate::syntax::{
    find_matching_delimiter, is_identifier, split_top_level, split_top_level_char,
    split_top_level_keyword, split_top_level_keyword_or_call_operator, strip_wrapping_parens,
};
use crate::xml::xml_list_items;
use crate::{as_dataweave_string, evaluate_expression_scoped, is_truthy, DwError};

pub(crate) fn evaluate_index_base(
    source: &str,
    payload: &Value,
    locals: &Map<String, Value>,
) -> Result<Value, DwError> {
    if source == "payload" || source.starts_with("payload.") || source.starts_with("payload?.") {
        return evaluate_payload_path_with_collapse(source, payload, false);
    }
    if let Some(value) = evaluate_local_path_with_collapse(source, locals, false)? {
        return Ok(value);
    }
    evaluate_expression_scoped(source, payload, locals)
}

pub(crate) fn evaluate_do_block_scoped(
    source: &str,
    payload: &Value,
    locals: &Map<String, Value>,
) -> Result<Value, DwError> {
    let rest = source
        .strip_prefix("do")
        .ok_or_else(|| DwError::Parse(format!("invalid do block {source}")))?
        .trim_start();
    if !rest.starts_with('{') {
        return Err(DwError::Parse(format!("invalid do block {source}")));
    }
    let close = find_matching_delimiter(rest, 0, '{', '}')
        .ok_or_else(|| DwError::Parse("unterminated do block".to_string()))?;
    if !rest[close + 1..].trim().is_empty() {
        return Err(DwError::Parse(format!("invalid do block {source}")));
    }

    let inner = &rest[1..close];
    let Some((delimiter_start, delimiter_end)) = parse_script_boundary_span(inner) else {
        return evaluate_expression_scoped(inner.trim(), payload, locals);
    };
    let header = inner[..delimiter_start].trim();
    let body = inner[delimiter_end..].trim();

    let mut scoped_locals = locals.clone();
    evaluate_header_declarations(header, payload, &mut scoped_locals)?;
    evaluate_expression_scoped(body, payload, &scoped_locals)
}

pub(crate) fn evaluate_using_expression_scoped(
    source: &str,
    payload: &Value,
    locals: &Map<String, Value>,
) -> Result<Value, DwError> {
    let rest = source
        .strip_prefix("using")
        .ok_or_else(|| DwError::Parse(format!("invalid using expression {source}")))?
        .trim_start();
    if !rest.starts_with('(') {
        return Err(DwError::Parse(format!("invalid using expression {source}")));
    }
    let close = find_matching_delimiter(rest, 0, '(', ')')
        .ok_or_else(|| DwError::Parse("unterminated using bindings".to_string()))?;
    let bindings_source = &rest[1..close];
    let body = rest[close + 1..].trim();
    if body.is_empty() {
        return Err(DwError::Parse("using expression missing body".to_string()));
    }

    let mut scoped_locals = locals.clone();
    for binding in split_top_level(bindings_source, ',') {
        let binding = binding.trim();
        if binding.is_empty() {
            continue;
        }
        let Some((name_source, value_source)) = split_top_level_char(binding, '=') else {
            return Err(DwError::Parse(format!(
                "using binding missing '=' in {binding}"
            )));
        };
        let name = name_source.split(':').next().unwrap_or_default().trim();
        if !is_identifier(name) {
            return Err(DwError::Parse(format!("invalid using binding name {name}")));
        }
        let value = evaluate_expression_scoped(value_source.trim(), payload, &scoped_locals)?;
        scoped_locals.insert(name.to_string(), value);
    }
    evaluate_expression_scoped(body, payload, &scoped_locals)
}

pub(crate) fn evaluate_object_literal_scoped(
    source: &str,
    payload: &Value,
    locals: &Map<String, Value>,
) -> Result<Value, DwError> {
    let inner = &source[1..source.len() - 1];
    if locals.is_empty() {
        if let Some(value) = evaluate_fast_payload_object_literal(inner, payload)? {
            return Ok(value);
        }
    }
    let mut map = serde_json::Map::new();
    let mut pairs = Vec::new();
    let mut has_duplicate_key = false;
    let entries = split_object_entries(inner);
    for (index, entry) in entries.iter().enumerate() {
        let entry = entry.trim();
        if entry.is_empty() {
            if index + 1 == entries.len() {
                continue;
            }
            return Err(DwError::Parse("empty object entry".to_string()));
        }
        let entry = if let Some((entry_source, condition_source)) =
            parse_conditional_literal_element(entry)
        {
            if !is_truthy(&evaluate_expression_scoped(
                condition_source,
                payload,
                locals,
            )?) {
                continue;
            }
            strip_wrapping_parens(entry_source)
        } else {
            entry
        };
        let Some((key_source, value_source)) = split_top_level_char(entry, ':') else {
            let merged = evaluate_expression_scoped(entry, payload, locals)?;
            if extend_object_spread(&mut map, &mut pairs, &mut has_duplicate_key, merged)? {
                continue;
            }
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
        let value = evaluate_expression_scoped(value_source.trim(), payload, locals)?;
        if map.contains_key(&key) {
            has_duplicate_key = true;
        }
        map.insert(key.clone(), value.clone());
        pairs.push((key, value));
    }
    if has_duplicate_key {
        Ok(duplicate_object_value(pairs))
    } else {
        Ok(Value::Object(map))
    }
}

fn evaluate_fast_payload_object_literal(
    inner: &str,
    payload: &Value,
) -> Result<Option<Value>, DwError> {
    let entries = split_object_entries(inner);
    if entries.is_empty() {
        return Ok(Some(Value::Object(Map::new())));
    }
    let mut output = Map::new();
    for (index, entry) in entries.iter().enumerate() {
        let entry = entry.trim();
        if entry.is_empty() {
            if index + 1 == entries.len() {
                continue;
            }
            return Ok(None);
        }
        if parse_conditional_literal_element(entry).is_some() {
            return Ok(None);
        }
        let Some((key_source, value_source)) = split_top_level_char(entry, ':') else {
            return Ok(None);
        };
        let Some(key) = parse_fast_static_key(key_source.trim()) else {
            return Ok(None);
        };
        if output.contains_key(key) {
            return Ok(None);
        }
        let Some(value) = evaluate_fast_payload_value(value_source.trim(), payload)? else {
            return Ok(None);
        };
        output.insert(key.to_string(), value);
    }
    Ok(Some(Value::Object(output)))
}

fn parse_fast_static_key(source: &str) -> Option<&str> {
    if is_identifier(source) {
        return Some(source);
    }
    source
        .strip_prefix('"')
        .and_then(|inner| inner.strip_suffix('"'))
        .or_else(|| {
            source
                .strip_prefix('\'')
                .and_then(|inner| inner.strip_suffix('\''))
        })
        .filter(|key| !matches!(*key, "$" | "$$" | "$$$"))
}

fn evaluate_fast_payload_value(source: &str, payload: &Value) -> Result<Option<Value>, DwError> {
    let source = strip_wrapping_parens(source.trim());
    if let Some(inner) = source
        .strip_prefix("upper(")
        .and_then(|inner| inner.strip_suffix(')'))
    {
        let Some(value) = evaluate_fast_payload_value(inner, payload)? else {
            return Ok(None);
        };
        return Ok(Some(Value::String(
            as_dataweave_string(&value).to_uppercase(),
        )));
    }
    if let Some((left, right)) = split_top_level_keyword(source, "default") {
        let Some(left_value) = evaluate_fast_payload_value(left, payload)? else {
            return Ok(None);
        };
        if left_value.is_null() {
            return evaluate_fast_default_literal(right.trim());
        }
        return Ok(Some(left_value));
    }
    if is_fast_payload_path(source) {
        return Ok(Some(evaluate_payload_path_with_collapse(
            source, payload, false,
        )?));
    }
    Ok(None)
}

fn is_fast_payload_path(source: &str) -> bool {
    (source == "payload" || source.starts_with("payload.") || source.starts_with("payload?."))
        && !source.chars().any(char::is_whitespace)
        && !source.contains('[')
        && !source.contains('"')
        && !source.contains('\'')
}

fn evaluate_fast_default_literal(source: &str) -> Result<Option<Value>, DwError> {
    if let Some(value) = parse_literal(source)? {
        return Ok(Some(value));
    }
    if let Some(value) = parse_string_literal(source)? {
        return Ok(Some(Value::String(value)));
    }
    Ok(None)
}

fn extend_object_spread(
    target: &mut Map<String, Value>,
    pairs: &mut Vec<(String, Value)>,
    has_duplicate_key: &mut bool,
    value: Value,
) -> Result<bool, DwError> {
    match value {
        Value::Object(merged) => {
            let entries = duplicate_object_pairs(&Value::Object(merged.clone()))
                .unwrap_or_else(|| merged.into_iter().collect());
            for (key, value) in entries {
                if target.contains_key(&key) {
                    *has_duplicate_key = true;
                }
                target.insert(key.clone(), value.clone());
                pairs.push((key, value));
            }
            Ok(true)
        }
        Value::Array(items) => {
            for item in items {
                let Value::Object(merged) = item else {
                    return Err(DwError::Parse(format!(
                        "object spread array item must be an object, got {item:?}"
                    )));
                };
                let entries = duplicate_object_pairs(&Value::Object(merged.clone()))
                    .unwrap_or_else(|| merged.into_iter().collect());
                for (key, value) in entries {
                    if target.contains_key(&key) {
                        *has_duplicate_key = true;
                    }
                    target.insert(key.clone(), value.clone());
                    pairs.push((key, value));
                }
            }
            Ok(true)
        }
        _ => Ok(false),
    }
}

fn split_object_entries(source: &str) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut start = 0usize;
    let mut depth = 0i32;
    let mut angle_depth = 0i32;
    let mut in_string: Option<char> = None;
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
            '/' if starts_regex_literal(source, index) => in_string = Some('/'),
            '"' | '\'' | '`' | '|' => in_string = Some(ch),
            '<' if starts_type_argument_list(source, index) => angle_depth += 1,
            '>' if angle_depth > 0 => angle_depth -= 1,
            '{' | '[' | '(' => depth += 1,
            '}' | ']' | ')' => depth -= 1,
            ',' if depth == 0 && angle_depth == 0 => {
                parts.push(&source[start..index]);
                start = index + ch.len_utf8();
            }
            '\n' if depth == 0
                && angle_depth == 0
                && object_entry_can_end_at_line(source[start..index].trim())
                && next_line_starts_object_entry(&source[index + ch.len_utf8()..]) =>
            {
                parts.push(&source[start..index]);
                start = index + ch.len_utf8();
            }
            _ => {}
        }
    }
    parts.push(&source[start..]);
    parts
}

fn object_entry_can_end_at_line(source: &str) -> bool {
    if source.trim_end().ends_with(':') {
        return false;
    }
    let Some((_, value_source)) = split_top_level_char(source, ':') else {
        return false;
    };
    if value_source.trim().is_empty() {
        return false;
    }
    !source_ends_with_collection_operator(source)
}

fn source_ends_with_collection_operator(source: &str) -> bool {
    let Some(last_token) = source.split_whitespace().last() else {
        return false;
    };
    matches!(
        last_token,
        "groupBy"
            | "map"
            | "pluck"
            | "mapObject"
            | "filter"
            | "filterObject"
            | "flatMap"
            | "distinctBy"
            | "orderBy"
            | "reduce"
            | "then"
            | "onNull"
            | "takeWhile"
            | "dropWhile"
            | "some"
            | "every"
            | "countCharactersBy"
            | "everyCharacter"
            | "mapString"
            | "someCharacter"
            | "substringBy"
            | "countBy"
            | "sumBy"
            | "firstWith"
            | "indexWhere"
            | "partition"
            | "splitWhere"
            | "everyEntry"
            | "someEntry"
            | "scan"
            | "mapLeafValues"
            | "nodeExists"
            | "filterArrayLeafs"
            | "filterObjectLeafs"
            | "failIf"
            | "wait"
    )
}

fn starts_regex_literal(source: &str, index: usize) -> bool {
    source[index..].starts_with('/')
        && source[..index]
            .chars()
            .rev()
            .find(|ch| !ch.is_whitespace())
            .is_none_or(|ch| matches!(ch, '(' | '[' | '{' | ':' | ',' | '='))
}

fn starts_type_argument_list(source: &str, index: usize) -> bool {
    source[index..].starts_with('<')
        && source[index + '<'.len_utf8()..]
            .chars()
            .next()
            .is_some_and(|next| next.is_ascii_alphabetic() || next == '_')
        && source[..index]
            .chars()
            .rev()
            .find(|ch| !ch.is_whitespace())
            .is_some_and(|previous| {
                previous.is_ascii_alphabetic() || previous == '_' || previous == '>'
            })
}

fn next_line_starts_object_entry(source: &str) -> bool {
    let rest = source.trim_start();
    if rest.is_empty() {
        return false;
    }
    let line = rest.lines().next().unwrap_or(rest);
    split_top_level_char(line, ':').is_some()
}

pub(crate) fn evaluate_array_literal_scoped(
    source: &str,
    payload: &Value,
    locals: &Map<String, Value>,
) -> Result<Value, DwError> {
    let inner = &source[1..source.len() - 1];
    if inner.trim().is_empty() {
        return Ok(Value::Array(Vec::new()));
    }
    let entries = split_top_level(inner, ',');
    let mut output = Vec::new();
    for (index, item) in entries.iter().enumerate() {
        let item = item.trim();
        if item.is_empty() {
            if index + 1 == entries.len() {
                continue;
            }
            return Err(DwError::Parse("empty array entry".to_string()));
        }
        if let Some((item_source, condition_source)) = parse_conditional_literal_element(item) {
            if is_truthy(&evaluate_expression_scoped(
                condition_source,
                payload,
                locals,
            )?) {
                output.push(evaluate_expression_scoped(item_source, payload, locals)?);
            }
            continue;
        }
        output.push(evaluate_expression_scoped(item, payload, locals)?);
    }
    Ok(Value::Array(output))
}

fn parse_conditional_literal_element(source: &str) -> Option<(&str, &str)> {
    let (item_source, _, condition_source) =
        split_top_level_keyword_or_call_operator(source, &["if"])?;
    let item_source = item_source.trim();
    if item_source.starts_with('(')
        && item_source.ends_with(')')
        && find_matching_delimiter(item_source, 0, '(', ')') == Some(item_source.len() - 1)
    {
        return Some((item_source, condition_source));
    }
    None
}

pub(crate) fn evaluate_selector_index_scoped(
    base: &Value,
    index_source: &str,
    payload: &Value,
    locals: &Map<String, Value>,
) -> Result<Option<Value>, DwError> {
    let index_source = index_source.trim();
    if let Some(predicate) = bracket_call_inner(index_source, '?') {
        return evaluate_filter_selector(base, predicate, payload, locals).map(Some);
    }
    if let Some(selector) = bracket_call_inner(index_source, '*') {
        let key = evaluate_expression_scoped(selector, payload, locals)?;
        return Ok(Some(select_dynamic_multi_property(
            base,
            &as_dataweave_string(&key),
        )));
    }
    if let Some(selector) = bracket_call_inner(index_source, '&') {
        let key = evaluate_expression_scoped(selector, payload, locals)?;
        return Ok(Some(select_key_value_pairs(
            base,
            &as_dataweave_string(&key),
        )));
    }
    Ok(None)
}

fn bracket_call_inner(source: &str, prefix: char) -> Option<&str> {
    source
        .strip_prefix(prefix)?
        .strip_prefix('(')?
        .strip_suffix(')')
        .map(str::trim)
}

fn evaluate_filter_selector(
    base: &Value,
    predicate: &str,
    payload: &Value,
    locals: &Map<String, Value>,
) -> Result<Value, DwError> {
    match base {
        Value::Null => Ok(Value::Null),
        Value::Array(items) => {
            let mut output = Vec::new();
            for (index, item) in items.iter().enumerate() {
                if selector_predicate_matches(
                    item,
                    Value::Number((index as i64).into()),
                    predicate,
                    payload,
                    locals,
                )? {
                    output.push(item.clone());
                }
            }
            if output.is_empty() {
                Ok(Value::Null)
            } else {
                Ok(Value::Array(output))
            }
        }
        Value::Object(map) => {
            let mut output = Map::new();
            for (key, value) in map {
                if let Some(items) = xml_list_items(value) {
                    let mut selected = None;
                    for (index, item) in items.iter().enumerate() {
                        let item = collapse_xml_like_value(item);
                        if selector_predicate_matches(
                            &item,
                            Value::Number((index as i64).into()),
                            predicate,
                            payload,
                            locals,
                        )? {
                            selected = Some(item);
                            break;
                        }
                    }
                    if let Some(selected) = selected {
                        output.insert(key.clone(), selected);
                    }
                    continue;
                }
                if selector_predicate_matches(
                    value,
                    Value::String(key.clone()),
                    predicate,
                    payload,
                    locals,
                )? {
                    output.insert(key.clone(), value.clone());
                }
            }
            if output.is_empty() {
                Ok(Value::Null)
            } else {
                Ok(Value::Object(output))
            }
        }
        value => {
            if selector_predicate_matches(
                value,
                Value::Number(0.into()),
                predicate,
                payload,
                locals,
            )? {
                Ok(value.clone())
            } else {
                Ok(Value::Null)
            }
        }
    }
}

fn selector_predicate_matches(
    value: &Value,
    key_or_index: Value,
    predicate: &str,
    payload: &Value,
    locals: &Map<String, Value>,
) -> Result<bool, DwError> {
    let mut scoped = locals.clone();
    scoped.insert("$".to_string(), value.clone());
    scoped.insert("$$".to_string(), key_or_index);
    Ok(is_truthy(&evaluate_expression_scoped(
        predicate, payload, &scoped,
    )?))
}
