use regex::Regex;
use serde_json::{Map, Value};

use crate::builtins::{binary_value, to_string_with_options};
use crate::literals::parse_string_literal;
use crate::periods::{
    concatenate_temporals, evaluate_period_additive, period_literal, period_or_temporal_to_number,
};
use crate::selectors::{duplicate_object_pairs, duplicate_object_value, value_with_metadata};
use crate::syntax::split_top_level;
use crate::{as_dataweave_string, number_result, DwError};

pub(crate) fn evaluate_matches(left: &Value, right: &Value) -> Result<Value, DwError> {
    if left.is_null() {
        return Ok(Value::Bool(false));
    }
    let text = as_dataweave_string(left);
    let mut pattern = as_dataweave_string(right);
    if pattern.starts_with('/') && pattern.ends_with('/') && pattern.len() >= 2 {
        pattern = pattern[1..pattern.len() - 1].to_string();
    }
    Ok(Value::Bool(simple_full_match(&text, &pattern)?))
}

pub(crate) fn simple_full_match(text: &str, pattern: &str) -> Result<bool, DwError> {
    if let Some(inner) = pattern
        .strip_prefix('^')
        .and_then(|value| value.strip_suffix('$'))
    {
        return simple_anchored_match(text, inner);
    }
    if !pattern.contains(['[', ']', '+', '*', '?', '\\', '.', '|', '(', ')', '^', '$']) {
        return Ok(text == pattern);
    }
    let regex = Regex::new(pattern)
        .map_err(|err| DwError::UnsupportedFeature(format!("regex pattern /{pattern}/: {err}")))?;
    let matches = regex
        .find_iter(text)
        .any(|matched| matched.start() == 0 && matched.end() == text.len());
    Ok(matches)
}

fn simple_anchored_match(text: &str, pattern: &str) -> Result<bool, DwError> {
    match pattern {
        "[0-9]+" | "\\d+" => Ok(!text.is_empty() && text.chars().all(|ch| ch.is_ascii_digit())),
        "[0-9]*" | "\\d*" => Ok(text.chars().all(|ch| ch.is_ascii_digit())),
        "[A-Za-z]+" => Ok(!text.is_empty() && text.chars().all(|ch| ch.is_ascii_alphabetic())),
        "[A-Za-z0-9]+" => Ok(!text.is_empty() && text.chars().all(|ch| ch.is_ascii_alphanumeric())),
        ".*" => Ok(true),
        _ if !pattern.contains(['[', ']', '+', '*', '?', '\\', '.', '|', '(', ')']) => {
            Ok(text == pattern)
        }
        _ => Err(DwError::UnsupportedFeature(format!(
            "regex pattern /{pattern}/"
        ))),
    }
}

pub(crate) fn evaluate_index_access(base: &Value, index: &Value) -> Result<Value, DwError> {
    match base {
        Value::Array(items) => {
            if let Value::Array(indices) = index {
                return indices
                    .iter()
                    .map(|index| evaluate_index_access(base, index))
                    .collect::<Result<Vec<_>, _>>()
                    .map(Value::Array);
            }
            let Some(index) = index.as_i64() else {
                return Err(DwError::UnsupportedFeature(format!(
                    "non-integer array index {index:?}"
                )));
            };
            let resolved_index = if index < 0 {
                items.len() as i64 + index
            } else {
                index
            };
            if resolved_index < 0 {
                return Ok(Value::Null);
            }
            Ok(items
                .get(resolved_index as usize)
                .cloned()
                .unwrap_or(Value::Null))
        }
        Value::String(value) => {
            if let Value::Array(indices) = index {
                let mut output = String::new();
                for index in indices {
                    if let Value::String(character) = evaluate_index_access(base, index)? {
                        output.push_str(&character);
                    }
                }
                return Ok(Value::String(output));
            }
            let Some(index) = index.as_i64() else {
                return Err(DwError::UnsupportedFeature(format!(
                    "non-integer string index {index:?}"
                )));
            };
            let chars = value.chars().collect::<Vec<_>>();
            let resolved_index = if index < 0 {
                chars.len() as i64 + index
            } else {
                index
            };
            if resolved_index < 0 {
                return Ok(Value::Null);
            }
            Ok(chars
                .get(resolved_index as usize)
                .map(|ch| Value::String(ch.to_string()))
                .unwrap_or(Value::Null))
        }
        Value::Object(map) => match index {
            Value::String(key) => {
                if let Some(pairs) = duplicate_object_pairs(base) {
                    return Ok(pairs
                        .into_iter()
                        .find(|(pair_key, _)| pair_key == key)
                        .map(|(_, value)| value)
                        .unwrap_or(Value::Null));
                }
                Ok(map.get(key).cloned().unwrap_or(Value::Null))
            }
            Value::Number(key) => {
                if let Some(value) = map.get(&key.to_string()) {
                    return Ok(value.clone());
                }
                let Some(index) = key.as_i64() else {
                    return Ok(Value::Null);
                };
                let resolved_index = if index < 0 {
                    map.len() as i64 + index
                } else {
                    index
                };
                if resolved_index < 0 {
                    return Ok(Value::Null);
                }
                Ok(map
                    .values()
                    .nth(resolved_index as usize)
                    .cloned()
                    .unwrap_or(Value::Null))
            }
            _ => Err(DwError::UnsupportedFeature(format!(
                "unsupported object index {index:?}"
            ))),
        },
        _ => Ok(Value::Null),
    }
}

pub(crate) fn evaluate_index_range(
    base: &Value,
    start: &Value,
    end: &Value,
) -> Result<Value, DwError> {
    let start = number_value(start)? as i64;
    let end = number_value(end)? as i64;
    match base {
        Value::Array(items) => {
            let indices = resolved_index_range(items.len(), start, end);
            indices
                .into_iter()
                .map(|index| evaluate_index_access(base, &Value::Number(index.into())))
                .collect::<Result<Vec<_>, _>>()
                .map(Value::Array)
        }
        Value::String(value) => {
            let indices = resolved_index_range(value.chars().count(), start, end);
            let mut output = String::new();
            for index in indices {
                if let Value::String(character) =
                    evaluate_index_access(base, &Value::Number(index.into()))?
                {
                    output.push_str(&character);
                }
            }
            Ok(Value::String(output))
        }
        _ => Ok(Value::Null),
    }
}

fn resolved_index_range(len: usize, start: i64, end: i64) -> Vec<i64> {
    let len = len as i64;
    let resolve = |index: i64| if index < 0 { len + index } else { index };
    let start = resolve(start);
    let end = resolve(end);
    let step = if end >= start { 1 } else { -1 };
    let mut values = Vec::new();
    let mut current = start;
    loop {
        values.push(current);
        if current == end {
            break;
        }
        current += step;
    }
    values
}

pub(crate) fn evaluate_range(left: &Value, right: &Value) -> Result<Value, DwError> {
    let start = number_value(left)? as i64;
    let end = number_value(right)? as i64;
    let step = if end >= start { 1 } else { -1 };
    let mut values = Vec::new();
    let mut current = start;
    loop {
        values.push(Value::Number(current.into()));
        if current == end {
            break;
        }
        current += step;
    }
    Ok(Value::Array(values))
}

pub(crate) fn evaluate_comparison(
    left: &Value,
    operator: &str,
    right: &Value,
) -> Result<Value, DwError> {
    let result = match operator {
        "==" => left == right,
        "!=" => left != right,
        ">" => compare_values(left, right)? > 0,
        "<" => compare_values(left, right)? < 0,
        ">=" => compare_values(left, right)? >= 0,
        "<=" => compare_values(left, right)? <= 0,
        _ => return Err(DwError::UnsupportedFeature(operator.to_string())),
    };
    Ok(Value::Bool(result))
}

pub(crate) fn evaluate_additive(
    left: &Value,
    operator: &str,
    right: &Value,
) -> Result<Value, DwError> {
    match operator {
        "++" => match (left, right) {
            _ if concatenate_temporals(left, right).is_some() => {
                Ok(concatenate_temporals(left, right).unwrap())
            }
            (Value::Array(left), Value::Array(right)) => {
                let mut combined = left.clone();
                combined.extend(right.clone());
                Ok(Value::Array(combined))
            }
            (Value::Object(left_map), Value::Object(right_map)) => {
                if duplicate_object_pairs(left).is_some() || duplicate_object_pairs(right).is_some()
                {
                    return Ok(duplicate_object_value(concat_object_pairs(left, right)));
                }
                let mut combined = left_map.clone();
                combined.extend(right_map.clone());
                Ok(Value::Object(combined))
            }
            _ => Ok(Value::String(format!(
                "{}{}",
                as_dataweave_string(left),
                as_dataweave_string(right)
            ))),
        },
        "--" => evaluate_difference(left, right),
        "+" => {
            if let Some(value) = evaluate_period_additive(left, operator, right)? {
                return Ok(value);
            }
            if let Value::Array(items) = left {
                let mut combined = items.clone();
                combined.push(right.clone());
                return Ok(Value::Array(combined));
            }
            if let Some(value) = add_date_and_period(left, right)? {
                return Ok(value);
            }
            if let Some(value) = add_date_and_period(right, left)? {
                return Ok(value);
            }
            number_result(number_value(left)? + number_value(right)?)
        }
        "-" => {
            if let Some(value) = evaluate_period_additive(left, operator, right)? {
                return Ok(value);
            }
            if !left.is_number() {
                return evaluate_difference(left, right);
            }
            number_result(number_value(left)? - number_value(right)?)
        }
        _ => Err(DwError::UnsupportedFeature(operator.to_string())),
    }
}

pub(crate) fn evaluate_shift(
    left: &Value,
    operator: &str,
    right: &Value,
) -> Result<Value, DwError> {
    match operator {
        ">>" => {
            if let Some(shifted) = shift_datetime_zone(left, right) {
                return Ok(shifted);
            }
            let Value::Array(items) = right else {
                return Err(DwError::UnsupportedFeature(format!(
                    "shift operator {left:?} >> {right:?}"
                )));
            };
            let mut output = Vec::with_capacity(items.len() + 1);
            output.push(left.clone());
            output.extend(items.clone());
            Ok(Value::Array(output))
        }
        "<<" => {
            let Value::Array(items) = left else {
                return Err(DwError::UnsupportedFeature(format!(
                    "shift operator {left:?} << {right:?}"
                )));
            };
            let mut output = items.clone();
            output.push(right.clone());
            Ok(Value::Array(output))
        }
        _ => Err(DwError::UnsupportedFeature(operator.to_string())),
    }
}

fn shift_datetime_zone(value: &Value, zone: &Value) -> Option<Value> {
    let source = as_dataweave_string(value);
    let target_zone = as_dataweave_string(zone);
    let target_offset = zone_offset_seconds(&target_zone, &source)?;
    let (epoch_seconds, millis) = parse_datetime_epoch_seconds(&source)?;
    let shifted = epoch_seconds + target_offset;
    let days = shifted.div_euclid(86_400);
    let seconds = shifted.rem_euclid(86_400);
    let (year, month, day) = civil_from_unix_days(days);
    let hour = seconds / 3_600;
    let minute = (seconds % 3_600) / 60;
    let second = seconds % 60;
    let offset = format_offset(target_offset);
    Some(Value::String(format!(
        "{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}.{millis:03}{offset}"
    )))
}

fn parse_datetime_epoch_seconds(source: &str) -> Option<(i64, i64)> {
    if source.len() < 20 || source.as_bytes().get(10) != Some(&b'T') {
        return None;
    }
    let (year, month, day) = parse_iso_date(source.get(..10)?)?;
    let hour = source.get(11..13)?.parse::<i64>().ok()?;
    let minute = source.get(14..16)?.parse::<i64>().ok()?;
    let second = source.get(17..19)?.parse::<i64>().ok()?;
    let (millis, offset) = parse_fraction_and_offset(source.get(19..)?)?;
    let local_seconds =
        unix_days_from_civil(year, month, day) * 86_400 + hour * 3_600 + minute * 60 + second;
    Some((local_seconds - offset, millis))
}

fn parse_fraction_and_offset(source: &str) -> Option<(i64, i64)> {
    let mut rest = source;
    let mut millis = 0;
    if let Some(fraction) = rest.strip_prefix('.') {
        let digits = fraction
            .chars()
            .take_while(|ch| ch.is_ascii_digit())
            .collect::<String>();
        let millis_text = format!("{digits:0<3}");
        millis = millis_text.get(..3)?.parse().ok()?;
        rest = &fraction[digits.len()..];
    }
    Some((millis, parse_offset_seconds(rest)?))
}

fn parse_offset_seconds(source: &str) -> Option<i64> {
    if source == "Z" {
        return Some(0);
    }
    if source.len() != 6 {
        return None;
    }
    let sign = match source.as_bytes()[0] {
        b'+' => 1,
        b'-' => -1,
        _ => return None,
    };
    if source.as_bytes()[3] != b':' {
        return None;
    }
    let hours = source.get(1..3)?.parse::<i64>().ok()?;
    let minutes = source.get(4..6)?.parse::<i64>().ok()?;
    Some(sign * (hours * 3_600 + minutes * 60))
}

fn zone_offset_seconds(zone: &str, source: &str) -> Option<i64> {
    match zone {
        "UTC" | "GMT" | "Z" => Some(0),
        "CET" => Some(3_600),
        "America/New_York" => Some(if is_rough_northern_dst(source) {
            -4 * 3_600
        } else {
            -5 * 3_600
        }),
        _ if zone.len() == 6 && matches!(zone.as_bytes()[0], b'+' | b'-') => {
            parse_offset_seconds(zone)
        }
        _ => None,
    }
}

fn is_rough_northern_dst(source: &str) -> bool {
    source
        .get(5..7)
        .and_then(|month| month.parse::<i64>().ok())
        .is_some_and(|month| (3..=10).contains(&month))
}

fn format_offset(seconds: i64) -> String {
    if seconds == 0 {
        return "Z".to_string();
    }
    let sign = if seconds < 0 { '-' } else { '+' };
    let seconds = seconds.abs();
    format!("{sign}{:02}:{:02}", seconds / 3_600, (seconds % 3_600) / 60)
}

fn concat_object_pairs(left: &Value, right: &Value) -> Vec<(String, Value)> {
    let mut output = object_concat_pairs(left);
    output.extend(object_concat_pairs(right));
    output
}

fn object_concat_pairs(value: &Value) -> Vec<(String, Value)> {
    if let Some(pairs) = duplicate_object_pairs(value) {
        return pairs;
    }
    match value {
        Value::Object(map) => map
            .iter()
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect(),
        _ => Vec::new(),
    }
}

fn evaluate_difference(left: &Value, right: &Value) -> Result<Value, DwError> {
    match left {
        Value::Null => Ok(Value::Null),
        Value::Array(items) => {
            let removals = match right {
                Value::Array(values) => values.clone(),
                value => vec![value.clone()],
            };
            Ok(Value::Array(
                items
                    .iter()
                    .filter(|item| !removals.iter().any(|removal| removal == *item))
                    .cloned()
                    .collect(),
            ))
        }
        Value::Object(map) => {
            let keys = difference_keys(right)?;
            Ok(Value::Object(
                map.iter()
                    .filter(|(key, _)| !keys.iter().any(|removal| removal == *key))
                    .map(|(key, value)| (key.clone(), value.clone()))
                    .collect(),
            ))
        }
        Value::String(value) => {
            let needle = as_dataweave_string(right);
            if needle.is_empty() {
                return Ok(Value::String(value.clone()));
            }
            Ok(Value::String(value.replace(&needle, "")))
        }
        _ => Err(DwError::UnsupportedFeature(format!(
            "difference between {left:?} and {right:?}"
        ))),
    }
}

fn difference_keys(value: &Value) -> Result<Vec<String>, DwError> {
    match value {
        Value::Object(map) => Ok(map.keys().cloned().collect()),
        Value::Array(items) => Ok(items.iter().map(as_dataweave_string).collect()),
        Value::String(key) => Ok(vec![key.clone()]),
        _ => Err(DwError::UnsupportedFeature(format!(
            "object difference with {value:?}"
        ))),
    }
}

pub(crate) fn evaluate_multiplicative(
    left: &Value,
    operator: &str,
    right: &Value,
) -> Result<Value, DwError> {
    match operator {
        "*" => number_result(number_value(left)? * number_value(right)?),
        "/" => number_result(number_value(left)? / number_value(right)?),
        _ => Err(DwError::UnsupportedFeature(operator.to_string())),
    }
}

pub(crate) fn evaluate_coercion(value: &Value, type_source: &str) -> Result<Value, DwError> {
    let target_type = type_source.split_whitespace().next().unwrap_or_default();
    if type_source.starts_with("Array<") && type_source.ends_with('>') {
        return match value {
            Value::Array(_) | Value::Null => Ok(value.clone()),
            _ => Err(DwError::UnsupportedFeature(format!(
                "cannot coerce {value:?} to {type_source}"
            ))),
        };
    }
    if target_type.is_empty() || type_source.contains('<') {
        return Err(DwError::UnsupportedFeature(format!(
            "coercion as {type_source}"
        )));
    }

    match target_type {
        "String" => {
            if value.is_null() {
                Ok(Value::Null)
            } else if let Some(options) = extract_coercion_options(type_source)? {
                let Some(format_value) = options.format else {
                    if !options.metadata.is_empty() {
                        return Ok(value_with_metadata(value.clone(), options.metadata));
                    }
                    return Ok(Value::String(as_dataweave_string(value)));
                };
                let format = Value::String(format_value);
                let locale = options.locale.map(Value::String);
                let coerced = to_string_with_options(value, Some(&format), locale.as_ref(), None)?;
                if options.metadata.is_empty() {
                    Ok(coerced)
                } else {
                    Ok(value_with_metadata(coerced, options.metadata))
                }
            } else {
                Ok(Value::String(as_dataweave_string(value)))
            }
        }
        "Number" => {
            if let Some(options) = extract_coercion_options(type_source)? {
                if let Some(unit) = options.unit.as_deref() {
                    if let Some(number) = period_or_temporal_to_number(value, &unit) {
                        return Ok(number);
                    }
                }
                let number = coerce_number(value)?;
                let metadata = coercion_metadata(options);
                if metadata.is_empty() {
                    Ok(number)
                } else {
                    Ok(value_with_metadata(number, metadata))
                }
            } else {
                coerce_number(value)
            }
        }
        "Boolean" => coerce_boolean(value),
        "Binary" => coerce_binary(value),
        "Time" => coerce_time(value, true),
        "LocalTime" => coerce_time(value, false),
        "Period" => coerce_period(value),
        "LocalDateTime" | "DateTime" => {
            let format = extract_coercion_options(type_source)?.and_then(|options| options.format);
            coerce_local_datetime(value, format.as_deref())
        }
        "Date" => {
            let options = extract_coercion_options(type_source)?;
            let date = coerce_date(
                value,
                options
                    .as_ref()
                    .and_then(|options| options.format.as_deref()),
            )?;
            if let Some(string_type) = type_source.split_once(" as ").map(|(_, right)| right) {
                return evaluate_coercion(&date, string_type);
            }
            if let Some(options) = options {
                let mut metadata = options.metadata;
                if let Some(format) = options.format {
                    metadata.insert("format".to_string(), Value::String(format));
                }
                if let Some(locale) = options.locale {
                    metadata.insert("locale".to_string(), Value::String(locale));
                }
                if metadata.is_empty() {
                    Ok(date)
                } else {
                    Ok(value_with_metadata(date, metadata))
                }
            } else {
                Ok(date)
            }
        }
        "Key" | "Regex" | "Uri" => Ok(Value::String(as_dataweave_string(value))),
        _ => Err(DwError::UnsupportedFeature(format!(
            "coercion as {target_type}"
        ))),
    }
}

fn coerce_period(value: &Value) -> Result<Value, DwError> {
    match value {
        Value::Null => Ok(Value::Null),
        Value::String(text) => period_literal(text).ok_or_else(|| {
            DwError::UnsupportedFeature(format!("cannot coerce string '{text}' to Period"))
        }),
        Value::Object(map) if map.contains_key(crate::periods::DW_PERIOD_MARKER) => {
            Ok(value.clone())
        }
        _ => Err(DwError::UnsupportedFeature(format!(
            "cannot coerce {} to Period",
            as_dataweave_string(value)
        ))),
    }
}

fn coerce_binary(value: &Value) -> Result<Value, DwError> {
    match value {
        Value::Null => Ok(Value::Null),
        Value::String(value) => Ok(binary_value(value.as_bytes().to_vec())),
        Value::Number(value) => {
            let Some(byte) = value.as_u64().filter(|byte| *byte <= u8::MAX as u64) else {
                return Err(DwError::UnsupportedFeature(format!(
                    "cannot coerce {value} to Binary"
                )));
            };
            Ok(binary_value(vec![byte as u8]))
        }
        Value::Object(map) if map.contains_key(crate::builtins::DW_BINARY_MARKER) => {
            Ok(value.clone())
        }
        _ => Err(DwError::UnsupportedFeature(format!(
            "cannot coerce {value:?} to Binary"
        ))),
    }
}

fn coerce_time(value: &Value, default_zone: bool) -> Result<Value, DwError> {
    let text = as_dataweave_string(value);
    let time = text
        .split_once('T')
        .map(|(_, time)| time)
        .unwrap_or(text.as_str());
    let has_zone = time.ends_with('Z')
        || (time.len() > 6
            && matches!(time.as_bytes()[time.len() - 6], b'+' | b'-')
            && time.as_bytes()[time.len() - 3] == b':');
    if default_zone && !has_zone {
        Ok(Value::String(format!("{time}Z")))
    } else {
        Ok(Value::String(time.to_string()))
    }
}

struct CoercionOptions {
    format: Option<String>,
    locale: Option<String>,
    unit: Option<String>,
    metadata: serde_json::Map<String, Value>,
}

fn coercion_metadata(options: CoercionOptions) -> Map<String, Value> {
    let mut metadata = options.metadata;
    if let Some(format) = options.format {
        metadata.insert("format".to_string(), Value::String(format));
    }
    if let Some(locale) = options.locale {
        metadata.insert("locale".to_string(), Value::String(locale));
    }
    metadata
}

fn extract_coercion_options(type_source: &str) -> Result<Option<CoercionOptions>, DwError> {
    let Some(open) = type_source.find('{') else {
        return Ok(None);
    };
    let close = type_source
        .rfind('}')
        .ok_or_else(|| DwError::Parse(format!("invalid coercion options {type_source}")))?;
    let options = &type_source[open + 1..close];
    let mut format = None;
    let mut locale = None;
    let mut unit = None;
    let mut metadata = serde_json::Map::new();
    for entry in split_top_level(options, ',') {
        let Some((key, value)) = entry.split_once(':') else {
            continue;
        };
        let parsed = parse_string_literal(value.trim())?;
        match key.trim() {
            "format" => format = parsed,
            "locale" => locale = parsed,
            "unit" => unit = parsed,
            _ => {
                if let Some(value) = parsed {
                    metadata.insert(key.trim().to_string(), Value::String(value));
                }
            }
        }
    }
    if format.is_none() && locale.is_none() && unit.is_none() && metadata.is_empty() {
        return Ok(None);
    }
    Ok(Some(CoercionOptions {
        format,
        locale,
        unit,
        metadata,
    }))
}

fn coerce_number(value: &Value) -> Result<Value, DwError> {
    match value {
        Value::Null => Ok(Value::Null),
        Value::Number(_) => Ok(value.clone()),
        Value::Bool(true) => Ok(Value::Number(1.into())),
        Value::Bool(false) => Ok(Value::Number(0.into())),
        Value::String(value) => {
            if let Some((epoch_seconds, _)) = parse_datetime_epoch_seconds(value) {
                return Ok(Value::Number(epoch_seconds.into()));
            }
            if let Ok(number) = value.parse::<i64>() {
                return Ok(Value::Number(number.into()));
            }
            let number = value.parse::<f64>().map_err(|_| {
                DwError::UnsupportedFeature(format!("cannot coerce string '{value}' to Number"))
            })?;
            number_result(number)
        }
        _ => Err(DwError::UnsupportedFeature(format!(
            "cannot coerce {value:?} to Number"
        ))),
    }
}

fn coerce_boolean(value: &Value) -> Result<Value, DwError> {
    match value {
        Value::Null => Ok(Value::Null),
        Value::Bool(_) => Ok(value.clone()),
        Value::Number(number) => Ok(Value::Bool(number.as_f64().unwrap_or(0.0) != 0.0)),
        Value::String(value) if value.is_empty() => Ok(Value::Bool(false)),
        Value::String(value) if value.eq_ignore_ascii_case("true") => Ok(Value::Bool(true)),
        Value::String(value) if value.eq_ignore_ascii_case("false") => Ok(Value::Bool(false)),
        Value::String(value) => Err(DwError::UnsupportedFeature(format!(
            "cannot coerce string '{value}' to Boolean"
        ))),
        _ => Err(DwError::UnsupportedFeature(format!(
            "cannot coerce {value:?} to Boolean"
        ))),
    }
}

fn coerce_date(value: &Value, format: Option<&str>) -> Result<Value, DwError> {
    match value {
        Value::Null => Ok(Value::Null),
        Value::String(value) => {
            if let Some(format) = format {
                if let Some(date) = parse_formatted_date(value, format) {
                    return Ok(Value::String(date));
                }
            }
            let date = value
                .get(..10)
                .filter(|date| is_iso_date(date))
                .ok_or_else(|| {
                    DwError::UnsupportedFeature(format!("cannot coerce string '{value}' to Date"))
                })?;
            Ok(Value::String(date.to_string()))
        }
        _ => Err(DwError::UnsupportedFeature(format!(
            "cannot coerce {value:?} to Date"
        ))),
    }
}

fn parse_formatted_date(value: &str, format: &str) -> Option<String> {
    match format {
        "dd-MMM-yy" => {
            let mut parts = value.split('-');
            let day = parts.next()?.parse::<i64>().ok()?;
            let month = month_name_number(parts.next()?)?;
            let year = parts.next()?.parse::<i64>().ok()?;
            if parts.next().is_some() {
                return None;
            }
            Some(format!("{:04}-{month:02}-{day:02}", 2000 + year))
        }
        "dd-MM-yyyy" => {
            let mut parts = value.split('-');
            let day = parts.next()?.parse::<i64>().ok()?;
            let month = parts.next()?.parse::<i64>().ok()?;
            let year = parts.next()?.parse::<i64>().ok()?;
            if parts.next().is_some() {
                return None;
            }
            Some(format!("{year:04}-{month:02}-{day:02}"))
        }
        _ => None,
    }
}

fn month_name_number(value: &str) -> Option<i64> {
    match value.to_ascii_uppercase().trim_end_matches('.') {
        "JAN" => Some(1),
        "FEB" => Some(2),
        "MAR" => Some(3),
        "APR" => Some(4),
        "MAY" => Some(5),
        "JUN" => Some(6),
        "JUL" => Some(7),
        "AUG" => Some(8),
        "SEP" => Some(9),
        "OCT" => Some(10),
        "NOV" => Some(11),
        "DEC" => Some(12),
        _ => None,
    }
}

fn coerce_local_datetime(value: &Value, format: Option<&str>) -> Result<Value, DwError> {
    let text = as_dataweave_string(value);
    if let Some(format) = format {
        if format == "uuuuMMddHHmm" && text.len() == 12 {
            let year = parse_i64(&text[0..4])?;
            let month = parse_i64(&text[4..6])?;
            let day = parse_i64(&text[6..8])?;
            let hour = parse_i64(&text[8..10])?;
            let minute = parse_i64(&text[10..12])?;
            return Ok(Value::String(format!(
                "{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:00"
            )));
        }
        if format == "M/dd/uuuu h:mm:ss a" {
            if let Some(value) = parse_us_datetime_with_ampm(&text) {
                return Ok(Value::String(value));
            }
        }
    }
    if text.len() >= 19 && text.as_bytes().get(10) == Some(&b'T') {
        return Ok(Value::String(text));
    }
    Err(DwError::UnsupportedFeature(format!(
        "cannot coerce string '{text}' to LocalDateTime"
    )))
}

fn parse_us_datetime_with_ampm(text: &str) -> Option<String> {
    let mut parts = text.split_whitespace();
    let date = parts.next()?;
    let time = parts.next()?;
    let period = parts.next()?;
    if parts.next().is_some() {
        return None;
    }
    let date_parts = date.split('/').collect::<Vec<_>>();
    if date_parts.len() != 3 {
        return None;
    }
    let month = date_parts[0].parse::<i64>().ok()?;
    let day = date_parts[1].parse::<i64>().ok()?;
    let year = date_parts[2].parse::<i64>().ok()?;
    let time_parts = time.split(':').collect::<Vec<_>>();
    if time_parts.len() != 3 {
        return None;
    }
    let mut hour = time_parts[0].parse::<i64>().ok()?;
    let minute = time_parts[1].parse::<i64>().ok()?;
    let second = time_parts[2].parse::<i64>().ok()?;
    match period {
        "AM" if hour == 12 => hour = 0,
        "AM" => {}
        "PM" if hour != 12 => hour += 12,
        "PM" => {}
        _ => return None,
    }
    Some(format!(
        "{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}"
    ))
}

fn parse_i64(value: &str) -> Result<i64, DwError> {
    value
        .parse::<i64>()
        .map_err(|_| DwError::UnsupportedFeature(format!("invalid datetime component {value}")))
}

fn add_date_and_period(date_value: &Value, period_value: &Value) -> Result<Option<Value>, DwError> {
    let (Value::String(date), Value::String(period)) = (date_value, period_value) else {
        return Ok(None);
    };
    let Some((year, month, day)) = parse_iso_date(date) else {
        return Ok(None);
    };
    let Some(days) = parse_day_period(period) else {
        return Ok(None);
    };
    let shifted = unix_days_from_civil(year, month, day) + days;
    let (year, month, day) = civil_from_unix_days(shifted);
    Ok(Some(Value::String(format!(
        "{year:04}-{month:02}-{day:02}"
    ))))
}

fn parse_day_period(source: &str) -> Option<i64> {
    let inner = source.strip_prefix('P')?.strip_suffix('D')?;
    inner.parse::<i64>().ok()
}

fn parse_iso_date(source: &str) -> Option<(i64, i64, i64)> {
    let date = source.get(..10)?;
    if !is_iso_date(date) {
        return None;
    }
    Some((
        date[0..4].parse().ok()?,
        date[5..7].parse().ok()?,
        date[8..10].parse().ok()?,
    ))
}

fn unix_days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let year = year - if month <= 2 { 1 } else { 0 };
    let era = if year >= 0 { year } else { year - 399 }.div_euclid(400);
    let yoe = year - era * 400;
    let month = month + if month > 2 { -3 } else { 9 };
    let doy = (153 * month + 2).div_euclid(5) + day - 1;
    let doe = yoe * 365 + yoe.div_euclid(4) - yoe.div_euclid(100) + doy;
    era * 146_097 + doe - 719_468
}

fn civil_from_unix_days(days: i64) -> (i64, i64, i64) {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 }.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe.div_euclid(1_460) + doe.div_euclid(36_524) - doe.div_euclid(146_096))
        .div_euclid(365);
    let mut year = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe.div_euclid(4) - yoe.div_euclid(100));
    let mp = (5 * doy + 2).div_euclid(153);
    let day = doy - (153 * mp + 2).div_euclid(5) + 1;
    let month = mp + if mp < 10 { 3 } else { -9 };
    if month <= 2 {
        year += 1;
    }
    (year, month, day)
}

fn is_iso_date(value: &str) -> bool {
    let bytes = value.as_bytes();
    bytes.len() == 10
        && bytes[4] == b'-'
        && bytes[7] == b'-'
        && bytes
            .iter()
            .enumerate()
            .all(|(index, byte)| matches!(index, 4 | 7) || byte.is_ascii_digit())
}

fn compare_values(left: &Value, right: &Value) -> Result<i8, DwError> {
    if left.is_number() && right.is_number() {
        let left = number_value(left)?;
        let right = number_value(right)?;
        return Ok(if left < right {
            -1
        } else if left > right {
            1
        } else {
            0
        });
    }
    match (left, right) {
        (Value::String(left), Value::String(right)) => Ok(if left < right {
            -1
        } else if left > right {
            1
        } else {
            0
        }),
        _ => Err(DwError::UnsupportedFeature(format!(
            "comparison between {left:?} and {right:?}"
        ))),
    }
}

pub(crate) fn number_value(value: &Value) -> Result<f64, DwError> {
    match value {
        Value::Number(value) => value
            .as_f64()
            .ok_or_else(|| DwError::InvalidJson(value.to_string())),
        Value::Bool(true) => Ok(1.0),
        Value::Bool(false) => Ok(0.0),
        Value::String(value) => value.parse::<f64>().map_err(|_| {
            DwError::UnsupportedFeature(format!("cannot coerce string '{value}' to Number"))
        }),
        _ => Err(DwError::UnsupportedFeature(format!(
            "expected number, got {value:?}"
        ))),
    }
}
