use serde_json::{Map, Value};

use crate::literals::{parse_call_args, parse_string_literal, string_literal_inner};
use crate::selectors::{parse_path_segments, PathSegment};
use crate::syntax::{
    is_identifier, parse_index_access, split_top_level, split_top_level_char,
    split_top_level_keyword, split_top_level_operator, strip_wrapping_parens,
};
use crate::types::{
    field_descriptor, type_any, type_array, type_boolean, type_kind, type_null, type_number,
    type_object, type_string, type_union,
};
use crate::DwError;

pub(crate) fn infer_expression_type(
    source: &str,
    payload_type: &Value,
    vars_type: &Value,
    attributes_type: &Value,
) -> Result<Value, DwError> {
    let source = strip_wrapping_parens(source.trim());
    if source.is_empty() {
        return Err(DwError::Parse("empty expression".to_string()));
    }

    if source.starts_with("not ") || source.starts_with('!') {
        return Ok(type_boolean());
    }

    if let Some((left, right)) = split_top_level_keyword(source, "default") {
        return Ok(type_union(vec![
            infer_expression_type(left, payload_type, vars_type, attributes_type)?,
            infer_expression_type(right, payload_type, vars_type, attributes_type)?,
        ]));
    }

    if let Some((left, _operator, right)) = split_top_level_operator(source, &["++", "+"]) {
        let left_type = infer_expression_type(left, payload_type, vars_type, attributes_type)?;
        let right_type = infer_expression_type(right, payload_type, vars_type, attributes_type)?;
        return Ok(infer_concat_type(&left_type, &right_type));
    }

    if source.starts_with('{') && source.ends_with('}') {
        return infer_object_literal_type(source, payload_type, vars_type, attributes_type);
    }

    if source.starts_with('[') && source.ends_with(']') {
        return infer_array_literal_type(source, payload_type, vars_type, attributes_type);
    }

    if let Some((base, index)) = parse_index_access(source) {
        let base_type = infer_expression_type(base, payload_type, vars_type, attributes_type)?;
        let index_type = infer_expression_type(index, payload_type, vars_type, attributes_type)?;
        return Ok(infer_index_type(&base_type, &index_type));
    }

    if string_literal_inner(source)?.is_some() {
        return Ok(type_string());
    }
    if source == "null" {
        return Ok(type_null());
    }
    if source == "true" || source == "false" {
        return Ok(type_boolean());
    }
    if source.parse::<i64>().is_ok() || source.parse::<f64>().is_ok() {
        return Ok(type_number());
    }

    if let Some((function_name, _arguments)) = parse_call_args(source) {
        if matches!(
            function_name,
            "p" | "Mule::p" | "prop" | "Mule::prop" | "dw::Runtime::p" | "dw::Runtime::prop"
        ) {
            return Ok(type_union(vec![type_string(), type_null()]));
        }
        if matches!(
            function_name,
            "props" | "Mule::props" | "dw::Runtime::props"
        ) {
            return Ok(type_object(Map::new(), true));
        }
    }

    if source == "payload" || source.starts_with("payload.") || source.starts_with("payload?.") {
        return infer_path_type(source, "payload", payload_type);
    }
    if source == "vars" || source.starts_with("vars.") || source.starts_with("vars?.") {
        return infer_path_type(source, "vars", vars_type);
    }
    if source == "attributes"
        || source.starts_with("attributes.")
        || source.starts_with("attributes?.")
    {
        return infer_path_type(source, "attributes", attributes_type);
    }

    Err(DwError::UnsupportedFeature(format!(
        "type inference {source}"
    )))
}

fn infer_object_literal_type(
    source: &str,
    payload_type: &Value,
    vars_type: &Value,
    attributes_type: &Value,
) -> Result<Value, DwError> {
    let inner = &source[1..source.len() - 1];
    let mut fields = Map::new();
    let mut open = false;
    for (index, entry) in split_top_level(inner, ',').iter().enumerate() {
        let entry = entry.trim();
        if entry.is_empty() {
            if index + 1 == split_top_level(inner, ',').len() {
                continue;
            }
            return Err(DwError::Parse("empty object entry".to_string()));
        }
        let Some((key_source, value_source)) = split_top_level_char(entry, ':') else {
            return Err(DwError::Parse(format!(
                "object entry missing ':' in {entry}"
            )));
        };
        let key_source = key_source.trim();
        let key = if key_source.starts_with('(') {
            open = true;
            None
        } else if let Some(key) = parse_string_literal(key_source)? {
            if key.contains("$(") {
                open = true;
                None
            } else {
                Some(key)
            }
        } else if is_identifier(key_source) {
            Some(key_source.to_string())
        } else {
            open = true;
            None
        };
        let value_type =
            infer_expression_type(value_source, payload_type, vars_type, attributes_type)?;
        if let Some(key) = key {
            fields.insert(key, field_descriptor(value_type, false, false));
        }
    }
    Ok(type_object(fields, open || source.trim() == "{}"))
}

fn infer_array_literal_type(
    source: &str,
    payload_type: &Value,
    vars_type: &Value,
    attributes_type: &Value,
) -> Result<Value, DwError> {
    let inner = &source[1..source.len() - 1];
    if inner.trim().is_empty() {
        return Ok(type_array(type_any()));
    }
    let mut element_types = Vec::new();
    for item in split_top_level(inner, ',') {
        let item = item.trim();
        if !item.is_empty() {
            element_types.push(infer_expression_type(
                item,
                payload_type,
                vars_type,
                attributes_type,
            )?);
        }
    }
    Ok(type_array(type_union(element_types)))
}

fn infer_path_type(source: &str, root: &str, root_type: &Value) -> Result<Value, DwError> {
    if source == root {
        return Ok(root_type.clone());
    }
    let mut current = root_type.clone();
    let tail = source.strip_prefix(root).unwrap_or_default();
    for segment in parse_path_segments(tail)? {
        match segment {
            PathSegment::Property {
                attribute, present, ..
            } => {
                if present {
                    current = type_boolean();
                    continue;
                }
                let is_multi = attribute.starts_with('*');
                let attribute = attribute.strip_prefix('*').unwrap_or(&attribute);
                current = property_type(&current, attribute);
                if is_multi && type_kind(&current) != Some("Array") {
                    current = type_array(current);
                }
            }
            PathSegment::KeyValue {
                attribute, present, ..
            } => {
                current = if present {
                    type_boolean()
                } else {
                    let fields = Map::from_iter([(
                        attribute.clone(),
                        field_descriptor(property_type(&current, &attribute), false, true),
                    )]);
                    type_object(fields, false)
                };
            }
            PathSegment::Descendant {
                attribute: Some(attribute),
                present,
                ..
            } => {
                current = if present {
                    type_boolean()
                } else {
                    type_array(property_type(&current, &attribute))
                };
            }
            PathSegment::Descendant {
                attribute: None,
                present,
                ..
            } => {
                current = if present {
                    type_boolean()
                } else {
                    type_array(type_any())
                };
            }
            PathSegment::Metadata { .. } => {
                current = type_any();
            }
            PathSegment::Index { .. } => {
                current = infer_index_type(&current, &type_number());
            }
        }
    }
    Ok(current)
}

fn property_type(base_type: &Value, attribute: &str) -> Value {
    match type_kind(base_type) {
        Some("Array") => {
            let element = base_type.get("element").cloned().unwrap_or_else(type_any);
            type_array(property_type(&element, attribute))
        }
        Some("Object") => {
            if let Some(field) = base_type
                .get("fields")
                .and_then(Value::as_object)
                .and_then(|fields| fields.get(attribute))
            {
                let mut field_type = field.get("type").cloned().unwrap_or_else(type_any);
                if field
                    .get("repeatable")
                    .and_then(Value::as_bool)
                    .unwrap_or(false)
                {
                    field_type = type_array(field_type);
                }
                if field
                    .get("optional")
                    .and_then(Value::as_bool)
                    .unwrap_or(false)
                {
                    field_type = type_union(vec![field_type, type_null()]);
                }
                field_type
            } else if base_type
                .get("open")
                .and_then(Value::as_bool)
                .unwrap_or(true)
            {
                type_any()
            } else {
                type_null()
            }
        }
        Some("Union") => type_union(
            base_type
                .get("options")
                .and_then(Value::as_array)
                .map(|options| {
                    options
                        .iter()
                        .map(|option| property_type(option, attribute))
                        .collect()
                })
                .unwrap_or_default(),
        ),
        _ => type_any(),
    }
}

fn infer_concat_type(left: &Value, right: &Value) -> Value {
    match (type_kind(left), type_kind(right)) {
        (Some("String"), Some("String")) => type_string(),
        (Some("Number"), Some("Number")) => type_number(),
        (Some("Array"), Some("Array")) => type_array(type_union(vec![
            left.get("element").cloned().unwrap_or_else(type_any),
            right.get("element").cloned().unwrap_or_else(type_any),
        ])),
        (Some("Object"), Some("Object")) => {
            let mut fields = left
                .get("fields")
                .and_then(Value::as_object)
                .cloned()
                .unwrap_or_default();
            if let Some(right_fields) = right.get("fields").and_then(Value::as_object) {
                for (key, value) in right_fields {
                    fields.insert(key.clone(), value.clone());
                }
            }
            type_object(
                fields,
                left.get("open").and_then(Value::as_bool).unwrap_or(true)
                    || right.get("open").and_then(Value::as_bool).unwrap_or(true),
            )
        }
        _ => type_any(),
    }
}

fn infer_index_type(base_type: &Value, index_type: &Value) -> Value {
    match type_kind(base_type) {
        Some("Array") => base_type.get("element").cloned().unwrap_or_else(type_any),
        Some("String") => type_string(),
        Some("Object") if type_kind(index_type) == Some("String") => {
            if let Some(fields) = base_type.get("fields").and_then(Value::as_object) {
                type_union(
                    fields
                        .values()
                        .filter_map(|field| field.get("type").cloned())
                        .collect(),
                )
            } else if base_type
                .get("open")
                .and_then(Value::as_bool)
                .unwrap_or(true)
            {
                type_any()
            } else {
                type_null()
            }
        }
        Some("Union") => type_union(
            base_type
                .get("options")
                .and_then(Value::as_array)
                .map(|options| {
                    options
                        .iter()
                        .map(|option| infer_index_type(option, index_type))
                        .collect()
                })
                .unwrap_or_default(),
        ),
        _ => type_any(),
    }
}
