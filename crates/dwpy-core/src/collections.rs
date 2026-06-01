use serde_json::{Map, Value};

use crate::selectors::{duplicate_object_pairs, duplicate_object_value};
use crate::syntax::{
    split_top_level, split_top_level_arrow, split_top_level_char, split_top_level_keyword,
    split_top_level_keyword_or_call_operator, split_top_level_operator, strip_wrapping_parens,
};
use crate::{
    as_dataweave_string, compare_sort_keys, evaluate_expression_scoped, group_key, is_truthy,
    number_result, numeric_value, stable_marker, DwError,
};

pub(crate) fn evaluate_map(
    input: &Value,
    mapper: &str,
    payload: &Value,
    outer_locals: &Map<String, Value>,
) -> Result<Value, DwError> {
    let Value::Array(items) = input else {
        return Ok(Value::Null);
    };
    let lambda = parse_lambda(mapper);
    let fast_lambda = FastLambda::parse(&lambda);
    let mut output = Vec::with_capacity(items.len());
    for (index, item) in items.iter().enumerate() {
        output.push(evaluate_lambda(
            &lambda,
            fast_lambda.as_ref(),
            outer_locals,
            item,
            index,
            payload,
        )?);
    }
    Ok(Value::Array(output))
}

pub(crate) fn evaluate_map_leaf_values(
    input: &Value,
    mapper: &str,
    payload: &Value,
    outer_locals: &Map<String, Value>,
) -> Result<Value, DwError> {
    let lambda = parse_lambda(mapper);
    map_leaf_value(
        input,
        &Value::Array(Vec::new()),
        &lambda,
        payload,
        outer_locals,
    )
}

pub(crate) fn evaluate_node_exists(
    input: &Value,
    predicate: &str,
    payload: &Value,
    outer_locals: &Map<String, Value>,
) -> Result<Value, DwError> {
    let lambda = parse_lambda(predicate);
    Ok(Value::Bool(node_exists_value(
        input,
        &Value::Array(Vec::new()),
        &lambda,
        payload,
        outer_locals,
    )?))
}

pub(crate) fn evaluate_filter_array_leafs(
    input: &Value,
    predicate: &str,
    payload: &Value,
    outer_locals: &Map<String, Value>,
) -> Result<Value, DwError> {
    let lambda = parse_lambda(predicate);
    Ok(filter_leaf_value(
        input,
        &Value::Array(Vec::new()),
        &lambda,
        "ARRAY_TYPE",
        payload,
        outer_locals,
    )?
    .unwrap_or_else(|| match input {
        Value::Array(_) => Value::Array(Vec::new()),
        Value::Object(_) => Value::Object(Map::new()),
        _ => Value::Null,
    }))
}

pub(crate) fn evaluate_filter_object_leafs(
    input: &Value,
    predicate: &str,
    payload: &Value,
    outer_locals: &Map<String, Value>,
) -> Result<Value, DwError> {
    let lambda = parse_lambda(predicate);
    Ok(filter_leaf_value(
        input,
        &Value::Array(Vec::new()),
        &lambda,
        "OBJECT_TYPE",
        payload,
        outer_locals,
    )?
    .unwrap_or_else(|| match input {
        Value::Array(_) => Value::Array(Vec::new()),
        Value::Object(_) => Value::Object(Map::new()),
        _ => Value::Null,
    }))
}

fn map_leaf_value(
    value: &Value,
    path: &Value,
    lambda: &ParsedLambda<'_>,
    payload: &Value,
    outer_locals: &Map<String, Value>,
) -> Result<Value, DwError> {
    match value {
        Value::Array(items) => items
            .iter()
            .enumerate()
            .map(|(index, item)| {
                map_leaf_value(
                    item,
                    &path_with_segment(path, "ARRAY_TYPE", &index.to_string()),
                    lambda,
                    payload,
                    outer_locals,
                )
            })
            .collect::<Result<Vec<_>, _>>()
            .map(Value::Array),
        Value::Object(map) => map
            .iter()
            .map(|(key, item)| {
                Ok((
                    key.clone(),
                    map_leaf_value(
                        item,
                        &path_with_segment(path, "OBJECT_TYPE", key),
                        lambda,
                        payload,
                        outer_locals,
                    )?,
                ))
            })
            .collect::<Result<Map<String, Value>, DwError>>()
            .map(Value::Object),
        leaf => evaluate_tree_lambda(lambda, outer_locals, leaf, path, payload),
    }
}

fn filter_leaf_value(
    value: &Value,
    path: &Value,
    lambda: &ParsedLambda<'_>,
    target_kind: &str,
    payload: &Value,
    outer_locals: &Map<String, Value>,
) -> Result<Option<Value>, DwError> {
    match value {
        Value::Array(items) => {
            let mut output = Vec::new();
            for (index, item) in items.iter().enumerate() {
                if let Some(filtered) = filter_leaf_value(
                    item,
                    &path_with_segment(path, "ARRAY_TYPE", &index.to_string()),
                    lambda,
                    target_kind,
                    payload,
                    outer_locals,
                )? {
                    output.push(filtered);
                }
            }
            Ok(Some(Value::Array(output)))
        }
        Value::Object(map) => {
            let mut output = Map::new();
            for (key, item) in map {
                if let Some(filtered) = filter_leaf_value(
                    item,
                    &path_with_segment(path, "OBJECT_TYPE", key),
                    lambda,
                    target_kind,
                    payload,
                    outer_locals,
                )? {
                    output.insert(key.clone(), filtered);
                }
            }
            Ok(Some(Value::Object(output)))
        }
        leaf => {
            if path_ends_with_kind(path, target_kind)
                && !is_truthy(&evaluate_tree_lambda(
                    lambda,
                    outer_locals,
                    leaf,
                    path,
                    payload,
                )?)
            {
                Ok(None)
            } else {
                Ok(Some(leaf.clone()))
            }
        }
    }
}

fn path_ends_with_kind(path: &Value, expected: &str) -> bool {
    path.as_array()
        .and_then(|items| items.last())
        .and_then(Value::as_object)
        .and_then(|segment| segment.get("kind"))
        .map(as_dataweave_string)
        .is_some_and(|kind| kind == expected)
}

fn node_exists_value(
    value: &Value,
    path: &Value,
    lambda: &ParsedLambda<'_>,
    payload: &Value,
    outer_locals: &Map<String, Value>,
) -> Result<bool, DwError> {
    if is_truthy(&evaluate_tree_lambda(
        lambda,
        outer_locals,
        value,
        path,
        payload,
    )?) {
        return Ok(true);
    }
    match value {
        Value::Array(items) => {
            for (index, item) in items.iter().enumerate() {
                if node_exists_value(
                    item,
                    &path_with_segment(path, "ARRAY_TYPE", &index.to_string()),
                    lambda,
                    payload,
                    outer_locals,
                )? {
                    return Ok(true);
                }
            }
            Ok(false)
        }
        Value::Object(map) => {
            for (key, item) in map {
                if node_exists_value(
                    item,
                    &path_with_segment(path, "OBJECT_TYPE", key),
                    lambda,
                    payload,
                    outer_locals,
                )? {
                    return Ok(true);
                }
            }
            Ok(false)
        }
        _ => Ok(false),
    }
}

fn evaluate_tree_lambda(
    lambda: &ParsedLambda<'_>,
    outer_locals: &Map<String, Value>,
    value: &Value,
    path: &Value,
    payload: &Value,
) -> Result<Value, DwError> {
    let mut locals = outer_locals.clone();
    locals.insert("$".to_string(), value.clone());
    locals.insert("$$".to_string(), path.clone());
    if let Some(first) = lambda.parameters.first() {
        locals.insert(first.name.to_string(), value.clone());
    }
    if let Some(second) = lambda.parameters.get(1) {
        locals.insert(second.name.to_string(), path.clone());
    }
    evaluate_expression_scoped(lambda.body, payload, &locals)
}

fn path_with_segment(path: &Value, kind: &str, selector: &str) -> Value {
    let mut items = path.as_array().cloned().unwrap_or_default();
    items.push(Value::Object(Map::from_iter([
        ("kind".to_string(), Value::String(kind.to_string())),
        ("selector".to_string(), Value::String(selector.to_string())),
        ("namespace".to_string(), Value::Null),
    ])));
    Value::Array(items)
}

pub(crate) fn evaluate_filter(
    input: &Value,
    predicate: &str,
    payload: &Value,
    outer_locals: &Map<String, Value>,
) -> Result<Value, DwError> {
    match input {
        Value::Array(items) => {
            let lambda = parse_lambda(predicate);
            let fast_lambda = FastLambda::parse(&lambda);
            let mut output = Vec::new();
            for (index, item) in items.iter().enumerate() {
                if is_truthy(&evaluate_lambda(
                    &lambda,
                    fast_lambda.as_ref(),
                    outer_locals,
                    item,
                    index,
                    payload,
                )?) {
                    output.push(item.clone());
                }
            }
            Ok(Value::Array(output))
        }
        Value::String(text) => {
            let lambda = parse_lambda(predicate);
            let mut output = String::new();
            for (index, ch) in text.chars().enumerate() {
                let item = Value::String(ch.to_string());
                if is_truthy(&evaluate_lambda(
                    &lambda,
                    None,
                    outer_locals,
                    &item,
                    index,
                    payload,
                )?) {
                    output.push(ch);
                }
            }
            Ok(Value::String(output))
        }
        _ => Ok(Value::Null),
    }
}

pub(crate) fn evaluate_count_characters_by(
    input: &Value,
    predicate: &str,
    payload: &Value,
    outer_locals: &Map<String, Value>,
) -> Result<Value, DwError> {
    if input.is_null() {
        return Ok(Value::Null);
    }
    let lambda = parse_lambda(predicate);
    let mut count = 0i64;
    for (index, ch) in as_dataweave_string(input).chars().enumerate() {
        let item = Value::String(ch.to_string());
        if is_truthy(&evaluate_lambda(
            &lambda,
            None,
            outer_locals,
            &item,
            index,
            payload,
        )?) {
            count += 1;
        }
    }
    Ok(Value::Number(count.into()))
}

pub(crate) fn evaluate_every_character(
    input: &Value,
    predicate: &str,
    payload: &Value,
    outer_locals: &Map<String, Value>,
) -> Result<Value, DwError> {
    if input.is_null() {
        return Ok(Value::Bool(false));
    }
    let source = as_dataweave_string(input);
    if source.is_empty() {
        return Ok(Value::Bool(false));
    }
    let lambda = parse_lambda(predicate);
    for (index, ch) in source.chars().enumerate() {
        let item = Value::String(ch.to_string());
        if !is_truthy(&evaluate_lambda(
            &lambda,
            None,
            outer_locals,
            &item,
            index,
            payload,
        )?) {
            return Ok(Value::Bool(false));
        }
    }
    Ok(Value::Bool(true))
}

pub(crate) fn evaluate_map_string(
    input: &Value,
    mapper: &str,
    payload: &Value,
    outer_locals: &Map<String, Value>,
) -> Result<Value, DwError> {
    if input.is_null() {
        return Ok(Value::Null);
    }
    let lambda = parse_lambda(mapper);
    let mut output = String::new();
    for (index, ch) in as_dataweave_string(input).chars().enumerate() {
        let item = Value::String(ch.to_string());
        output.push_str(&as_dataweave_string(&evaluate_lambda(
            &lambda,
            None,
            outer_locals,
            &item,
            index,
            payload,
        )?));
    }
    Ok(Value::String(output))
}

pub(crate) fn evaluate_some_character(
    input: &Value,
    predicate: &str,
    payload: &Value,
    outer_locals: &Map<String, Value>,
) -> Result<Value, DwError> {
    if input.is_null() {
        return Ok(Value::Bool(false));
    }
    let lambda = parse_lambda(predicate);
    for (index, ch) in as_dataweave_string(input).chars().enumerate() {
        let item = Value::String(ch.to_string());
        if is_truthy(&evaluate_lambda(
            &lambda,
            None,
            outer_locals,
            &item,
            index,
            payload,
        )?) {
            return Ok(Value::Bool(true));
        }
    }
    Ok(Value::Bool(false))
}

pub(crate) fn evaluate_substring_by(
    input: &Value,
    predicate: &str,
    payload: &Value,
    outer_locals: &Map<String, Value>,
) -> Result<Value, DwError> {
    if input.is_null() {
        return Ok(Value::Null);
    }
    let lambda = parse_lambda(predicate);
    let mut output = Vec::new();
    let mut current = String::new();
    for (index, ch) in as_dataweave_string(input).chars().enumerate() {
        let item = Value::String(ch.to_string());
        if is_truthy(&evaluate_lambda(
            &lambda,
            None,
            outer_locals,
            &item,
            index,
            payload,
        )?) {
            output.push(Value::String(current));
            current = String::new();
        } else {
            current.push(ch);
        }
    }
    output.push(Value::String(current));
    Ok(Value::Array(output))
}

pub(crate) fn evaluate_pluck(
    input: &Value,
    mapper: &str,
    payload: &Value,
    outer_locals: &Map<String, Value>,
) -> Result<Value, DwError> {
    let Value::Object(entries) = input else {
        return Ok(Value::Null);
    };
    let lambda = parse_lambda(mapper);
    entries
        .iter()
        .enumerate()
        .map(|(index, (key, value))| {
            let locals = object_lambda_locals(outer_locals, value, key, index, &lambda.parameters);
            evaluate_expression_scoped(lambda.body, payload, &locals)
        })
        .collect::<Result<Vec<_>, _>>()
        .map(Value::Array)
}

pub(crate) fn evaluate_map_object(
    input: &Value,
    mapper: &str,
    payload: &Value,
    outer_locals: &Map<String, Value>,
) -> Result<Value, DwError> {
    let Value::Object(entries) = input else {
        return Ok(Value::Null);
    };
    let lambda = parse_lambda(mapper);
    if let Some(fast_mapper) = FastObjectMapper::parse(&lambda) {
        let mut output = Map::new();
        for (index, (key, value)) in entries.iter().enumerate() {
            if let Some((mapped_key, mapped_value)) = fast_mapper.evaluate(value, key, index)? {
                output.insert(mapped_key, mapped_value);
            }
        }
        return Ok(Value::Object(output));
    }
    let mut output = Map::new();
    for (index, (key, value)) in entries.iter().enumerate() {
        let locals = object_lambda_locals(outer_locals, value, key, index, &lambda.parameters);
        match evaluate_expression_scoped(lambda.body, payload, &locals)? {
            Value::Null => {}
            Value::Object(mapped) => output.extend(mapped),
            other => {
                return Err(DwError::UnsupportedFeature(format!(
                    "mapObject mapper returned {other:?}"
                )))
            }
        }
    }
    Ok(Value::Object(output))
}

pub(crate) fn evaluate_filter_object(
    input: &Value,
    predicate: &str,
    payload: &Value,
    outer_locals: &Map<String, Value>,
) -> Result<Value, DwError> {
    let Value::Object(entries) = input else {
        return Ok(Value::Null);
    };
    let lambda = parse_lambda(predicate);
    let fast_predicate = FastObjectPredicate::parse(&lambda);
    let mut output = Map::new();
    for (index, (key, value)) in entries.iter().enumerate() {
        if evaluate_object_predicate(
            &lambda,
            fast_predicate.as_ref(),
            outer_locals,
            value,
            key,
            index,
            payload,
        )? {
            output.insert(key.clone(), value.clone());
        }
    }
    Ok(Value::Object(output))
}

pub(crate) fn evaluate_take_while_object(
    input: &Value,
    predicate: &str,
    payload: &Value,
    outer_locals: &Map<String, Value>,
) -> Result<Value, DwError> {
    let Value::Object(entries) = input else {
        return Ok(Value::Object(Map::new()));
    };
    let lambda = parse_lambda(predicate);
    let fast_predicate = FastObjectPredicate::parse(&lambda);
    let mut output = Map::new();
    for (index, (key, value)) in entries.iter().enumerate() {
        if evaluate_object_predicate(
            &lambda,
            fast_predicate.as_ref(),
            outer_locals,
            value,
            key,
            index,
            payload,
        )? {
            output.insert(key.clone(), value.clone());
        } else {
            break;
        }
    }
    Ok(Value::Object(output))
}

pub(crate) fn evaluate_take_while(
    input: &Value,
    predicate: &str,
    payload: &Value,
    outer_locals: &Map<String, Value>,
) -> Result<Value, DwError> {
    match input {
        Value::Array(items) => {
            let lambda = parse_lambda(predicate);
            let fast_lambda = FastLambda::parse(&lambda);
            let mut output = Vec::new();
            for (index, item) in items.iter().enumerate() {
                if is_truthy(&evaluate_lambda(
                    &lambda,
                    fast_lambda.as_ref(),
                    outer_locals,
                    item,
                    index,
                    payload,
                )?) {
                    output.push(item.clone());
                } else {
                    break;
                }
            }
            Ok(Value::Array(output))
        }
        Value::Object(_) => evaluate_take_while_object(input, predicate, payload, outer_locals),
        Value::Null => Ok(Value::Null),
        _ => Ok(Value::Null),
    }
}

pub(crate) fn evaluate_drop_while(
    input: &Value,
    predicate: &str,
    payload: &Value,
    outer_locals: &Map<String, Value>,
) -> Result<Value, DwError> {
    let Value::Array(items) = input else {
        return Ok(Value::Null);
    };
    let lambda = parse_lambda(predicate);
    let fast_lambda = FastLambda::parse(&lambda);
    let mut start = items.len();
    for (index, item) in items.iter().enumerate() {
        if !is_truthy(&evaluate_lambda(
            &lambda,
            fast_lambda.as_ref(),
            outer_locals,
            item,
            index,
            payload,
        )?) {
            start = index;
            break;
        }
    }
    Ok(Value::Array(items[start..].to_vec()))
}

pub(crate) fn evaluate_some(
    input: &Value,
    predicate: &str,
    payload: &Value,
    outer_locals: &Map<String, Value>,
) -> Result<Value, DwError> {
    let Value::Array(items) = input else {
        return Ok(Value::Bool(false));
    };
    if items.is_empty() {
        return Ok(Value::Bool(false));
    }
    let lambda = parse_lambda(predicate);
    let fast_lambda = FastLambda::parse(&lambda);
    for (index, item) in items.iter().enumerate() {
        if is_truthy(&evaluate_lambda(
            &lambda,
            fast_lambda.as_ref(),
            outer_locals,
            item,
            index,
            payload,
        )?) {
            return Ok(Value::Bool(true));
        }
    }
    Ok(Value::Bool(false))
}

pub(crate) fn evaluate_every(
    input: &Value,
    predicate: &str,
    payload: &Value,
    outer_locals: &Map<String, Value>,
) -> Result<Value, DwError> {
    let Value::Array(items) = input else {
        return Ok(Value::Bool(false));
    };
    if items.is_empty() {
        return Ok(Value::Bool(false));
    }
    let lambda = parse_lambda(predicate);
    let fast_lambda = FastLambda::parse(&lambda);
    for (index, item) in items.iter().enumerate() {
        if !is_truthy(&evaluate_lambda(
            &lambda,
            fast_lambda.as_ref(),
            outer_locals,
            item,
            index,
            payload,
        )?) {
            return Ok(Value::Bool(false));
        }
    }
    Ok(Value::Bool(true))
}

pub(crate) fn evaluate_count_by(
    input: &Value,
    predicate: &str,
    payload: &Value,
    outer_locals: &Map<String, Value>,
) -> Result<Value, DwError> {
    let Value::Array(items) = input else {
        return Ok(Value::Null);
    };
    let lambda = parse_lambda(predicate);
    let fast_lambda = FastLambda::parse(&lambda);
    let mut count = 0i64;
    for (index, item) in items.iter().enumerate() {
        if is_truthy(&evaluate_lambda(
            &lambda,
            fast_lambda.as_ref(),
            outer_locals,
            item,
            index,
            payload,
        )?) {
            count += 1;
        }
    }
    Ok(Value::Number(count.into()))
}

pub(crate) fn evaluate_sum_by(
    input: &Value,
    selector: &str,
    payload: &Value,
    outer_locals: &Map<String, Value>,
) -> Result<Value, DwError> {
    let Value::Array(items) = input else {
        return Ok(Value::Null);
    };
    let lambda = parse_lambda(selector);
    let fast_lambda = FastLambda::parse(&lambda);
    let mut total = 0.0;
    for (index, item) in items.iter().enumerate() {
        let value = evaluate_lambda(
            &lambda,
            fast_lambda.as_ref(),
            outer_locals,
            item,
            index,
            payload,
        )?;
        total += numeric_value(&value)?;
    }
    crate::number_result(total)
}

pub(crate) fn evaluate_first_with(
    input: &Value,
    predicate: &str,
    payload: &Value,
    outer_locals: &Map<String, Value>,
) -> Result<Value, DwError> {
    let Value::Array(items) = input else {
        return Ok(Value::Null);
    };
    let lambda = parse_lambda(predicate);
    let fast_lambda = FastLambda::parse(&lambda);
    for (index, item) in items.iter().enumerate() {
        if is_truthy(&evaluate_lambda(
            &lambda,
            fast_lambda.as_ref(),
            outer_locals,
            item,
            index,
            payload,
        )?) {
            return Ok(item.clone());
        }
    }
    Ok(Value::Null)
}

pub(crate) fn evaluate_index_where(
    input: &Value,
    predicate: &str,
    payload: &Value,
    outer_locals: &Map<String, Value>,
) -> Result<Value, DwError> {
    let Value::Array(items) = input else {
        return Ok(Value::Null);
    };
    let lambda = parse_lambda(predicate);
    let fast_lambda = FastLambda::parse(&lambda);
    for (index, item) in items.iter().enumerate() {
        if is_truthy(&evaluate_lambda(
            &lambda,
            fast_lambda.as_ref(),
            outer_locals,
            item,
            index,
            payload,
        )?) {
            return Ok(Value::Number((index as i64).into()));
        }
    }
    Ok(Value::Number((-1).into()))
}

pub(crate) fn evaluate_partition(
    input: &Value,
    predicate: &str,
    payload: &Value,
    outer_locals: &Map<String, Value>,
) -> Result<Value, DwError> {
    let Value::Array(items) = input else {
        return Ok(Value::Null);
    };
    let lambda = parse_lambda(predicate);
    let fast_lambda = FastLambda::parse(&lambda);
    let mut success = Vec::new();
    let mut failure = Vec::new();
    for (index, item) in items.iter().enumerate() {
        if is_truthy(&evaluate_lambda(
            &lambda,
            fast_lambda.as_ref(),
            outer_locals,
            item,
            index,
            payload,
        )?) {
            success.push(item.clone());
        } else {
            failure.push(item.clone());
        }
    }
    Ok(Value::Object(Map::from_iter([
        ("success".to_string(), Value::Array(success)),
        ("failure".to_string(), Value::Array(failure)),
    ])))
}

pub(crate) fn evaluate_split_where(
    input: &Value,
    predicate: &str,
    payload: &Value,
    outer_locals: &Map<String, Value>,
) -> Result<Value, DwError> {
    let Value::Array(items) = input else {
        return Ok(Value::Null);
    };
    let lambda = parse_lambda(predicate);
    let fast_lambda = FastLambda::parse(&lambda);
    let mut split_index = items.len();
    for (index, item) in items.iter().enumerate() {
        let value = evaluate_lambda(
            &lambda,
            fast_lambda.as_ref(),
            outer_locals,
            item,
            index,
            payload,
        )?;
        if is_truthy(&value) {
            split_index = index;
            break;
        }
    }
    Ok(split_pair(&items[..split_index], &items[split_index..]))
}

pub(crate) fn evaluate_join(
    left: &Value,
    right: &Value,
    left_key_selector: &str,
    right_key_selector: &str,
    mode: JoinMode,
    payload: &Value,
    outer_locals: &Map<String, Value>,
) -> Result<Value, DwError> {
    let (Value::Array(left_items), Value::Array(right_items)) = (left, right) else {
        return Ok(Value::Null);
    };
    let left_lambda = parse_lambda(left_key_selector);
    let right_lambda = parse_lambda(right_key_selector);
    let mut output = Vec::new();
    let mut matched_right = vec![false; right_items.len()];

    for (left_index, left_item) in left_items.iter().enumerate() {
        let left_key = {
            let locals =
                lambda_locals(outer_locals, left_item, left_index, &left_lambda.parameters);
            stable_marker(&evaluate_expression_scoped(
                left_lambda.body,
                payload,
                &locals,
            )?)
        };
        let mut matched_left = false;
        for (right_index, right_item) in right_items.iter().enumerate() {
            let right_key = {
                let locals = lambda_locals(
                    outer_locals,
                    right_item,
                    right_index,
                    &right_lambda.parameters,
                );
                stable_marker(&evaluate_expression_scoped(
                    right_lambda.body,
                    payload,
                    &locals,
                )?)
            };
            if left_key == right_key {
                matched_left = true;
                matched_right[right_index] = true;
                output.push(join_pair(Some(left_item.clone()), Some(right_item.clone())));
            }
        }
        if !matched_left && matches!(mode, JoinMode::Left | JoinMode::Outer) {
            output.push(join_pair(Some(left_item.clone()), None));
        }
    }

    if matches!(mode, JoinMode::Outer) {
        for (index, right_item) in right_items.iter().enumerate() {
            if !matched_right[index] {
                output.push(join_pair(None, Some(right_item.clone())));
            }
        }
    }
    Ok(Value::Array(output))
}

#[derive(Clone, Copy)]
pub(crate) enum JoinMode {
    Inner,
    Left,
    Outer,
}

fn join_pair(left: Option<Value>, right: Option<Value>) -> Value {
    let mut output = Map::new();
    if let Some(left) = left {
        output.insert("l".to_string(), left);
    }
    if let Some(right) = right {
        output.insert("r".to_string(), right);
    }
    Value::Object(output)
}

pub(crate) fn evaluate_take(input: &Value, amount: &Value) -> Result<Value, DwError> {
    let Value::Array(items) = input else {
        return Ok(Value::Null);
    };
    let count = nonnegative_index(amount)?;
    Ok(Value::Array(items.iter().take(count).cloned().collect()))
}

pub(crate) fn evaluate_drop(input: &Value, amount: &Value) -> Result<Value, DwError> {
    let Value::Array(items) = input else {
        return Ok(Value::Null);
    };
    let count = nonnegative_index(amount)?;
    Ok(Value::Array(items.iter().skip(count).cloned().collect()))
}

pub(crate) fn evaluate_slice(input: &Value, from: &Value, until: &Value) -> Result<Value, DwError> {
    let Value::Array(items) = input else {
        return Ok(Value::Null);
    };
    let start = nonnegative_index(from)?.min(items.len());
    let end = nonnegative_index(until)?.min(items.len());
    if end <= start {
        return Ok(Value::Array(Vec::new()));
    }
    Ok(Value::Array(items[start..end].to_vec()))
}

pub(crate) fn evaluate_split_at(input: &Value, amount: &Value) -> Result<Value, DwError> {
    let Value::Array(items) = input else {
        return Ok(Value::Null);
    };
    let index = nonnegative_index(amount)?.min(items.len());
    Ok(split_pair(&items[..index], &items[index..]))
}

pub(crate) fn evaluate_every_entry(
    input: &Value,
    predicate: &str,
    payload: &Value,
    outer_locals: &Map<String, Value>,
) -> Result<Value, DwError> {
    let Value::Object(entries) = input else {
        return Ok(Value::Bool(true));
    };
    let lambda = parse_lambda(predicate);
    let fast_predicate = FastObjectPredicate::parse(&lambda);
    for (index, (key, value)) in entries.iter().enumerate() {
        if !evaluate_object_predicate(
            &lambda,
            fast_predicate.as_ref(),
            outer_locals,
            value,
            key,
            index,
            payload,
        )? {
            return Ok(Value::Bool(false));
        }
    }
    Ok(Value::Bool(true))
}

fn nonnegative_index(value: &Value) -> Result<usize, DwError> {
    Ok((numeric_value(value)? as i64).max(0) as usize)
}

fn split_pair(left: &[Value], right: &[Value]) -> Value {
    Value::Object(Map::from_iter([
        ("l".to_string(), Value::Array(left.to_vec())),
        ("r".to_string(), Value::Array(right.to_vec())),
    ]))
}

pub(crate) fn evaluate_some_entry(
    input: &Value,
    predicate: &str,
    payload: &Value,
    outer_locals: &Map<String, Value>,
) -> Result<Value, DwError> {
    let Value::Object(entries) = input else {
        return Ok(Value::Bool(false));
    };
    let lambda = parse_lambda(predicate);
    let fast_predicate = FastObjectPredicate::parse(&lambda);
    for (index, (key, value)) in entries.iter().enumerate() {
        if evaluate_object_predicate(
            &lambda,
            fast_predicate.as_ref(),
            outer_locals,
            value,
            key,
            index,
            payload,
        )? {
            return Ok(Value::Bool(true));
        }
    }
    Ok(Value::Bool(false))
}

pub(crate) fn evaluate_flat_map(
    input: &Value,
    mapper: &str,
    payload: &Value,
    outer_locals: &Map<String, Value>,
) -> Result<Value, DwError> {
    let Value::Array(items) = input else {
        return Ok(Value::Null);
    };
    let lambda = parse_lambda(mapper);
    let fast_lambda = FastLambda::parse(&lambda);
    let mut output = Vec::new();
    for (index, item) in items.iter().enumerate() {
        let mapped = evaluate_lambda(
            &lambda,
            fast_lambda.as_ref(),
            outer_locals,
            item,
            index,
            payload,
        )?;
        match mapped {
            Value::Array(items) => output.extend(items),
            Value::Null => {}
            value => output.push(value),
        }
    }
    Ok(Value::Array(output))
}

pub(crate) fn evaluate_group_by(
    input: &Value,
    criteria: &str,
    payload: &Value,
    outer_locals: &Map<String, Value>,
) -> Result<Value, DwError> {
    match input {
        Value::Array(items) => {
            let lambda = parse_lambda(criteria);
            let fast_lambda = FastLambda::parse(&lambda);
            let mut grouped = Map::new();
            for (index, item) in items.iter().enumerate() {
                let key = group_key(&evaluate_lambda(
                    &lambda,
                    fast_lambda.as_ref(),
                    outer_locals,
                    item,
                    index,
                    payload,
                )?);
                let bucket = grouped
                    .entry(key)
                    .or_insert_with(|| Value::Array(Vec::new()));
                if let Value::Array(values) = bucket {
                    values.push(item.clone());
                }
            }
            Ok(Value::Object(grouped))
        }
        Value::Object(entries) => {
            let lambda = parse_lambda(criteria);
            let mut grouped = Map::new();
            for (index, (entry_key, entry_value)) in entries.iter().enumerate() {
                let locals = object_lambda_locals(
                    outer_locals,
                    entry_value,
                    entry_key,
                    index,
                    &lambda.parameters,
                );
                let key = group_key(&evaluate_expression_scoped(lambda.body, payload, &locals)?);
                let bucket = grouped
                    .entry(key)
                    .or_insert_with(|| Value::Object(Map::new()));
                if let Value::Object(values) = bucket {
                    values.insert(entry_key.clone(), entry_value.clone());
                }
            }
            Ok(Value::Object(grouped))
        }
        Value::String(text) => {
            let lambda = parse_lambda(criteria);
            let fast_lambda = FastLambda::parse(&lambda);
            let mut grouped = Map::new();
            for (index, ch) in text.chars().enumerate() {
                let value = Value::String(ch.to_string());
                let key = group_key(&evaluate_lambda(
                    &lambda,
                    fast_lambda.as_ref(),
                    outer_locals,
                    &value,
                    index,
                    payload,
                )?);
                let bucket = grouped
                    .entry(key)
                    .or_insert_with(|| Value::String(String::new()));
                if let Value::String(output) = bucket {
                    output.push(ch);
                }
            }
            Ok(Value::Object(grouped))
        }
        _ => Ok(Value::Null),
    }
}

pub(crate) fn evaluate_distinct_by(
    input: &Value,
    criteria: &str,
    payload: &Value,
    outer_locals: &Map<String, Value>,
) -> Result<Value, DwError> {
    let lambda = parse_lambda(criteria);
    if let Some(pairs) = duplicate_object_pairs(input) {
        let distinct = distinct_object_pairs(pairs, &lambda, payload, outer_locals)?;
        return Ok(duplicate_object_value(distinct));
    }
    if let Value::Object(map) = input {
        let pairs = map
            .iter()
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect::<Vec<_>>();
        let distinct = distinct_object_pairs(pairs, &lambda, payload, outer_locals)?;
        return Ok(Value::Object(distinct.into_iter().collect()));
    }
    let Value::Array(items) = input else {
        return Ok(Value::Null);
    };
    let fast_lambda = FastLambda::parse(&lambda);
    let mut seen = Vec::new();
    let mut output = Vec::new();
    for (index, item) in items.iter().enumerate() {
        let key = stable_marker(&evaluate_lambda(
            &lambda,
            fast_lambda.as_ref(),
            outer_locals,
            item,
            index,
            payload,
        )?);
        if !seen.contains(&key) {
            seen.push(key);
            output.push(item.clone());
        }
    }
    Ok(Value::Array(output))
}

fn distinct_object_pairs(
    pairs: Vec<(String, Value)>,
    lambda: &ParsedLambda<'_>,
    payload: &Value,
    outer_locals: &Map<String, Value>,
) -> Result<Vec<(String, Value)>, DwError> {
    let mut seen = Vec::new();
    let mut output = Vec::new();
    for (index, (key, value)) in pairs.into_iter().enumerate() {
        let locals = object_lambda_locals(outer_locals, &value, &key, index, &lambda.parameters);
        let marker = stable_marker(&evaluate_expression_scoped(lambda.body, payload, &locals)?);
        if !seen.contains(&marker) {
            seen.push(marker);
            output.push((key, value));
        }
    }
    Ok(output)
}

pub(crate) fn evaluate_order_by(
    input: &Value,
    criteria: &str,
    payload: &Value,
    outer_locals: &Map<String, Value>,
) -> Result<Value, DwError> {
    let Value::Array(items) = input else {
        return Ok(Value::Null);
    };
    let lambda = parse_lambda(criteria);
    let fast_lambda = FastLambda::parse(&lambda);
    let mut decorated = Vec::new();
    for (index, item) in items.iter().enumerate() {
        let key = evaluate_lambda(
            &lambda,
            fast_lambda.as_ref(),
            outer_locals,
            item,
            index,
            payload,
        )?;
        decorated.push((key, index, item.clone()));
    }
    decorated.sort_by(|left, right| {
        compare_sort_keys(&left.0, &right.0).then_with(|| left.1.cmp(&right.1))
    });
    Ok(Value::Array(
        decorated.into_iter().map(|(_, _, item)| item).collect(),
    ))
}

pub(crate) fn evaluate_by(
    input: &Value,
    criteria: &str,
    max: bool,
    payload: &Value,
    outer_locals: &Map<String, Value>,
) -> Result<Value, DwError> {
    let Value::Array(items) = input else {
        return Ok(Value::Null);
    };
    if items.is_empty() {
        return Ok(Value::Null);
    }
    let lambda = parse_lambda(criteria);
    let mut selected_item = items[0].clone();
    let mut selected_key = {
        let locals = lambda_locals(outer_locals, &selected_item, 0, &lambda.parameters);
        evaluate_expression_scoped(lambda.body, payload, &locals)?
    };
    for (index, item) in items.iter().enumerate().skip(1) {
        let locals = lambda_locals(outer_locals, item, index, &lambda.parameters);
        let key = evaluate_expression_scoped(lambda.body, payload, &locals)?;
        let ordering = compare_sort_keys(&key, &selected_key);
        if (max && ordering.is_gt()) || (!max && ordering.is_lt()) {
            selected_key = key;
            selected_item = item.clone();
        }
    }
    Ok(selected_item)
}

struct ParsedLambda<'a> {
    parameters: Vec<LambdaParameter<'a>>,
    body: &'a str,
}

struct LambdaParameter<'a> {
    name: &'a str,
    default: Option<&'a str>,
}

fn parse_lambda(source: &str) -> ParsedLambda<'_> {
    let source = strip_wrapping_parens(source.trim());
    if let Some((params_source, body)) = split_top_level_arrow(source) {
        let params_source = strip_wrapping_parens(params_source.trim());
        let parameters = split_top_level(params_source, ',')
            .into_iter()
            .filter_map(parse_lambda_parameter)
            .collect::<Vec<_>>();
        return ParsedLambda {
            parameters,
            body: body.trim(),
        };
    }
    ParsedLambda {
        parameters: Vec::new(),
        body: source,
    }
}

fn lambda_locals(
    outer: &Map<String, Value>,
    item: &Value,
    index: usize,
    parameters: &[LambdaParameter<'_>],
) -> Map<String, Value> {
    let mut locals = outer.clone();
    locals.insert("$".to_string(), item.clone());
    locals.insert("$$".to_string(), Value::Number((index as i64).into()));
    if let Some(first) = parameters.first() {
        locals.insert(first.name.to_string(), item.clone());
    }
    if let Some(second) = parameters.get(1) {
        locals.insert(
            second.name.to_string(),
            Value::Number((index as i64).into()),
        );
    }
    locals
}

fn object_lambda_locals(
    outer: &Map<String, Value>,
    value: &Value,
    key: &str,
    index: usize,
    parameters: &[LambdaParameter<'_>],
) -> Map<String, Value> {
    let mut locals = outer.clone();
    locals.insert("$".to_string(), value.clone());
    locals.insert("$$".to_string(), Value::String(key.to_string()));
    locals.insert("$$$".to_string(), Value::Number((index as i64).into()));
    if let Some(first) = parameters.first() {
        locals.insert(first.name.to_string(), value.clone());
    }
    if let Some(second) = parameters.get(1) {
        locals.insert(second.name.to_string(), Value::String(key.to_string()));
    }
    if let Some(third) = parameters.get(2) {
        locals.insert(third.name.to_string(), Value::Number((index as i64).into()));
    }
    locals
}

fn reduce_locals(
    outer: &Map<String, Value>,
    item: &Value,
    accumulator: &Value,
    parameters: &[LambdaParameter<'_>],
) -> Map<String, Value> {
    let mut locals = outer.clone();
    locals.insert("$".to_string(), item.clone());
    locals.insert("$$".to_string(), accumulator.clone());
    if let Some(first) = parameters.first() {
        locals.insert(first.name.to_string(), item.clone());
    }
    if let Some(second) = parameters.get(1) {
        locals.insert(second.name.to_string(), accumulator.clone());
    }
    locals
}

pub(crate) fn evaluate_reduce(
    input: &Value,
    reducer: &str,
    payload: &Value,
    outer_locals: &Map<String, Value>,
) -> Result<Value, DwError> {
    let lambda = parse_lambda(reducer);
    let accumulator_default = lambda
        .parameters
        .get(1)
        .and_then(|parameter| parameter.default)
        .map(|default| evaluate_expression_scoped(default, payload, outer_locals))
        .transpose()?;
    let items = match input {
        Value::Array(items) => items.clone(),
        Value::String(text) => text
            .chars()
            .map(|ch| Value::String(ch.to_string()))
            .collect::<Vec<_>>(),
        _ => return Ok(Value::Null),
    };

    if items.is_empty() {
        return Ok(accumulator_default.unwrap_or(Value::Null));
    }

    let mut start_index = 0usize;
    let mut accumulator = if let Some(default) = accumulator_default {
        default
    } else {
        start_index = 1;
        items[0].clone()
    };

    for item in items.iter().skip(start_index) {
        let locals = reduce_locals(outer_locals, item, &accumulator, &lambda.parameters);
        accumulator = evaluate_expression_scoped(lambda.body, payload, &locals)?;
    }
    Ok(accumulator)
}

pub(crate) fn evaluate_then(
    input: &Value,
    mapper: &str,
    payload: &Value,
    outer_locals: &Map<String, Value>,
) -> Result<Value, DwError> {
    if input.is_null() {
        return Ok(Value::Null);
    }
    evaluate_then_non_null(input, mapper, payload, outer_locals)
}

pub(crate) fn evaluate_on_null(
    input: &Value,
    mapper: &str,
    payload: &Value,
    outer_locals: &Map<String, Value>,
) -> Result<Value, DwError> {
    evaluate_then_non_null(input, mapper, payload, outer_locals)
}

fn evaluate_then_non_null(
    input: &Value,
    mapper: &str,
    payload: &Value,
    outer_locals: &Map<String, Value>,
) -> Result<Value, DwError> {
    let lambda = parse_lambda(mapper);
    evaluate_lambda(
        &lambda,
        FastLambda::parse(&lambda).as_ref(),
        outer_locals,
        input,
        0,
        payload,
    )
}

fn evaluate_lambda(
    lambda: &ParsedLambda<'_>,
    fast_lambda: Option<&FastLambda<'_>>,
    outer_locals: &Map<String, Value>,
    item: &Value,
    index: usize,
    payload: &Value,
) -> Result<Value, DwError> {
    if let Some(fast_lambda) = fast_lambda {
        return fast_lambda.evaluate(item, index);
    }
    let locals = lambda_locals(outer_locals, item, index, &lambda.parameters);
    evaluate_expression_scoped(lambda.body, payload, &locals)
}

fn evaluate_object_predicate(
    lambda: &ParsedLambda<'_>,
    fast_predicate: Option<&FastObjectPredicate>,
    outer_locals: &Map<String, Value>,
    value: &Value,
    key: &str,
    index: usize,
    payload: &Value,
) -> Result<bool, DwError> {
    if let Some(fast_predicate) = fast_predicate {
        return fast_predicate.evaluate(value);
    }
    let locals = object_lambda_locals(outer_locals, value, key, index, &lambda.parameters);
    Ok(is_truthy(&evaluate_expression_scoped(
        lambda.body,
        payload,
        &locals,
    )?))
}

fn parse_lambda_parameter(source: &str) -> Option<LambdaParameter<'_>> {
    let source = source.trim();
    if source.is_empty() {
        return None;
    }
    if let Some((name, default)) = split_top_level_char(source, '=') {
        return Some(LambdaParameter {
            name: name.trim(),
            default: Some(default.trim()),
        });
    }
    Some(LambdaParameter {
        name: source,
        default: None,
    })
}

enum FastLambda<'a> {
    Expr(FastExpr<'a>),
}

struct FastObjectPredicate {
    operator: &'static str,
    threshold: f64,
}

struct FastObjectMapper<'a> {
    predicate: Option<SimplePath<'a>>,
    key: FastObjectKey,
    value: FastExpr<'a>,
}

#[derive(Clone, Copy)]
enum FastObjectKey {
    Key,
    Index,
}

enum FastExpr<'a> {
    Path(SimplePath<'a>),
    Upper(SimplePath<'a>),
    UpperDefault(SimplePath<'a>, &'a str),
    SumMap {
        collection: SimplePath<'a>,
        mapper: SimplePath<'a>,
    },
    FilterMap {
        collection: SimplePath<'a>,
        predicate: FastComparison<'a>,
        mapper: SimplePath<'a>,
    },
    EqBool(SimplePath<'a>, bool),
    Object(Vec<FastObjectField<'a>>),
}

struct FastComparison<'a> {
    path: SimplePath<'a>,
    operator: &'static str,
    threshold: f64,
}

struct FastObjectField<'a> {
    key: &'a str,
    value: FastExpr<'a>,
}

#[derive(Clone, Copy)]
enum SimpleRoot {
    Item,
    Index,
}

enum PathPart<'a> {
    Field(&'a str),
    Index(usize),
}

struct SimplePath<'a> {
    root: SimpleRoot,
    parts: Vec<PathPart<'a>>,
}

impl<'a> FastLambda<'a> {
    fn parse(lambda: &ParsedLambda<'a>) -> Option<Self> {
        let item_alias = lambda
            .parameters
            .first()
            .map(|param| param.name)
            .unwrap_or("$");
        let index_alias = lambda
            .parameters
            .get(1)
            .map(|param| param.name)
            .unwrap_or("$$");
        parse_fast_expr(lambda.body, item_alias, index_alias).map(FastLambda::Expr)
    }

    fn evaluate(&self, item: &Value, index: usize) -> Result<Value, DwError> {
        match self {
            FastLambda::Expr(expr) => expr.evaluate(item, index),
        }
    }
}

impl FastObjectPredicate {
    fn parse<'a>(lambda: &ParsedLambda<'a>) -> Option<Self> {
        let first_parameter_name = lambda
            .parameters
            .first()
            .map(|param| param.name)
            .unwrap_or("$");
        let (left, operator, right) =
            split_top_level_operator(lambda.body, &["==", "!=", ">=", "<=", ">", "<"])?;
        if left != first_parameter_name && left != "$" {
            return None;
        }
        let threshold = right.parse::<f64>().ok()?;
        Some(Self {
            operator,
            threshold,
        })
    }

    fn evaluate(&self, value: &Value) -> Result<bool, DwError> {
        let value = numeric_value(value)?;
        Ok(match self.operator {
            "==" => value == self.threshold,
            "!=" => value != self.threshold,
            ">" => value > self.threshold,
            ">=" => value >= self.threshold,
            "<" => value < self.threshold,
            "<=" => value <= self.threshold,
            _ => unreachable!(),
        })
    }
}

impl<'a> FastObjectMapper<'a> {
    fn parse(lambda: &ParsedLambda<'a>) -> Option<Self> {
        let value_alias = lambda
            .parameters
            .first()
            .map(|param| param.name)
            .unwrap_or("$");
        let key_alias = lambda
            .parameters
            .get(1)
            .map(|param| param.name)
            .unwrap_or("$$");
        let index_alias = lambda
            .parameters
            .get(2)
            .map(|param| param.name)
            .unwrap_or("$$$");
        let body = strip_wrapping_parens(lambda.body.trim());
        if let Some((condition, object_source)) = parse_fast_if_object_body(body) {
            return Some(Self {
                predicate: Some(parse_simple_path(condition, value_alias, index_alias)?),
                key: parse_fast_object_key(object_source, key_alias, index_alias)?,
                value: parse_fast_object_value(object_source, value_alias, index_alias)?,
            });
        }
        Some(Self {
            predicate: None,
            key: parse_fast_object_key(body, key_alias, index_alias)?,
            value: parse_fast_object_value(body, value_alias, index_alias)?,
        })
    }

    fn evaluate(
        &self,
        value: &Value,
        key: &str,
        index: usize,
    ) -> Result<Option<(String, Value)>, DwError> {
        if self
            .predicate
            .as_ref()
            .is_some_and(|predicate| !is_truthy(&predicate.evaluate(value, index)))
        {
            return Ok(None);
        }
        let output_key = match self.key {
            FastObjectKey::Key => key.to_string(),
            FastObjectKey::Index => index.to_string(),
        };
        Ok(Some((output_key, self.value.evaluate(value, index)?)))
    }
}

impl<'a> FastExpr<'a> {
    fn evaluate(&self, item: &Value, index: usize) -> Result<Value, DwError> {
        match self {
            FastExpr::Path(path) => Ok(path.evaluate(item, index)),
            FastExpr::Upper(path) => Ok(Value::String(
                crate::as_dataweave_string(&path.evaluate(item, index)).to_uppercase(),
            )),
            FastExpr::UpperDefault(path, default) => {
                let value = path.evaluate(item, index);
                let value = if value.is_null() {
                    Value::String((*default).to_string())
                } else {
                    value
                };
                Ok(Value::String(
                    crate::as_dataweave_string(&value).to_uppercase(),
                ))
            }
            FastExpr::SumMap { collection, mapper } => {
                let collection = collection.evaluate(item, index);
                let Value::Array(items) = collection else {
                    return Ok(Value::Number(0.into()));
                };
                let mut total = 0.0;
                for (nested_index, nested_item) in items.iter().enumerate() {
                    total += numeric_value(&mapper.evaluate(nested_item, nested_index))?;
                }
                number_result(total)
            }
            FastExpr::FilterMap {
                collection,
                predicate,
                mapper,
            } => {
                let collection = collection.evaluate(item, index);
                let Value::Array(items) = collection else {
                    return Ok(Value::Array(Vec::new()));
                };
                let mut output = Vec::new();
                for (nested_index, nested_item) in items.iter().enumerate() {
                    if predicate.evaluate(nested_item, nested_index)? {
                        output.push(mapper.evaluate(nested_item, nested_index));
                    }
                }
                Ok(Value::Array(output))
            }
            FastExpr::EqBool(path, expected) => Ok(Value::Bool(
                path.evaluate(item, index).as_bool() == Some(*expected),
            )),
            FastExpr::Object(fields) => {
                let mut output = Map::new();
                for field in fields {
                    output.insert(field.key.to_string(), field.value.evaluate(item, index)?);
                }
                Ok(Value::Object(output))
            }
        }
    }
}

impl<'a> FastComparison<'a> {
    fn evaluate(&self, item: &Value, index: usize) -> Result<bool, DwError> {
        let value = numeric_value(&self.path.evaluate(item, index))?;
        Ok(match self.operator {
            "==" => value == self.threshold,
            "!=" => value != self.threshold,
            ">" => value > self.threshold,
            ">=" => value >= self.threshold,
            "<" => value < self.threshold,
            "<=" => value <= self.threshold,
            _ => unreachable!(),
        })
    }
}

impl<'a> SimplePath<'a> {
    fn evaluate(&self, item: &Value, index: usize) -> Value {
        match self.root {
            SimpleRoot::Index => Value::Number((index as i64).into()),
            SimpleRoot::Item => {
                let mut current = item;
                for part in &self.parts {
                    match part {
                        PathPart::Field(field) => {
                            let Some(next) = current.get(*field) else {
                                return Value::Null;
                            };
                            current = next;
                        }
                        PathPart::Index(index) => {
                            let Some(next) = current.get(*index) else {
                                return Value::Null;
                            };
                            current = next;
                        }
                    }
                }
                current.clone()
            }
        }
    }
}

fn parse_fast_if_object_body<'a>(source: &'a str) -> Option<(&'a str, &'a str)> {
    let source = source.strip_prefix("if")?.trim_start();
    let condition_source = source.strip_prefix('(')?;
    let condition_end = condition_source.find(')')?;
    let condition = condition_source[..condition_end].trim();
    let rest = condition_source[condition_end + 1..].trim_start();
    let object_end = find_matching_object_end(rest)?;
    if rest[object_end + 1..].trim() != "else {}" {
        return None;
    }
    Some((condition, &rest[..=object_end]))
}

fn find_matching_object_end(source: &str) -> Option<usize> {
    if !source.starts_with('{') {
        return None;
    }
    let mut depth = 0i64;
    for (index, ch) in source.char_indices() {
        match ch {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(index);
                }
            }
            _ => {}
        }
    }
    None
}

fn parse_fast_object_key(
    source: &str,
    key_alias: &str,
    index_alias: &str,
) -> Option<FastObjectKey> {
    let (key_source, _) = parse_single_fast_object_entry(source)?;
    let key_source = strip_wrapping_parens(key_source.trim());
    if key_source == key_alias || key_source == "$$" {
        return Some(FastObjectKey::Key);
    }
    if key_source == index_alias || key_source == "$$$" {
        return Some(FastObjectKey::Index);
    }
    None
}

fn parse_fast_object_value<'a>(
    source: &'a str,
    value_alias: &'a str,
    index_alias: &'a str,
) -> Option<FastExpr<'a>> {
    let (_, value_source) = parse_single_fast_object_entry(source)?;
    parse_fast_expr(value_source.trim(), value_alias, index_alias)
}

fn parse_single_fast_object_entry(source: &str) -> Option<(&str, &str)> {
    let source = source.trim();
    if !(source.starts_with('{') && source.ends_with('}')) {
        return None;
    }
    let inner = source[1..source.len() - 1].trim();
    let entries = split_top_level(inner, ',');
    if entries.len() != 1 {
        return None;
    }
    let (key, value) = split_top_level_char(entries[0].trim(), ':')?;
    Some((key, value))
}

fn parse_fast_expr<'a>(
    source: &'a str,
    item_alias: &'a str,
    index_alias: &'a str,
) -> Option<FastExpr<'a>> {
    let source = strip_wrapping_parens(source.trim());
    if let Some(path) = parse_simple_path(source, item_alias, index_alias) {
        return Some(FastExpr::Path(path));
    }
    if let Some(inner) = source
        .strip_prefix("upper(")
        .and_then(|inner| inner.strip_suffix(')'))
    {
        if let Some((path_source, default_source)) =
            split_top_level_keyword(inner.trim(), "default")
        {
            let default = parse_fast_string_literal(default_source.trim())?;
            return parse_simple_path(path_source.trim(), item_alias, index_alias)
                .map(|path| FastExpr::UpperDefault(path, default));
        }
        return parse_simple_path(inner.trim(), item_alias, index_alias).map(FastExpr::Upper);
    }
    if let Some(sum_map) = parse_fast_sum_map(source, item_alias, index_alias) {
        return Some(sum_map);
    }
    if let Some(filter_map) = parse_fast_filter_map(source, item_alias, index_alias) {
        return Some(filter_map);
    }
    if let Some((left, right)) = source.split_once("==") {
        let expected = match right.trim() {
            "true" => true,
            "false" => false,
            _ => return None,
        };
        return parse_simple_path(left.trim(), item_alias, index_alias)
            .map(|path| FastExpr::EqBool(path, expected));
    }
    parse_fast_object(source, item_alias, index_alias)
}

fn parse_fast_sum_map<'a>(
    source: &'a str,
    item_alias: &'a str,
    index_alias: &'a str,
) -> Option<FastExpr<'a>> {
    let inner = source
        .strip_prefix("sum(")
        .and_then(|inner| inner.strip_suffix(')'))?;
    let (collection_source, _, mapper_source) =
        split_top_level_keyword_or_call_operator(inner.trim(), &["map"])?;
    let mapper_lambda = parse_lambda(mapper_source);
    let mapper_item_alias = mapper_lambda
        .parameters
        .first()
        .map(|parameter| parameter.name)
        .unwrap_or("$");
    Some(FastExpr::SumMap {
        collection: parse_simple_path(collection_source, item_alias, index_alias)?,
        mapper: parse_simple_path(mapper_lambda.body, mapper_item_alias, "$$")?,
    })
}

fn parse_fast_filter_map<'a>(
    source: &'a str,
    item_alias: &'a str,
    index_alias: &'a str,
) -> Option<FastExpr<'a>> {
    let source = strip_wrapping_parens(source);
    let (filter_source, _, mapper_source) =
        split_top_level_keyword_or_call_operator(source, &["map"])?;
    let filter_source = strip_wrapping_parens(filter_source);
    let (collection_source, _, predicate_source) =
        split_top_level_keyword_or_call_operator(filter_source, &["filter"])?;
    let predicate_lambda = parse_lambda(predicate_source);
    let mapper_lambda = parse_lambda(mapper_source);
    let predicate_alias = predicate_lambda
        .parameters
        .first()
        .map(|parameter| parameter.name)
        .unwrap_or("$");
    let mapper_alias = mapper_lambda
        .parameters
        .first()
        .map(|parameter| parameter.name)
        .unwrap_or("$");
    Some(FastExpr::FilterMap {
        collection: parse_simple_path(collection_source, item_alias, index_alias)?,
        predicate: parse_fast_comparison(predicate_lambda.body, predicate_alias, "$$")?,
        mapper: parse_simple_path(mapper_lambda.body, mapper_alias, "$$")?,
    })
}

fn parse_fast_comparison<'a>(
    source: &'a str,
    item_alias: &'a str,
    index_alias: &'a str,
) -> Option<FastComparison<'a>> {
    let (left, operator, right) =
        split_top_level_operator(source, &["==", "!=", ">=", "<=", ">", "<"])?;
    Some(FastComparison {
        path: parse_simple_path(left.trim(), item_alias, index_alias)?,
        operator,
        threshold: right.trim().parse().ok()?,
    })
}

fn parse_fast_string_literal(source: &str) -> Option<&str> {
    source
        .strip_prefix('"')
        .and_then(|inner| inner.strip_suffix('"'))
        .or_else(|| {
            source
                .strip_prefix('\'')
                .and_then(|inner| inner.strip_suffix('\''))
        })
}

fn parse_fast_object<'a>(
    source: &'a str,
    item_alias: &'a str,
    index_alias: &'a str,
) -> Option<FastExpr<'a>> {
    if !(source.starts_with('{') && source.ends_with('}')) {
        return None;
    }
    let inner = &source[1..source.len() - 1];
    let mut fields = Vec::new();
    for entry in split_top_level(inner, ',') {
        let entry = entry.trim();
        if entry.is_empty() {
            continue;
        }
        let (key, value) = split_top_level_char(entry, ':')?;
        let key = parse_static_object_key(key.trim())?;
        let value = parse_fast_expr(value.trim(), item_alias, index_alias)?;
        fields.push(FastObjectField { key, value });
    }
    Some(FastExpr::Object(fields))
}

fn parse_static_object_key(source: &str) -> Option<&str> {
    if source.is_empty() {
        return None;
    }
    if source.starts_with('(') && source.ends_with(')') {
        return None;
    }
    if let Some(inner) = source
        .strip_prefix('"')
        .and_then(|inner| inner.strip_suffix('"'))
    {
        if matches!(inner, "$" | "$$" | "$$$") {
            return None;
        }
        return Some(inner);
    }
    if let Some(inner) = source
        .strip_prefix('\'')
        .and_then(|inner| inner.strip_suffix('\''))
    {
        if matches!(inner, "$" | "$$" | "$$$") {
            return None;
        }
        return Some(inner);
    }
    if matches!(source, "$" | "$$" | "$$$") {
        return None;
    }
    Some(source)
}

fn parse_simple_path<'a>(
    source: &'a str,
    item_alias: &'a str,
    index_alias: &'a str,
) -> Option<SimplePath<'a>> {
    if source == index_alias || source == "$$" {
        return Some(SimplePath {
            root: SimpleRoot::Index,
            parts: Vec::new(),
        });
    }
    let tail = if source == item_alias || source == "$" {
        ""
    } else {
        source
            .strip_prefix(item_alias)
            .or_else(|| source.strip_prefix('$'))?
    };
    let mut rest = tail;
    let mut parts = Vec::new();
    while !rest.is_empty() {
        if let Some(after_dot) = rest.strip_prefix('.') {
            let field_len = after_dot
                .char_indices()
                .find_map(|(index, ch)| {
                    if !is_simple_path_field_char(ch) {
                        Some(index)
                    } else {
                        None
                    }
                })
                .unwrap_or(after_dot.len());
            if field_len == 0 {
                return None;
            }
            let field = &after_dot[..field_len];
            if matches!(field, "@" | "#") {
                return None;
            }
            parts.push(PathPart::Field(field));
            rest = &after_dot[field_len..];
        } else if let Some(after_bracket) = rest.strip_prefix('[') {
            let close = after_bracket.find(']')?;
            let index = after_bracket[..close].parse::<usize>().ok()?;
            parts.push(PathPart::Index(index));
            rest = &after_bracket[close + 1..];
        } else {
            return None;
        }
    }
    Some(SimplePath {
        root: SimpleRoot::Item,
        parts,
    })
}

fn is_simple_path_field_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-' | '@')
}
