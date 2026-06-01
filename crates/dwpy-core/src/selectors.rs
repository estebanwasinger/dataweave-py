use serde_json::Map;
use serde_json::Value;

use crate::periods::temporal_field_value;
use crate::syntax::{is_identifier, is_top_level_index};
use crate::xml::{matching_xml_key, xml_list_items};
use crate::DwError;

pub(crate) fn evaluate_payload_path(source: &str, payload: &Value) -> Result<Value, DwError> {
    evaluate_payload_path_with_collapse(source, payload, true)
}

pub(crate) fn evaluate_payload_path_with_collapse(
    source: &str,
    payload: &Value,
    collapse_final: bool,
) -> Result<Value, DwError> {
    if source == "payload" {
        return Ok(payload.clone());
    }
    let mut current = payload.clone();
    let tail = source.strip_prefix("payload").unwrap_or_default();
    let segments = parse_path_segments(tail)?;
    for (index, segment) in segments.iter().enumerate() {
        let collapse = collapse_final && index + 1 == segments.len();
        current = select_path_segment(&current, segment, collapse)?;
    }
    Ok(current)
}

pub(crate) fn evaluate_local_path(
    source: &str,
    locals: &Map<String, Value>,
) -> Result<Option<Value>, DwError> {
    evaluate_local_path_with_collapse(source, locals, true)
}

pub(crate) fn evaluate_local_path_with_collapse(
    source: &str,
    locals: &Map<String, Value>,
    collapse_final: bool,
) -> Result<Option<Value>, DwError> {
    let (root, tail) = match source {
        "$" => return Ok(locals.get("$").cloned()),
        "$$" => return Ok(locals.get("$$").cloned()),
        _ if source.starts_with("$.") || source.starts_with("$?.") => ("$", &source[1..]),
        _ if source.starts_with("$$.") || source.starts_with("$$?.") => ("$$", &source[2..]),
        _ => {
            if let Some((root, tail)) = split_path_root(source) {
                (root, tail)
            } else if let Some(value) = locals.get(source) {
                return Ok(Some(value.clone()));
            } else {
                return Ok(None);
            }
        }
    };

    let Some(mut current) = locals.get(root).cloned() else {
        return Ok(None);
    };
    let segments = parse_path_segments(tail)?;
    for (index, segment) in segments.iter().enumerate() {
        let collapse = collapse_final && index + 1 == segments.len();
        current = select_path_segment(&current, segment, collapse)?;
    }
    Ok(Some(current))
}

fn split_path_root(source: &str) -> Option<(&str, &str)> {
    let dot = source.find('.');
    let optional_dot = source.find("?.");
    let index = match (dot, optional_dot) {
        (Some(dot), Some(optional_dot)) => dot.min(optional_dot),
        (Some(dot), None) => dot,
        (None, Some(optional_dot)) => optional_dot,
        (None, None) => return None,
    };
    Some((&source[..index], &source[index..]))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PathSegment {
    Property {
        attribute: String,
        present: bool,
        assert_present: bool,
    },
    KeyValue {
        attribute: String,
        present: bool,
        assert_present: bool,
    },
    Descendant {
        attribute: Option<String>,
        multi_value: bool,
        key_value: bool,
        present: bool,
        assert_present: bool,
    },
    Metadata {
        attribute: String,
    },
    Index {
        index: i64,
    },
}

pub const DW_OBJECT_PAIRS_MARKER: &str = "__dwpy_object_pairs";
pub const DW_METADATA_MARKER: &str = "__dwpy_metadata";
pub const DW_METADATA_VALUE_MARKER: &str = "__dwpy_metadata_value";
const DW_XML_KEY_NAMESPACE_MARKER: &str = "__dwpy_xml_key_namespace";
const DW_XML_KEY_ATTRIBUTES_MARKER: &str = "__dwpy_xml_key_attributes";

pub(crate) fn value_with_metadata(value: Value, metadata: Map<String, Value>) -> Value {
    Value::Object(Map::from_iter([
        (DW_METADATA_VALUE_MARKER.to_string(), value),
        (DW_METADATA_MARKER.to_string(), Value::Object(metadata)),
    ]))
}

pub(crate) fn metadata_value(value: &Value, attribute: &str) -> Option<Value> {
    let Value::Object(map) = value else {
        return None;
    };
    let Value::Object(metadata) = map.get(DW_METADATA_MARKER)? else {
        return None;
    };
    metadata.get(attribute).cloned()
}

pub(crate) fn unwrap_metadata_value(value: &Value) -> Option<Value> {
    let Value::Object(map) = value else {
        return None;
    };
    map.get(DW_METADATA_VALUE_MARKER).cloned()
}

pub(crate) fn duplicate_object_value(pairs: Vec<(String, Value)>) -> Value {
    Value::Object(Map::from_iter([(
        DW_OBJECT_PAIRS_MARKER.to_string(),
        Value::Array(
            pairs
                .into_iter()
                .map(|(key, value)| {
                    Value::Object(Map::from_iter([
                        ("key".to_string(), Value::String(key)),
                        ("value".to_string(), value),
                    ]))
                })
                .collect(),
        ),
    )]))
}

pub(crate) fn duplicate_object_pairs(value: &Value) -> Option<Vec<(String, Value)>> {
    let Value::Object(map) = value else {
        return None;
    };
    let Value::Array(items) = map.get(DW_OBJECT_PAIRS_MARKER)? else {
        return None;
    };
    let mut pairs = Vec::new();
    for item in items {
        let Value::Object(pair) = item else {
            return None;
        };
        let key = pair.get("key")?.as_str()?.to_string();
        let value = pair.get("value")?.clone();
        pairs.push((key, value));
    }
    Some(pairs)
}

pub(crate) fn parse_path_segments(source: &str) -> Result<Vec<PathSegment>, DwError> {
    let mut segments = Vec::new();
    let mut index = 0usize;
    while index < source.len() {
        let remaining = &source[index..];
        if remaining.starts_with('[') {
            let close = remaining
                .find(']')
                .ok_or_else(|| DwError::Parse(format!("unterminated path index {source}")))?;
            let index_source = remaining[1..close].trim();
            let item_index = index_source
                .parse::<i64>()
                .map_err(|_| DwError::UnsupportedFeature(format!("path {source}")))?;
            segments.push(PathSegment::Index { index: item_index });
            index += close + 1;
            continue;
        }

        let recursive = if remaining.starts_with("..") {
            index += 2;
            true
        } else if remaining.starts_with("?.") {
            index += 2;
            false
        } else if remaining.starts_with('.') {
            index += 1;
            false
        } else if index == 0 {
            false
        } else {
            return Err(DwError::UnsupportedFeature(format!("path {source}")));
        };

        if index >= source.len() {
            if recursive {
                segments.push(PathSegment::Descendant {
                    attribute: None,
                    multi_value: false,
                    key_value: false,
                    present: false,
                    assert_present: false,
                });
                break;
            }
            return Err(DwError::Parse(format!("empty path segment {source}")));
        }

        let remaining = &source[index..];
        let mut multi_value = false;
        let mut key_value = false;
        let mut metadata = false;
        if recursive && remaining.starts_with('*') {
            multi_value = true;
            index += 1;
        } else if recursive && remaining.starts_with('&') {
            key_value = true;
            index += 1;
        } else if !recursive && remaining.starts_with('&') {
            key_value = true;
            index += 1;
        } else if !recursive && remaining.starts_with('^') {
            metadata = true;
            index += 1;
        }

        let attribute_prefix = if !recursive && source[index..].starts_with("@.") {
            index += 2;
            true
        } else if !recursive && source[index..].starts_with('@') {
            index += 1;
            true
        } else {
            false
        };

        if attribute_prefix && index >= source.len() {
            let (present, assert_present) = parse_selector_suffix(source, &mut index)?;
            segments.push(PathSegment::Property {
                attribute: "@".to_string(),
                present,
                assert_present,
            });
            continue;
        }

        if !recursive && !key_value && source[index..].starts_with('#') {
            index += 1;
            let (present, assert_present) = parse_selector_suffix(source, &mut index)?;
            segments.push(PathSegment::Property {
                attribute: "#".to_string(),
                present,
                assert_present,
            });
            continue;
        }

        let segment_start = index;
        let remaining = &source[index..];
        if let Some(quote) = remaining
            .chars()
            .next()
            .filter(|ch| *ch == '"' || *ch == '\'')
        {
            let mut escaped = false;
            let mut closed = false;
            index += quote.len_utf8();
            while index < source.len() {
                let ch = source[index..].chars().next().unwrap_or_default();
                index += ch.len_utf8();
                if escaped {
                    escaped = false;
                } else if ch == '\\' {
                    escaped = true;
                } else if ch == quote {
                    let mut attribute = source
                        [segment_start + quote.len_utf8()..index - quote.len_utf8()]
                        .to_string();
                    if attribute_prefix {
                        attribute = format!("@{attribute}");
                    }
                    let (present, assert_present) = parse_selector_suffix(source, &mut index)?;
                    segments.push(if recursive {
                        PathSegment::Descendant {
                            attribute: Some(attribute),
                            multi_value,
                            key_value,
                            present,
                            assert_present,
                        }
                    } else if key_value {
                        PathSegment::KeyValue {
                            attribute,
                            present,
                            assert_present,
                        }
                    } else {
                        PathSegment::Property {
                            attribute,
                            present,
                            assert_present,
                        }
                    });
                    closed = true;
                    break;
                }
            }
            if !closed {
                return Err(DwError::Parse(format!(
                    "unterminated path segment {source}"
                )));
            }
            continue;
        }

        while index < source.len() {
            let remaining = &source[index..];
            if remaining.starts_with('.') || remaining.starts_with("?.") {
                break;
            }
            let ch = remaining.chars().next().unwrap_or_default();
            if ch == '!' || (ch == '?' && !remaining.starts_with("?.") && index > segment_start) {
                break;
            }
            if ch == '[' && index > segment_start {
                break;
            }
            if !(ch.is_ascii_alphanumeric()
                || ch == '_'
                || (!recursive && !key_value && index == segment_start && ch == '*'))
            {
                return Err(DwError::UnsupportedFeature(format!("path {source}")));
            }
            index += ch.len_utf8();
        }
        let segment = &source[segment_start..index];
        let identifier = segment.strip_prefix('*').unwrap_or(segment);
        if segment.is_empty() || identifier.is_empty() || !is_identifier(identifier) {
            return Err(DwError::UnsupportedFeature(format!("path {source}")));
        }
        let attribute = if attribute_prefix {
            format!("@{identifier}")
        } else {
            identifier.to_string()
        };
        let (present, assert_present) = parse_selector_suffix(source, &mut index)?;
        segments.push(if metadata {
            PathSegment::Metadata {
                attribute: identifier.to_string(),
            }
        } else if recursive {
            PathSegment::Descendant {
                attribute: Some(attribute),
                multi_value,
                key_value,
                present,
                assert_present,
            }
        } else if key_value {
            PathSegment::KeyValue {
                attribute,
                present,
                assert_present,
            }
        } else {
            PathSegment::Property {
                attribute: if attribute_prefix {
                    attribute
                } else {
                    segment.to_string()
                },
                present,
                assert_present,
            }
        });
    }
    Ok(segments)
}

fn parse_selector_suffix(source: &str, index: &mut usize) -> Result<(bool, bool), DwError> {
    let remaining = &source[*index..];
    if remaining.starts_with('!') {
        *index += 1;
        return Ok((false, true));
    }
    if remaining.starts_with('?') && !remaining.starts_with("?.") {
        *index += 1;
        return Ok((true, false));
    }
    Ok((false, false))
}

pub(crate) fn parse_postfix_path_access(source: &str) -> Option<(&str, &str)> {
    for (index, ch) in source.char_indices().rev() {
        if ch != '.' || !is_top_level_index(source, index) {
            continue;
        }
        let base = source[..index].trim();
        if base.ends_with(')') || base.ends_with(']') {
            return Some((base, &source[index..]));
        }
    }
    None
}

pub(crate) fn collapse_xml_like_value(value: &Value) -> Value {
    if let Some(value) = unwrap_metadata_value(value) {
        return collapse_xml_like_value(&value);
    }
    if let Some(items) = xml_list_items(value) {
        return items
            .first()
            .map(collapse_xml_like_value)
            .unwrap_or(Value::Null);
    }
    match value {
        Value::Object(map) if should_collapse_text_node(map) => {
            map.get("#text").cloned().unwrap_or_else(|| value.clone())
        }
        _ => value.clone(),
    }
}

fn should_collapse_text_node(map: &Map<String, Value>) -> bool {
    map.contains_key("#text")
        && map
            .keys()
            .all(|key| key == "#text" || key.starts_with('@') || key == DW_METADATA_MARKER)
}

fn should_preserve_xml_node_for_attribute_access(value: &Value) -> bool {
    matches!(value, Value::Object(map) if map.keys().any(|key| key.starts_with('@')))
}

fn select_property(value: &Value, part: &str) -> Value {
    collapse_xml_like_value(&select_property_raw(value, part))
}

pub(crate) fn select_path_segment(
    value: &Value,
    segment: &PathSegment,
    collapse: bool,
) -> Result<Value, DwError> {
    match segment {
        PathSegment::Property {
            attribute,
            present,
            assert_present,
        } => {
            let selected = if collapse {
                select_property(value, attribute)
            } else {
                select_property_raw(value, attribute)
            };
            apply_selector_presence(selected, attribute, *present, *assert_present)
        }
        PathSegment::KeyValue {
            attribute,
            present,
            assert_present,
        } => {
            let selected = select_key_value_pairs(value, attribute);
            apply_selector_presence(selected, attribute, *present, *assert_present)
        }
        PathSegment::Descendant {
            attribute,
            multi_value,
            key_value,
            present,
            assert_present,
        } => {
            let selected = if *key_value {
                let Some(attribute) = attribute else {
                    return Err(DwError::UnsupportedFeature(
                        "descendant key selector".to_string(),
                    ));
                };
                select_descendant_key_value_pairs(value, attribute)
            } else {
                select_descendant_property(value, attribute.as_deref(), *multi_value)
            };
            let display_name = attribute.as_deref().unwrap_or("selector");
            apply_selector_presence(selected, display_name, *present, *assert_present)
        }
        PathSegment::Metadata { attribute } => {
            Ok(metadata_value(value, attribute).unwrap_or(Value::Null))
        }
        PathSegment::Index { index } => Ok(select_index(value, *index)),
    }
}

fn select_index(value: &Value, index: i64) -> Value {
    let Value::Array(items) = value else {
        return Value::Null;
    };
    let resolved = if index < 0 {
        items.len() as i64 + index
    } else {
        index
    };
    if resolved < 0 {
        return Value::Null;
    }
    items.get(resolved as usize).cloned().unwrap_or(Value::Null)
}

fn apply_selector_presence(
    selected: Value,
    attribute: &str,
    present: bool,
    assert_present: bool,
) -> Result<Value, DwError> {
    if present {
        return Ok(Value::Bool(selector_value_present(&selected)));
    }
    if assert_present && !selector_value_present(&selected) {
        return Err(DwError::UnsupportedFeature(format!(
            "There is no key named '{}'",
            attribute.trim_start_matches('@')
        )));
    }
    Ok(selected)
}

pub(crate) fn selector_value_present(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::Array(items) => !items.is_empty(),
        Value::Object(map) if map.contains_key(DW_OBJECT_PAIRS_MARKER) => map
            .get(DW_OBJECT_PAIRS_MARKER)
            .and_then(Value::as_array)
            .is_some_and(|items| !items.is_empty()),
        _ => true,
    }
}

pub(crate) fn select_dynamic_multi_property(value: &Value, attribute: &str) -> Value {
    let selected = select_multi_property(value, attribute);
    if selector_value_present(&selected) {
        selected
    } else {
        Value::Null
    }
}

pub(crate) fn select_key_value_pairs(value: &Value, attribute: &str) -> Value {
    let mut pairs = Vec::new();
    collect_key_value_pairs(value, attribute, &mut pairs);
    if pairs.is_empty() {
        Value::Null
    } else {
        duplicate_object_value(pairs)
    }
}

fn collect_key_value_pairs(value: &Value, attribute: &str, output: &mut Vec<(String, Value)>) {
    match value {
        Value::Array(items) => {
            for item in items {
                collect_key_value_pairs(item, attribute, output);
            }
        }
        value if duplicate_object_pairs(value).is_some() => {
            let Some(pairs) = duplicate_object_pairs(value) else {
                return;
            };
            output.extend(
                pairs
                    .into_iter()
                    .filter(|(key, _)| key == attribute)
                    .map(|(key, value)| (key, collapse_xml_like_value(&value))),
            );
        }
        Value::Object(map) => {
            let selected = map
                .get(attribute)
                .or_else(|| matching_xml_key(map, attribute).and_then(|key| map.get(key)));
            if let Some(value) = selected {
                let key = matching_xml_key(map, attribute)
                    .map_or(attribute, |value| value.as_str())
                    .to_string();
                if let Some(items) = xml_list_items(value) {
                    output.extend(
                        items
                            .iter()
                            .map(|item| (key.clone(), collapse_xml_like_value(item))),
                    );
                } else {
                    output.push((key, collapse_xml_like_value(value)));
                }
            }
        }
        _ => {}
    }
}

fn select_property_raw(value: &Value, part: &str) -> Value {
    if let Some(unwrapped) = unwrap_metadata_value(value) {
        return select_property_raw(&unwrapped, part);
    }
    if part == "#" {
        if let Value::Object(map) = value {
            return map
                .get(DW_XML_KEY_NAMESPACE_MARKER)
                .cloned()
                .unwrap_or(Value::Null);
        }
        return Value::Null;
    }
    if part == "@" {
        if let Value::Object(map) = value {
            if let Some(attributes) = map.get(DW_XML_KEY_ATTRIBUTES_MARKER) {
                return attributes.clone();
            }
        }
    }
    if part == "@" {
        return select_attributes(value);
    }
    if let Some(value) = temporal_field_value(value, part) {
        return value;
    }
    if let Some(items) = xml_list_items(value) {
        for item in items {
            let selected = select_property_raw(item, part);
            if !selected.is_null() {
                return selected;
            }
        }
        return Value::Null;
    }
    if let Some(part) = part.strip_prefix('*') {
        return select_multi_property(value, part);
    }
    if let Some(pairs) = duplicate_object_pairs(value) {
        return pairs
            .into_iter()
            .find(|(key, _)| key == part)
            .map(|(_, value)| value)
            .unwrap_or(Value::Null);
    }
    match value {
        Value::Object(map) => map
            .get(part)
            .or_else(|| matching_xml_key(map, part).and_then(|key| map.get(key)))
            .cloned()
            .unwrap_or(Value::Null),
        Value::Array(items) => Value::Array(
            items
                .iter()
                .filter_map(|item| match item {
                    Value::Object(map) => map.get(part).cloned(),
                    _ => None,
                })
                .collect(),
        ),
        _ => Value::Null,
    }
}

fn select_attributes(value: &Value) -> Value {
    match value {
        Value::Object(map) => {
            Value::Object(Map::from_iter(map.iter().filter_map(|(key, value)| {
                key.strip_prefix('@')
                    .map(|name| (name.to_string(), collapse_xml_like_value(value)))
            })))
        }
        Value::Array(items) => Value::Array(items.iter().map(select_attributes).collect()),
        _ => Value::Null,
    }
}

fn select_multi_property(value: &Value, part: &str) -> Value {
    match value {
        value if duplicate_object_pairs(value).is_some() => {
            let Some(pairs) = duplicate_object_pairs(value) else {
                return Value::Array(Vec::new());
            };
            Value::Array(
                pairs
                    .into_iter()
                    .filter(|(key, _)| key == part)
                    .map(|(_, value)| collapse_xml_like_value(&value))
                    .collect(),
            )
        }
        Value::Object(map) => {
            let selected = map
                .get(part)
                .or_else(|| matching_xml_key(map, part).and_then(|key| map.get(key)));
            if selected.is_some_and(should_preserve_xml_node_for_attribute_access) {
                return Value::Array(vec![selected.cloned().unwrap_or(Value::Null)]);
            }
            match selected.map(collapse_xml_like_value) {
                Some(_value) if selected.and_then(xml_list_items).is_some() => {
                    let Some(items) = selected.and_then(xml_list_items) else {
                        return Value::Array(Vec::new());
                    };
                    Value::Array(
                        items
                            .iter()
                            .map(|item| {
                                if should_preserve_xml_node_for_attribute_access(item) {
                                    item.clone()
                                } else {
                                    collapse_xml_like_value(item)
                                }
                            })
                            .collect(),
                    )
                }
                Some(Value::Array(items)) => Value::Array(items),
                Some(value) => Value::Array(vec![value]),
                None => Value::Array(Vec::new()),
            }
        }
        Value::Array(items) => {
            let mut output = Vec::new();
            for item in items {
                match select_multi_property(item, part) {
                    Value::Array(values) => output.extend(values),
                    value if !value.is_null() => output.push(value),
                    _ => {}
                }
            }
            Value::Array(output)
        }
        _ => Value::Array(Vec::new()),
    }
}

fn append_selector_value(
    output: &mut Vec<Value>,
    value: &Value,
    expand_xml_list: bool,
    expand_array: bool,
) {
    if expand_xml_list {
        if let Some(items) = xml_list_items(value) {
            output.extend(items.iter().cloned());
            return;
        }
    }
    if expand_array {
        if let Value::Array(items) = value {
            output.extend(items.iter().cloned());
            return;
        }
    }
    output.push(value.clone());
}

fn select_descendant_property(value: &Value, attribute: Option<&str>, multi_value: bool) -> Value {
    let mut output = Vec::new();
    collect_descendant_property(value, attribute, multi_value, &mut output);
    Value::Array(output)
}

fn collect_descendant_property(
    value: &Value,
    attribute: Option<&str>,
    multi_value: bool,
    output: &mut Vec<Value>,
) {
    match value {
        Value::Array(items) => {
            for item in items {
                if attribute.is_none() {
                    append_selector_value(output, item, true, false);
                }
                collect_descendant_property(item, attribute, multi_value, output);
            }
        }
        Value::Object(map) => {
            if let Some(attribute) = attribute {
                let selected = map
                    .get(attribute)
                    .or_else(|| matching_xml_key(map, attribute).and_then(|key| map.get(key)));
                if let Some(value) = selected {
                    if multi_value {
                        append_selector_value(output, value, true, true);
                    } else if let Some(items) = xml_list_items(value) {
                        if let Some(item) = items.first() {
                            output.push(item.clone());
                        }
                    } else {
                        output.push(value.clone());
                    }
                }
                for value in map.values() {
                    collect_descendant_property(value, Some(attribute), multi_value, output);
                }
            } else {
                for value in map.values() {
                    append_selector_value(output, value, true, false);
                    collect_descendant_property(value, None, multi_value, output);
                }
            }
        }
        _ => {}
    }
}

fn select_descendant_key_value_pairs(value: &Value, attribute: &str) -> Value {
    let mut output = Vec::new();
    collect_descendant_key_value_pairs(value, attribute, &mut output);
    Value::Array(output)
}

fn collect_descendant_key_value_pairs(value: &Value, attribute: &str, output: &mut Vec<Value>) {
    match value {
        Value::Array(items) => {
            for item in items {
                collect_descendant_key_value_pairs(item, attribute, output);
            }
        }
        Value::Object(map) => {
            let selected = map
                .get(attribute)
                .or_else(|| matching_xml_key(map, attribute).and_then(|key| map.get(key)));
            if let Some(value) = selected {
                output.push(Value::Object(Map::from_iter([(
                    attribute.to_string(),
                    collapse_xml_like_value(value),
                )])));
            }
            for value in map.values() {
                collect_descendant_key_value_pairs(value, attribute, output);
            }
        }
        _ => {}
    }
}
