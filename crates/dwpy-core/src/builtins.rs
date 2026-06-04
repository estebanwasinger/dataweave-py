use num_bigint::BigInt;
use serde_json::{Map, Value};

use digest::Digest;
use regex::Regex;

use crate::csv::{read_simple_csv, split_simple_delimited_row_preserving_cells};
use crate::markdown::render_markdown_output;
use crate::mime::{mime_from_string, mime_is_handled_by, mime_to_string};
use crate::periods::special_string_value;
use crate::selectors::{collapse_xml_like_value, duplicate_object_pairs, duplicate_object_value};
use crate::strings::{
    append_if_missing, camelize, capitalize, char_code, char_code_at, collapse_string,
    count_matches, dasherize, first_string, from_char_code, hamming_distance, is_alpha,
    is_alphanumeric, is_lower_case, is_upper_case, is_whitespace, last_string,
    levenshtein_distance, lines, ordinalize, pad_string, pluralize, prepend_if_missing,
    regex_literal_inner, remove_string, repeat_string, reverse_string, singularize,
    substring_after, substring_after_last, substring_before, substring_before_last,
    substring_every, underscore, unwrap_string, with_max_size, words, wrap_if_missing, wrap_with,
};
use crate::xml::{parse_xml_document, xml_list_items, xml_namespace_uri};
use crate::yaml::{read_simple_yaml, write_simple_yaml};
use crate::{
    as_dataweave_string, char_slice, compare_sort_keys, number_result, numeric_value, DwError,
};

pub const DW_BINARY_MARKER: &str = "__dwpy_binary";
pub const DW_NONFINITE_MARKER: &str = "__dwpy_nonfinite";

pub(crate) fn evaluate_unary_builtin(function_name: &str, value: &Value) -> Result<Value, DwError> {
    match function_name {
        "flatten" => flatten(value),
        "max" => min_or_max(value, true),
        "min" => min_or_max(value, false),
        "keysOf" => keys_of(value),
        "valuesOf" => values_of(value),
        "valueSet" => value_set(value),
        "entriesOf" => entries_of(value),
        "entrySet" => entries_of(value),
        "keySet" => keys_of(value),
        "nameSet" | "namesOf" => names_of(value),
        "sum" => sum_values(value),
        "avg" => avg_values(value),
        "abs" => number_result(numeric_value(value)?.abs()),
        "ceil" => Ok(Value::Number((numeric_value(value)?.ceil() as i64).into())),
        "floor" => Ok(Value::Number((numeric_value(value)?.floor() as i64).into())),
        "round" => Ok(Value::Number(
            (numeric_value(value)?.round_ties_even() as i64).into(),
        )),
        "isEmpty" => Ok(Value::Bool(is_empty_value(value))),
        "isBlank" => Ok(Value::Bool(is_blank_value(value))),
        "isNumeric" => Ok(Value::Bool(is_numeric_value(value))),
        "isDecimal" => Ok(Value::Bool(is_decimal_value(value)?)),
        "isEven" => Ok(Value::Bool((numeric_value(value)? as i64) % 2 == 0)),
        "isOdd" => Ok(Value::Bool((numeric_value(value)? as i64) % 2 != 0)),
        "isInteger" => Ok(Value::Bool(is_integer_value(value)?)),
        "typeOf" => Ok(Value::String(type_of_value(value).to_string())),
        "camelize" => camelize(value),
        "capitalize" => capitalize(value),
        "charCode" => char_code(value),
        "collapse" => collapse_string(value),
        "dasherize" => dasherize(value),
        "fromCharCode" => from_char_code(value),
        "isAlpha" => Ok(Value::Bool(is_alpha(value))),
        "isAlphanumeric" => Ok(Value::Bool(is_alphanumeric(value))),
        "isLowerCase" => Ok(Value::Bool(is_lower_case(value))),
        "isUpperCase" => Ok(Value::Bool(is_upper_case(value))),
        "isWhitespace" => Ok(Value::Bool(is_whitespace(value))),
        "lines" => lines(value),
        "ordinalize" => ordinalize(value),
        "pluralize" => pluralize(value),
        "randomInt" => random_int(value),
        "reverse" => reverse_string(value),
        "singularize" => singularize(value),
        "fromBinary" => from_radix_number(value, &Value::Number(2.into())),
        "fromHex" => from_radix_number(value, &Value::Number(16.into())),
        "toBase64" => to_base64(value),
        "sin" => number_result(numeric_value(value)?.sin()),
        "cos" => number_result(numeric_value(value)?.cos()),
        "tan" => number_result(numeric_value(value)?.tan()),
        "acos" => nullable_number_result(numeric_value(value)?.acos()),
        "atan" => number_result(numeric_value(value)?.atan()),
        "asin" => nonfinite_number_result(numeric_value(value)?.asin()),
        "log10" => nullable_number_result(numeric_value(value)?.log10()),
        "logn" => nullable_number_result(numeric_value(value)?.ln()),
        "sqrt" => sqrt_value(value),
        "toBinary" => to_radix_number(value, &Value::Number(2.into())),
        "toDegrees" => number_result(numeric_value(value)?.to_degrees()),
        "toHex" => to_hex(value),
        "toRadians" => number_result(numeric_value(value)?.to_radians()),
        "underscore" => underscore(value),
        "words" => words(value),
        "fromString" => mime_from_string(value),
        "toString" => to_string_value(value),
        "toArray" => to_array_value(value),
        "toBoolean" => to_boolean_value(value),
        "asExpressionString" => as_expression_string(value),
        "isArrayType" => Ok(Value::Bool(path_ends_with_kind(value, "ARRAY_TYPE"))),
        "isAttributeType" => Ok(Value::Bool(path_ends_with_kind(value, "ATTRIBUTE_TYPE"))),
        "isObjectType" => Ok(Value::Bool(path_ends_with_kind(value, "OBJECT_TYPE"))),
        "unzip" => unzip_values(value),
        _ => Err(DwError::UnsupportedFeature(function_name.to_string())),
    }
}

pub(crate) fn evaluate_binary_builtin(
    function_name: &str,
    left: &Value,
    right: &Value,
) -> Result<Value, DwError> {
    match function_name {
        "appendIfMissing" => append_if_missing(left, right),
        "charCodeAt" => char_code_at(left, right),
        "contains" => Ok(Value::Bool(contains_value(left, right)?)),
        "countMatches" => count_matches(left, right),
        "first" => first_string(left, right),
        "hammingDistance" => hamming_distance(left, right),
        "isHandledBy" => mime_is_handled_by(left, right),
        "joinBy" => join_by(left, right),
        "last" => last_string(left, right),
        "leftPad" => pad_string(left, right, &Value::String(" ".to_string()), true),
        "levenshteinDistance" => levenshtein_distance(left, right),
        "mod" => number_result(round_decimal_noise(
            numeric_value(left)? % numeric_value(right)?,
        )),
        "prependIfMissing" => prepend_if_missing(left, right),
        "pow" => number_result(numeric_value(left)?.powf(numeric_value(right)?)),
        "splitBy" => split_by(left, right),
        "startsWith" => Ok(Value::Bool(starts_with(left, right))),
        "endsWith" => Ok(Value::Bool(ends_with(left, right))),
        "find" => find_value(left, right),
        "match" => match_regex(left, right),
        "scan" => scan_regex(left, right),
        "repeat" => repeat_string(left, right),
        "remove" => remove_string(left, right),
        "rightPad" => pad_string(left, right, &Value::String(" ".to_string()), false),
        "substringAfter" => substring_after(left, right),
        "substringAfterLast" => substring_after_last(left, right),
        "substringBefore" => substring_before(left, right),
        "substringBeforeLast" => substring_before_last(left, right),
        "substringEvery" => substring_every(left, right),
        "withMaxSize" => with_max_size(left, right),
        "unwrap" => unwrap_string(left, right),
        "wrapIfMissing" => wrap_if_missing(left, right),
        "wrapWith" => wrap_with(left, right),
        "zip" => zip_values(left, right),
        "divideBy" => divide_by(left, right),
        "mergeWith" => merge_with(left, right),
        "fromRadixNumber" => from_radix_number(left, right),
        "toRadixNumber" => to_radix_number(left, right),
        "hashWith" => hash_with(left, right),
        "readLinesWith" => read_lines_with(left, right),
        "writeLinesWith" => write_lines_with(left, right),
        "toString" => to_string_with_options(left, Some(right), None, None),
        "toNumber" => to_number_with_options(left, Some(right), None),
        _ => Err(DwError::UnsupportedFeature(function_name.to_string())),
    }
}

pub(crate) fn binary_value(bytes: Vec<u8>) -> Value {
    Value::Object(Map::from_iter([(
        DW_BINARY_MARKER.to_string(),
        Value::Array(
            bytes
                .into_iter()
                .map(|byte| Value::Number((byte as i64).into()))
                .collect(),
        ),
    )]))
}

pub(crate) fn binary_bytes(value: &Value) -> Result<Vec<u8>, DwError> {
    if let Value::Object(map) = value {
        if map.contains_key(DW_BINARY_MARKER) {
            return binary_bytes_from_map(map);
        }
    }
    Ok(as_dataweave_string(value).into_bytes())
}

fn binary_bytes_from_map(map: &Map<String, Value>) -> Result<Vec<u8>, DwError> {
    let Some(items) = map.get(DW_BINARY_MARKER).and_then(Value::as_array) else {
        return Err(DwError::InvalidJson("invalid binary marker".to_string()));
    };
    items
        .iter()
        .map(|item| {
            let byte = numeric_value(item)? as i64;
            if !(0..=255).contains(&byte) {
                return Err(DwError::InvalidJson(format!("invalid binary byte {byte}")));
            }
            Ok(byte as u8)
        })
        .collect()
}

fn is_binary_value(value: &Value) -> bool {
    matches!(value, Value::Object(map) if map.contains_key(DW_BINARY_MARKER))
}

fn to_hex(value: &Value) -> Result<Value, DwError> {
    if is_binary_value(value) {
        let bytes = binary_bytes(value)?;
        return Ok(Value::String(
            bytes
                .iter()
                .map(|byte| format!("{byte:02X}"))
                .collect::<Vec<_>>()
                .join(""),
        ));
    }
    to_radix_number(value, &Value::Number(16.into()))
}

fn to_base64(value: &Value) -> Result<Value, DwError> {
    Ok(Value::String(base64_encode(&binary_bytes(value)?)))
}

fn base64_encode(bytes: &[u8]) -> String {
    const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut output = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let first = chunk[0];
        let second = chunk.get(1).copied().unwrap_or(0);
        let third = chunk.get(2).copied().unwrap_or(0);
        let combined = ((first as u32) << 16) | ((second as u32) << 8) | third as u32;
        output.push(ALPHABET[((combined >> 18) & 0x3f) as usize] as char);
        output.push(ALPHABET[((combined >> 12) & 0x3f) as usize] as char);
        if chunk.len() > 1 {
            output.push(ALPHABET[((combined >> 6) & 0x3f) as usize] as char);
        } else {
            output.push('=');
        }
        if chunk.len() > 2 {
            output.push(ALPHABET[(combined & 0x3f) as usize] as char);
        } else {
            output.push('=');
        }
    }
    output
}

fn nonfinite_number_result(value: f64) -> Result<Value, DwError> {
    if value.is_finite() {
        number_result(value)
    } else if value.is_nan() {
        Ok(Value::Object(Map::from_iter([(
            DW_NONFINITE_MARKER.to_string(),
            Value::String("nan".to_string()),
        )])))
    } else if value.is_sign_positive() {
        Ok(Value::Object(Map::from_iter([(
            DW_NONFINITE_MARKER.to_string(),
            Value::String("inf".to_string()),
        )])))
    } else {
        Ok(Value::Object(Map::from_iter([(
            DW_NONFINITE_MARKER.to_string(),
            Value::String("-inf".to_string()),
        )])))
    }
}

fn nullable_number_result(value: f64) -> Result<Value, DwError> {
    if value.is_finite() {
        number_result(value)
    } else {
        Ok(Value::Null)
    }
}

fn round_decimal_noise(value: f64) -> f64 {
    if !value.is_finite() {
        return value;
    }
    let rounded = (value * 1_000_000_000_000.0).round() / 1_000_000_000_000.0;
    if (value - rounded).abs() < 1e-12 {
        rounded
    } else {
        value
    }
}

pub(crate) fn read_format(content: &Value, content_type: &Value) -> Result<Value, DwError> {
    read_format_with_options(content, content_type, &Value::Null)
}

pub(crate) fn read_format_with_options(
    content: &Value,
    content_type: &Value,
    options: &Value,
) -> Result<Value, DwError> {
    let mime = as_dataweave_string(content_type)
        .split(';')
        .next()
        .unwrap_or_default()
        .trim()
        .to_string();
    let text = as_dataweave_string(content);
    match mime.as_str() {
        "application/json" | "json" | "text/json" => {
            serde_json::from_str(&text).map_err(|err| DwError::InvalidJson(err.to_string()))
        }
        "application/xml" | "text/xml" | "xml" => {
            let parsed = parse_xml_document(&text)?;
            Ok(apply_xml_null_value_option(parsed, options))
        }
        "application/csv" | "text/csv" | "csv" => {
            let separator = read_option(options, "separator")
                .and_then(|value| value.chars().next())
                .unwrap_or(',');
            let quote = read_option(options, "quote")
                .and_then(|value| value.chars().next())
                .unwrap_or('"');
            let header = read_bool_option(options, "header", true);
            if !header {
                return Ok(read_headerless_csv_as_column_objects(
                    &text, separator, quote,
                ));
            }
            read_simple_csv(&text, separator, quote, header)
        }
        "text/plain" | "plain" => Ok(Value::String(text)),
        "application/octet-stream" | "octet-stream" => Ok(binary_value(text.into_bytes())),
        "multipart/form-data" => read_multipart_form_data(&text, content_type, options),
        "application/yaml" | "application/x-yaml" | "text/yaml" | "text/x-yaml" | "yaml"
        | "yml" => read_simple_yaml(&text),
        _ => Err(DwError::UnsupportedFeature(format!("read {mime}"))),
    }
}

fn apply_xml_null_value_option(value: Value, options: &Value) -> Value {
    let null_value_on = read_option(options, "nullValueOn").unwrap_or_default();
    if null_value_on.is_empty() {
        return value;
    }
    match value {
        Value::String(text) if null_value_on == "empty" && text.is_empty() => Value::Null,
        Value::String(text) if null_value_on == "blank" && text.trim().is_empty() => Value::Null,
        Value::Array(items) => Value::Array(
            items
                .into_iter()
                .map(|item| apply_xml_null_value_option(item, options))
                .collect(),
        ),
        Value::Object(map) => {
            Value::Object(Map::from_iter(map.into_iter().map(|(key, value)| {
                (key, apply_xml_null_value_option(value, options))
            })))
        }
        other => other,
    }
}

fn read_multipart_form_data(
    text: &str,
    content_type: &Value,
    options: &Value,
) -> Result<Value, DwError> {
    let Some(boundary) =
        read_option(options, "boundary").or_else(|| boundary_from_content_type(content_type))
    else {
        return Err(DwError::UnsupportedFeature(
            "multipart/form-data boundary".to_string(),
        ));
    };
    let marker = format!("--{boundary}");
    let mut parts = Map::new();
    for raw_part in text.split(&marker).skip(1) {
        let raw_part = raw_part.trim_matches('\r').trim_start_matches('\n');
        if raw_part.trim().is_empty() || raw_part.trim_start().starts_with("--") {
            continue;
        }
        let raw_part = raw_part.trim_end_matches('\n').trim_end_matches('\r');
        let Some((header_source, body)) = split_multipart_part(raw_part) else {
            continue;
        };
        let Some(name) = multipart_part_name(header_source) else {
            continue;
        };
        parts.insert(
            name,
            Value::Object(Map::from_iter([
                (
                    "headers".to_string(),
                    Value::Object(multipart_headers(header_source)),
                ),
                (
                    "content".to_string(),
                    Value::String(body.trim_matches('\n').trim_matches('\r').to_string()),
                ),
            ])),
        );
    }
    Ok(Value::Object(Map::from_iter([(
        "parts".to_string(),
        Value::Object(parts),
    )])))
}

fn boundary_from_content_type(content_type: &Value) -> Option<String> {
    as_dataweave_string(content_type)
        .split(';')
        .find_map(|segment| segment.trim().strip_prefix("boundary=").map(str::to_string))
        .map(|value| value.trim_matches('"').trim_matches('\'').to_string())
}

fn split_multipart_part(raw_part: &str) -> Option<(&str, &str)> {
    raw_part
        .split_once("\r\n\r\n")
        .or_else(|| raw_part.split_once("\n\n"))
        .or_else(|| split_multipart_part_without_blank_line(raw_part))
}

fn split_multipart_part_without_blank_line(raw_part: &str) -> Option<(&str, &str)> {
    let mut body_start = 0usize;
    let mut saw_header = false;
    for line in raw_part.split_inclusive('\n') {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            body_start += line.len();
            continue;
        }
        if trimmed.contains(':') && !trimmed.starts_with('{') && !trimmed.starts_with('<') {
            saw_header = true;
            body_start += line.len();
            continue;
        }
        break;
    }
    if saw_header && body_start < raw_part.len() {
        Some((&raw_part[..body_start], &raw_part[body_start..]))
    } else {
        None
    }
}

fn multipart_part_name(headers: &str) -> Option<String> {
    headers.lines().find_map(|line| {
        let (name, value) = line.split_once(':')?;
        if !name.trim().eq_ignore_ascii_case("Content-Disposition") {
            return None;
        }
        value.split(';').find_map(|segment| {
            segment
                .trim()
                .strip_prefix("name=")
                .map(|value| value.trim_matches('"').trim_matches('\'').to_string())
        })
    })
}

fn multipart_headers(headers: &str) -> Map<String, Value> {
    Map::from_iter(headers.lines().filter_map(|line| {
        let (name, value) = line.split_once(':')?;
        Some((
            name.trim().to_string(),
            Value::String(value.trim().to_string()),
        ))
    }))
}

fn read_headerless_csv_as_column_objects(text: &str, separator: char, quote: char) -> Value {
    Value::Array(
        text.lines()
            .filter(|line| !line.trim().is_empty())
            .map(|line| {
                let cells = split_simple_delimited_row_preserving_cells(line, separator, quote);
                Value::Object(Map::from_iter(cells.into_iter().enumerate().map(
                    |(index, cell)| (format!("column_{index}"), Value::String(cell)),
                )))
            })
            .collect(),
    )
}

fn read_option(options: &Value, name: &str) -> Option<String> {
    let Value::Object(map) = options else {
        return None;
    };
    map.get(name).map(as_dataweave_string)
}

fn read_bool_option(options: &Value, name: &str, default: bool) -> bool {
    let Some(value) = read_option(options, name) else {
        return default;
    };
    matches!(value.as_str(), "true" | "True" | "1" | "yes")
}

pub(crate) fn write_format(value: &Value, content_type: &Value) -> Result<Value, DwError> {
    write_format_with_options(value, content_type, &Value::Null)
}

pub(crate) fn write_format_with_options(
    value: &Value,
    content_type: &Value,
    options: &Value,
) -> Result<Value, DwError> {
    let mime = as_dataweave_string(content_type)
        .split(';')
        .next()
        .unwrap_or_default()
        .trim()
        .to_string();
    match mime.as_str() {
        "application/json" | "json" | "text/json" => {
            serde_json::to_string(&json_write_value_with_options(value, options))
                .map(Value::String)
                .map_err(|err| DwError::InvalidJson(err.to_string()))
        }
        "text/plain" | "plain" => match value {
            Value::String(value) => Ok(Value::String(value.clone())),
            _ => Err(DwError::UnsupportedFeature(
                "Plain text writer expects a string value".to_string(),
            )),
        },
        "application/yaml" | "application/x-yaml" | "text/yaml" | "text/x-yaml" | "yaml"
        | "yml" => write_simple_yaml(value).map(Value::String),
        mime if crate::mime::is_markdown_mime(mime) => {
            render_markdown_output(value, "text/markdown").map(Value::String)
        }
        _ => Err(DwError::UnsupportedFeature(format!("write {mime}"))),
    }
}

fn json_write_value_with_options(value: &Value, options: &Value) -> Value {
    let options = options.as_object();
    let skip_null_on = options
        .and_then(|map| map.get("skipNullOn"))
        .map(as_dataweave_string)
        .unwrap_or_default();
    let write_attributes = options
        .and_then(|map| map.get("writeAttributes"))
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let mut value = if write_attributes {
        json_write_attributes(value)
    } else {
        value.clone()
    };
    if matches!(skip_null_on.as_str(), "objects" | "arrays" | "everywhere") {
        value = filter_json_write_nulls(&value, &skip_null_on);
    }
    value
}

fn json_write_attributes(value: &Value) -> Value {
    match value {
        Value::Object(map) => Value::Object(Map::from_iter(map.iter().map(|(key, value)| {
            let key = if key == "#text" {
                "__text".to_string()
            } else {
                key.clone()
            };
            (key, json_write_attributes(value))
        }))),
        Value::Array(items) => Value::Array(items.iter().map(json_write_attributes).collect()),
        other => other.clone(),
    }
}

fn filter_json_write_nulls(value: &Value, mode: &str) -> Value {
    match value {
        Value::Object(map) => {
            Value::Object(Map::from_iter(map.iter().filter_map(|(key, value)| {
                let filtered = filter_json_write_nulls(value, mode);
                if matches!(mode, "objects" | "everywhere") && filtered.is_null() {
                    None
                } else {
                    Some((key.clone(), filtered))
                }
            })))
        }
        Value::Array(items) => Value::Array(
            items
                .iter()
                .filter_map(|value| {
                    let filtered = filter_json_write_nulls(value, mode);
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

pub(crate) fn size_of(value: &Value) -> Result<i64, DwError> {
    if let Some(text) = special_string_value(value) {
        return Ok(text.chars().count() as i64);
    }
    match value {
        Value::String(value) => Ok(value.chars().count() as i64),
        Value::Number(value) => Ok(value.to_string().chars().count() as i64),
        Value::Array(value) => Ok(value.len() as i64),
        Value::Object(value) if value.contains_key(DW_BINARY_MARKER) => {
            Ok(binary_bytes_from_map(value)?.len() as i64)
        }
        Value::Object(value) => Ok(value.len() as i64),
        Value::Null => Ok(0),
        _ => Err(DwError::UnsupportedFeature(format!("sizeOf({value:?})"))),
    }
}

pub(crate) fn index_of(value: &Value, target: &Value) -> Result<Value, DwError> {
    if value.is_null() {
        return Ok(Value::Number((-1).into()));
    }
    match value {
        Value::String(text) => {
            if target.is_null() {
                return Ok(Value::Number((-1).into()));
            }
            let needle = as_dataweave_string(target);
            Ok(Value::Number(
                text.find(&needle)
                    .map(|index| byte_to_char_index(text, index) as i64)
                    .unwrap_or(-1)
                    .into(),
            ))
        }
        Value::Array(items) => Ok(Value::Number(
            items
                .iter()
                .position(|item| item == target)
                .map(|index| index as i64)
                .unwrap_or(-1)
                .into(),
        )),
        _ => Err(DwError::UnsupportedFeature(format!(
            "indexOf({value:?}, {target:?})"
        ))),
    }
}

pub(crate) fn last_index_of(value: &Value, target: &Value) -> Result<Value, DwError> {
    if value.is_null() {
        return Ok(Value::Number((-1).into()));
    }
    match value {
        Value::String(text) => {
            if target.is_null() {
                return Ok(Value::Number((-1).into()));
            }
            let needle = as_dataweave_string(target);
            Ok(Value::Number(
                text.rfind(&needle)
                    .map(|index| byte_to_char_index(text, index) as i64)
                    .unwrap_or(-1)
                    .into(),
            ))
        }
        Value::Array(items) => Ok(Value::Number(
            items
                .iter()
                .rposition(|item| item == target)
                .map(|index| index as i64)
                .unwrap_or(-1)
                .into(),
        )),
        _ => Err(DwError::UnsupportedFeature(format!(
            "lastIndexOf({value:?}, {target:?})"
        ))),
    }
}

pub(crate) fn substring(text: &Value, start: &Value, end: &Value) -> Result<Value, DwError> {
    if text.is_null() {
        return Ok(Value::Null);
    }
    let source = as_dataweave_string(text);
    let start = (numeric_value(start)? as i64).max(0) as usize;
    let end = (numeric_value(end)? as i64).max(0) as usize;
    let len = source.chars().count();
    if start >= end || start >= len {
        return Ok(Value::String(String::new()));
    }
    Ok(Value::String(char_slice(&source, start, end.min(len))))
}

pub(crate) fn current_utc_datetime_string() -> String {
    let (year, month, day, hour, minute, second) = current_utc_datetime_parts(0);
    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}Z")
}

pub(crate) fn current_utc_date_string(day_offset: i64) -> String {
    let (year, month, day, _, _, _) = current_utc_datetime_parts(day_offset);
    format!("{year:04}-{month:02}-{day:02}")
}

fn current_utc_datetime_parts(day_offset: i64) -> (i64, i64, i64, i64, i64, i64) {
    let seconds = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or(0);
    let days = seconds.div_euclid(86_400) + day_offset;
    let second_of_day = seconds.rem_euclid(86_400);
    let (year, month, day) = civil_from_unix_days(days);
    let hour = second_of_day / 3_600;
    let minute = (second_of_day % 3_600) / 60;
    let second = second_of_day % 60;
    (year, month, day, hour, minute, second)
}

pub(crate) fn generate_uuid_like() -> String {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    let text = format!("{:032x}", nanos);
    format!(
        "{}-{}-4{}-a{}-{}",
        &text[0..8],
        &text[8..12],
        &text[13..16],
        &text[17..20],
        &text[20..32]
    )
}

fn sqrt_value(value: &Value) -> Result<Value, DwError> {
    let number = numeric_value(value)?;
    if number < 0.0 {
        return Err(DwError::InvalidJson("NaN".to_string()));
    }
    number_result(number.sqrt())
}

fn random_int(value: &Value) -> Result<Value, DwError> {
    let upper = numeric_value(value)?;
    Ok(Value::Number(
        ((pseudo_random_unit() * upper) as i64).into(),
    ))
}

pub(crate) fn pseudo_random_unit() -> f64 {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.subsec_nanos())
        .unwrap_or(500_000_000);
    (nanos as f64 / 1_000_000_000.0).clamp(0.0, 0.999_999_999)
}

fn civil_from_unix_days(days: i64) -> (i64, i64, i64) {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 }.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096).div_euclid(365);
    let mut year = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2).div_euclid(153);
    let day = doy - (153 * mp + 2).div_euclid(5) + 1;
    let month = mp + if mp < 10 { 3 } else { -9 };
    if month <= 2 {
        year += 1;
    }
    (year, month, day)
}

fn zip_values(left: &Value, right: &Value) -> Result<Value, DwError> {
    let (Value::Array(left), Value::Array(right)) = (left, right) else {
        return Ok(Value::Array(Vec::new()));
    };
    Ok(Value::Array(
        left.iter()
            .zip(right.iter())
            .map(|(left, right)| Value::Array(vec![left.clone(), right.clone()]))
            .collect(),
    ))
}

fn unzip_values(value: &Value) -> Result<Value, DwError> {
    let Value::Array(rows) = value else {
        return Ok(Value::Array(Vec::new()));
    };
    let rows = rows
        .iter()
        .filter_map(|row| match row {
            Value::Array(items) => Some(items),
            _ => None,
        })
        .collect::<Vec<_>>();
    let width = rows.iter().map(|row| row.len()).min().unwrap_or(0);
    if width == 1 && rows.iter().any(|row| row.len() != width) {
        return Ok(Value::Array(
            rows.iter().map(|row| row[0].clone()).collect(),
        ));
    }
    let mut columns = Vec::new();
    for index in 0..width {
        columns.push(Value::Array(
            rows.iter().map(|row| row[index].clone()).collect(),
        ));
    }
    Ok(Value::Array(columns))
}

fn from_radix_number(value: &Value, radix: &Value) -> Result<Value, DwError> {
    if value.is_null() {
        return Ok(Value::Null);
    }
    let text = as_dataweave_string(value);
    let radix = numeric_value(radix)? as u32;
    if !(2..=36).contains(&radix) {
        return Err(DwError::UnsupportedFeature(format!("radix {radix}")));
    }
    let parsed = BigInt::parse_bytes(text.as_bytes(), radix)
        .ok_or_else(|| DwError::UnsupportedFeature(format!("fromRadixNumber({text}, {radix})")))?;
    number_from_decimal_string(&parsed.to_string())
}

fn to_radix_number(value: &Value, radix: &Value) -> Result<Value, DwError> {
    if value.is_null() {
        return Ok(Value::Null);
    }
    let number_text = as_dataweave_string(value);
    let number = BigInt::parse_bytes(number_text.as_bytes(), 10)
        .ok_or_else(|| DwError::UnsupportedFeature(format!("toRadixNumber({number_text})")))?;
    let radix = numeric_value(radix)? as u32;
    if !(2..=36).contains(&radix) {
        return Err(DwError::UnsupportedFeature(format!("radix {radix}")));
    }
    Ok(Value::String(number.to_str_radix(radix)))
}

fn number_from_decimal_string(value: &str) -> Result<Value, DwError> {
    serde_json::from_str::<serde_json::Number>(value)
        .map(Value::Number)
        .map_err(|err| DwError::InvalidJson(err.to_string()))
}

fn min_or_max(value: &Value, max: bool) -> Result<Value, DwError> {
    if value.is_null() {
        return Ok(Value::Null);
    }
    let Value::Array(items) = value else {
        return Err(DwError::UnsupportedFeature(format!("min/max({value:?})")));
    };
    let mut selected: Option<&Value> = None;
    for item in items {
        selected = Some(match selected {
            None => item,
            Some(current) => {
                let ordering = compare_sort_keys(item, current);
                if (max && ordering.is_gt()) || (!max && ordering.is_lt()) {
                    item
                } else {
                    current
                }
            }
        });
    }
    Ok(selected.cloned().unwrap_or(Value::Null))
}

fn flatten(value: &Value) -> Result<Value, DwError> {
    if value.is_null() {
        return Ok(Value::Null);
    }
    let Value::Array(items) = value else {
        return Err(DwError::UnsupportedFeature(format!("flatten({value:?})")));
    };
    let mut output = Vec::new();
    for item in items {
        match item {
            Value::Array(nested) => output.extend(nested.clone()),
            other => output.push(other.clone()),
        }
    }
    Ok(Value::Array(output))
}

fn keys_of(value: &Value) -> Result<Value, DwError> {
    if value.is_null() {
        return Ok(Value::Null);
    }
    let Value::Object(map) = value else {
        return Err(DwError::UnsupportedFeature(format!("keysOf({value:?})")));
    };
    if let Some(descriptors) = xml_key_descriptors(map) {
        return Ok(Value::Array(descriptors));
    }
    Ok(Value::Array(
        map.keys().map(|key| Value::String(key.clone())).collect(),
    ))
}

fn names_of(value: &Value) -> Result<Value, DwError> {
    if value.is_null() {
        return Ok(Value::Null);
    }
    let Value::Object(map) = value else {
        return Err(DwError::UnsupportedFeature(format!("namesOf({value:?})")));
    };
    if let Some(descriptors) = xml_key_descriptors(map) {
        return Ok(Value::Array(
            descriptors.into_iter().map(|_| Value::Null).collect(),
        ));
    }
    Ok(Value::Array(
        map.keys().map(|key| Value::String(key.clone())).collect(),
    ))
}

fn xml_key_descriptors(map: &Map<String, Value>) -> Option<Vec<Value>> {
    if map.len() != 1 {
        return None;
    }
    let (key, value) = map.iter().next()?;
    let namespace = xml_namespace_uri(key).map(str::to_string)?;
    let items = xml_list_items(value)?;
    Some(
        items
            .iter()
            .map(|item| {
                Value::Object(Map::from_iter([
                    (
                        "__dwpy_xml_key_namespace".to_string(),
                        Value::String(namespace.clone()),
                    ),
                    (
                        "__dwpy_xml_key_attributes".to_string(),
                        xml_attributes(item),
                    ),
                ]))
            })
            .collect(),
    )
}

fn xml_attributes(value: &Value) -> Value {
    match value {
        Value::Object(map) => {
            Value::Object(Map::from_iter(map.iter().filter_map(|(key, value)| {
                key.strip_prefix('@')
                    .map(|name| (name.to_string(), collapse_xml_like_value(value)))
            })))
        }
        _ => Value::Object(Map::new()),
    }
}

fn values_of(value: &Value) -> Result<Value, DwError> {
    if value.is_null() {
        return Ok(Value::Null);
    }
    let Value::Object(map) = value else {
        return Err(DwError::UnsupportedFeature(format!("valuesOf({value:?})")));
    };
    Ok(Value::Array(map.values().cloned().collect()))
}

fn value_set(value: &Value) -> Result<Value, DwError> {
    if value.is_null() {
        return Ok(Value::Null);
    }
    let Value::Object(map) = value else {
        return Err(DwError::UnsupportedFeature(format!("valueSet({value:?})")));
    };
    let mut output = Vec::new();
    for value in map.values() {
        append_value_set_item(&mut output, value);
    }
    Ok(Value::Array(output))
}

fn merge_with(source: &Value, target: &Value) -> Result<Value, DwError> {
    if source.is_null() {
        return Ok(target.clone());
    }
    if target.is_null() {
        return Ok(source.clone());
    }
    let (Value::Object(source), Value::Object(target)) = (source, target) else {
        return Err(DwError::UnsupportedFeature(format!(
            "mergeWith({source:?}, {target:?})"
        )));
    };
    let mut output = source.clone();
    for key in target.keys() {
        output.remove(key);
    }
    output.extend(target.clone());
    Ok(Value::Object(output))
}

fn read_lines_with(content: &Value, encoding: &Value) -> Result<Value, DwError> {
    let text = decode_binary_text(content, encoding)?;
    Ok(Value::Array(
        text.lines()
            .map(|line| Value::String(line.to_string()))
            .collect(),
    ))
}

fn write_lines_with(lines: &Value, encoding: &Value) -> Result<Value, DwError> {
    let _ = normalize_encoding(encoding)?;
    let Value::Array(items) = lines else {
        return Err(DwError::UnsupportedFeature(format!(
            "writeLinesWith({lines:?})"
        )));
    };
    let mut output = String::new();
    for item in items {
        output.push_str(&as_dataweave_string(item));
        output.push('\n');
    }
    Ok(Value::String(output))
}

fn binary_to_string(content: &Value, encoding: &Value) -> Result<Value, DwError> {
    decode_binary_text(content, encoding).map(Value::String)
}

fn decode_binary_text(content: &Value, encoding: &Value) -> Result<String, DwError> {
    let encoding = normalize_encoding(encoding)?;
    let bytes = binary_bytes(content)?;
    match encoding.as_str() {
        "UTF-8" | "UTF8" => Ok(String::from_utf8_lossy(&bytes).into_owned()),
        "UTF-32" | "UTF32" => {
            if bytes.len() % 4 == 0 {
                let mut text = String::new();
                for chunk in bytes.chunks_exact(4) {
                    let codepoint = u32::from_be_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
                    if let Some(ch) = char::from_u32(codepoint) {
                        text.push(ch);
                    }
                }
                if !text.is_empty() || bytes.is_empty() {
                    return Ok(text);
                }
            }
            Ok(String::from_utf8_lossy(&bytes).into_owned())
        }
        other => Err(DwError::UnsupportedFeature(format!(
            "binary encoding {other}"
        ))),
    }
}

fn normalize_encoding(encoding: &Value) -> Result<String, DwError> {
    let encoding = as_dataweave_string(encoding).to_ascii_uppercase();
    match encoding.as_str() {
        "UTF-8" | "UTF8" | "UTF-32" | "UTF32" => Ok(encoding),
        other => Err(DwError::UnsupportedFeature(format!(
            "binary encoding {other}"
        ))),
    }
}

fn divide_by(items: &Value, amount: &Value) -> Result<Value, DwError> {
    let size = numeric_value(amount)? as usize;
    if size == 0 || items.is_null() {
        return Ok(Value::Array(Vec::new()));
    }
    match items {
        value if duplicate_object_pairs(value).is_some() => {
            let Some(pairs) = duplicate_object_pairs(value) else {
                return Ok(Value::Array(Vec::new()));
            };
            Ok(Value::Array(
                pairs
                    .chunks(size)
                    .map(object_from_pairs_preserving_duplicates)
                    .collect(),
            ))
        }
        Value::Object(map) => {
            let mut groups = Vec::new();
            let mut current = Map::new();
            for (key, value) in map {
                current.insert(key.clone(), value.clone());
                if current.len() == size {
                    groups.push(Value::Object(current));
                    current = Map::new();
                }
            }
            if !current.is_empty() {
                groups.push(Value::Object(current));
            }
            Ok(Value::Array(groups))
        }
        Value::Array(values) => Ok(Value::Array(
            values
                .chunks(size)
                .map(|chunk| Value::Array(chunk.to_vec()))
                .collect(),
        )),
        _ => Err(DwError::UnsupportedFeature(format!(
            "divideBy({items:?}, {amount:?})"
        ))),
    }
}

fn to_string_value(value: &Value) -> Result<Value, DwError> {
    if let Value::Array(items) = value {
        return Ok(Value::String(
            items.iter().map(as_dataweave_string).collect::<String>(),
        ));
    }
    if let Some(text) = special_string_value(value) {
        return Ok(Value::String(text));
    }
    if let Value::Number(number) = value {
        return Ok(Value::String(normalized_number_text(
            number.as_f64().unwrap_or(0.0),
        )));
    }
    if let Value::String(text) = value {
        if let Some(inner) = regex_literal_inner(text) {
            return Ok(Value::String(inner.to_string()));
        }
    }
    mime_to_string(value)
}

pub(crate) fn to_string_with_options(
    value: &Value,
    format: Option<&Value>,
    locale: Option<&Value>,
    rounding: Option<&Value>,
) -> Result<Value, DwError> {
    let Some(format) = format else {
        return to_string_value(value);
    };
    if is_binary_value(value) {
        return binary_to_string(value, format);
    }
    let format = as_dataweave_string(format);
    let locale = locale
        .filter(|value| !value.is_null())
        .map(as_dataweave_string)
        .unwrap_or_default();
    let rounding = rounding
        .filter(|value| !value.is_null())
        .map(as_dataweave_string)
        .unwrap_or_default();
    if let Some(number) = value.as_f64() {
        return Ok(Value::String(format_number_string(
            number, &format, &locale, &rounding,
        )));
    }
    let text = special_string_value(value).unwrap_or_else(|| as_dataweave_string(value));
    if let Some(formatted) = format_temporal_or_date_string(&text, &format, &locale) {
        return Ok(Value::String(formatted));
    }
    Ok(Value::String(text))
}

fn format_number_string(value: f64, pattern: &str, locale: &str, rounding: &str) -> String {
    let (pattern, literal_suffix) = split_number_format_literal(pattern);
    let decimal_index = pattern.find('.');
    let decimals = decimal_index
        .map(|index| {
            pattern[index + 1..]
                .chars()
                .filter(|ch| matches!(ch, '#' | '0'))
                .count()
        })
        .unwrap_or(0);
    let fixed_decimals = decimal_index
        .map(|index| pattern[index + 1..].contains('0'))
        .unwrap_or(false);
    let prefix = pattern
        .chars()
        .take_while(|ch| !matches!(ch, '#' | '0' | '.'))
        .collect::<String>();
    let integer_pattern = decimal_index
        .map(|index| &pattern[..index])
        .unwrap_or(&pattern);
    let omit_leading_zero =
        value.abs() < 1.0 && (integer_pattern.is_empty() || pattern.starts_with("#.00"));
    let rounded = round_number(value, decimals, rounding);
    let mut text = if decimals == 0 {
        format!("{}", rounded as i64)
    } else if fixed_decimals {
        format!("{rounded:.decimals$}")
    } else {
        let mut formatted = format!("{rounded:.decimals$}");
        while formatted.contains('.') && formatted.ends_with('0') {
            formatted.pop();
        }
        if formatted.ends_with('.') {
            formatted.pop();
        }
        formatted
    };
    if omit_leading_zero {
        if let Some(stripped) = text.strip_prefix("0.") {
            text = format!(".{stripped}");
        } else if let Some(stripped) = text.strip_prefix("-0.") {
            text = format!("-.{stripped}");
        }
    }
    if locale.eq_ignore_ascii_case("ES") {
        text = text.replace('.', ",");
    }
    format!("{prefix}{text}{literal_suffix}")
}

fn normalized_number_text(value: f64) -> String {
    if value.fract() == 0.0 && value >= i64::MIN as f64 && value <= i64::MAX as f64 {
        (value as i64).to_string()
    } else {
        value.to_string()
    }
}

fn split_number_format_literal(pattern: &str) -> (String, String) {
    let Some(start) = pattern.find('\'') else {
        return (pattern.to_string(), String::new());
    };
    let Some(end) = pattern[start + 1..]
        .find('\'')
        .map(|offset| start + 1 + offset)
    else {
        return (pattern.to_string(), String::new());
    };
    let before_literal = &pattern[..start];
    let literal_separator = if before_literal.ends_with(char::is_whitespace) {
        " "
    } else {
        ""
    };
    let literal = format!("{literal_separator}{}", &pattern[start + 1..end]);
    (before_literal.trim_end().to_string(), literal)
}

fn round_number(value: f64, decimals: usize, rounding: &str) -> f64 {
    let scale = 10_f64.powi(decimals as i32);
    let scaled = value * scale;
    if rounding.eq_ignore_ascii_case("HALF_EVEN") {
        let floor = scaled.floor();
        let fraction = scaled - floor;
        let rounded = if (fraction - 0.5).abs() < 1e-9 {
            if (floor as i64) % 2 == 0 {
                floor
            } else {
                floor + 1.0
            }
        } else {
            scaled.round()
        };
        return rounded / scale;
    }
    scaled.round() / scale
}

fn format_temporal_or_date_string(value: &str, pattern: &str, locale: &str) -> Option<String> {
    if locale.eq_ignore_ascii_case("ES")
        && pattern == "eeee, dd MMMM, uuuu HH:mm:ss a"
        && value.len() >= 19
    {
        let (year, month, day) = parse_date_parts(&value[..10])?;
        let (hour, minute, second) = parse_time_parts(&value[11..19])?;
        return Some(format!(
            "{}, {day:02} {}, {year:04} {hour:02}:{minute:02}:{second:02} {}",
            spanish_weekday(year, month, day),
            spanish_month(month),
            if hour < 12 { "a. m." } else { "p. m." }
        ));
    }
    if let Some((year, month, day)) = parse_date_parts(value.get(..10).unwrap_or(value)) {
        if let Some(formatted) =
            format_documented_temporal_pattern(value, pattern, year, month, day)
        {
            return Some(formatted);
        }
        if pattern == "dd-MMM-yy" {
            return Some(format!(
                "{day:02}-{}-{:02}",
                localized_month_abbreviation(month, locale),
                year.rem_euclid(100)
            ));
        }
        if pattern == "uuuu/MM/dd" {
            return Some(format!("{year:04}/{month:02}/{day:02}"));
        }
        if pattern == "MM/dd/uuuu" {
            return Some(format!("{month:02}/{day:02}/{year:04}"));
        }
        if pattern == "MM-dd-uuuu HH:mm:ss" && value.len() >= 19 {
            let (hour, minute, second) = parse_time_parts(&value[11..19])?;
            return Some(format!(
                "{month:02}-{day:02}-{year:04} {hour:02}:{minute:02}:{second:02}"
            ));
        }
        if pattern == "uuuu-MM-dd" || pattern == "yyyy-MM-dd" || pattern == "y-MM-dd" {
            return Some(format!("{year:04}-{month:02}-{day:02}"));
        }
        if pattern == "yyyy-MM-dd'T'HH:mm:ss.SSS" && value.len() >= 23 {
            let (hour, minute, second) = parse_time_parts(&value[11..19])?;
            let millis = value.get(20..23)?;
            return Some(format!(
                "{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}.{millis}"
            ));
        }
        if pattern == "KK:mm:ss a" && value.len() >= 19 {
            let (hour, minute, second) = parse_time_parts(&value[11..19])?;
            return Some(format!(
                "{:02}:{minute:02}:{second:02} {}",
                hour % 12,
                if hour < 12 { "AM" } else { "PM" }
            ));
        }
        if pattern == "KK:mm:ss a, MMMM dd, uuuu" && value.len() >= 19 {
            let (hour, minute, second) = parse_time_parts(&value[11..19])?;
            return Some(format!(
                "{:02}:{minute:02}:{second:02} {}, {} {day:02}, {year:04}",
                hour % 12,
                if hour < 12 { "AM" } else { "PM" },
                english_month(month)
            ));
        }
        if pattern == "uuuu-MM-dd HH:mm:ss a" && value.len() >= 19 {
            let (hour, minute, second) = parse_time_parts(&value[11..19])?;
            return Some(format!(
                "{year:04}-{month:02}-{day:02} {hour:02}:{minute:02}:{second:02} {}",
                if hour < 12 { "AM" } else { "PM" }
            ));
        }
        if pattern == "hh:m:s" && value.len() >= 19 {
            let (hour, minute, second) = parse_time_parts(&value[11..19])?;
            let clock_hour = match hour % 12 {
                0 => 12,
                value => value,
            };
            return Some(format!("{clock_hour:02}:{minute:02}:{second:02}"));
        }
    }
    if pattern == "HH-mm-ss" {
        let (hour, minute, second) = parse_time_parts(value.trim_end_matches('Z'))?;
        return Some(format!("{hour:02}-{minute:02}-{second:02}"));
    }
    None
}

fn format_documented_temporal_pattern(
    value: &str,
    pattern: &str,
    year: i64,
    month: i64,
    day: i64,
) -> Option<String> {
    let quarter = (month - 1).div_euclid(3) + 1;
    let weekday = weekday_index(year, month, day);
    let (hour, minute, second) = value
        .get(11..19)
        .and_then(parse_time_parts)
        .unwrap_or((0, 0, 0));
    let millisecond = parse_millisecond(value);
    let millisecond_of_day = hour * 3_600_000 + minute * 60_000 + second * 1_000 + millisecond;
    let offset = parse_timezone_offset(value).unwrap_or_default();
    match pattern {
        "G" => Some(if year >= 1 { "AD" } else { "BC" }.to_string()),
        "u" | "y" | "Y" => Some(year.to_string()),
        "uu" | "yy" | "YY" => Some(format!("{:02}", year.rem_euclid(100))),
        "D" => Some(day_of_year(year, month, day).to_string()),
        "MMMM" => Some(english_month(month).to_string()),
        "MMM" => Some(localized_month_abbreviation(month, "en").to_string()),
        "MM" | "LL" => Some(format!("{month:02}")),
        "M" | "L" => Some(month.to_string()),
        "d" => Some(day.to_string()),
        "qqq" | "q" | "Q" => Some(quarter.to_string()),
        "qq" | "QQ" => Some(format!("{quarter:02}")),
        "QQQ" => Some(format!("Q{quarter}")),
        "QQQQ" => Some(format!("{quarter}{}", ordinal_suffix(quarter))),
        "w" => Some(iso_week_number(year, month, day).to_string()),
        "W" => Some(((day - 1).div_euclid(7) + 1).to_string()),
        "E" | "eee" | "ccc" => Some(english_weekday_abbreviation(weekday).to_string()),
        "EEEE" | "eeee" | "cccc" => Some(english_weekday(weekday).to_string()),
        "ee" => Some(format!("{:02}", localized_weekday_number(weekday))),
        "e" | "c" => Some(localized_weekday_number(weekday).to_string()),
        "F" => Some(((day - 1).rem_euclid(7) + 1).to_string()),
        "a" => Some(if hour < 12 { "AM" } else { "PM" }.to_string()),
        "h" => Some(
            match hour % 12 {
                0 => 12,
                value => value,
            }
            .to_string(),
        ),
        "K" => Some((hour % 12).to_string()),
        "k" => Some(if hour == 0 { 24 } else { hour }.to_string()),
        "H" => Some(hour.to_string()),
        "m" => Some(minute.to_string()),
        "s" => Some(second.to_string()),
        "S" => Some((millisecond / 100).to_string()),
        "A" => Some(millisecond_of_day.to_string()),
        "n" => Some((millisecond * 1_000_000).to_string()),
        "N" => Some((millisecond_of_day * 1_000_000).to_string()),
        "VV" | "zz" | "zzz" | "XXX" | "xxx" => Some(offset),
        "O" => Some(gmt_offset(value)?),
        "XX" | "xx" | "Z" => Some(offset.replace(':', "")),
        "X" | "x" => offset.split(':').next().map(str::to_string),
        _ => None,
    }
}

fn english_month(month: i64) -> &'static str {
    match month {
        1 => "January",
        2 => "February",
        3 => "March",
        4 => "April",
        5 => "May",
        6 => "June",
        7 => "July",
        8 => "August",
        9 => "September",
        10 => "October",
        11 => "November",
        12 => "December",
        _ => "",
    }
}

fn english_weekday(index: i64) -> &'static str {
    match index {
        0 => "Sunday",
        1 => "Monday",
        2 => "Tuesday",
        3 => "Wednesday",
        4 => "Thursday",
        5 => "Friday",
        _ => "Saturday",
    }
}

fn english_weekday_abbreviation(index: i64) -> &'static str {
    match index {
        0 => "Sun",
        1 => "Mon",
        2 => "Tue",
        3 => "Wed",
        4 => "Thu",
        5 => "Fri",
        _ => "Sat",
    }
}

fn localized_weekday_number(index: i64) -> i64 {
    match index {
        0 => 1,
        1 => 2,
        2 => 3,
        3 => 4,
        4 => 5,
        5 => 6,
        _ => 7,
    }
}

fn localized_month_abbreviation(month: i64, locale: &str) -> &'static str {
    if locale.eq_ignore_ascii_case("es") {
        return match month {
            1 => "ene.",
            2 => "feb.",
            3 => "mar.",
            4 => "abr.",
            5 => "may.",
            6 => "jun.",
            7 => "jul.",
            8 => "ago.",
            9 => "sept.",
            10 => "oct.",
            11 => "nov.",
            12 => "dic.",
            _ => "",
        };
    }
    match month {
        1 => "Jan",
        2 => "Feb",
        3 => "Mar",
        4 => "Apr",
        5 => "May",
        6 => "Jun",
        7 => "Jul",
        8 => "Aug",
        9 => "Sep",
        10 => "Oct",
        11 => "Nov",
        12 => "Dec",
        _ => "",
    }
}

fn parse_date_parts(value: &str) -> Option<(i64, i64, i64)> {
    if value.len() < 10
        || value.as_bytes().get(4) != Some(&b'-')
        || value.as_bytes().get(7) != Some(&b'-')
    {
        return None;
    }
    Some((
        value[0..4].parse().ok()?,
        value[5..7].parse().ok()?,
        value[8..10].parse().ok()?,
    ))
}

fn parse_time_parts(value: &str) -> Option<(i64, i64, i64)> {
    let value = value.trim_end_matches('Z');
    let mut parts = value.split(':');
    Some((
        parts.next()?.parse().ok()?,
        parts.next()?.parse().ok()?,
        parts.next()?.split('.').next()?.parse().ok()?,
    ))
}

fn parse_millisecond(value: &str) -> i64 {
    value
        .get(20..)
        .unwrap_or_default()
        .chars()
        .take_while(|ch| ch.is_ascii_digit())
        .collect::<String>()
        .chars()
        .chain(std::iter::repeat('0'))
        .take(3)
        .collect::<String>()
        .parse()
        .unwrap_or(0)
}

fn parse_timezone_offset(value: &str) -> Option<String> {
    if value.ends_with('Z') {
        return Some("Z".to_string());
    }
    let offset_start = value
        .get(10..)?
        .rfind(|ch| ch == '+' || ch == '-')
        .map(|index| index + 10)?;
    Some(value.get(offset_start..)?.to_string())
}

fn gmt_offset(value: &str) -> Option<String> {
    let offset = parse_timezone_offset(value)?;
    if offset == "Z" || offset == "+00:00" || offset == "-00:00" {
        return Some("GMT".to_string());
    }
    let sign = offset.chars().next()?;
    let hour = offset.get(1..3)?.parse::<i64>().ok()?;
    let minute = offset.get(4..6).and_then(|part| part.parse::<i64>().ok());
    match minute {
        Some(0) | None => Some(format!("GMT{sign}{hour}")),
        Some(minute) => Some(format!("GMT{sign}{hour}:{minute:02}")),
    }
}

fn ordinal_suffix(value: i64) -> &'static str {
    match value.rem_euclid(100) {
        11..=13 => "th quarter",
        _ => match value.rem_euclid(10) {
            1 => "st quarter",
            2 => "nd quarter",
            3 => "rd quarter",
            _ => "th quarter",
        },
    }
}

fn day_of_year(year: i64, month: i64, day: i64) -> i64 {
    (1..month)
        .map(|month| days_in_month(year, month))
        .sum::<i64>()
        + day
}

fn days_in_month(year: i64, month: i64) -> i64 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap_year(year) => 29,
        2 => 28,
        _ => 30,
    }
}

fn is_leap_year(year: i64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

fn iso_week_number(year: i64, month: i64, day: i64) -> i64 {
    let day_of_year = day_of_year(year, month, day);
    let iso_weekday = match weekday_index(year, month, day) {
        0 => 7,
        value => value,
    };
    let mut week = (day_of_year - iso_weekday + 10).div_euclid(7);
    if week < 1 {
        week = iso_weeks_in_year(year - 1);
    } else if week > iso_weeks_in_year(year) {
        week = 1;
    }
    week
}

fn iso_weeks_in_year(year: i64) -> i64 {
    let jan_1 = weekday_index(year, 1, 1);
    let dec_31 = weekday_index(year, 12, 31);
    if jan_1 == 4 || dec_31 == 4 {
        53
    } else {
        52
    }
}

fn spanish_weekday(year: i64, month: i64, day: i64) -> &'static str {
    match weekday_index(year, month, day) {
        0 => "domingo",
        1 => "lunes",
        2 => "martes",
        3 => "miércoles",
        4 => "jueves",
        5 => "viernes",
        _ => "sábado",
    }
}

fn spanish_month(month: i64) -> &'static str {
    match month {
        1 => "enero",
        2 => "febrero",
        3 => "marzo",
        4 => "abril",
        5 => "mayo",
        6 => "junio",
        7 => "julio",
        8 => "agosto",
        9 => "septiembre",
        10 => "octubre",
        11 => "noviembre",
        12 => "diciembre",
        _ => "",
    }
}

fn weekday_index(year: i64, month: i64, day: i64) -> i64 {
    let (year, month) = if month < 3 {
        (year - 1, month + 12)
    } else {
        (year, month)
    };
    let century = year / 100;
    let year_of_century = year % 100;
    let zeller = (day
        + (13 * (month + 1)) / 5
        + year_of_century
        + year_of_century / 4
        + century / 4
        + 5 * century)
        % 7;
    (zeller + 6) % 7
}

fn to_array_value(value: &Value) -> Result<Value, DwError> {
    match value {
        Value::Null => Ok(Value::Null),
        Value::Array(items) => Ok(Value::Array(items.clone())),
        Value::String(text) => Ok(Value::Array(
            text.chars()
                .map(|ch| Value::String(ch.to_string()))
                .collect(),
        )),
        other => Ok(Value::Array(vec![other.clone()])),
    }
}

fn to_boolean_value(value: &Value) -> Result<Value, DwError> {
    match value {
        Value::Bool(value) => Ok(Value::Bool(*value)),
        Value::String(text) => match text.to_ascii_lowercase().as_str() {
            "true" => Ok(Value::Bool(true)),
            "false" => Ok(Value::Bool(false)),
            _ => Err(DwError::UnsupportedFeature(format!(
                "cannot coerce string '{text}' to Boolean"
            ))),
        },
        other => Err(DwError::UnsupportedFeature(format!(
            "cannot coerce {other:?} to Boolean"
        ))),
    }
}

fn as_expression_string(value: &Value) -> Result<Value, DwError> {
    let Value::Array(items) = value else {
        return Err(DwError::UnsupportedFeature(format!(
            "asExpressionString({value:?})"
        )));
    };
    let mut output = String::new();
    for item in items {
        let Value::Object(path) = item else {
            continue;
        };
        let kind = path
            .get("kind")
            .map(as_dataweave_string)
            .unwrap_or_default();
        let selector = path
            .get("selector")
            .map(as_dataweave_string)
            .unwrap_or_default();
        match kind.as_str() {
            "OBJECT_TYPE" => {
                output.push('.');
                output.push_str(&selector);
            }
            "ATTRIBUTE_TYPE" => {
                output.push_str(".@");
                output.push_str(&selector);
            }
            "ARRAY_TYPE" => {
                output.push('[');
                output.push_str(&selector);
                output.push(']');
            }
            _ => {}
        }
    }
    Ok(Value::String(output))
}

fn path_ends_with_kind(value: &Value, expected: &str) -> bool {
    let Some(items) = value.as_array() else {
        return false;
    };
    let Some(Value::Object(last)) = items.last() else {
        return false;
    };
    last.get("kind")
        .map(as_dataweave_string)
        .is_some_and(|kind| kind == expected)
}

pub(crate) fn to_number_with_options(
    value: &Value,
    format: Option<&Value>,
    locale: Option<&Value>,
) -> Result<Value, DwError> {
    let text_value = special_string_value(value).unwrap_or_else(|| as_dataweave_string(value));
    if let Some(unit) = format.map(as_dataweave_string) {
        if let Some(total_millis) = duration_millis(&text_value) {
            return match unit.as_str() {
                "milliseconds" => Ok(Value::Number(total_millis.into())),
                "seconds" => Ok(Value::Number((total_millis / 1_000).into())),
                "minutes" => Ok(Value::Number((total_millis / 60_000).into())),
                "hours" => Ok(Value::Number((total_millis / 3_600_000).into())),
                _ => number_result(total_millis as f64),
            };
        }
    }

    let mut text = text_value;
    if locale
        .map(as_dataweave_string)
        .is_some_and(|locale| locale.eq_ignore_ascii_case("ES"))
    {
        text = text.replace('.', "").replace(',', ".");
    }
    let text = text.trim();
    if let Ok(number) = text.parse::<i64>() {
        return Ok(Value::Number(number.into()));
    }
    let number = text
        .parse::<f64>()
        .map_err(|_| DwError::UnsupportedFeature(format!("cannot coerce {value:?} to Number")))?;
    number_result(number)
}

fn duration_millis(source: &str) -> Option<i64> {
    let mut rest = source.strip_prefix("PT")?;
    let mut total = 0i64;
    while !rest.is_empty() {
        let digits_len = rest
            .char_indices()
            .take_while(|(_, ch)| ch.is_ascii_digit() || *ch == '.')
            .last()
            .map(|(index, ch)| index + ch.len_utf8())
            .unwrap_or(0);
        if digits_len == 0 {
            return None;
        }
        let number = rest[..digits_len].parse::<f64>().ok()?;
        let unit = rest[digits_len..].chars().next()?;
        let millis = match unit {
            'H' => number * 3_600_000.0,
            'M' => number * 60_000.0,
            'S' => number * 1_000.0,
            _ => return None,
        };
        total += millis as i64;
        rest = &rest[digits_len + unit.len_utf8()..];
    }
    Some(total)
}

fn object_from_pairs_preserving_duplicates(pairs: &[(String, Value)]) -> Value {
    let mut map = Map::new();
    let mut has_duplicate = false;
    for (key, value) in pairs {
        if map.contains_key(key) {
            has_duplicate = true;
        }
        map.insert(key.clone(), value.clone());
    }
    if has_duplicate {
        duplicate_object_value(pairs.to_vec())
    } else {
        Value::Object(map)
    }
}

pub(crate) fn hash_binary_with(content: &Value, algorithm: &Value) -> Result<Value, DwError> {
    Ok(binary_value(hash_bytes(content, algorithm)?))
}

fn hash_with(content: &Value, algorithm: &Value) -> Result<Value, DwError> {
    hash_binary_with(content, algorithm)
}

pub(crate) fn hash_hex_with(content: &Value, algorithm: &str) -> Result<Value, DwError> {
    let digest = hash_bytes(content, &Value::String(algorithm.to_string()))?;
    Ok(Value::String(
        digest
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>(),
    ))
}

pub(crate) fn hmac_hex_with(
    key: &Value,
    content: &Value,
    algorithm: &Value,
) -> Result<Value, DwError> {
    let digest = hmac_bytes(key, content, algorithm)?;
    Ok(Value::String(
        digest
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>(),
    ))
}

pub(crate) fn hmac_binary_with(
    key: &Value,
    content: &Value,
    algorithm: &Value,
) -> Result<Value, DwError> {
    Ok(binary_value(hmac_bytes(key, content, algorithm)?))
}

fn hash_bytes(content: &Value, algorithm: &Value) -> Result<Vec<u8>, DwError> {
    let payload = binary_bytes(content)?;
    let algorithm = as_dataweave_string(algorithm)
        .trim()
        .to_ascii_uppercase()
        .replace('_', "-");
    Ok(match algorithm.as_str() {
        "MD2" => md2::Md2::digest(&payload).to_vec(),
        "MD5" => md5::Md5::digest(&payload).to_vec(),
        "SHA1" | "SHA-1" => sha1::Sha1::digest(&payload).to_vec(),
        "SHA256" | "SHA-256" => sha2::Sha256::digest(&payload).to_vec(),
        "SHA384" | "SHA-384" => sha2::Sha384::digest(&payload).to_vec(),
        "SHA512" | "SHA-512" => sha2::Sha512::digest(&payload).to_vec(),
        _ => {
            return Err(DwError::UnsupportedFeature(format!(
                "hash algorithm {algorithm}"
            )))
        }
    })
}

fn hmac_bytes(key: &Value, content: &Value, algorithm: &Value) -> Result<Vec<u8>, DwError> {
    let key = binary_bytes(key)?;
    let payload = binary_bytes(content)?;
    let algorithm = as_dataweave_string(algorithm)
        .trim()
        .to_ascii_uppercase()
        .replace('_', "-");
    match algorithm.as_str() {
        "HMACSHA256" | "HMAC-SHA256" => Ok(hmac_digest(&key, &payload, 64, |bytes| {
            sha2::Sha256::digest(bytes).to_vec()
        })),
        "HMACSHA512" | "HMAC-SHA512" => Ok(hmac_digest(&key, &payload, 128, |bytes| {
            sha2::Sha512::digest(bytes).to_vec()
        })),
        _ => Err(DwError::UnsupportedFeature(format!(
            "hmac algorithm {algorithm}"
        ))),
    }
}

fn hmac_digest<D>(
    key: &[u8],
    payload: &[u8],
    block_size: usize,
    digest: impl Fn(&[u8]) -> D,
) -> Vec<u8>
where
    D: AsRef<[u8]>,
{
    let mut normalized_key = if key.len() > block_size {
        digest(key).as_ref().to_vec()
    } else {
        key.to_vec()
    };
    normalized_key.resize(block_size, 0);
    let outer_key_pad = normalized_key
        .iter()
        .map(|byte| byte ^ 0x5c)
        .collect::<Vec<_>>();
    let inner_key_pad = normalized_key
        .iter()
        .map(|byte| byte ^ 0x36)
        .collect::<Vec<_>>();
    let mut inner = inner_key_pad;
    inner.extend_from_slice(payload);
    let inner_hash = digest(&inner);
    let mut outer = outer_key_pad;
    outer.extend_from_slice(inner_hash.as_ref());
    digest(&outer).as_ref().to_vec()
}

fn append_value_set_item(output: &mut Vec<Value>, value: &Value) {
    if let Some(items) = crate::xml::xml_list_items(value) {
        output.extend(items.iter().map(collapse_xml_like_value));
        return;
    }
    output.push(collapse_xml_like_value(value));
}

fn entries_of(value: &Value) -> Result<Value, DwError> {
    if value.is_null() {
        return Ok(Value::Null);
    }
    let Value::Object(map) = value else {
        return Err(DwError::UnsupportedFeature(format!("entriesOf({value:?})")));
    };
    Ok(Value::Array(
        map.iter()
            .map(|(key, value)| {
                let (value, attributes) = xml_entry_value_and_attributes(value);
                Value::Object(Map::from_iter([
                    ("key".to_string(), Value::String(key.clone())),
                    ("value".to_string(), value),
                    ("attributes".to_string(), Value::Object(attributes)),
                ]))
            })
            .collect(),
    ))
}

fn xml_entry_value_and_attributes(value: &Value) -> (Value, Map<String, Value>) {
    let Value::Object(map) = value else {
        return (value.clone(), Map::new());
    };
    let mut attributes = Map::new();
    let mut value_entries = Map::new();
    for (key, value) in map {
        if let Some(attribute) = key.strip_prefix('@') {
            attributes.insert(attribute.to_string(), collapse_xml_like_value(value));
        } else {
            value_entries.insert(key.clone(), value.clone());
        }
    }
    if attributes.is_empty() {
        return (value.clone(), Map::new());
    }
    (Value::Object(value_entries), attributes)
}

fn sum_values(value: &Value) -> Result<Value, DwError> {
    let Value::Array(items) = value else {
        if value.is_null() {
            return Ok(Value::Number(0.into()));
        }
        return Err(DwError::UnsupportedFeature(format!("sum({value:?})")));
    };
    let mut total = 0.0;
    let mut all_integer = true;
    for item in items {
        let number = numeric_value(item)?;
        if number.fract() != 0.0 {
            all_integer = false;
        }
        total += number;
    }
    if all_integer {
        Ok(Value::Number((total as i64).into()))
    } else {
        number_result(total)
    }
}

fn avg_values(value: &Value) -> Result<Value, DwError> {
    let Value::Array(items) = value else {
        return Err(DwError::UnsupportedFeature(format!("avg({value:?})")));
    };
    if items.is_empty() {
        return Err(DwError::UnsupportedFeature(
            "avg expects a non-empty array".to_string(),
        ));
    }
    let mut total = 0.0;
    for item in items {
        total += numeric_value(item)?;
    }
    number_result(total / items.len() as f64)
}

fn is_empty_value(value: &Value) -> bool {
    match value {
        Value::Null => true,
        Value::String(value) => value.is_empty(),
        Value::Array(value) => value.is_empty(),
        Value::Object(value) if value.contains_key(DW_BINARY_MARKER) => {
            binary_bytes_from_map(value)
                .map(|bytes| bytes.is_empty())
                .unwrap_or(false)
        }
        Value::Object(value) => value.is_empty(),
        _ => false,
    }
}

fn is_blank_value(value: &Value) -> bool {
    match value {
        Value::Null => true,
        _ => as_dataweave_string(value).trim().is_empty(),
    }
}

fn is_numeric_value(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::String(value) => !value.is_empty() && value.chars().all(|ch| ch.is_numeric()),
        Value::Number(_) => true,
        _ => false,
    }
}

fn is_decimal_value(value: &Value) -> Result<bool, DwError> {
    if value.is_null() {
        return Ok(false);
    }
    let number = numeric_value(value)?;
    Ok((number - number.round()).abs() > f64::EPSILON)
}

fn is_integer_value(value: &Value) -> Result<bool, DwError> {
    if value.is_null() {
        return Ok(false);
    }
    let number = numeric_value(value)?;
    Ok(number.fract() == 0.0)
}

fn type_of_value(value: &Value) -> &'static str {
    match value {
        Value::Null => "Null",
        Value::Bool(_) => "Boolean",
        Value::Number(_) => "Number",
        Value::String(value) if is_time_literal_text(value) => "Time",
        Value::String(value) if is_date_time_literal_text(value) => "DateTime",
        Value::String(value) if is_date_literal_text(value) => "Date",
        Value::String(_) => "String",
        Value::Array(_) => "Array",
        Value::Object(value) if value.contains_key(DW_BINARY_MARKER) => "Binary",
        Value::Object(_) => "Object",
    }
}

fn is_time_literal_text(value: &str) -> bool {
    let value = value.trim_end_matches('Z');
    value.len() >= 8
        && value.as_bytes().get(2) == Some(&b':')
        && value.as_bytes().get(5) == Some(&b':')
        && parse_time_parts(value).is_some()
}

fn is_date_time_literal_text(value: &str) -> bool {
    value.len() >= 19
        && value.as_bytes().get(10) == Some(&b'T')
        && parse_date_parts(&value[..10]).is_some()
        && parse_time_parts(&value[11..19]).is_some()
}

fn is_date_literal_text(value: &str) -> bool {
    value.len() == 10 && parse_date_parts(value).is_some()
}

fn contains_value(items: &Value, element: &Value) -> Result<bool, DwError> {
    match items {
        Value::String(text) => {
            if element.is_null() {
                Ok(false)
            } else if let Some(pattern) = regex_literal_inner(&as_dataweave_string(element)) {
                Ok(Regex::new(pattern)
                    .map_err(|err| {
                        DwError::UnsupportedFeature(format!("regex pattern /{pattern}/: {err}"))
                    })?
                    .is_match(text))
            } else {
                Ok(text.contains(&as_dataweave_string(element)))
            }
        }
        Value::Object(map) => Ok(map
            .iter()
            .any(|(key, value)| Value::String(key.clone()) == *element || value == element)),
        Value::Array(items) => Ok(items.iter().any(|item| item == element)),
        Value::Null => Ok(false),
        _ => Err(DwError::UnsupportedFeature(format!(
            "contains({items:?}, {element:?})"
        ))),
    }
}

fn join_by(elements: &Value, separator: &Value) -> Result<Value, DwError> {
    if elements.is_null() {
        return Ok(Value::Null);
    }
    let Value::Array(elements) = elements else {
        return Err(DwError::UnsupportedFeature(format!(
            "joinBy expects an array, got {elements:?}"
        )));
    };
    let separator = if separator.is_null() {
        String::new()
    } else {
        as_dataweave_string(separator)
    };
    Ok(Value::String(
        elements
            .iter()
            .map(|element| {
                if element.is_null() {
                    String::new()
                } else {
                    as_dataweave_string(element)
                }
            })
            .collect::<Vec<_>>()
            .join(&separator),
    ))
}

fn split_by(text: &Value, separator: &Value) -> Result<Value, DwError> {
    if text.is_null() {
        return Ok(Value::Null);
    }
    let string = as_dataweave_string(text);
    if separator.is_null() {
        return Ok(Value::Array(vec![Value::String(string)]));
    }
    let separator = as_dataweave_string(separator);
    if separator.is_empty() {
        return Ok(Value::Array(
            string
                .chars()
                .map(|ch| Value::String(ch.to_string()))
                .collect(),
        ));
    }
    if let Some(pattern) = regex_literal_inner(&separator) {
        if pattern == r#"[.](?=(?:[^`]*`[^`]*`)*[^`]*$)"# {
            return Ok(Value::Array(
                split_dots_outside_backticks(&string)
                    .into_iter()
                    .map(Value::String)
                    .collect(),
            ));
        }
        let regex = Regex::new(pattern).map_err(|err| {
            DwError::UnsupportedFeature(format!("regex pattern /{pattern}/: {err}"))
        })?;
        return Ok(Value::Array(
            regex
                .split(&string)
                .map(|segment| Value::String(segment.to_string()))
                .collect(),
        ));
    }
    Ok(Value::Array(
        string
            .split(&separator)
            .map(|segment| Value::String(segment.to_string()))
            .collect(),
    ))
}

fn split_dots_outside_backticks(source: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut in_backticks = false;
    for ch in source.chars() {
        match ch {
            '`' => {
                in_backticks = !in_backticks;
                current.push(ch);
            }
            '.' if !in_backticks => {
                parts.push(current);
                current = String::new();
            }
            _ => current.push(ch),
        }
    }
    parts.push(current);
    parts
}

fn starts_with(text: &Value, prefix: &Value) -> bool {
    if text.is_null() {
        return false;
    }
    let prefix = if prefix.is_null() {
        String::new()
    } else {
        as_dataweave_string(prefix)
    };
    as_dataweave_string(text).starts_with(&prefix)
}

fn ends_with(text: &Value, suffix: &Value) -> bool {
    if text.is_null() {
        return false;
    }
    let suffix = if suffix.is_null() {
        String::new()
    } else {
        as_dataweave_string(suffix)
    };
    as_dataweave_string(text).ends_with(&suffix)
}

fn find_value(value: &Value, matcher: &Value) -> Result<Value, DwError> {
    if value.is_null() {
        return Ok(Value::Array(Vec::new()));
    }
    match value {
        Value::String(text) => {
            if matcher.is_null() {
                return Ok(Value::Array(Vec::new()));
            }
            let needle = as_dataweave_string(matcher);
            if let Some(pattern) = regex_literal_inner(&needle) {
                let regex = Regex::new(pattern).map_err(|err| {
                    DwError::UnsupportedFeature(format!("regex pattern /{pattern}/: {err}"))
                })?;
                return Ok(Value::Array(
                    regex
                        .find_iter(text)
                        .map(|matched| {
                            Value::Array(vec![
                                Value::Number(
                                    (byte_to_char_index(text, matched.start()) as i64).into(),
                                ),
                                Value::Number(
                                    (byte_to_char_index(text, matched.end()) as i64).into(),
                                ),
                            ])
                        })
                        .collect(),
                ));
            }
            let step = needle.len().max(1);
            let mut start = 0usize;
            let mut indices = Vec::new();
            while let Some(index) = text[start..].find(&needle) {
                let absolute_index = start + index;
                indices.push(Value::Number(
                    (byte_to_char_index(text, absolute_index) as i64).into(),
                ));
                start = absolute_index + step;
                if start > text.len() {
                    break;
                }
            }
            Ok(Value::Array(indices))
        }
        Value::Array(items) => Ok(Value::Array(
            items
                .iter()
                .enumerate()
                .filter_map(|(index, item)| {
                    if item == matcher {
                        Some(Value::Number((index as i64).into()))
                    } else {
                        None
                    }
                })
                .collect(),
        )),
        _ => Err(DwError::UnsupportedFeature(format!(
            "find({value:?}, {matcher:?})"
        ))),
    }
}

fn match_regex(value: &Value, pattern: &Value) -> Result<Value, DwError> {
    if value.is_null() || pattern.is_null() {
        return Ok(Value::Array(Vec::new()));
    }
    let text = as_dataweave_string(value);
    let pattern_text = as_dataweave_string(pattern);
    let pattern = regex_literal_inner(&pattern_text).unwrap_or(&pattern_text);
    let regex = Regex::new(pattern)
        .map_err(|err| DwError::UnsupportedFeature(format!("regex pattern /{pattern}/: {err}")))?;
    let Some(captures) = regex.captures(&text) else {
        return Ok(Value::Array(Vec::new()));
    };
    Ok(Value::Array(
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

fn scan_regex(value: &Value, pattern: &Value) -> Result<Value, DwError> {
    if value.is_null() || pattern.is_null() {
        return Ok(Value::Array(Vec::new()));
    }
    let text = as_dataweave_string(value);
    let pattern_text = as_dataweave_string(pattern);
    let pattern = regex_literal_inner(&pattern_text).unwrap_or(&pattern_text);
    let regex = Regex::new(pattern)
        .map_err(|err| DwError::UnsupportedFeature(format!("regex pattern /{pattern}/: {err}")))?;
    Ok(Value::Array(
        regex
            .captures_iter(&text)
            .map(|captures| {
                Value::Array(
                    (0..captures.len())
                        .map(|index| {
                            captures
                                .get(index)
                                .map(|matched| Value::String(matched.as_str().to_string()))
                                .unwrap_or(Value::Null)
                        })
                        .collect(),
                )
            })
            .collect(),
    ))
}

fn byte_to_char_index(text: &str, byte_index: usize) -> usize {
    text[..byte_index].chars().count()
}
