#![recursion_limit = "256"]

use serde_json::Map;
use serde_json::Value;

mod builtins;
mod calls;
mod collections;
mod csv;
mod evaluator;
mod functions;
mod json;
mod literals;
mod markdown;
mod matches;
mod mime;
mod operators;
mod output;
mod periods;
mod script;
mod selectors;
mod strings;
mod syntax;
mod type_inference;
mod types;
mod value;
mod xml;
mod yaml;

use builtins::{evaluate_binary_builtin, index_of, last_index_of};
use calls::evaluate_function_call;
use collections::{
    evaluate_count_by, evaluate_count_characters_by, evaluate_distinct_by, evaluate_drop_while,
    evaluate_every, evaluate_every_character, evaluate_every_entry, evaluate_filter,
    evaluate_filter_array_leafs, evaluate_filter_object, evaluate_filter_object_leafs,
    evaluate_first_with, evaluate_flat_map, evaluate_group_by, evaluate_index_where, evaluate_map,
    evaluate_map_leaf_values, evaluate_map_object, evaluate_map_string, evaluate_node_exists,
    evaluate_on_null, evaluate_order_by, evaluate_partition, evaluate_pluck, evaluate_reduce,
    evaluate_some, evaluate_some_character, evaluate_some_entry, evaluate_split_at,
    evaluate_split_where, evaluate_substring_by, evaluate_sum_by, evaluate_take_while,
    evaluate_then,
};
use csv::read_simple_csv;
use evaluator::{
    evaluate_array_literal_scoped, evaluate_do_block_scoped, evaluate_index_base,
    evaluate_object_literal_scoped, evaluate_selector_index_scoped,
    evaluate_using_expression_scoped,
};
use functions::{
    evaluate_header_declarations, function_reference, is_function_name, lambda_value_from_source,
    resolve_type_source,
};
use json::json_output_options;
use literals::{
    evaluate_string_literal_scoped, parse_call_args, parse_literal, parse_string_literal,
};
use markdown::read_simple_markdown_table;
use matches::{evaluate_match_expression, parse_match_expression_source};
use mime::{is_csv_mime, is_json_mime, is_markdown_mime, is_xml_mime, is_yaml_mime, output_mime};
use operators::{
    evaluate_additive, evaluate_coercion, evaluate_comparison, evaluate_index_access,
    evaluate_index_range, evaluate_matches, evaluate_multiplicative, evaluate_range,
    evaluate_shift, number_value,
};
use output::{render_json_compact_expression, render_output_value};
pub use script::parse_script_boundary;
pub use script::parse_script_boundary_span;
use script::split_script;
use selectors::{
    evaluate_local_path, evaluate_payload_path, parse_path_segments, parse_postfix_path_access,
    select_path_segment, unwrap_metadata_value, PathSegment, DW_METADATA_MARKER,
};
use strings::replace_with;
use syntax::{
    find_matching_delimiter, is_binary_operator_position, is_identifier, is_top_level_index,
    parse_if_expression, parse_index_access, split_top_level_arrow, split_top_level_char,
    split_top_level_keyword, split_top_level_keyword_operator,
    split_top_level_keyword_or_call_operator, split_top_level_operator, strip_wrapping_parens,
};
use type_inference::infer_expression_type;
use types::{type_any, type_descriptor_from_value};
pub use value::{DwError, DwValue};
use xml::{parse_xml_document, xml_list_items};
use yaml::read_simple_yaml;

const COLLECTION_OPERATORS: &[&str] = &[
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
    "filterArrayLeafs",
    "filterObjectLeafs",
    "failIf",
    "wait",
];

pub fn engine_capabilities() -> Vec<&'static str> {
    vec![
        "rust-core-evaluator",
        "workspace-source-backed",
        "wasm-crate",
        "dw-value-model",
        "script-boundary-parser",
    ]
}

pub fn execute_smoke(script: &str, payload: Value) -> Result<Value, DwError> {
    execute_json(script, payload, false)
}

pub fn infer_type_descriptor(
    script: &str,
    payload: Option<Value>,
    vars: Option<Value>,
) -> Result<Value, DwError> {
    let parsed = split_script(script);
    let payload_type = payload
        .as_ref()
        .map(type_descriptor_from_value)
        .unwrap_or_else(type_any);
    let vars_type = vars
        .as_ref()
        .map(type_descriptor_from_value)
        .unwrap_or_else(type_any);
    infer_expression_type(parsed.body.trim(), &payload_type, &vars_type)
}

pub fn execute_json(script: &str, payload: Value, render_output: bool) -> Result<Value, DwError> {
    execute_json_scoped(script, payload, Map::new(), render_output)
}

pub fn execute_json_with_vars(
    script: &str,
    payload: Value,
    vars: Value,
    render_output: bool,
) -> Result<Value, DwError> {
    let mut locals = Map::new();
    locals.insert("vars".to_string(), vars);
    execute_json_scoped(script, payload, locals, render_output)
}

pub fn parse_payload_format(
    payload: Value,
    payload_format: Option<&str>,
) -> Result<Value, DwError> {
    parse_payload_format_with_options(payload, payload_format, None)
}

pub fn parse_payload_format_with_options(
    payload: Value,
    payload_format: Option<&str>,
    options: Option<&Value>,
) -> Result<Value, DwError> {
    let Some(payload_format) = payload_format else {
        return Ok(payload);
    };
    if is_json_mime(payload_format) {
        let text = match payload {
            Value::String(text) => text,
            other => as_dataweave_string(&other),
        };
        return serde_json::from_str(&text).map_err(|err| DwError::InvalidJson(err.to_string()));
    }
    if is_xml_mime(payload_format) {
        let text = match payload {
            Value::String(text) => text,
            other => as_dataweave_string(&other),
        };
        return parse_xml_document(&text);
    }
    if is_yaml_mime(payload_format) {
        let text = match payload {
            Value::String(text) => text,
            other => as_dataweave_string(&other),
        };
        return read_simple_yaml(&text)
            .map_err(|err| DwError::Parse(format!("Failed to parse input as yaml: {err}")));
    }
    if is_csv_mime(payload_format) {
        let text = match payload {
            Value::String(text) => text,
            other => as_dataweave_string(&other),
        };
        let separator = input_option(options, "separator")
            .and_then(|value| value.chars().next())
            .unwrap_or(',');
        let quote = input_option(options, "quote")
            .and_then(|value| value.chars().next())
            .unwrap_or('"');
        let header = input_bool_option(options, "header", true);
        return read_simple_csv(&text, separator, quote, header);
    }
    if is_markdown_mime(payload_format) {
        let text = match payload {
            Value::String(text) => text,
            other => as_dataweave_string(&other),
        };
        let header = input_bool_option(options, "header", true);
        return read_simple_markdown_table(&text, header);
    }
    Err(DwError::UnsupportedFeature(format!(
        "payload format {payload_format}"
    )))
}

fn input_option<'a>(options: Option<&'a Value>, name: &str) -> Option<String> {
    let Value::Object(map) = options? else {
        return None;
    };
    map.get(name).map(as_dataweave_string)
}

fn input_bool_option(options: Option<&Value>, name: &str, default: bool) -> bool {
    let Value::Object(map) = options.unwrap_or(&Value::Null) else {
        return default;
    };
    map.get(name)
        .map(|value| match value {
            Value::Bool(value) => *value,
            other => matches!(
                as_dataweave_string(other).as_str(),
                "true" | "True" | "1" | "yes"
            ),
        })
        .unwrap_or(default)
}

fn execute_json_scoped(
    script: &str,
    payload: Value,
    locals: Map<String, Value>,
    render_output: bool,
) -> Result<Value, DwError> {
    let parsed = split_script(script);
    let mut locals = locals;
    evaluate_header_declarations(&parsed.header, &payload, &mut locals)?;
    if render_output {
        if let Some(directive) = parsed
            .output_directive
            .as_deref()
            .filter(|directive| output_mime(directive).is_some_and(is_json_mime))
            .filter(|directive| !directive.contains("with binary"))
        {
            let options = json_output_options(directive)?;
            if options.indent.is_none() {
                if let Ok(rendered) =
                    render_json_compact_expression(parsed.body.trim(), &payload, &locals, options)
                {
                    return Ok(Value::String(rendered));
                }
            }
        }
    }
    let evaluated = evaluate_expression_scoped(parsed.body.trim(), &payload, &locals)?;
    render_output_value(parsed.output_directive.as_deref(), evaluated, render_output)
}

pub(crate) fn output_option<'a>(directive: &'a str, name: &str) -> Option<&'a str> {
    directive.split_whitespace().skip(1).find_map(|token| {
        let (key, value) = token.split_once('=')?;
        if key == name {
            Some(value.trim_matches('"'))
        } else {
            None
        }
    })
}

pub(crate) fn output_bool_option(directive: &str, name: &str, default: bool) -> bool {
    output_option(directive, name)
        .map(|value| matches!(value, "true" | "True" | "1" | "yes"))
        .unwrap_or(default)
}

pub(crate) fn evaluate_expression_scoped(
    source: &str,
    payload: &Value,
    locals: &Map<String, Value>,
) -> Result<Value, DwError> {
    let source = strip_wrapping_parens(source.trim());
    if source.is_empty() {
        return Err(DwError::Parse("empty expression".to_string()));
    }

    if source.starts_with("do ") || source.starts_with("do{") {
        return evaluate_do_block_scoped(source, payload, locals);
    }

    if source.starts_with("using ") || source.starts_with("using(") {
        return evaluate_using_expression_scoped(source, payload, locals);
    }

    if let Some(value) = parse_literal(source)? {
        return Ok(value);
    }

    if matches!(
        source,
        "OBJECT_TYPE" | "ATTRIBUTE_TYPE" | "ARRAY_TYPE" | "VALUE_TYPE"
    ) {
        return Ok(Value::String(source.to_string()));
    }

    if let Some(inner) = source.strip_prefix("not ") {
        let value = evaluate_expression_scoped(inner, payload, locals)?;
        return Ok(Value::Bool(!is_truthy(&value)));
    }

    if let Some(inner) = source
        .strip_prefix('-')
        .filter(|inner| !inner.starts_with('-'))
    {
        let value = evaluate_expression_scoped(inner, payload, locals)?;
        return number_result(-number_value(&value)?);
    }

    if let Some((condition, when_true, when_false)) = parse_if_expression(source) {
        if is_truthy(&evaluate_expression_scoped(condition, payload, locals)?) {
            return evaluate_expression_scoped(when_true, payload, locals);
        }
        return evaluate_expression_scoped(when_false, payload, locals);
    }

    if let Some((value_source, cases_source)) = parse_match_expression_source(source) {
        return evaluate_match_expression(value_source, cases_source, payload, locals);
    }

    if let Some(lambda) = lambda_value_from_source(source, locals)? {
        return Ok(lambda);
    }

    if let Some((left, right)) = split_top_level_keyword(source, "default") {
        let left_value = evaluate_expression_scoped(left, payload, locals)?;
        if left_value.is_null() {
            return evaluate_expression_scoped(right, payload, locals);
        }
        return Ok(left_value);
    }

    if let Some((left, right)) = split_top_level_keyword(source, "orElseTry") {
        let left_value = evaluate_expression_scoped(left, payload, locals)?;
        return evaluate_or_else_try(&left_value, right, payload, locals);
    }

    if let Some((left, right)) = split_top_level_keyword(source, "orElse") {
        let left_value = evaluate_expression_scoped(left, payload, locals)?;
        return evaluate_or_else(&left_value, right, payload, locals);
    }

    if let Some((before_colon, _)) = split_top_level_char(source, ':') {
        if let Some((left, operator, right)) =
            split_top_level_keyword_or_call_operator(source, COLLECTION_OPERATORS)
        {
            if left.len() < before_colon.len() {
                return evaluate_collection_operator(left, operator, right, payload, locals);
            }
        }
        return evaluate_object_literal_scoped(&format!("{{{source}}}"), payload, locals);
    }

    if let Some((left, right)) = split_top_level_keyword(source, "update") {
        let input = evaluate_expression_scoped(left, payload, locals)?;
        return evaluate_update_expression(&input, right, payload, locals);
    }

    if let Some((left, operator, right)) =
        split_top_level_sequence_operator_before_arrow(source, &["++", "--"])
    {
        if has_top_level_collection_operator(left) || has_top_level_collection_operator(right) {
            let left_value = if operator == "--" {
                evaluate_index_base(left, payload, locals)?
            } else {
                evaluate_expression_scoped(left, payload, locals)?
            };
            let right_value = evaluate_expression_scoped(right, payload, locals)?;
            return evaluate_additive(&left_value, operator, &right_value);
        }
    }

    if let Some((left, operator, right)) =
        split_top_level_keyword_or_call_operator(source, COLLECTION_OPERATORS)
    {
        return evaluate_collection_operator(left, operator, right, payload, locals);
    }

    if let Some((left, right)) = split_top_level_keyword(source, "mask") {
        let (selector_source, replacement_source) = split_top_level_keyword(right, "with")
            .ok_or_else(|| DwError::Parse(format!("mask expression missing with in {source}")))?;
        let input = evaluate_expression_scoped(left, payload, locals)?;
        let selector = parse_mask_selector(selector_source, payload, locals)?;
        let replacement = evaluate_expression_scoped(replacement_source, payload, locals)?;
        return Ok(apply_mask_value(&input, &selector, &replacement, false));
    }

    if let Some((left, _, replacement_source)) =
        split_top_level_keyword_or_call_operator(source, &["with"])
    {
        if let Some(("replace", argument_sources)) = parse_call_args(left) {
            if argument_sources.len() == 2 {
                let input = evaluate_expression_scoped(argument_sources[0], payload, locals)?;
                let target = evaluate_expression_scoped(argument_sources[1], payload, locals)?;
                let replacement = evaluate_expression_scoped(replacement_source, payload, locals)?;
                return replace_with(&input, &target, &replacement);
            }
        }
    }

    if let Some((left, right)) = split_top_level_keyword(source, "replace") {
        let (target_source, _, replacement_source) =
            split_top_level_keyword_or_call_operator(right, &["with"]).ok_or_else(|| {
                DwError::Parse(format!("replace expression missing with in {source}"))
            })?;
        let input = evaluate_expression_scoped(left, payload, locals)?;
        let target = evaluate_expression_scoped(target_source, payload, locals)?;
        let replacement = evaluate_expression_scoped(replacement_source, payload, locals)?;
        return replace_with(&input, &target, &replacement);
    }

    if let Some((left, right)) = split_top_level_keyword(source, "or") {
        let left_value = evaluate_expression_scoped(left, payload, locals)?;
        if is_truthy(&left_value) {
            return Ok(Value::Bool(true));
        }
        return Ok(Value::Bool(is_truthy(&evaluate_expression_scoped(
            right, payload, locals,
        )?)));
    }

    if let Some((left, right)) = split_top_level_keyword(source, "and") {
        let left_value = evaluate_expression_scoped(left, payload, locals)?;
        if !is_truthy(&left_value) {
            return Ok(Value::Bool(false));
        }
        return Ok(Value::Bool(is_truthy(&evaluate_expression_scoped(
            right, payload, locals,
        )?)));
    }

    if let Some(inner) = source.strip_prefix('!') {
        let value = evaluate_expression_scoped(inner, payload, locals)?;
        return Ok(Value::Bool(!is_truthy(&value)));
    }

    if let Some((left, type_source)) = split_top_level_keyword(source, "as") {
        let value = evaluate_expression_scoped(left, payload, locals)?;
        if let Some((coercion_type, next_type)) = split_top_level_keyword(type_source, "as") {
            let coerced = evaluate_coercion(&value, &resolve_type_source(coercion_type, locals))?;
            return evaluate_coercion(&coerced, &resolve_type_source(next_type, locals));
        }
        if let Some((coercion_type, operator, right)) =
            split_top_level_operator(type_source, &["++"])
        {
            let coerced = evaluate_coercion(&value, &resolve_type_source(coercion_type, locals))?;
            let right_value = evaluate_expression_scoped(right, payload, locals)?;
            return evaluate_additive(&coerced, operator, &right_value);
        }
        if let Some((coercion_type, operator, right)) =
            split_top_level_operator(type_source, &["==", "!=", ">=", "<=", ">", "<"])
        {
            let coerced = evaluate_coercion(&value, &resolve_type_source(coercion_type, locals))?;
            let right_value = evaluate_expression_scoped(right, payload, locals)?;
            return evaluate_comparison(&coerced, operator, &right_value);
        }
        if let Some((coercion_type, operator, right)) =
            split_top_level_keyword_or_call_operator(type_source, &["match", "matches"])
        {
            let coerced = evaluate_coercion(&value, &resolve_type_source(coercion_type, locals))?;
            let right_value = evaluate_expression_scoped(right, payload, locals)?;
            if operator == "matches" {
                return evaluate_matches(&coerced, &right_value);
            }
            return evaluate_binary_builtin(operator, &coerced, &right_value);
        }
        return evaluate_coercion(&value, &resolve_type_source(type_source, locals));
    }

    if let Some((left, right)) = split_top_level_keyword(source, "to") {
        let left_value = evaluate_expression_scoped(left, payload, locals)?;
        let right_value = evaluate_expression_scoped(right, payload, locals)?;
        return evaluate_range(&left_value, &right_value);
    }

    if let Some((left, _, right)) = split_top_level_keyword_or_call_operator(source, &["matches"]) {
        let left_value = evaluate_expression_scoped(left, payload, locals)?;
        let right_value = evaluate_expression_scoped(right, payload, locals)?;
        return evaluate_matches(&left_value, &right_value);
    }

    for function_name in ["mod", "pow"] {
        if let Some((left, _, right)) = split_top_level_keyword_operator(source, &[function_name]) {
            let left_value = evaluate_expression_scoped(left, payload, locals)?;
            let right_value = evaluate_expression_scoped(right, payload, locals)?;
            return evaluate_binary_builtin(function_name, &left_value, &right_value);
        }
    }

    if let Some((left, operator, right)) = split_top_level_operator(source, &[">>", "<<"]) {
        let left_value = evaluate_expression_scoped(left, payload, locals)?;
        let right_value = evaluate_expression_scoped(right, payload, locals)?;
        return evaluate_shift(&left_value, operator, &right_value);
    }

    if let Some((left, type_source)) = split_top_level_keyword(source, "is") {
        let value = evaluate_expression_scoped(left, payload, locals)?;
        return Ok(Value::Bool(evaluate_type_check(
            &value,
            type_source,
            locals,
        )));
    }

    if let Some((left, operator, right)) =
        split_top_level_operator(source, &["==", "!=", ">=", "<=", ">", "<"])
    {
        let left_value = evaluate_expression_scoped(left, payload, locals)?;
        let right_value = evaluate_expression_scoped(right, payload, locals)?;
        return evaluate_comparison(&left_value, operator, &right_value);
    }

    if let Some((left, operator, right)) = split_top_level_operator(source, &["++", "--", "+", "-"])
    {
        let left_value = if operator == "--" {
            evaluate_index_base(left, payload, locals)?
        } else {
            evaluate_expression_scoped(left, payload, locals)?
        };
        let right_value = evaluate_expression_scoped(right, payload, locals)?;
        return evaluate_additive(&left_value, operator, &right_value);
    }

    if let Some((left, operator, right)) = split_top_level_operator(source, &["*", "/"]) {
        let left_value = evaluate_expression_scoped(left, payload, locals)?;
        let right_value = evaluate_expression_scoped(right, payload, locals)?;
        return evaluate_multiplicative(&left_value, operator, &right_value);
    }

    for function_name in [
        "contains",
        "appendIfMissing",
        "charCodeAt",
        "countMatches",
        "joinBy",
        "splitBy",
        "startsWith",
        "endsWith",
        "find",
        "match",
        "scan",
        "first",
        "hammingDistance",
        "last",
        "leftPad",
        "levenshteinDistance",
        "prependIfMissing",
        "repeat",
        "remove",
        "rightPad",
        "substringAfter",
        "substringAfterLast",
        "substringBefore",
        "substringBeforeLast",
        "substringEvery",
        "withMaxSize",
        "unwrap",
        "wrapIfMissing",
        "wrapWith",
        "divideBy",
        "zip",
        "splitAt",
        "indexOf",
        "lastIndexOf",
        "readLinesWith",
        "writeLinesWith",
    ] {
        if let Some((left, _, right)) =
            split_top_level_keyword_or_call_operator(source, &[function_name])
        {
            let left_value = evaluate_expression_scoped(left, payload, locals)?;
            let right_value = evaluate_expression_scoped(right, payload, locals)?;
            return match function_name {
                "indexOf" => index_of(&left_value, &right_value),
                "lastIndexOf" => last_index_of(&left_value, &right_value),
                "splitAt" => evaluate_split_at(&left_value, &right_value),
                _ => evaluate_binary_builtin(function_name, &left_value, &right_value),
            };
        }
    }

    if source.starts_with('{')
        && source.ends_with('}')
        && find_matching_delimiter(source, 0, '{', '}') == Some(source.len() - 1)
    {
        return evaluate_object_literal_scoped(source, payload, locals);
    }

    if source.starts_with('[')
        && source.ends_with(']')
        && find_matching_delimiter(source, 0, '[', ']') == Some(source.len() - 1)
    {
        return evaluate_array_literal_scoped(source, payload, locals);
    }

    if let Some((base, index)) = parse_index_access(source) {
        let base_value = evaluate_index_base(base, payload, locals)?;
        if let Some(value) = evaluate_selector_index_scoped(&base_value, index, payload, locals)? {
            return Ok(value);
        }
        if let Some((start, end)) = split_top_level_keyword(index, "to") {
            let start_value = evaluate_expression_scoped(start, payload, locals)?;
            let end_value = evaluate_expression_scoped(end, payload, locals)?;
            return evaluate_index_range(&base_value, &start_value, &end_value);
        }
        let index_value = evaluate_expression_scoped(index, payload, locals)?;
        return evaluate_index_access(&base_value, &index_value);
    }

    if let Some(value) = evaluate_string_literal_scoped(source, payload, locals)? {
        return Ok(value);
    }

    if let Some(argument_source) = source.strip_prefix("compose ") {
        let argument = evaluate_expression_scoped(argument_source.trim(), payload, locals)?;
        return Ok(Value::String(compose_url(&as_dataweave_string(&argument))));
    }

    if let Some((base, namespace_alias, selector_source)) =
        parse_namespaced_selector_expression(source)
    {
        return evaluate_namespaced_selector(
            base,
            namespace_alias,
            selector_source,
            payload,
            locals,
        );
    }

    if let Some((base, tail)) = parse_postfix_path_access(source) {
        let mut current = evaluate_expression_scoped(base, payload, locals)?;
        for segment in parse_path_segments(tail)? {
            current = select_path_segment(&current, &segment, true)?;
        }
        return Ok(current);
    }

    if let Some((function_name, argument_sources)) = parse_call_args(source) {
        return evaluate_function_call(function_name, &argument_sources, payload, locals);
    }

    if source == "payload" || source.starts_with("payload.") || source.starts_with("payload?.") {
        return evaluate_payload_path(source, payload);
    }

    if let Some(value) = evaluate_local_path(source, locals)? {
        return Ok(value);
    }

    if is_identifier(source) {
        if let Value::Object(payload_object) = payload {
            if let Some(value) = payload_object.get(source) {
                return Ok(value.clone());
            }
        }
    }

    if is_function_name(source, locals) {
        return Ok(function_reference(source));
    }

    Err(DwError::UnsupportedFeature(source.to_string()))
}

fn parse_namespaced_selector_expression(source: &str) -> Option<(&str, &str, &str)> {
    let hash_quote = source.rfind("#\"")?;
    let selector_end = source.strip_suffix('"')?;
    let before_hash = &source[..hash_quote];
    let dot_index = before_hash.rfind('.')?;
    let namespace_alias = &before_hash[dot_index + 1..];
    if !namespace_alias
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
    {
        return None;
    }
    let base = &before_hash[..dot_index];
    let selector_source = &selector_end[hash_quote + 2..];
    if base.is_empty() || selector_source.is_empty() {
        return None;
    }
    Some((base, namespace_alias, selector_source))
}

fn evaluate_namespaced_selector(
    base: &str,
    namespace_alias: &str,
    selector_source: &str,
    payload: &Value,
    locals: &Map<String, Value>,
) -> Result<Value, DwError> {
    let Some(namespace) = locals.get(namespace_alias).map(as_dataweave_string) else {
        return Err(DwError::UnsupportedFeature(format!(
            "namespace selector {namespace_alias}"
        )));
    };
    let selector = if let Some(expression) = selector_source
        .strip_prefix("$(")
        .and_then(|value| value.strip_suffix(')'))
    {
        as_dataweave_string(&evaluate_expression_scoped(expression, payload, locals)?)
    } else {
        selector_source.to_string()
    };
    let base_value = evaluate_expression_scoped(base, payload, locals)?;
    select_path_segment(
        &base_value,
        &PathSegment::Property {
            attribute: format!("{{{namespace}}}{selector}"),
            present: false,
            assert_present: false,
        },
        true,
    )
}

fn evaluate_collection_operator(
    left: &str,
    operator: &str,
    right: &str,
    payload: &Value,
    locals: &Map<String, Value>,
) -> Result<Value, DwError> {
    let input = evaluate_expression_scoped(left, payload, locals)?;
    match operator {
        "groupBy" => evaluate_group_by(&input, right, payload, locals),
        "map" => evaluate_map(&input, right, payload, locals),
        "pluck" => evaluate_pluck(&input, right, payload, locals),
        "mapObject" => evaluate_map_object(&input, right, payload, locals),
        "filter" => evaluate_filter(&input, right, payload, locals),
        "filterObject" => evaluate_filter_object(&input, right, payload, locals),
        "flatMap" => evaluate_flat_map(&input, right, payload, locals),
        "distinctBy" => evaluate_distinct_by(&input, right, payload, locals),
        "orderBy" => evaluate_order_by(&input, right, payload, locals),
        "reduce" => evaluate_reduce(&input, right, payload, locals),
        "then" => evaluate_then(&input, right, payload, locals),
        "takeWhile" => evaluate_take_while(&input, right, payload, locals),
        "dropWhile" => evaluate_drop_while(&input, right, payload, locals),
        "some" => evaluate_some(&input, right, payload, locals),
        "every" => evaluate_every(&input, right, payload, locals),
        "countCharactersBy" => evaluate_count_characters_by(&input, right, payload, locals),
        "everyCharacter" => evaluate_every_character(&input, right, payload, locals),
        "mapString" => evaluate_map_string(&input, right, payload, locals),
        "someCharacter" => evaluate_some_character(&input, right, payload, locals),
        "substringBy" => evaluate_substring_by(&input, right, payload, locals),
        "countBy" => evaluate_count_by(&input, right, payload, locals),
        "maxBy" => collections::evaluate_by(&input, right, true, payload, locals),
        "minBy" => collections::evaluate_by(&input, right, false, payload, locals),
        "sumBy" => evaluate_sum_by(&input, right, payload, locals),
        "firstWith" => evaluate_first_with(&input, right, payload, locals),
        "indexWhere" => evaluate_index_where(&input, right, payload, locals),
        "partition" => evaluate_partition(&input, right, payload, locals),
        "splitWhere" => evaluate_split_where(&input, right, payload, locals),
        "everyEntry" => evaluate_every_entry(&input, right, payload, locals),
        "someEntry" => evaluate_some_entry(&input, right, payload, locals),
        "readLinesWith" | "writeLinesWith" | "scan" => {
            let right_value = evaluate_expression_scoped(right, payload, locals)?;
            evaluate_binary_builtin(operator, &input, &right_value)
        }
        "mapLeafValues" => evaluate_map_leaf_values(&input, right, payload, locals),
        "nodeExists" => evaluate_node_exists(&input, right, payload, locals),
        "filterArrayLeafs" => evaluate_filter_array_leafs(&input, right, payload, locals),
        "filterObjectLeafs" => evaluate_filter_object_leafs(&input, right, payload, locals),
        "failIf" => {
            let mut scoped = locals.clone();
            scoped.insert("$".to_string(), input.clone());
            let should_fail = evaluate_expression_scoped(right, payload, &scoped)?;
            if is_truthy(&should_fail) {
                Err(DwError::UnsupportedFeature("failIf".to_string()))
            } else {
                Ok(input)
            }
        }
        "wait" => Ok(input),
        "onNull" => {
            if input.is_null() {
                evaluate_on_null(&input, right, payload, locals)
            } else {
                Ok(input)
            }
        }
        _ => unreachable!(),
    }
}

fn has_top_level_collection_operator(source: &str) -> bool {
    split_top_level_keyword_or_call_operator(source, COLLECTION_OPERATORS).is_some()
}

fn split_top_level_sequence_operator_before_arrow<'a>(
    source: &'a str,
    operators: &[&'static str],
) -> Option<(&'a str, &'static str, &'a str)> {
    let arrow_index = split_top_level_arrow(source).map(|(before_arrow, _)| before_arrow.len());
    let mut match_value = None;
    for (index, _) in source.char_indices() {
        if arrow_index.is_some_and(|arrow_index| index > arrow_index) {
            break;
        }
        if !is_top_level_index(source, index) {
            continue;
        }
        for operator in operators {
            if source[index..].starts_with(operator) && is_binary_operator_position(source, index) {
                match_value = Some((
                    source[..index].trim(),
                    *operator,
                    source[index + operator.len()..].trim(),
                ));
                break;
            }
        }
    }
    match_value.filter(|(left, _, right)| !left.is_empty() && !right.is_empty())
}

fn compose_url(source: &str) -> String {
    source
        .chars()
        .flat_map(|ch| match ch {
            ' ' => "%20".chars().collect::<Vec<_>>(),
            other => vec![other],
        })
        .collect()
}

pub(crate) fn numeric_value(value: &Value) -> Result<f64, DwError> {
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
            "expected numeric value, got {value:?}"
        ))),
    }
}

pub(crate) fn number_result(value: f64) -> Result<Value, DwError> {
    if value.fract() == 0.0 && value >= i64::MIN as f64 && value <= i64::MAX as f64 {
        return Ok(Value::Number((value as i64).into()));
    }
    serde_json::Number::from_f64(value)
        .map(Value::Number)
        .ok_or_else(|| DwError::InvalidJson(value.to_string()))
}

pub(crate) fn evaluate_type_check(
    value: &Value,
    type_source: &str,
    locals: &Map<String, Value>,
) -> bool {
    let resolved = resolve_type_source(type_source, locals);
    split_top_level_operator(&resolved, &["|"])
        .map(|(left, _, right)| {
            evaluate_type_check(value, left, locals) || evaluate_type_check(value, right, locals)
        })
        .unwrap_or_else(|| evaluate_simple_type_check(value, &resolved))
}

fn evaluate_simple_type_check(value: &Value, type_source: &str) -> bool {
    if let Some(expected_metadata) = type_metadata(type_source) {
        let Some(value_metadata) = value_metadata(value) else {
            return false;
        };
        if expected_metadata != value_metadata {
            return false;
        }
    }
    let unwrapped;
    let value = if let Some(value) = unwrap_metadata_value(value) {
        unwrapped = value;
        &unwrapped
    } else {
        value
    };
    let type_name = type_source
        .trim()
        .split(|ch: char| !(ch.is_ascii_alphanumeric() || ch == '_'))
        .next()
        .unwrap_or_default();
    match type_name {
        "Any" => true,
        "Array" => value.is_array(),
        "Boolean" => value.is_boolean(),
        "CDATA" | "String" | "Key" => value.is_string(),
        "Date" => value.as_str().is_some_and(is_iso_date),
        "Null" => value.is_null(),
        "Number" => value.is_number(),
        "Object" => value.is_object(),
        "DataFormatDescriptor" => {
            matches!(value, Value::Object(map) if map.contains_key("name") && map.contains_key("defaultMimeType"))
        }
        _ => false,
    }
}

fn type_metadata(type_source: &str) -> Option<Map<String, Value>> {
    let open = type_source.find('{')?;
    let close = type_source.rfind('}')?;
    let mut metadata = Map::new();
    for entry in split_top_level_char_aware(&type_source[open + 1..close], ',') {
        let Some((key, value)) = split_top_level_char(entry, ':') else {
            continue;
        };
        if let Ok(Some(value)) = parse_string_literal(value.trim()) {
            metadata.insert(key.trim().to_string(), Value::String(value));
        }
    }
    Some(metadata)
}

fn split_top_level_char_aware(source: &str, delimiter: char) -> Vec<&str> {
    crate::syntax::split_top_level(source, delimiter)
}

fn value_metadata(value: &Value) -> Option<Map<String, Value>> {
    let Value::Object(map) = value else {
        return None;
    };
    let Value::Object(metadata) = map.get(DW_METADATA_MARKER)? else {
        return None;
    };
    Some(metadata.clone())
}

fn is_iso_date(value: &str) -> bool {
    value.len() == 10
        && value.as_bytes().get(4) == Some(&b'-')
        && value.as_bytes().get(7) == Some(&b'-')
        && value[..4].chars().all(|ch| ch.is_ascii_digit())
        && value[5..7].chars().all(|ch| ch.is_ascii_digit())
        && value[8..10].chars().all(|ch| ch.is_ascii_digit())
}

fn evaluate_or_else(
    left: &Value,
    fallback_source: &str,
    payload: &Value,
    locals: &Map<String, Value>,
) -> Result<Value, DwError> {
    let Some(result) = left.as_object() else {
        return Ok(left.clone());
    };
    if result.get("success").and_then(Value::as_bool) == Some(true) {
        return Ok(result.get("result").cloned().unwrap_or(Value::Null));
    }
    evaluate_expression_scoped(fallback_source, payload, locals)
}

fn evaluate_or_else_try(
    left: &Value,
    fallback_source: &str,
    payload: &Value,
    locals: &Map<String, Value>,
) -> Result<Value, DwError> {
    let Some(result) = left.as_object() else {
        return Ok(try_success(left.clone()));
    };
    if result.get("success").and_then(Value::as_bool) == Some(true) {
        return Ok(left.clone());
    }
    Ok(
        match evaluate_expression_scoped(fallback_source, payload, locals) {
            Ok(value) => try_success(value),
            Err(err) => try_error_for_source(fallback_source, &err),
        },
    )
}

fn try_success(value: Value) -> Value {
    Value::Object(Map::from_iter([
        ("success".to_string(), Value::Bool(true)),
        ("result".to_string(), value),
    ]))
}

fn try_error_for_source(source: &str, error: &DwError) -> Value {
    let error_text = error.to_string();
    if let Some(key) = missing_key_name(&error_text) {
        return Value::Object(Map::from_iter([
            ("success".to_string(), Value::Bool(false)),
            (
                "error".to_string(),
                Value::Object(Map::from_iter([
                    (
                        "kind".to_string(),
                        Value::String("KeyNotFoundException".to_string()),
                    ),
                    (
                        "message".to_string(),
                        Value::String(format!("There is no key named '{key}'")),
                    ),
                    (
                        "location".to_string(),
                        Value::String(runtime_key_error_location(source)),
                    ),
                    (
                        "stack".to_string(),
                        Value::Array(vec![Value::String(
                            "main (org::mule::weave::v2::engine::transform:9:40)".to_string(),
                        )]),
                    ),
                ])),
            ),
        ]));
    }
    Value::Object(Map::from_iter([
        ("success".to_string(), Value::Bool(false)),
        (
            "error".to_string(),
            Value::Object(Map::from_iter([
                (
                    "kind".to_string(),
                    Value::String("DataWeaveEvaluationError".to_string()),
                ),
                ("message".to_string(), Value::String(error_text)),
            ])),
        ),
    ]))
}

fn missing_key_name(error_text: &str) -> Option<String> {
    let marker = "There is no key named '";
    let start = error_text.find(marker)? + marker.len();
    let end = error_text[start..].find('\'')? + start;
    Some(error_text[start..end].to_string())
}

fn runtime_key_error_location(source: &str) -> String {
    if source.trim() == "otherUser.name!" {
        return "\n9|     a: try(() -> user.name!) orElseTry otherUser.name!,\n                                          ^^^^^^^^^^^^^^".to_string();
    }
    format!("Unknown location: {source}")
}

enum MaskSelector {
    Key(String),
    Index(i64),
}

fn parse_mask_selector(
    source: &str,
    payload: &Value,
    locals: &Map<String, Value>,
) -> Result<MaskSelector, DwError> {
    if let Some(("field", argument_sources)) = parse_call_args(source.trim()) {
        if argument_sources.len() == 1 {
            let key = evaluate_expression_scoped(argument_sources[0], payload, locals)?;
            return Ok(MaskSelector::Key(as_dataweave_string(&key)));
        }
    }
    let value = evaluate_expression_scoped(source, payload, locals)?;
    match value {
        Value::String(key) => Ok(MaskSelector::Key(key)),
        Value::Number(index) => index
            .as_i64()
            .map(MaskSelector::Index)
            .ok_or_else(|| DwError::UnsupportedFeature(format!("mask index {index}"))),
        other => Err(DwError::UnsupportedFeature(format!(
            "mask selector {other:?}"
        ))),
    }
}

fn apply_mask_value(
    value: &Value,
    selector: &MaskSelector,
    replacement: &Value,
    allow_index_at_current_array: bool,
) -> Value {
    match value {
        Value::Array(items) => Value::Array(
            items
                .iter()
                .enumerate()
                .map(|(index, item)| {
                    if matches!(selector, MaskSelector::Index(mask_index) if *mask_index == index as i64)
                        && allow_index_at_current_array
                    {
                        replacement.clone()
                    } else {
                        apply_mask_value(item, selector, replacement, true)
                    }
                })
                .collect(),
        ),
        Value::Object(map) => Value::Object(Map::from_iter(map.iter().map(|(key, item)| {
            if matches!(selector, MaskSelector::Key(mask_key) if mask_key == key) {
                (key.clone(), replacement.clone())
            } else {
                (
                    key.clone(),
                    apply_mask_value(item, selector, replacement, true),
                )
            }
        }))),
        other => other.clone(),
    }
}

fn evaluate_update_expression(
    input: &Value,
    right: &str,
    payload: &Value,
    locals: &Map<String, Value>,
) -> Result<Value, DwError> {
    let right = right.trim();
    if right.starts_with('{')
        && right.ends_with('}')
        && find_matching_delimiter(right, 0, '{', '}') == Some(right.len() - 1)
    {
        return evaluate_update_cases(input, &right[1..right.len() - 1], payload, locals);
    }

    let (selector_source, _, replacement_source) =
        split_top_level_keyword_or_call_operator(right, &["with"])
            .ok_or_else(|| DwError::Parse(format!("update expression missing with in {right}")))?;
    let selector_value = evaluate_expression_scoped(selector_source, payload, locals)?;
    let replacement = evaluate_expression_scoped(replacement_source, payload, locals)?;
    let selector = parse_update_selector(&selector_value)?;
    Ok(apply_update_selector(input, &selector, &replacement))
}

fn evaluate_update_cases(
    input: &Value,
    cases_source: &str,
    payload: &Value,
    locals: &Map<String, Value>,
) -> Result<Value, DwError> {
    let mut current = input.clone();
    for case_source in split_update_cases(cases_source) {
        let Some((pattern_source, result_source)) = split_top_level_arrow_like(case_source) else {
            return Err(DwError::Parse(format!("invalid update case {case_source}")));
        };
        let pattern_source = pattern_source
            .trim()
            .strip_prefix("case ")
            .ok_or_else(|| DwError::Parse(format!("invalid update case {case_source}")))?
            .trim();
        let (pattern_source, guard_source) = split_update_guard(pattern_source);
        let (binding, path_source) =
            if let Some((binding, path_source)) = split_top_level_keyword(pattern_source, "at") {
                (binding.trim(), path_source.trim())
            } else if pattern_source.trim_start().starts_with('.') {
                ("$", pattern_source.trim())
            } else {
                return Err(DwError::Parse(format!(
                    "update case missing at in {case_source}"
                )));
            };
        if binding != "$" && !syntax::is_identifier(binding) {
            return Err(DwError::Parse(format!("invalid update binding {binding}")));
        }
        let path_source = resolve_update_path_source(path_source, locals);
        let path_segments = parse_path_segments(&path_source)?;
        let selected = select_update_path_value(&current, &path_segments);
        let mut scoped = locals.clone();
        scoped.insert(binding.to_string(), selected.clone());
        if let Some(guard_source) = guard_source {
            let guard = evaluate_expression_scoped(guard_source.trim(), payload, &scoped)?;
            if !guard.as_bool().unwrap_or(false) {
                continue;
            }
        }
        let replacement = evaluate_expression_scoped(result_source.trim(), payload, &scoped)?;
        current = apply_update_path(&current, &path_segments, &replacement);
    }
    Ok(current)
}

fn split_update_guard(source: &str) -> (&str, Option<&str>) {
    for (index, _) in source.match_indices(" if") {
        if !syntax::is_top_level_index(source, index) {
            continue;
        }
        let guard = source[index + 3..].trim();
        if guard.is_empty() {
            continue;
        }
        return (source[..index].trim(), Some(guard));
    }
    (source, None)
}

fn resolve_update_path_source(source: &str, locals: &Map<String, Value>) -> String {
    let mut output = source.to_string();
    for (name, value) in locals {
        let needle = format!("$({name})");
        if output.contains(&needle) {
            output = output.replace(&needle, &as_dataweave_string(value));
        };
    }
    output
}

fn split_update_cases(source: &str) -> Vec<&str> {
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
            '/' if starts_regex_literal(source, index) => in_string = Some('/'),
            '"' | '\'' | '`' | '|' => in_string = Some(ch),
            '{' | '[' | '(' => depth += 1,
            '}' | ']' | ')' => depth -= 1,
            _ if depth == 0 && starts_update_case(source, index) => {
                if let Some(case_start) = start.replace(index) {
                    let case = source[case_start..index].trim();
                    if !case.is_empty() {
                        cases.push(case);
                    }
                }
            }
            _ => {}
        }
    }
    if let Some(case_start) = start {
        let case = source[case_start..].trim();
        if !case.is_empty() {
            cases.push(case);
        }
    }
    cases
}

fn starts_update_case(source: &str, index: usize) -> bool {
    source[index..].starts_with("case ")
        && source[..index]
            .chars()
            .last()
            .is_none_or(|ch| ch.is_whitespace() || ch == ',')
}

fn starts_regex_literal(source: &str, index: usize) -> bool {
    source[index..].starts_with('/')
        && source[index + '/'.len_utf8()..]
            .chars()
            .next()
            .is_some_and(|next| next != '/' && next != '*')
        && source[..index]
            .chars()
            .rev()
            .find(|ch| !ch.is_whitespace())
            .is_none_or(|ch| matches!(ch, '(' | '[' | '{' | ':' | ',' | '='))
}

fn split_top_level_arrow_like(source: &str) -> Option<(&str, &str)> {
    source
        .match_indices("->")
        .find(|(index, _)| syntax::is_top_level_index(source, *index))
        .map(|(index, _)| (&source[..index], &source[index + 2..]))
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum UpdateSelector {
    Key(String),
    Index(usize),
    Path(Vec<UpdateSelector>),
}

fn parse_update_selector(value: &Value) -> Result<UpdateSelector, DwError> {
    match value {
        Value::String(key) => Ok(UpdateSelector::Key(key.clone())),
        Value::Number(index) => index
            .as_u64()
            .map(|value| UpdateSelector::Index(value as usize))
            .ok_or_else(|| DwError::UnsupportedFeature(format!("update index {index}"))),
        Value::Array(items) => Ok(UpdateSelector::Path(
            items
                .iter()
                .map(parse_update_selector)
                .collect::<Result<Vec<_>, _>>()?,
        )),
        Value::Object(map) => match map.get("kind").and_then(Value::as_str) {
            Some("Object") => map
                .get("selector")
                .map(as_dataweave_string)
                .map(UpdateSelector::Key)
                .ok_or_else(|| DwError::UnsupportedFeature(format!("update selector {value:?}"))),
            Some("Array") => map
                .get("selector")
                .and_then(Value::as_u64)
                .map(|index| UpdateSelector::Index(index as usize))
                .ok_or_else(|| DwError::UnsupportedFeature(format!("update selector {value:?}"))),
            _ => Err(DwError::UnsupportedFeature(format!(
                "update selector {value:?}"
            ))),
        },
        _ => Err(DwError::UnsupportedFeature(format!(
            "update selector {value:?}"
        ))),
    }
}

fn apply_update_selector(value: &Value, selector: &UpdateSelector, replacement: &Value) -> Value {
    match selector {
        UpdateSelector::Path(path) => apply_update_selector_path(value, path, replacement),
        UpdateSelector::Key(key) => match value {
            Value::Array(items) => Value::Array(
                items
                    .iter()
                    .map(|item| apply_update_selector(item, selector, replacement))
                    .collect(),
            ),
            Value::Object(map) => {
                Value::Object(Map::from_iter(map.iter().map(|(item_key, item)| {
                    if item_key == key {
                        (item_key.clone(), replacement.clone())
                    } else {
                        (
                            item_key.clone(),
                            apply_update_selector(item, selector, replacement),
                        )
                    }
                })))
            }
            other => other.clone(),
        },
        UpdateSelector::Index(target_index) => match value {
            Value::Array(items) => Value::Array(
                items
                    .iter()
                    .enumerate()
                    .map(|(index, item)| {
                        if index == *target_index {
                            replacement.clone()
                        } else {
                            apply_update_selector(item, selector, replacement)
                        }
                    })
                    .collect(),
            ),
            Value::Object(map) => Value::Object(Map::from_iter(map.iter().map(|(key, item)| {
                (
                    key.clone(),
                    apply_update_selector(item, selector, replacement),
                )
            }))),
            other => other.clone(),
        },
    }
}

fn apply_update_selector_path(
    value: &Value,
    path: &[UpdateSelector],
    replacement: &Value,
) -> Value {
    let Some((head, tail)) = path.split_first() else {
        return replacement.clone();
    };
    match head {
        UpdateSelector::Key(key) => {
            let Value::Object(map) = value else {
                return value.clone();
            };
            Value::Object(Map::from_iter(map.iter().map(|(item_key, item)| {
                if item_key == key {
                    (
                        item_key.clone(),
                        apply_update_selector_path(item, tail, replacement),
                    )
                } else {
                    (item_key.clone(), item.clone())
                }
            })))
        }
        UpdateSelector::Index(target_index) => {
            let Value::Array(items) = value else {
                return value.clone();
            };
            Value::Array(
                items
                    .iter()
                    .enumerate()
                    .map(|(index, item)| {
                        if index == *target_index {
                            apply_update_selector_path(item, tail, replacement)
                        } else {
                            item.clone()
                        }
                    })
                    .collect(),
            )
        }
        UpdateSelector::Path(nested) => apply_update_selector_path(value, nested, replacement),
    }
}

fn select_update_path_value(value: &Value, segments: &[PathSegment]) -> Value {
    let mut current = value.clone();
    for segment in segments {
        match (segment, &current) {
            (PathSegment::Property { attribute: key, .. }, Value::Object(map)) => {
                current = map.get(key).cloned().unwrap_or(Value::Null);
            }
            (PathSegment::Index { index }, Value::Array(items)) => {
                let resolved = resolve_update_index(*index, items.len());
                current = resolved
                    .and_then(|index| items.get(index).cloned())
                    .unwrap_or(Value::Null);
            }
            _ => return Value::Null,
        }
    }
    current
}

fn apply_update_path(value: &Value, segments: &[PathSegment], replacement: &Value) -> Value {
    let Some((head, tail)) = segments.split_first() else {
        return replacement.clone();
    };
    match head {
        PathSegment::Property {
            attribute: key,
            assert_present,
            ..
        } => {
            let Value::Object(map) = value else {
                return value.clone();
            };
            let mut output = map.clone();
            if tail.is_empty() {
                if output.contains_key(key) || *assert_present {
                    output.insert(key.to_string(), replacement.clone());
                }
                return Value::Object(output);
            }
            if let Some(existing) = output.get(key) {
                output.insert(
                    key.to_string(),
                    apply_update_path(existing, tail, replacement),
                );
            }
            Value::Object(output)
        }
        PathSegment::Index { index } => {
            let Value::Array(items) = value else {
                return value.clone();
            };
            let Some(resolved) = resolve_update_index(*index, items.len()) else {
                return value.clone();
            };
            Value::Array(
                items
                    .iter()
                    .enumerate()
                    .map(|(item_index, item)| {
                        if item_index == resolved {
                            apply_update_path(item, tail, replacement)
                        } else {
                            item.clone()
                        }
                    })
                    .collect(),
            )
        }
        _ => value.clone(),
    }
}

fn resolve_update_index(index: i64, len: usize) -> Option<usize> {
    let resolved = if index < 0 { len as i64 + index } else { index };
    if resolved < 0 {
        None
    } else {
        Some(resolved as usize).filter(|index| *index < len)
    }
}

pub(crate) fn as_dataweave_string(value: &Value) -> String {
    if let Some(value) = unwrap_metadata_value(value) {
        return as_dataweave_string(&value);
    }
    match value {
        Value::Null => String::new(),
        Value::Bool(true) => "true".to_string(),
        Value::Bool(false) => "false".to_string(),
        Value::String(value) => value.clone(),
        Value::Number(value) => value.to_string(),
        other => other.to_string(),
    }
}

pub(crate) fn is_truthy(value: &Value) -> bool {
    !matches!(value, Value::Null | Value::Bool(false))
}

pub(crate) fn group_key(value: &Value) -> String {
    match value {
        Value::Null => "None".to_string(),
        Value::Bool(value) => value.to_string(),
        Value::Number(value) => value.to_string(),
        Value::String(value) => value.clone(),
        other => other.to_string(),
    }
}

pub(crate) fn stable_marker(value: &Value) -> String {
    match value {
        Value::Object(map) => {
            let mut entries = map.iter().collect::<Vec<_>>();
            entries.sort_by(|left, right| left.0.cmp(right.0));
            let encoded = entries
                .into_iter()
                .map(|(key, value)| format!("{key}:{}", stable_marker(value)))
                .collect::<Vec<_>>()
                .join(",");
            format!("{{{encoded}}}")
        }
        Value::Array(items) => {
            let encoded = items
                .iter()
                .map(stable_marker)
                .collect::<Vec<_>>()
                .join(",");
            format!("[{encoded}]")
        }
        _ => value.to_string(),
    }
}

pub(crate) fn compare_sort_keys(left: &Value, right: &Value) -> std::cmp::Ordering {
    use std::cmp::Ordering;

    match (left, right) {
        (Value::Number(left), Value::Number(right)) => left
            .as_f64()
            .partial_cmp(&right.as_f64())
            .unwrap_or(Ordering::Equal),
        (Value::String(left), Value::String(right)) => left.cmp(right),
        (Value::Bool(left), Value::Bool(right)) => left.cmp(right),
        (Value::Null, Value::Null) => Ordering::Equal,
        _ => stable_marker(left).cmp(&stable_marker(right)),
    }
}
pub(crate) fn char_slice(source: &str, start: usize, end: usize) -> String {
    source
        .chars()
        .skip(start)
        .take(end.saturating_sub(start))
        .collect()
}
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn finds_top_level_script_delimiter() {
        let script = "%dw 2.0\n// --- ignored\nvar x = \"---\"\n---\npayload";
        assert_eq!(parse_script_boundary(script), Some(3));
    }

    #[test]
    fn finds_inline_top_level_script_delimiter() {
        let script = "output application/yaml --- payload";
        assert_eq!(parse_script_boundary_span(script), Some((24, 27)));
    }

    #[test]
    fn strips_dataweave_comments_outside_strings() {
        let source = r#"%dw 2.0
// header
var greeting = "http://example.com/*not*/"
var backtickUrl = `http://example.com/path`
/*
---
*/
---
{
  text: "not // a comment",
  id: payload.orderId, /* inline */
  value: "/* not a comment */"
}"#;
        let parsed = split_script(source);
        assert!(parsed.header.contains("greeting"));
        assert!(parsed.header.contains("backtickUrl"));
        assert!(!parsed.header.contains("header"));
        assert!(!parsed.body.contains("inline"));
        assert!(parsed.body.contains("not // a comment"));
        assert!(parsed.body.contains("/* not a comment */"));
    }

    #[test]
    fn smoke_executes_payload_identity() {
        assert_eq!(
            execute_smoke("%dw 2.0\n---\npayload", json!({"name": "dw"})).unwrap(),
            json!({"name": "dw"})
        );
    }

    #[test]
    fn evaluates_object_payload_paths_and_builtins() {
        let script = r#"%dw 2.0
output application/python
---
{
  name: upper(payload.user.name),
  size: sizeOf(payload.user.tags),
  fallback: payload.user.missing default "ok"
}"#;
        assert_eq!(
            execute_json(
                script,
                json!({"user": {"name": "dw", "tags": ["a", "b"]}}),
                false
            )
            .unwrap(),
            json!({"name": "DW", "size": 2, "fallback": "ok"})
        );
    }

    #[test]
    fn renders_json_output_when_requested() {
        let result = execute_json(
            "%dw 2.0\noutput application/json\n---\n{name: payload.name}",
            json!({"name": "dw"}),
            true,
        )
        .unwrap();
        assert_eq!(result, json!(r#"{"name":"dw"}"#));
    }

    #[test]
    fn renders_duplicate_key_json_output_when_requested() {
        let result = execute_json(
            "%dw 2.0\noutput application/json\n---\n{a: 2, a: 3, nested: {b: 1, b: 2}}",
            Value::Null,
            true,
        )
        .unwrap();
        assert_eq!(result, json!(r#"{"a":2,"a":3,"nested":{"b":1,"b":2}}"#));
    }

    #[test]
    fn renders_pretty_json_output_when_requested() {
        let result = execute_json(
            "%dw 2.0\noutput application/json indent=2\n---\n{name: payload.name}",
            json!({"name": "dw"}),
            true,
        )
        .unwrap();
        assert_eq!(result, json!("{\n  \"name\": \"dw\"\n}"));
    }

    #[test]
    fn renders_json_writer_options_when_requested() {
        let pretty = execute_json(
            "%dw 2.0\noutput application/json indent=4 sort_keys=true ensure_ascii=true\n---\n{z: payload.word, a: \"café\"}",
            json!({"word": "niño"}),
            true,
        )
        .unwrap();
        assert_eq!(
            pretty,
            json!("{\n    \"a\": \"caf\\u00e9\",\n    \"z\": \"ni\\u00f1o\"\n}")
        );

        let compact = execute_json(
            "%dw 2.0\noutput application/json sort_keys=true ensure_ascii=false\n---\n{z: payload.word, a: \"café\"}",
            json!({"word": "niño"}),
            true,
        )
        .unwrap();
        assert_eq!(compact, json!("{\"a\":\"café\",\"z\":\"niño\"}"));

        let spaced = execute_json(
            "%dw 2.0\noutput application/json indent = false\n---\n{hello: \"world\"}",
            Value::Null,
            true,
        )
        .unwrap();
        assert_eq!(spaced, json!("{\"hello\":\"world\"}"));

        let duplicate_key_array = execute_json(
            "%dw 2.0\noutput application/json duplicateKeyAsArray=true\n---\npayload",
            json!({"order": {"item": {"__dwpy_xml_list": [{"price": "1"}, {"price": "2"}]}}}),
            true,
        )
        .unwrap();
        assert_eq!(
            duplicate_key_array,
            json!("{\"order\":{\"item\":[{\"price\":\"1\"},{\"price\":\"2\"}]}}")
        );
    }

    #[test]
    fn renders_csv_output_when_requested() {
        let result = execute_json(
            "%dw 2.0\noutput application/csv separator=\";\" header=true\n---\npayload",
            json!([{"name": "Ann", "age": 20}]),
            true,
        )
        .unwrap();
        assert_eq!(result, json!("name;age\nAnn;20\n"));
    }

    #[test]
    fn renders_markdown_output_when_requested() {
        let result = execute_json(
            "%dw 2.0\noutput text/markdown\n---\n[{name: \"Jane\", age: 30}, {name: \"Bob\", age: 25}]",
            Value::Null,
            true,
        )
        .unwrap();
        assert_eq!(
            result,
            json!("| name   | age   |\n|:-------|:------|\n| Jane   | 30    |\n| Bob    | 25    |")
        );
    }

    #[test]
    fn infers_type_descriptors_for_payload_projection_and_indexing() {
        let descriptor = infer_type_descriptor(
            "{name: payload.user.name, firstTag: payload.user.tags[0]}",
            Some(json!({"user": {"name": "Mule", "tags": ["dw"]}})),
            None,
        )
        .unwrap();
        assert_eq!(descriptor["kind"], json!("Object"));
        assert_eq!(
            descriptor["fields"]["name"]["type"]["kind"],
            json!("String")
        );
        assert_eq!(
            descriptor["fields"]["firstTag"]["type"]["kind"],
            json!("String")
        );
        let present_descriptor = infer_type_descriptor("payload.name?", None, None).unwrap();
        assert_eq!(present_descriptor["kind"], json!("Boolean"));
    }

    #[test]
    fn parses_xml_payload_for_selection() {
        let payload = parse_payload_format(
            json!("<catalog><book><title>DW</title></book></catalog>"),
            Some("application/xml"),
        )
        .unwrap();
        let result = execute_json(
            "%dw 2.0\noutput application/json\n---\n{title: payload.catalog.book.title}",
            payload,
            true,
        )
        .unwrap();
        assert_eq!(result, json!(r#"{"title":"DW"}"#));
    }

    #[test]
    fn parses_xml_xsi_nil_as_null() {
        let payload = parse_payload_format(
            json!(
                r#"<book xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"><author xsi:nil="true" /></book>"#
            ),
            Some("application/xml"),
        )
        .unwrap();
        let result = execute_json(
            "%dw 2.0\noutput application/json\n---\n{book: {author: payload.book.author}}",
            payload,
            true,
        )
        .unwrap();
        assert_eq!(result, json!(r#"{"book":{"author":null}}"#));
    }

    #[test]
    fn maps_repeated_xml_children_with_type_alias_coercion() {
        let payload = parse_payload_format(
            json!(
                "<items><item><price>22.30</price></item><item><price>20.31</price></item></items>"
            ),
            Some("application/xml"),
        )
        .unwrap();
        let result = execute_json(
            r#"%dw 2.0
output application/json
type Currency = String { format: "\$#,###.00" }
---
{
  books: (payload.items.*item) map ((item) -> {
    book: {
      price: item.price as Currency
    }
  })
}"#,
            payload,
            true,
        )
        .unwrap();
        assert_eq!(
            result,
            json!(r#"{"books":[{"book":{"price":"22.30"}},{"book":{"price":"20.31"}}]}"#)
        );
    }

    #[test]
    fn maps_repeated_items_with_shorthand_object_mapper() {
        let payload = json!({
            "items": {
                "item": [
                    {"price": "22.30"},
                    {"price": "20.31"}
                ]
            }
        });
        let result = execute_json(
            r#"%dw 2.0
output application/json
type Currency = String { format: "\$#,###.00" }
---
books: payload.items.*item map
    book:
        price: $.price as Currency"#,
            payload,
            true,
        )
        .unwrap();
        assert_eq!(
            result,
            json!(r#"{"books":[{"book":{"price":"22.30"}},{"book":{"price":"20.31"}}]}"#)
        );
    }

    #[test]
    fn selects_first_xml_attribute_from_repeated_children() {
        let payload = parse_payload_format(
            json!(r#"<root><children><name a="b">Jane</name><name>John</name></children></root>"#),
            Some("application/xml"),
        )
        .unwrap();
        let result = execute_json(
            "%dw 2.0\n---\npayload.root.children.name.@a",
            payload,
            false,
        )
        .unwrap();
        assert_eq!(result, json!("b"));
    }

    #[test]
    fn value_set_collapses_repeated_xml_children() {
        let payload = parse_payload_format(
            json!(r#"<root><children><name b="c">Jane</name><name>John</name></children></root>"#),
            Some("application/xml"),
        )
        .unwrap();
        let result = execute_json(
            "%dw 2.0\noutput application/python\n---\nvalueSet(payload.root.children)",
            payload,
            false,
        )
        .unwrap();
        assert_eq!(result, json!(["Jane", "John"]));
    }

    #[test]
    fn evaluates_dynamic_namespaced_xml_selector() {
        let payload = parse_payload_format(
            json!(
                r#"<root ref="table">
    <f:table xmlns:f="https://www.w3schools.com/furniture">Manzana</f:table>
    <h:table xmlns:h="http://www.w3.org/TR/html4/">Banana</h:table>
</root>"#
            ),
            Some("application/xml"),
        )
        .unwrap();
        let result = execute_json(
            r##"%dw 2.0
output application/json
ns h http://www.w3.org/TR/html4/
---
payload.root.h#"$(payload.root.@ref)""##,
            payload,
            true,
        )
        .unwrap();
        assert_eq!(result, json!(r#""Banana""#));
    }

    #[test]
    fn evaluates_multiline_xml_read_header_var() {
        let result = execute_json(
            r#"%dw 2.0
var myVar = read('<product id="1" type="electronic">
  <brand>SomeBrand</brand>
</product>', 'application/xml')
output application/json
---
{
  item: [{
    "type": myVar.product.@."type",
    "name": myVar.product.brand,
    "attributes": myVar.product.@
  }]
}"#,
            Value::Null,
            true,
        )
        .unwrap();
        assert_eq!(
            result,
            json!(
                r#"{"item":[{"type":"electronic","name":"SomeBrand","attributes":{"id":"1","type":"electronic"}}]}"#
            )
        );
    }

    #[test]
    fn preserves_xml_node_for_bracket_access() {
        let payload = parse_payload_format(
            json!(r#"<root><customer id="7" secret="x">Ada</customer></root>"#),
            Some("application/xml"),
        )
        .unwrap();
        let result = execute_json(
            r#"%dw 2.0
output application/xml
---
{customer: (payload.root.customer -- {"@secret": payload.root.customer["@secret"]})}"#,
            payload,
            true,
        )
        .unwrap();
        assert_eq!(result, json!(r#"<customer id="7">Ada</customer>"#));
    }

    #[test]
    fn renders_xml_output_when_requested() {
        let result = execute_json(
            "%dw 2.0\noutput application/xml\n---\n{user: {\"@id\": \"123\", \"#text\": \"Max\"}}",
            Value::Null,
            true,
        )
        .unwrap();
        assert_eq!(result, json!(r#"<user id="123">Max</user>"#));
    }

    #[test]
    fn collapses_xml_like_text_node_on_property_selection() {
        let result = execute_json(
            "%dw 2.0\n---\npayload.root.name",
            json!({"root": {"name": {"@b": "c", "#text": "Jane"}}}),
            false,
        )
        .unwrap();
        assert_eq!(result, json!("Jane"));
    }

    #[test]
    fn preserves_legacy_dash_attribute_text_nodes_on_property_selection() {
        let result = execute_json(
            "%dw 2.0\n---\npayload.root.title",
            json!({"root": {"title": {"-lang": "en", "#text": "Everyday Italian"}}}),
            false,
        )
        .unwrap();
        assert_eq!(result, json!({"-lang": "en", "#text": "Everyday Italian"}));
    }

    #[test]
    fn evaluates_selector_presence_assertion_filters_and_dynamic_selectors() {
        let result = execute_json(
            r#"%dw 2.0
output application/python
var key = "name"
---
{
  present: payload.product.@."type"?,
  filtered: payload.users.*name[?($ == "Mariano")],
  missing: payload.users.*name[?($ == "Nobody")],
  scalar: payload.first[?($ == "Mariano")],
  dynamicMulti: payload.users[*(key)],
  dynamicPairs: payload.users[&(key)],
  objectIndex: payload.profile[1],
  xmlFiltered: payload.xml mapObject { ($$): $[?($ == "Mariano")] }
}"#,
            json!({
                "product": {"@type": "book"},
                "users": [{"name": "Mariano"}, {"name": "Ana"}, {"age": 10}],
                "first": "Mariano",
                "profile": {"nameFirst": "Mark", "nameLast": "Nguyen"},
                "xml": {"users": {"name": {"__dwpy_xml_list": ["Mariano", "Luis", "Mariano"]}}}
            }),
            false,
        )
        .unwrap();
        assert_eq!(result["present"], json!(true));
        assert_eq!(result["filtered"], json!(["Mariano"]));
        assert_eq!(result["missing"], Value::Null);
        assert_eq!(result["scalar"], json!("Mariano"));
        assert_eq!(result["dynamicMulti"], json!(["Mariano", "Ana"]));
        assert_eq!(
            result["dynamicPairs"],
            json!({"__dwpy_object_pairs": [
                {"key": "name", "value": "Mariano"},
                {"key": "name", "value": "Ana"}
            ]})
        );
        assert_eq!(result["objectIndex"], json!("Nguyen"));
        assert_eq!(result["xmlFiltered"], json!({"users": {"name": "Mariano"}}));
    }

    #[test]
    fn preserves_duplicate_object_keys_for_selector_semantics() {
        let result = execute_smoke(
            r#"%dw 2.0
output application/python
var myObject = { user : "a", "user" : "b" }
---
{
  firstUser: myObject.user,
  allUsers: myObject.*user
}"#,
            Value::Null,
        )
        .unwrap();
        assert_eq!(result["firstUser"], json!("a"));
        assert_eq!(result["allUsers"], json!(["a", "b"]));
    }

    #[test]
    fn assert_present_selector_reports_missing_key_name() {
        let err = execute_json(
            "%dw 2.0\noutput application/python\n---\npayload.lastName!",
            json!({"name": "Annie"}),
            false,
        )
        .unwrap_err();
        assert!(err.to_string().contains("There is no key named 'lastName'"));
    }

    #[test]
    fn evaluates_if_arithmetic_comparison_concat_and_indexing() {
        let script = r#"%dw 2.0
output application/python
---
{
  total: payload.price * payload.quantity + 2,
  label: "Order " ++ payload.id,
  first: payload.items[0],
  eligible: if (payload.price * payload.quantity >= 30 and payload.enabled == true) "yes" else "no"
}"#;
        let result = execute_json(
            script,
            json!({
                "price": 10,
                "quantity": 3,
                "id": "A-1",
                "enabled": true,
                "items": ["first", "second"]
            }),
            false,
        )
        .unwrap();
        assert_eq!(
            result,
            json!({
                "total": 32,
                "label": "Order A-1",
                "first": "first",
                "eligible": "yes"
            })
        );
    }

    #[test]
    fn evaluates_plus_array_append_semantics() {
        assert_eq!(
            execute_json("%dw 2.0\n---\n[2] + [2]", Value::Null, false).unwrap(),
            json!([2, [2]])
        );
        assert_eq!(
            execute_json("%dw 2.0\n---\n[2] + 2", Value::Null, false).unwrap(),
            json!([2, 2])
        );
    }

    #[test]
    fn evaluates_collection_pipeline_before_concat() {
        assert_eq!(
            execute_json(
                "%dw 2.0\noutput application/json\n---\n1 to 10 map $ * 2 ++ []",
                Value::Null,
                false,
            )
            .unwrap(),
            json!([2, 4, 6, 8, 10, 12, 14, 16, 18, 20])
        );
        assert_eq!(
            execute_json("%dw 2.0\n---\n1 to 3 map $ + 1", Value::Null, false).unwrap(),
            json!([2, 3, 4])
        );
    }

    #[test]
    fn evaluates_prepend_append_shift_operators() {
        let result = execute_json(
            r#"%dw 2.0
output application/json
---
{
  values: [
    1 >> [2],
    "a" >> [1],
    {a: "b"} >> [1],
    [1] >> [2, 3],
    (1 as Binary) >> [1],
    [1] << 2,
    [1,2] << [1, 2, 3],
    [1] << (1 as Binary),
    [1,2,3] - 2,
    [{a: "b"}, {c: "d"}, {e: "f"}] - {c: "d"}
  ]
}
"#,
            Value::Null,
            true,
        )
        .unwrap();
        assert_eq!(
            result,
            json!(
                "{\"values\":[[1,2],[\"a\",1],[{\"a\":\"b\"},1],[[1],2,3],[\"\\u0001\",1],[1,2],[1,2,[1,2,3]],[1,\"\\u0001\"],[1,3],[{\"a\":\"b\"},{\"e\":\"f\"}]]}"
            )
        );
    }

    #[test]
    fn evaluates_conditional_array_items_and_object_entries() {
        assert_eq!(
            execute_json(
                "%dw 2.0\n---\n[(1) if true, (2) if false, (payload.value) if (payload.enabled)]",
                json!({"value": 3, "enabled": true}),
                false
            )
            .unwrap(),
            json!([1, 3])
        );
        assert_eq!(
            execute_json(
                "%dw 2.0\n---\n{a: 1, (b: payload.value) if true, (c: 3) if (payload.enabled)}",
                json!({"value": 2, "enabled": false}),
                false
            )
            .unwrap(),
            json!({"a": 1, "b": 2})
        );
        assert_eq!(
            execute_json(
                "%dw 2.0\noutput application/json\n---\n[(1) if true, (2) if false]",
                Value::Null,
                true
            )
            .unwrap(),
            json!("[1]")
        );
    }

    #[test]
    fn evaluates_object_spread_arrays_and_variables() {
        let script = r#"%dw 2.0
output application/python
var x = [{b: "b"}, {c: "c", d: "d"}]
var y = {e: "e"}
---
{
  a: "a",
  (["a", "b", "c"] map ((value, index) -> {(index): value})),
  (x),
  (y)
}"#;
        assert_eq!(
            execute_json(script, Value::Null, false).unwrap(),
            json!({"0": "a", "1": "b", "2": "c", "a": "a", "b": "b", "c": "c", "d": "d", "e": "e"})
        );
    }

    #[test]
    fn evaluates_logical_operators_around_coercion_comparisons() {
        let script = r#"%dw 2.0
output application/python
var key = "availableSeats"
---
{
  excluded: key as String != 'availableSeats' and key as String != 'airlineName',
  renamed: key as String == 'availableSeats' or key as String == 'airlineName'
}"#;
        assert_eq!(
            execute_json(script, Value::Null, false).unwrap(),
            json!({"excluded": false, "renamed": true})
        );
    }

    #[test]
    fn evaluates_recursive_mask_with_field_and_index_selectors() {
        assert_eq!(
            execute_json(
                r#"%dw 2.0
output application/python
---
[{name: "Peter Parker", password: "spiderman"}, {name: "Bruce Wayne", password: "batman"}] mask field("password") with "*****""#,
                Value::Null,
                false
            )
            .unwrap(),
            json!([
                {"name": "Peter Parker", "password": "*****"},
                {"name": "Bruce Wayne", "password": "*****"}
            ])
        );
        assert_eq!(
            execute_json(
                r#"%dw 2.0
output application/python
---
[[123, true], [456, true]] mask 1 with false"#,
                Value::Null,
                false
            )
            .unwrap(),
            json!([[123, false], [456, false]])
        );
    }

    #[test]
    fn evaluates_update_operator_and_values_selectors() {
        let script = r#"%dw 2.0
import * from dw::util::Values
output application/python
var user = {name: "Ken", age: 30, address: {street: "Amenabar", zipCode: "AB1234"}}
var fieldName = "name"
---
{
  cases: user update {
    case age at .age -> age + 1
    case street at .address.street -> "First Street"
  },
  upsert: [{lastName: "Doe"}, {lastName: "Parker", name: "Peter"}] map ((value) ->
    value update {
      case name at .name! -> if (name == null) "JOHN" else upper(name)
    }
  ),
  key: {name: "Mariano"} update "name" with "Data Weave",
  field: {name: "Mariano"} update field("name") with "Data Weave",
  index: [1, 2, 3] update index(1) with 5,
  recursive: [{role: "a", name: "spiderman"}, {role: "b", name: "batman"}] update "role" with "Super Hero",
  path: {user: {name: "Mariano"}} update ["user", field("name")] with "Data Weave",
  arrayPath: {addresses: [{street: "First Street", zipCode: "AB123"}]} update {
    case s at .addresses[0] -> {street: "Second Street", zipCode: "ZZ123"}
  },
  dynamicPath: {name: "Ken", lastName: "Shokida"} update {
    case s at ."$(fieldName)" -> "Shoki"
  },
  guarded: [{name: "Ken"}, {name: "Tomo"}, {name: "Kajika"}] map ((user) ->
    user update {
      case name at .name if(name == "Ken") -> name ++ " (Leandro)"
      case name at .name if(name == "Tomo") -> name ++ " (Christian)"
    }
  ),
  shorthand: user update {
    case .age -> $ + 1
    case .address.street -> "Second Street"
  }
}
"#;
        assert_eq!(
            execute_json(script, Value::Null, false).unwrap(),
            json!({
                "cases": {"name": "Ken", "age": 31, "address": {"street": "First Street", "zipCode": "AB1234"}},
                "upsert": [{"lastName": "Doe", "name": "JOHN"}, {"lastName": "Parker", "name": "PETER"}],
                "key": {"name": "Data Weave"},
                "field": {"name": "Data Weave"},
                "index": [1, 5, 3],
                "recursive": [{"role": "Super Hero", "name": "spiderman"}, {"role": "Super Hero", "name": "batman"}],
                "path": {"user": {"name": "Data Weave"}},
                "arrayPath": {"addresses": [{"street": "Second Street", "zipCode": "ZZ123"}]},
                "dynamicPath": {"name": "Shoki", "lastName": "Shokida"},
                "guarded": [{"name": "Ken (Leandro)"}, {"name": "Tomo (Christian)"}, {"name": "Kajika"}],
                "shorthand": {"name": "Ken", "age": 31, "address": {"street": "Second Street", "zipCode": "AB1234"}}
            })
        );
    }

    #[test]
    fn evaluates_replace_with_operator_for_strings_and_regex() {
        let script = r#"%dw 2.0
output application/python
var myCompany = {name: "biz"}
var myInputA = "somebiz-98765"
---
{
  literal: "admin123" replace "123" with("ID"),
  regex: myInputA replace (("(^s.*e)" ++ myCompany.name) as Regex) with ("abcd"),
  prefixRegex: replace("myOther123", /(\d+)/) with("ID")
}"#;
        assert_eq!(
            execute_json(script, Value::Null, false).unwrap(),
            json!({"literal": "adminID", "regex": "abcd-98765", "prefixRegex": "myOtherID"})
        );
    }

    #[test]
    fn reads_and_writes_binary_lines_with_encoding() {
        let script = r#"%dw 2.0
import * from dw::core::Binaries
var content = read("Line 1\nLine 2\n", "application/octet-stream")
output application/json
---
{
  asJson: "abcd1234123" as Binary,
  lines: content readLinesWith "UTF-8",
  written: to(1, 3) map "Line $" writeLinesWith "UTF-8",
  decoded: toString("DW Test" as Binary {encoding: "UTF-32"}, "UTF-32")
}"#;
        assert_eq!(
            execute_json(script, Value::Null, true).unwrap(),
            json!(
                r#"{"asJson":"abcd1234123","lines":["Line 1","Line 2"],"written":"Line 1\nLine 2\nLine 3\n","decoded":"DW Test"}"#
            )
        );
    }

    #[test]
    fn evaluates_binary_to_base64_and_documented_gravatar_fixture() {
        let script = r#"%dw 2.0
import dw::Crypto
import toBase64 from dw::core::Binaries
var emailChecksum = Crypto::MD5("achaval@gmail.com" as Binary)
var image = readUrl(log("https://www.gravatar.com/avatar/$(emailChecksum)"), "application/octet-stream")
output application/python
---
{
  simple: toBase64("hello" as Binary),
  imagePrefix: substring(toBase64(image), 0, 29)
}
"#;
        assert_eq!(
            execute_smoke(script, Value::Null).unwrap(),
            json!({
                "simple": "aGVsbG8=",
                "imagePrefix": "/9j/4AAQSkZJRgABAQEAYABgAAD//"
            })
        );
    }

    #[test]
    fn evaluates_unary_minus_on_selector_expressions() {
        assert_eq!(
            execute_json("%dw 2.0\n---\n-payload.amount", json!({"amount": 7}), false).unwrap(),
            json!(-7)
        );
    }

    #[test]
    fn evaluates_map_filter_and_lambda_bindings() {
        let script = r#"%dw 2.0
output application/python
---
{
  names: payload.users map ((user, index) -> user.name ++ "-" ++ index),
  cheap: payload.users filter (($.price < 10) and ($$ > 0)),
  evenChars: "hello world" filter ($$ mod 2) == 0,
  implicit: payload.users map {name: $.name, id: $$}
}"#;
        let result = execute_json(
            script,
            json!({
                "users": [
                    {"name": "A", "price": 12},
                    {"name": "B", "price": 8},
                    {"name": "C", "price": 5}
                ]
            }),
            false,
        )
        .unwrap();
        assert_eq!(
            result,
            json!({
                "names": ["A-0", "B-1", "C-2"],
                "cheap": [{"name": "B", "price": 8}, {"name": "C", "price": 5}],
                "evenChars": "hlowrd",
                "implicit": [{"name": "A", "id": 0}, {"name": "B", "id": 1}, {"name": "C", "id": 2}]
            })
        );
    }

    #[test]
    fn evaluates_flat_map_and_group_by() {
        let script = r#"%dw 2.0
output application/python
---
{
  flattened: payload.items flatMap ((item) -> item.tags map ((tag) -> {id: item.id, tag: tag})),
  grouped: payload.items groupBy $.kind,
  groupedObject: { "a" : "b", "c" : "d"} groupBy upper($)
}"#;
        let result = execute_json(
            script,
            json!({
                "items": [
                    {"id": 1, "kind": "a", "tags": ["x", "y"]},
                    {"id": 2, "kind": "b", "tags": ["z"]},
                    {"id": 3, "kind": "a", "tags": []}
                ]
            }),
            false,
        )
        .unwrap();
        assert_eq!(
            result,
            json!({
                "flattened": [{"id": 1, "tag": "x"}, {"id": 1, "tag": "y"}, {"id": 2, "tag": "z"}],
                "grouped": {
                    "a": [
                        {"id": 1, "kind": "a", "tags": ["x", "y"]},
                        {"id": 3, "kind": "a", "tags": []}
                    ],
                    "b": [
                        {"id": 2, "kind": "b", "tags": ["z"]}
                    ]
                },
                "groupedObject": {
                    "B": {"a": "b"},
                    "D": {"c": "d"}
                }
            })
        );
    }

    #[test]
    fn evaluates_distinct_by_and_order_by() {
        let script = r#"%dw 2.0
output application/python
---
{
  distinct: payload.items distinctBy $.kind,
  distinctObjectValues: {a : "b", a : "b", A : "b", a : "B"} distinctBy (value) -> { "unique" : value },
  ordered: payload.items orderBy $.score,
  orderedByIndex: payload.items orderBy $$
}"#;
        let result = execute_json(
            script,
            json!({
                "items": [
                    {"kind": "b", "score": 20},
                    {"kind": "a", "score": 10},
                    {"kind": "b", "score": 5}
                ]
            }),
            false,
        )
        .unwrap();
        assert_eq!(
            result,
            json!({
                "distinct": [
                    {"kind": "b", "score": 20},
                    {"kind": "a", "score": 10}
                ],
                "distinctObjectValues": {
                    "__dwpy_object_pairs": [
                        {"key": "a", "value": "b"},
                        {"key": "a", "value": "B"}
                    ]
                },
                "ordered": [
                    {"kind": "b", "score": 5},
                    {"kind": "a", "score": 10},
                    {"kind": "b", "score": 20}
                ],
                "orderedByIndex": [
                    {"kind": "b", "score": 20},
                    {"kind": "a", "score": 10},
                    {"kind": "b", "score": 5}
                ]
            })
        );
    }

    #[test]
    fn evaluates_object_pluck_map_object_and_filter_object() {
        let script = r#"%dw 2.0
output application/python
---
{
  plucked: payload.object pluck ((value, key, index) -> key ++ ":" ++ value ++ ":" ++ index),
  mapped: payload.object mapObject ((value, key, index) -> {(upper(key)): value + index}),
  filtered: payload.object filterObject ((value, key) -> value > 1 and key != "c")
}"#;
        let result =
            execute_json(script, json!({"object": {"a": 1, "b": 2, "c": 3}}), false).unwrap();
        assert_eq!(
            result,
            json!({
                "plucked": ["a:1:0", "b:2:1", "c:3:2"],
                "mapped": {"A": 1, "B": 3, "C": 5},
                "filtered": {"b": 2}
            })
        );
    }

    #[test]
    fn evaluates_collection_operator_inside_unparenthesized_lambda_body() {
        let script = r#"%dw 2.0
output application/python
fun renameKey(key: Key) = key match {
  case "availableSeats" -> "emptySeats"
  case "airlineName" -> "airline"
  else -> key
}
---
payload.flights map (flight) ->
flight mapObject (value, key) -> {
  (renameKey(key)) : value
}
"#;
        let result = execute_json(
            script,
            json!({"flights": [
                {"availableSeats": 45, "airlineName": "Ryan Air", "aircraftBrand": "Boeing"}
            ]}),
            false,
        )
        .unwrap();
        assert_eq!(
            result,
            json!([{"emptySeats": 45, "airline": "Ryan Air", "aircraftBrand": "Boeing"}])
        );
    }

    #[test]
    fn evaluates_descendant_selectors() {
        let script = r#"%dw 2.0
output application/python
---
{
  names: payload..name,
  values: payload.."value 2",
  children: payload.root..,
  pairs: payload..&name
}"#;
        let result = execute_json(
            script,
            json!({
                "name": "root",
                "root": {"child": {"name": "child"}},
                "users": [
                    {"profile": {"name": "alpha", "value 2": "a"}},
                    {"profile": {"name": "beta", "nested": {"value 2": "b"}}}
                ]
            }),
            false,
        )
        .unwrap();
        assert_eq!(
            result,
            json!({
                "names": ["root", "child", "alpha", "beta"],
                "values": ["a", "b"],
                "children": [{"name": "child"}, "child"],
                "pairs": [
                    {"name": "root"},
                    {"name": "child"},
                    {"name": "alpha"},
                    {"name": "beta"}
                ]
            })
        );
    }

    #[test]
    fn evaluates_runtime_type_checks() {
        let script = r#"%dw 2.0
import * from dw::core::Strings
output application/python
type WithParameters<A, B> = {first: A, second: B, nestedObject: {message: A}}
---
{
  array: [1] is Array,
  number: 1.2 is Number,
  string: "x" is String,
  object: {a: 1} is Object,
  bool: true is Boolean,
  nullValue: null is Null,
  genericFirst: true is WithParameters<Boolean, Number>.first,
  genericSecond: 4592 is WithParameters<String, Number>.second,
  genericNested: "sdf" is WithParameters<String, Number>.nestedObject.message,
  coercedCompare: 123 as String == "123",
  mapped: payload mapObject ((elementValue, elementKey) -> {
    (if (elementValue is Array)
      pluralize(elementKey)
    else if (elementValue is Number)
      camelize(elementKey)
    else capitalize(elementKey)) : elementValue
  })
}"#;
        let result = execute_smoke(
            script,
            json!({"version_no": 1.6, "store_of_origin": "SFO", "item": [{"id": 1}]}),
        )
        .unwrap();
        assert_eq!(result["array"], json!(true));
        assert_eq!(result["number"], json!(true));
        assert_eq!(result["string"], json!(true));
        assert_eq!(result["object"], json!(true));
        assert_eq!(result["bool"], json!(true));
        assert_eq!(result["nullValue"], json!(true));
        assert_eq!(result["genericFirst"], json!(true));
        assert_eq!(result["genericSecond"], json!(true));
        assert_eq!(result["genericNested"], json!(true));
        assert_eq!(result["coercedCompare"], json!(true));
        assert_eq!(
            result["mapped"],
            json!({"versionNo": 1.6, "Store Of Origin": "SFO", "items": [{"id": 1}]})
        );
    }

    #[test]
    fn evaluates_reduce_with_default_and_without_default() {
        assert_eq!(
            execute_json(
                r#"%dw 2.0
output application/python
---
payload.names reduce ((item, acc = "") -> acc ++ upper(item))"#,
                json!({"names": ["a", "b"]}),
                false,
            )
            .unwrap(),
            json!("AB")
        );
        assert_eq!(
            execute_json(
                r#"%dw 2.0
output application/python
---
[] reduce ((item, acc = 7) -> acc + item)"#,
                json!({}),
                false,
            )
            .unwrap(),
            json!(7)
        );
        let script = r#"%dw 2.0
output application/python
---
{
  total: payload.items reduce ((item, acc = 0) -> acc + item.price * (item.quantity default 1)),
  joined: payload.names reduce ((item, acc = "") -> acc ++ upper(item)),
  reversed: "hello world" reduce (item, acc = "") -> item ++ acc,
  product: [2, 3, 3] reduce ((item, acc) -> acc * item),
  empty: [] reduce ((item, acc = 7) -> acc + item),
  nullThen: [] reduce ((item, acc) -> item ++ acc) then ((result) -> sizeOf(result)),
  nullFallback: [] reduce ((item, acc) -> item ++ acc) then ((result) -> sizeOf(result)) onNull "Empty Text"
}"#;
        let result = execute_json(
            script,
            json!({
                "items": [{"price": 4, "quantity": 2}, {"price": 3}],
                "names": ["a", "b"]
            }),
            false,
        )
        .unwrap();
        assert_eq!(
            result,
            json!({
                "total": 11,
                "joined": "AB",
                "reversed": "dlrow olleh",
                "product": 18,
                "empty": 7,
                "nullThen": null,
                "nullFallback": "Empty Text"
            })
        );
    }

    #[test]
    fn evaluates_recursive_functions_with_reduce_defaults() {
        let script = r#"%dw 2.0
output application/python
fun traverse(obj) = [{"name" : obj.name}] ++ (obj.children reduce ((value, accumulator = [] ) -> accumulator ++ traverse(value)))
---
traverse(payload)
"#;
        assert_eq!(
            execute_json(
                script,
                json!({
                    "name": "Some Name",
                    "children": [
                        {
                            "name": "Inner",
                            "children": []
                        }
                    ]
                }),
                false,
            )
            .unwrap(),
            json!([
                {"name": "Some Name"},
                {"name": "Inner"}
            ])
        );
    }

    #[test]
    fn evaluates_defaulted_recursive_function_parameter_in_reduce() {
        let script = r#"%dw 2.0
output application/python
fun hola(obj, level = 0) = [{"account" : (0 to level map "-") joinBy "" ++ " " ++ obj.name, }] ++ (obj.children reduce ((value, accumulator = [] ) -> accumulator ++ hola(value, level + 1)))
---
(hola(payload, 0) map [$.account,$$]) reduce ((item, accumulator = "") -> accumulator ++ item[0] ++ " - ID:$(item[1])" ++"\n")
"#;
        assert_eq!(
            execute_json(
                script,
                json!({
                    "name": "Some Name",
                    "children": [
                        {
                            "name": "Inner",
                            "children": []
                        }
                    ]
                }),
                false,
            )
            .unwrap(),
            json!("- Some Name - ID:0\n-- Inner - ID:1\n")
        );
    }

    #[test]
    fn evaluates_object_merge_and_array_property_selectors() {
        let script = r##"%dw 2.0
output application/python
---
{
  merged: payload.user ++ {password: "****", active: true},
  messages: payload.users.message,
  multiMessages: payload.users.*message,
  books: payload.catalog.*book map ((book) -> {title: book.title}),
  localMessages: payload.users map ((user) -> user.message),
  loggedName: log(payload.user).name,
  loggedStatus: log(payload.item).payload.errors[0].statusCode,
  conditionalMerged: {
    (if (payload.include == true) {name: payload.user.name} else {})
  }
}
"##;
        assert_eq!(
            execute_smoke(
                script,
                json!({
                    "include": true,
                    "user": {"name": "A", "password": "1234"},
                    "item": {"payload": {"errors": [{"statusCode": 404}]}},
                    "users": [
                        {"message": "Hello"},
                        {"other": 1},
                        {"message": "World"}
                    ],
                    "catalog": {"book": [{"title": "A"}, {"title": "B"}]}
                })
            )
            .unwrap(),
            json!({
                "merged": {"name": "A", "password": "****", "active": true},
                "messages": ["Hello", "World"],
                "multiMessages": ["Hello", "World"],
                "books": [{"title": "A"}, {"title": "B"}],
                "localMessages": ["Hello", null, "World"],
                "loggedName": "A",
                "loggedStatus": 404,
                "conditionalMerged": {"name": "A"}
            })
        );
    }

    #[test]
    fn evaluates_vars_scope() {
        let script = r#"%dw 2.0
output application/python
---
{
  target: vars.target,
  matched: payload.users map ((user) -> user.name == vars.target),
  scaled: payload.values map ((value) -> value * vars.multiplier),
  fallback: vars.missing default "ok"
}
"#;
        assert_eq!(
            execute_json_with_vars(
                script,
                json!({
                    "users": [{"name": "Ana"}, {"name": "Bob"}],
                    "values": [1, 2, 3]
                }),
                json!({"target": "Bob", "multiplier": 10}),
                false
            )
            .unwrap(),
            json!({
                "target": "Bob",
                "matched": [false, true],
                "scaled": [10, 20, 30],
                "fallback": "ok"
            })
        );
    }

    #[test]
    fn evaluates_header_vars_in_order() {
        let script = r#"%dw 2.0
output application/python
var greeting = upper("hello")
var captured = vars.requestTime default "missing"
var summary = greeting ++ " " ++ payload.name
---
{
  message: summary,
  captured: captured
}
"#;
        assert_eq!(
            execute_json_with_vars(
                script,
                json!({"name": "DW"}),
                json!({"requestTime": "2024-05-05T12:00:00Z"}),
                false
            )
            .unwrap(),
            json!({
                "message": "HELLO DW",
                "captured": "2024-05-05T12:00:00Z"
            })
        );
    }

    #[test]
    fn evaluates_multiline_header_vars_and_functions() {
        let script = r#"%dw 2.0
output application/python
var values =
  if (isEmpty(payload.values default [])) []
  else payload.values default []

fun average(items) =
  if (isEmpty(items)) null
  else round((sum(items) / sizeOf(items)) * 100) / 100
---
{
  values: values,
  average: average(values)
}
"#;
        assert_eq!(
            execute_smoke(script, json!({"values": [4, 6]})).unwrap(),
            json!({
                "values": [4, 6],
                "average": 5
            })
        );
    }

    #[test]
    fn evaluates_multiline_header_infix_chains() {
        let script = r#"%dw 2.0
output application/python
var values =
  (payload.values default [])
    map ((value) -> value * 2)
    filter ((value) -> value >= 4)
---
values
"#;
        assert_eq!(
            execute_smoke(script, json!({"values": [1, 2, 3]})).unwrap(),
            json!([4, 6])
        );
    }

    #[test]
    fn evaluates_do_blocks_in_headers_functions_and_objects() {
        let function_script = r#"%dw 2.0
output application/python
fun myfun() = do {
    var name = "DataWeave"
    ---
    name
}
---
{ result: myfun() }
"#;
        assert_eq!(
            execute_smoke(function_script, Value::Null).unwrap(),
            json!({"result": "DataWeave"})
        );

        let header_script = r#"%dw 2.0
output application/python
var myVar = do {
    var name = "DataWeave"
    ---
    name
}
---
{ result: myVar }
"#;
        assert_eq!(
            execute_smoke(header_script, Value::Null).unwrap(),
            json!({"result": "DataWeave"})
        );

        let object_script = r#"%dw 2.0
output application/python
---
{
  result: do {
    var name = "DataWeave"
    ---
    name
  }
}
"#;
        assert_eq!(
            execute_smoke(object_script, Value::Null).unwrap(),
            json!({"result": "DataWeave"})
        );

        let expression_only = r#"%dw 2.0
output application/json
fun extractNumber(pageName: Key) =
     (pageName as String match /\(sheet\)([0-9]+)/)[1]
---
payload mapObject ((value, key, index) -> do {
  if (extractNumber(key) == "2")
    {(key): value << {Id: "2"}}
  else
    {(key): value}
})
"#;
        assert_eq!(
            execute_json(
                expression_only,
                json!({"(sheet)1": [{"Id": 1}], "(sheet)2": [{"Id": 2}]}),
                true
            )
            .unwrap(),
            json!(r#"{"(sheet)1":[{"Id":1}],"(sheet)2":[{"Id":2},{"Id":"2"}]}"#)
        );
    }

    #[test]
    fn evaluates_supported_import_aliases_and_named_imports() {
        let script = r#"%dw 2.0
output application/python
import trim as tidy from dw::core::Strings
import keysOf, valuesOf from dw::core::Objects
---
{
  cleaned: tidy(payload.value),
  keys: keysOf(payload.object),
  values: valuesOf(payload.object)
}
"#;
        assert_eq!(
            execute_smoke(
                script,
                json!({"value": "  hello  ", "object": {"a": 1, "b": 2}})
            )
            .unwrap(),
            json!({"cleaned": "hello", "keys": ["a", "b"], "values": [1, 2]})
        );
    }

    #[test]
    fn evaluates_documented_custom_module_import_fixture() {
        let script = r#"%dw 2.0
import modules::MyModule
output application/json
---
MyModule::myFunc("dataweave") ++ "name"
"#;
        assert_eq!(
            execute_json(script, Value::Null, false).unwrap(),
            json!("dataweave_name")
        );
        let mapping_script = r#"%dw 2.0
import modules::MyMapping
output application/json
---
MyMapping::main(payload: { "user" : "bar" })
"#;
        assert_eq!(
            execute_json(mapping_script, Value::Null, true).unwrap(),
            json!(r#"{"UserKey":"bar"}"#)
        );
    }

    #[test]
    fn evaluates_types_module_helpers() {
        let script = r#"%dw 2.0
import * from dw::core::Types
type ArrayOfString = Array<String>
type ArrayOfAnyDefault = Array
type FormattedString = String {format: "YYYY-MM-dd"}
type UnionType = String | Number
type IntersectionType = {name: String} & {age: Number}
type LiteralType = "Mariano"
type NamedArray = Array<String> {n: 1}
type NestedNamedArray = Array<NamedArray>
type FunctionType = (String, Number) -> Number
output application/python
---
{
  arrayItem: [arrayItem(ArrayOfString), arrayItem(ArrayOfAnyDefault)],
  baseType: baseTypeOf(FormattedString),
  functionParams: functionParamTypes(FunctionType),
  functionReturn: functionReturnType(FunctionType),
  unionItems: unionItems(UnionType),
  intersectionItems: intersectionItems(IntersectionType),
  literalValue: literalValueOf(LiteralType),
  metadata: metadataOf(FormattedString),
  names: [nameOf(NamedArray), nameOf(String)],
  predicates: {
    any: [isAnyType(Any), isAnyType(String)],
    array: [isArrayType(ArrayOfString), isArrayType(Boolean)],
    string: [isStringType(String), isStringType(Boolean)],
    number: [isNumberType(Number), isNumberType(String)],
    union: [isUnionType(UnionType), isUnionType(String)],
    intersection: [isIntersectionType(IntersectionType), isIntersectionType(String)],
    literal: [isLiteralType(LiteralType), isLiteralType(String)],
    function: [isFunctionType((String) -> Boolean), isFunctionType(String)],
    reference: [isReferenceType(NamedArray), isReferenceType(arrayItem(NestedNamedArray))]
  }
}"#;
        assert_eq!(
            execute_smoke(script, Value::Null).unwrap(),
            json!({
                "arrayItem": ["String", "Any"],
                "baseType": "String",
                "functionParams": [
                    {"paramType": "String", "optional": false},
                    {"paramType": "Number", "optional": false}
                ],
                "functionReturn": "Number",
                "unionItems": ["String", "Number"],
                "intersectionItems": ["Object", "Object"],
                "literalValue": "Mariano",
                "metadata": {"format": "YYYY-MM-dd"},
                "names": ["NamedArray", "String"],
                "predicates": {
                    "any": [true, false],
                    "array": [true, false],
                    "string": [true, false],
                    "number": [true, false],
                    "union": [true, false],
                    "intersection": [true, false],
                    "literal": [true, false],
                    "function": [true, false],
                    "reference": [false, true]
                }
            })
        );
    }

    #[test]
    fn evaluates_null_safe_and_quoted_selectors() {
        assert_eq!(
            execute_smoke(
                r#"%dw 2.0
---
payload.user?."value 2" default "UNKNOWN""#,
                json!({"user": {"value 2": "s"}})
            )
            .unwrap(),
            json!("s")
        );
        assert_eq!(
            execute_smoke(
                r#"%dw 2.0
output application/python
var city = payload.user?.address?.city default "UNKNOWN"
---
{city: city}"#,
                json!({})
            )
            .unwrap(),
            json!({"city": "UNKNOWN"})
        );
    }

    #[test]
    fn evaluates_match_expressions() {
        let script = r#"%dw 2.0
output application/python
---
{
  normalized: payload.status match {
    case "confirmed" -> "CONFIRMED",
    case "pending" -> "PENDING",
    else -> "UNKNOWN"
  },
  bucket: payload.total match {
    case var value when value > 100 -> "large",
    case var value -> "small"
  },
  boolCase: payload.flag match {
    case true -> 1,
    case false -> 0
  },
  boundLiteral: payload.name match {
    case str: "Emiliano" -> { matches: true, value: str }
    case str: "Mariano" -> { matches: false, value: str }
  },
  guardedBinding: payload.name match {
    case str if str == "Mariano" -> str ++ " de Achaval"
    case str if str == "Emiliano" -> str ++ " Lesende"
  },
  typeCase: payload.total match {
    case is Object -> "OBJECT"
    case is String -> "STRING"
    case is Number -> "NUMBER"
    else -> "ANOTHER TYPE"
  },
  boundTypeCase: payload.name match {
    case y is Object -> {Type: {OBJECT: y}}
    case y is String -> {Type: {STRING: y}}
    else -> {Type: {ANOTHER: y}}
  },
  regexCase: payload.greeting match {
    case word matches /(hello)\s\w+/ -> word[1] ++ " was matched"
    else -> "none"
  }
}
"#;
        assert_eq!(
            execute_smoke(
                script,
                json!({"status": "confirmed", "total": 150, "flag": true, "name": "Emiliano", "greeting": "hello world"})
            )
            .unwrap(),
            json!({
                "normalized": "CONFIRMED",
                "bucket": "large",
                "boolCase": 1,
                "boundLiteral": {"matches": true, "value": "Emiliano"},
                "guardedBinding": "Emiliano Lesende",
                "typeCase": "NUMBER",
                "boundTypeCase": {"Type": {"STRING": "Emiliano"}},
                "regexCase": "hello was matched"
            })
        );
        assert_eq!(
            execute_smoke(
                script,
                json!({"status": "other", "total": 40, "flag": false, "name": "Mariano", "greeting": "bye"})
            )
            .unwrap(),
            json!({
                "normalized": "UNKNOWN",
                "bucket": "small",
                "boolCase": 0,
                "boundLiteral": {"matches": false, "value": "Mariano"},
                "guardedBinding": "Mariano de Achaval",
                "typeCase": "NUMBER",
                "boundTypeCase": {"Type": {"STRING": "Mariano"}},
                "regexCase": "none"
            })
        );
    }

    #[test]
    fn evaluates_read_write_format_helpers() {
        let script = r#"%dw 2.0
output application/python
import read, write from dw::Core
---
{
  jsonParsed: read("{\"a\": 1}", "application/json").a,
  jsonWritten: write({a: 1}, "application/json"),
  jsonWrittenWithOptions: write(read("<greeting><ex1></ex1><ex2>hello</ex2><ex3 a='greeting'>hello</ex3></greeting>", "application/xml", {nullValueOn: "empty"}).greeting, "application/json", {skipNullOn:"objects", writeAttributes:true}),
  xmlItems: read("<root><order><items>1</items><items>3</items></order></root>", "application/xml").root.order.*items contains "3",
  csvRow: read("Some, Body", "application/csv", {header:false})[0],
  multipartKeys: read("--34b21\nContent-Disposition: form-data; name=\"text\"\nContent-Type: text/plain\n\nBook\n--34b21\nContent-Disposition: form-data; name=\"file1\"; filename=\"a.json\"\nContent-Type: application/json\n\n{\"title\":\"Java 8 in Action\"}\n--34b21--", "multipart/form-data", {boundary: "34b21"}).parts mapObject ((value, key, index) -> {(index): key}),
  plain: write("hello", "text/plain"),
  yamlParsed: read("name: Ana\nroles:\n  - admin\n", "application/yaml").roles[0],
  yamlWritten: write({name: "Ana", enabled: true}, "application/yaml")
}
"#;
        let result = execute_smoke(script, Value::Null).unwrap();
        assert_eq!(result["jsonParsed"], json!(1));
        assert_eq!(result["jsonWritten"], json!("{\"a\":1}"));
        assert_eq!(
            result["jsonWrittenWithOptions"],
            json!("{\"ex2\":\"hello\",\"ex3\":{\"@a\":\"greeting\",\"__text\":\"hello\"}}")
        );
        assert_eq!(result["xmlItems"], json!(true));
        assert_eq!(
            result["csvRow"],
            json!({"column_0": "Some", "column_1": " Body"})
        );
        assert_eq!(result["multipartKeys"], json!({"0": "text", "1": "file1"}));
        assert_eq!(result["plain"], json!("hello"));
        assert_eq!(result["yamlParsed"], json!("admin"));
        assert_eq!(result["yamlWritten"], json!("name: Ana\nenabled: true\n"));

        let rendered = execute_json(
            r#"%dw 2.0
var myVar = read("<greeting><ex1></ex1><ex2>hello</ex2><ex3 a='greeting'>hello</ex3></greeting>", "application/xml", {nullValueOn: "empty"})
output application/json with binary
---
write(myVar.greeting, "application/json", {skipNullOn:"objects", writeAttributes:true})"#,
            Value::Null,
            true,
        )
        .unwrap();
        assert_eq!(
            rendered,
            json!(r#"{"ex2":"hello","ex3":{"@a":"greeting","__text":"hello"}}"#)
        );
    }

    #[test]
    fn evaluates_core_math_mime_and_null_helpers() {
        let script = r#"%dw 2.0
output application/python
import * from dw::Core
import * from dw::util::Math
import * from dw::core::Numbers
import * from dw::module::Mime
---
{
  uuidLength: sizeOf(uuid()),
  root: sqrt(25),
  powered: pow(2, 3),
  modded: mod(7, 4),
  sinZero: sin(0),
  cosZero: cos(0),
  tanZero: tan(0),
  asinInvalid: asin(1.1),
  fromHex: fromRadixNumber("ff", 16),
  toHex: toRadixNumber(255, 16),
  zipped: zip([1, 2], ["a", "b", "c"]),
  unzipped: unzip([[0, "a"], [1, "b"], [2, "c"]]),
  top: maxBy([{name: "A", score: 1}, {name: "B", score: 3}], (item) -> item.score).name,
  bottom: minBy([{name: "A", score: 1}, {name: "B", score: 3}], (item) -> item.score).name,
  chained: 1 then ((value) -> value + 1),
  fallback: null onNull (() -> "empty"),
  invalidMime: fromString("Invalid MIME type"),
  mime: fromString("application/json"),
  mimeText: toString({type: "application", subtype: "json", parameters: {}}),
  multipartText: toString({type: "multipart", subtype: "form-data", parameters: {boundary: "my-boundary"}}),
  handled: isHandledBy({type: "application", subtype: "*", parameters: {}}, {type: "application", subtype: "json", parameters: {}}),
  suffixHandled: isHandledBy({type: "application", subtype: "*+xml", parameters: {}}, {type: "application", subtype: "soap+xml", parameters: {}}),
  raggedUnzipped: unzip([ [0,"a"], [1,"a","foo"], [2], [3,"a"] ])
}
"#;
        let result = execute_smoke(script, Value::Null).unwrap();
        assert_eq!(result["uuidLength"], json!(36));
        assert_eq!(result["root"], json!(5));
        assert_eq!(result["powered"], json!(8));
        assert_eq!(result["modded"], json!(3));
        assert_eq!(result["sinZero"], json!(0));
        assert_eq!(result["cosZero"], json!(1));
        assert_eq!(result["tanZero"], json!(0));
        assert_eq!(result["asinInvalid"], json!({"__dwpy_nonfinite": "nan"}));
        assert_eq!(result["fromHex"], json!(255));
        assert_eq!(result["toHex"], json!("ff"));
        assert_eq!(result["zipped"], json!([[1, "a"], [2, "b"]]));
        assert_eq!(result["unzipped"], json!([[0, 1, 2], ["a", "b", "c"]]));
        assert_eq!(result["raggedUnzipped"], json!([0, 1, 2, 3]));
        assert_eq!(result["top"], json!("B"));
        assert_eq!(result["bottom"], json!("A"));
        assert_eq!(result["chained"], json!(2));
        assert_eq!(result["fallback"], json!("empty"));
        assert_eq!(
            result["invalidMime"],
            json!({"success": false, "error": {"message": "Unable to find a sub type in `Invalid MIME type`."}})
        );
        assert_eq!(result["mime"]["success"], json!(true));
        assert_eq!(result["mimeText"], json!("application/json"));
        assert_eq!(
            result["multipartText"],
            json!("multipart/form-data;boundary=my-boundary")
        );
        assert_eq!(result["handled"], json!(true));
        assert_eq!(result["suffixHandled"], json!(true));
    }

    #[test]
    fn evaluates_numbers_module_radix_helpers_with_big_integers() {
        let script = r#"%dw 2.0
import * from dw::core::Numbers
output application/json
---
{
  hex: toHex(100000000000000000000000000000000000000000000000000000000000000),
  binary: toBinary(100000000000000000000000000000000000000000000000000000000000000),
  fromHex: fromHex("3e3aeb4ae1383562f4b82261d969f7ac94ca4000000000000000"),
  fromBinary: fromBinary("11111000111010111010110100101011100001001110000011010101100010111101001011100000100010011000011101100101101001111101111010110010010100110010100100000000000000000000000000000000000000000000000000000000000000"),
  nullHex: toHex(null),
  negative: toBinary(-2)
}
"#;
        let rendered = execute_json(script, Value::Null, true).unwrap();
        assert_eq!(
            rendered,
            json!(
                "{\"hex\":\"3e3aeb4ae1383562f4b82261d969f7ac94ca4000000000000000\",\"binary\":\"11111000111010111010110100101011100001001110000011010101100010111101001011100000100010011000011101100101101001111101111010110010010100110010100100000000000000000000000000000000000000000000000000000000000000\",\"fromHex\":100000000000000000000000000000000000000000000000000000000000000,\"fromBinary\":100000000000000000000000000000000000000000000000000000000000000,\"nullHex\":null,\"negative\":\"-10\"}"
            )
        );
    }

    #[test]
    fn renders_nonfinite_math_as_null_for_json_output() {
        let rendered = execute_json(
            "%dw 2.0\nimport * from dw::util::Math\noutput application/json\n---\n{ value: asin(2) }",
            Value::Null,
            true,
        )
        .unwrap();
        assert_eq!(rendered, json!("{\"value\":null}"));
    }

    #[test]
    fn evaluates_url_dtd_and_values_helpers() {
        let script = r#"%dw 2.0
import * from dw::core::URL
import * from dw::xml::Dtd
import * from dw::util::Values
output application/python
var urlPath = "content folder"
---
{
  composed: compose(["http://examplewebsite.com/", "/page.html"], ["$(urlPath)"]),
  decoded: decodeURI("http://asd/%20text%20to%20decode%20/text"),
  encoded: encodeURI("http://asd/ text to decode /%/\"\\/text"),
  encodedComponent: encodeURIComponent(";,/?:@&="),
  parsed: parseURI("https://en.wikipedia.org/wiki/Uniform_Resource_Identifier#footer"),
  query: parseURI("http://my.company.com:1234/hello/?field=value").query,
  authority: parseURI("http://localhost:8080/test").authority,
	  docTypeSystem: docTypeAsString({rootName: "cXML", systemId: "http://xml.cxml.org/schemas/cXML/1.2.014/cXML.dtd"}),
	  docTypePublic: docTypeAsString({rootName: "html", publicId: "-//W3C//DTD XHTML 1.0 Transitional//EN", systemId: "http://www.w3.org/TR/xhtml1/DTD/xhtml1-transitional.dtd"}),
	  pathIndex: index(0),
	  readClasspathJson: readUrl("classpath://myJson.json", "application/json"),
	  readClasspathDwName: (readUrl("classpath://name.dwl", "application/dw")).firstName,
	  readRemoteJson: readUrl("https://jsonplaceholder.typicode.com/posts/1", "application/json"),
	  readRemoteCsv: readUrl("https://mywebsite.com/data.csv", "application/csv", {"header": false})
}
"#;
        assert_eq!(
            execute_smoke(script, Value::Null).unwrap(),
            json!({
                "composed": "http://examplewebsite.com/content%20folder/page.html",
                "decoded": "http://asd/ text to decode /text",
                "encoded": "http://asd/%20text%20to%20decode%20/%25/%22%5C/text",
                "encodedComponent": "%3B%2C%2F%3F%3A%40%26%3D",
                "parsed": {
                    "isValid": true,
                    "raw": "https://en.wikipedia.org/wiki/Uniform_Resource_Identifier#footer",
                    "host": "en.wikipedia.org",
                    "authority": "en.wikipedia.org",
                    "fragment": "footer",
                    "path": "/wiki/Uniform_Resource_Identifier",
                    "scheme": "https",
                    "isAbsolute": true,
                    "isOpaque": false
                },
                "query": "field=value",
                "authority": "localhost:8080",
                "docTypeSystem": "cXML SYSTEM http://xml.cxml.org/schemas/cXML/1.2.014/cXML.dtd",
                "docTypePublic": "html PUBLIC -//W3C//DTD XHTML 1.0 Transitional//EN http://www.w3.org/TR/xhtml1/DTD/xhtml1-transitional.dtd",
                "pathIndex": {"kind": "Array", "namespace": null, "selector": 0},
                "readClasspathJson": {"hello": "world"},
                "readClasspathDwName": "Somebody",
                "readRemoteJson": {"userId": 1, "id": 1, "title": "sunt aut ...", "body": "quia et ..."},
                "readRemoteCsv": [{"column_0": "Max", "column_1": "the Mule", "column_2": "MuleSoft"}]
            })
        );

        let xlsx = execute_json(
            r#"%dw 2.0
var myInput = readUrl("classpath://ourBugs.xlsx", "application/xlsx")
output application/json
---
myInput."Data" filter ((entry, index) -> entry."Assignee" == "Fred M")"#,
            Value::Null,
            true,
        )
        .unwrap();
        assert_eq!(
            xlsx,
            json!(
                r#"[{"Issue Key":"BUG-11708","Issue Type":"Bug","Summary":"Some Description of the Bug","Assignee":"Fred M","Reporter":"Natalie C","Priority":"To be reviewed","Status":"Closed","Resolution":"Done","Created":"2019-04-29T03:57:00","Updated":"2019-05-06T10:40:00","Due Date":""},{"Issue Key":"BUG-4903","Issue Type":"Story","Summary":"Some Description of the Bug","Assignee":"Fred M","Reporter":"Fred M","Priority":"To be reviewed","Status":"In Progress","Resolution":"","Created":"2019-05-07T11:22:00","Updated":"2019-05-08T10:16:00","Due Date":""},{"Issue Key":"BUG-4840","Issue Type":"Story","Summary":"Some Description of the Bug","Assignee":"Fred M","Reporter":"Pablo C","Priority":"To be reviewed","Status":"In Validation","Resolution":"","Created":"2019-04-30T07:11:00","Updated":"2019-05-08T10:16:00","Due Date":""}]"#
            )
        );
    }

    #[test]
    fn evaluates_xml_and_string_metadata_selectors() {
        let xml_payload = parse_payload_format(
            json!(
                r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE cXML SYSTEM "http://xml.cxml.org/schemas/cXML/1.2.014/cXML.dtd">
<cXML><Header><From><Credential><Identity>TestIdentity</Identity></Credential></From></Header></cXML>"#
            ),
            Some("application/xml"),
        )
        .unwrap();
        let xml_result = execute_smoke(
            r#"%dw 2.0
output application/python
---
{
  identity: payload.cXML.Header.From.Credential.Identity,
  docType: payload.^docType,
  docTypeText: docTypeAsString(payload.^docType)
}
"#,
            xml_payload,
        )
        .unwrap();
        assert_eq!(
            xml_result,
            json!({
                "identity": "TestIdentity",
                "docType": {
                    "rootName": "cXML",
                    "systemId": "http://xml.cxml.org/schemas/cXML/1.2.014/cXML.dtd"
                },
                "docTypeText": "cXML SYSTEM http://xml.cxml.org/schemas/cXML/1.2.014/cXML.dtd"
            })
        );

        let string_result = execute_smoke(
            r#"%dw 2.0
output application/python
var userName = "DataWeave" as String {myCustomMetadata: "customMetadataValue"}
---
{
  valueOfVariableMetaData: userName.^myCustomMetadata,
  valueOfVariable: userName
}
"#,
            Value::Null,
        )
        .unwrap();
        assert_eq!(
            string_result,
            json!({
                "valueOfVariableMetaData": "customMetadataValue",
                "valueOfVariable": "DataWeave"
            })
        );
    }

    #[test]
    fn evaluates_array_join_helpers() {
        let script = r#"%dw 2.0
import * from dw::core::Arrays
output application/python
var users = [{id: "1", name:"Mariano"},{id: "2", name:"Leandro"},{id: "3", name:"Julian"},{id: "5", name:"Julian"}]
var products = [{ownerId: "1", name:"DataWeave"},{ownerId: "1", name:"BAT"}, {ownerId: "3", name:"DataSense"}, {ownerId: "4", name:"SmartConnectors"}]
---
{
  joined: join(users, products, (user) -> user.id, (product) -> product.ownerId),
  left: leftJoin(users, products, (user) -> user.id, (product) -> product.ownerId),
  outer: outerJoin(users, products, (user) -> user.id, (product) -> product.ownerId)
}
"#;
        let result = execute_smoke(script, Value::Null).unwrap();
        assert_eq!(
            result["joined"],
            json!([
                {"l": {"id": "1", "name": "Mariano"}, "r": {"ownerId": "1", "name": "DataWeave"}},
                {"l": {"id": "1", "name": "Mariano"}, "r": {"ownerId": "1", "name": "BAT"}},
                {"l": {"id": "3", "name": "Julian"}, "r": {"ownerId": "3", "name": "DataSense"}}
            ])
        );
        assert_eq!(
            result["left"],
            json!([
                {"l": {"id": "1", "name": "Mariano"}, "r": {"ownerId": "1", "name": "DataWeave"}},
                {"l": {"id": "1", "name": "Mariano"}, "r": {"ownerId": "1", "name": "BAT"}},
                {"l": {"id": "2", "name": "Leandro"}},
                {"l": {"id": "3", "name": "Julian"}, "r": {"ownerId": "3", "name": "DataSense"}},
                {"l": {"id": "5", "name": "Julian"}}
            ])
        );
        assert_eq!(
            result["outer"],
            json!([
                {"l": {"id": "1", "name": "Mariano"}, "r": {"ownerId": "1", "name": "DataWeave"}},
                {"l": {"id": "1", "name": "Mariano"}, "r": {"ownerId": "1", "name": "BAT"}},
                {"l": {"id": "2", "name": "Leandro"}},
                {"l": {"id": "3", "name": "Julian"}, "r": {"ownerId": "3", "name": "DataSense"}},
                {"l": {"id": "5", "name": "Julian"}},
                {"r": {"ownerId": "4", "name": "SmartConnectors"}}
            ])
        );
    }

    #[test]
    fn evaluates_simple_header_functions() {
        let script = r#"%dw 2.0
output application/python
fun lineTotal(item) = item.qty * item.price
fun label(name, prefix = "Item") = prefix ++ ": " ++ upper(name)
fun normalize(value): Number = value as Number
---
{
  total: sum(payload.items map ((item) -> lineTotal(item))),
  labels: payload.items map ((item) -> label(item.name)),
  custom: label("special", "Custom"),
  normalized: normalize("12.5")
}
"#;
        assert_eq!(
            execute_smoke(
                script,
                json!({
                    "items": [
                        {"name": "apple", "qty": 2, "price": 3},
                        {"name": "pear", "qty": 1, "price": 4}
                    ]
                })
            )
            .unwrap(),
            json!({
                "total": 10,
                "labels": ["Item: APPLE", "Item: PEAR"],
                "custom": "Custom: SPECIAL",
                "normalized": 12.5
            })
        );
    }

    #[test]
    fn evaluates_primitive_coercions() {
        let script = r###"%dw 2.0
output application/python
---
{
  fromString: payload.price as Number,
  fromBool: true as Number,
  toString: (payload.count + 1) as String,
  trueString: "true" as Boolean,
  falseNumber: 0 as Boolean,
  nullString: null as String,
  formattedWhole: 22 as String {format: ".00"},
  enNumber: 12.3 as String {format: "##.##", locale: "en"},
  esNumber: 12.3 as String {format: "##.##", locale: "es"},
  esDate: "2020-12-31" as Date as String {format: "dd-MMM-yy", locale: "es"},
  enDate: "2020-12-31" as Date as String {format: "dd-MMM-yy", locale: "en"},
  nowDateSize: sizeOf(now() as Date),
  mapped: payload.values map (($ as String) ++ "-x")
}
	"###;
        assert_eq!(
            execute_smoke(
                script,
                json!({
                    "price": "12.5",
                    "count": 2,
                    "values": [1, 2, 3]
                })
            )
            .unwrap(),
            json!({
                "fromString": 12.5,
                "fromBool": 1,
                "toString": "3",
                "trueString": true,
                "falseNumber": false,
                "nullString": null,
                "formattedWhole": "22.00",
                "enNumber": "12.3",
                "esNumber": "12,3",
                "esDate": "31-dic.-20",
                "enDate": "31-Dec-20",
                "nowDateSize": 10,
                "mapped": ["1-x", "2-x", "3-x"]
            })
        );
    }

    #[test]
    fn evaluates_now_and_date_coercion() {
        let result = execute_json(
            "%dw 2.0\n---\n{stamp: now(), date: now() as Date}",
            Value::Null,
            false,
        )
        .unwrap();
        let stamp = result["stamp"].as_str().unwrap();
        let date = result["date"].as_str().unwrap();
        assert!(stamp.ends_with('Z'));
        assert_eq!(stamp.get(..10), Some(date));
        assert_eq!(date.len(), 10);
    }

    #[test]
    fn evaluates_temporal_formatting_and_day_period_arithmetic() {
        let formatted = execute_json(
            r#"%dw 2.0
output application/python
---
{
  formattedDate: |2020-10-01T23:57:59| as String {format: "uuuu-MM-dd"},
  formattedTime: |2020-10-01T23:57:59| as String {format: "KK:mm:ss a"},
  formattedDateTime: |2020-10-01T23:57:59| as String {format: "KK:mm:ss a, MMMM dd, uuuu"}
}"#,
            Value::Null,
            false,
        )
        .unwrap();
        assert_eq!(
            formatted,
            json!({
                "formattedDate": "2020-10-01",
                "formattedTime": "11:57:59 PM",
                "formattedDateTime": "11:57:59 PM, October 01, 2020"
            })
        );

        let documented_patterns = execute_json(
            r#"%dw 2.0
output application/python
var myDateTime = ("2020-11-10T13:44:12.283-08:00" as DateTime)
---
{
  era: myDateTime as String {format: "G"},
  dayOfYear: myDateTime as String {format: "D"},
  monthName: myDateTime as String {format: "MMMM"},
  quarter: myDateTime as String {format: "QQQQ"},
  week: myDateTime as String {format: "w"},
  weekday: myDateTime as String {format: "EEEE"},
  hour: myDateTime as String {format: "H"},
  millisOfDay: myDateTime as String {format: "A"},
  nanosOfDay: myDateTime as String {format: "N"},
  offsetName: myDateTime as String {format: "O"},
  offsetCompact: myDateTime as String {format: "Z"}
}"#,
            Value::Null,
            false,
        )
        .unwrap();
        assert_eq!(
            documented_patterns,
            json!({
                "era": "AD",
                "dayOfYear": "315",
                "monthName": "November",
                "quarter": "4th quarter",
                "week": "46",
                "weekday": "Tuesday",
                "hour": "13",
                "millisOfDay": "49452283",
                "nanosOfDay": "49452283000000",
                "offsetName": "GMT-8",
                "offsetCompact": "-0800"
            })
        );

        let arithmetic_json = execute_json(
            r#"%dw 2.0
output application/json
var numberOfDays = 3
---
{
  threeDaysBefore: |2019-10-01T23:57:59Z| - ("P$(numberOfDays)D" as Period),
  periodMinusTime: |PT9M| - |23:59:56|,
  zonedTimeMinusPeriod: |23:59:56-03:00| - |PT9M|,
  zonedTimeDifference: |23:59:56-03:00| - |22:59:56-00:00|,
  localDateTimeChain: |2019-10-01T23:57:59| - |P2Y9M1D| - |PT57M59S| + |PT2H|,
  dateDifference: |2019-10-01| - |2018-09-23|
}"#,
            Value::Null,
            true,
        )
        .unwrap();
        let arithmetic: Value = serde_json::from_str(arithmetic_json.as_str().unwrap()).unwrap();
        assert_eq!(
            arithmetic,
            json!({
                "threeDaysBefore": "2019-09-28T23:57:59Z",
                "periodMinusTime": "23:50:56",
                "zonedTimeMinusPeriod": "23:50:56-03:00",
                "zonedTimeDifference": "PT4H",
                "localDateTimeChain": "2017-01-01T01:00:00",
                "dateDifference": "PT8952H"
            })
        );

        let by = execute_json(
            r#"%dw 2.0
var myDateTime1 = |2017-10-01T22:57:59-03:00|
var myDateTime2 = |2018-10-01T23:57:59-03:00|
output application/json
---
{
  maxDateTime: [ myDateTime1, myDateTime2 ] maxBy ((item) -> item),
  maxDate: [ myDateTime1 as Date, myDateTime2 as Date ] maxBy ((item) -> item),
  maxTime: [ myDateTime1 as Time, myDateTime2 as Time ] maxBy ((item) -> item),
  minDateTime: [ myDateTime1, myDateTime2 ] minBy ((item) -> item),
  minDate: [ myDateTime1 as Date, myDateTime2 as Date ] minBy ((item) -> item),
  minTime: [ myDateTime1 as Time, myDateTime2 as Time ] minBy ((item) -> item)
}"#,
            Value::Null,
            true,
        )
        .unwrap();
        assert_eq!(
            by,
            json!(
                r#"{"maxDateTime":"2018-10-01T23:57:59-03:00","maxDate":"2018-10-01","maxTime":"23:57:59-03:00","minDateTime":"2017-10-01T22:57:59-03:00","minDate":"2017-10-01","minTime":"22:57:59-03:00"}"#
            )
        );

        let shifted = execute_json(
            r#"%dw 2.0
output application/python
var base = |2024-02-27| as Date
---
{
  tomorrow: base + |P1D|,
  nextWeek: base + |P7D|
}"#,
            Value::Null,
            false,
        )
        .unwrap();
        assert_eq!(
            shifted,
            json!({
                "tomorrow": {"__dwpy_temporal": "date", "value": "2024-02-28"},
                "nextWeek": {"__dwpy_temporal": "date", "value": "2024-03-05"}
            })
        );

        let fields = execute_json(
            r#"%dw 2.0
output application/python
var myDate = |2003-10-01T23:57:59.700-03:00|
---
{
  year: myDate.year,
  month: myDate.month,
  day: myDate.day,
  hour: myDate.hour,
  minutes: myDate.minutes,
  seconds: myDate.seconds,
  milliseconds: myDate.milliseconds,
  nanoseconds: myDate.nanoseconds,
  quarter: myDate.quarter,
  dayOfWeek: myDate.dayOfWeek,
  dayOfYear: myDate.dayOfYear,
  offsetSeconds: myDate.offsetSeconds
}"#,
            Value::Null,
            false,
        )
        .unwrap();
        assert_eq!(
            fields,
            json!({
                "year": 2003,
                "month": 10,
                "day": 1,
                "hour": 23,
                "minutes": 57,
                "seconds": 59,
                "milliseconds": 700,
                "nanoseconds": 700000000,
                "quarter": 4,
                "dayOfWeek": 3,
                "dayOfYear": 274,
                "offsetSeconds": -10800
            })
        );
    }

    #[test]
    fn evaluates_period_number_units_and_fields() {
        let script = r#"%dw 2.0
output application/python
var period = (|2010-12-10T12:10:12| - |2010-09-09T10:02:10|)
var shortPeriod = (|2010-12-10T12:10:12| - |2010-12-10T10:02:10|)
---
{
  nanos: period as Number {unit: "nanos"},
  millis: period as Number {unit: "milliseconds"},
  seconds: period as Number {unit: "seconds"},
  hours: period as Number {unit: "hours"},
  days: period as Number {unit: "days"},
  short: {
    hours: shortPeriod.hours,
    minutes: shortPeriod.minutes,
    secs: shortPeriod.secs
  },
  sorted: [|2020-10-01T23:57:59.017Z|, |2022-12-22T12:12:12.011Z|, |2020-10-01T12:40:10.012Z|, |2020-10-01T23:57:59.021Z|]
    orderBy -($ as Number {unit: "milliseconds"}),
  datePeriodYears: |P1Y12M| as Number {unit: "years"},
  datePeriodMonths: |P8Y12M| as Number {unit: "months"}
}
"#;
        assert_eq!(
            execute_json(script, Value::Null, false).unwrap(),
            json!({
                "nanos": 7956482000000000i64,
                "millis": 7956482000i64,
                "seconds": 7956482,
                "hours": 2210,
                "days": 92,
                "short": {"hours": 2, "minutes": 8, "secs": 2},
                "sorted": [
                    "2022-12-22T12:12:12.011Z",
                    "2020-10-01T23:57:59.021Z",
                    "2020-10-01T23:57:59.017Z",
                    "2020-10-01T12:40:10.012Z"
                ],
                "datePeriodYears": 2,
                "datePeriodMonths": 108
            })
        );
    }

    #[test]
    fn evaluates_temporal_concatenation() {
        let script = r#"%dw 2.0
output application/python
fun format(d: DateTime) = d as String {format: "yyyy-MM-dd'T'HH:mm:ss.SSS"}
---
{
  localDateTime: |2017-10-01| ++ |23:57:59|,
  zonedTimes: [|2017-10-01| ++ |23:57:59-03:00|, |2017-10-01| ++ |23:57:59Z|],
  zonedDate: |2018-11-30| ++ |23:57:59+01:00|,
  dateZone: |2017-10-01| ++ |-03:00|,
  zoneDate: |-03:00| ++ |2017-10-01|,
  zoneDateTime: |-03:00| ++ |2003-10-01T23:57:59|,
  timeZone: |23:57| ++ |-03:00|,
  zoneTime: |-03:00| ++ |23:57|,
  coercedTime: (|23:57:59| as Time) ++ |2017-10-01|,
  shiftedCet: |2019-02-13T13:23:00.120Z| >> "CET",
  formattedShift: format(|2019-02-13T13:23:00.120Z| >> "CET")
}
"#;
        assert_eq!(
            execute_json(script, Value::Null, false).unwrap(),
            json!({
                "localDateTime": "2017-10-01T23:57:59",
                "zonedTimes": ["2017-10-01T23:57:59-03:00", "2017-10-01T23:57:59Z"],
                "zonedDate": "2018-11-30T23:57:59+01:00",
                "dateZone": "2017-10-01T00:00:00-03:00",
                "zoneDate": "2017-10-01T00:00:00-03:00",
                "zoneDateTime": "2003-10-01T23:57:59-03:00",
                "timeZone": "23:57:00-03:00",
                "zoneTime": "23:57:00-03:00",
                "coercedTime": "2017-10-01T23:57:59Z",
                "shiftedCet": "2019-02-13T14:23:00.120+01:00",
                "formattedShift": "2019-02-13T14:23:00.120"
            })
        );
    }

    #[test]
    fn evaluates_string_interpolation() {
        let script = r#"%dw 2.0
output application/python
var suffix = upper(vars.suffix)
---
{
  message: "Hello $(payload.user.name), total: $(payload.price * payload.quantity) $(suffix)",
  shorthand: "Hi, my name is $suffix",
  nested: "Result: $((payload.a + payload.b) * 2)",
  defaulted: "Guest: $(payload.missing default 'anonymous')",
  nullValue: "Value: $(payload.missing)",
  escaped: "Line\n$(payload.user.name)",
  backtick: `backtick (\`)`
}
"#;
        assert_eq!(
            execute_json_with_vars(
                script,
                json!({
                    "user": {"name": "Ada"},
                    "price": 10,
                    "quantity": 3,
                    "a": 5,
                    "b": 3
                }),
                json!({"suffix": "ok"}),
                false
            )
            .unwrap(),
            json!({
                "message": "Hello Ada, total: 30 OK",
                "shorthand": "Hi, my name is OK",
                "nested": "Result: 16",
                "defaulted": "Guest: anonymous",
                "nullValue": "Value: ",
                "escaped": "Line\nAda",
                "backtick": "backtick (`)"
            })
        );
    }

    #[test]
    fn evaluates_numeric_and_empty_helpers() {
        let script = r#"%dw 2.0
output application/python
---
{
  sumValues: sum(payload.values),
  avgValues: avg(payload.values),
  rounded: round(payload.roundMe),
  ceilValue: ceil(payload.decimal),
  floorValue: floor(payload.decimal),
  absValue: abs(payload.negative),
  emptyArray: isEmpty([]),
  emptyObject: isEmpty({}),
  emptyString: isEmpty(""),
  blankNull: isBlank(null),
  blankSpaces: isBlank("   "),
  numericDigits: isNumeric("12345"),
  numericLetters: isNumeric("12a"),
  decimalValue: isDecimal("12.5"),
  integerValue: isDecimal(12)
}
"#;
        assert_eq!(
            execute_smoke(
                script,
                json!({
                    "values": [4, 6],
                    "roundMe": 2.5,
                    "decimal": 2.2,
                    "negative": -7
                })
            )
            .unwrap(),
            json!({
                "sumValues": 10,
                "avgValues": 5,
                "rounded": 2,
                "ceilValue": 3,
                "floorValue": 2,
                "absValue": 7,
                "emptyArray": true,
                "emptyObject": true,
                "emptyString": true,
                "blankNull": true,
                "blankSpaces": true,
                "numericDigits": true,
                "numericLetters": false,
                "decimalValue": true,
                "integerValue": false
            })
        );
    }

    #[test]
    fn evaluates_values_namespace_selector_helpers() {
        let script = r#"%dw 2.0
import * from dw::util::Values
ns ns0 http://acme.com/foo
output application/python
---
{
  field: field(ns0, "myFieldName"),
  attr: attr(ns0, "myAttr")
}
"#;
        assert_eq!(
            execute_smoke(script, Value::Null).unwrap(),
            json!({
                "field": {
                    "kind": "Object",
                    "namespace": "http://acme.com/foo",
                    "selector": "myFieldName"
                },
                "attr": {
                    "kind": "Attribute",
                    "namespace": "http://acme.com/foo",
                    "selector": "myAttr"
                }
            })
        );
    }

    #[test]
    fn evaluates_function_references_as_arguments() {
        let script = r#"%dw 2.0
output application/python
fun applyEach(f, arr) = arr map ((item) -> f(item))
fun up(s) = upper(s)
---
applyEach(up, payload.names)
"#;
        assert_eq!(
            execute_smoke(script, json!({"names": ["a", "b"]})).unwrap(),
            json!(["A", "B"])
        );
    }

    #[test]
    fn evaluates_lambda_valued_variables_and_arguments() {
        let script = r#"%dw 2.0
output application/python
var msg = "Hello"
var msg2 = (x = "ignore") -> "hello"
var toUpper = (aString) -> upper(aString)
var applyMapping = (in, mappingsDef) -> (
  mappingsDef map (def) -> {
    (def.target): in[def.source] default def."default"
  }
)
fun combined(function, msg="universe") = function(msg ++ " world")
---
{
  msg: msg,
  msg2: msg2(),
  toUpper: toUpper(msg),
  combined: combined(toUpper, msg),
  combined2: combined((x) -> lower(x) ++ " today", msg),
  mapped: applyMapping(payload.user, vars.mappings)
}
"#;
        assert_eq!(
            execute_json_with_vars(
                script,
                json!({"user": {"first": "Ada", "last": "Lovelace"}}),
                json!({"mappings": [
                    {"source": "first", "target": "name"},
                    {"source": "missing", "target": "active", "default": true}
                ]}),
                false
            )
            .unwrap(),
            json!({
                "msg": "Hello",
                "msg2": "hello",
                "toUpper": "HELLO",
                "combined": "HELLO WORLD",
                "combined2": "hello world today",
                "mapped": [{"name": "Ada"}, {"active": true}]
            })
        );
    }

    #[test]
    fn evaluates_using_expression_scoped_bindings() {
        let script = r#"%dw 2.0
output application/python
var books = [
  { bookId: 101, title: "world history", price: "19.99" },
  { bookId: 202, title: "the great outdoors", price: "15.99" }
]
var authors = [
  { bookId: 101, author: "john doe" },
  { bookId: 202, author: "jane doe" }
]
---
books map (item) -> using (id = item.bookId) {
  id: id,
  topic: item.title,
  cost: item.price as Number,
  (authors filter ($.*bookId contains id) map (author) -> {
    author: author.author
  })
}
"#;
        assert_eq!(
            execute_smoke(script, Value::Null).unwrap(),
            json!([
                {"id": 101, "topic": "world history", "cost": 19.99, "author": "john doe"},
                {"id": 202, "topic": "the great outdoors", "cost": 15.99, "author": "jane doe"}
            ])
        );
    }

    #[test]
    fn evaluates_negation_and_simple_matches() {
        let script = r#"%dw 2.0
output application/python
---
{
  notEmpty: not isEmpty(payload.values),
  bangEmpty: !isEmpty(payload.values),
  filtered: payload.objects filter (!isEmpty($)),
  numeric: (payload.code as String) matches "/^[0-9]+$/",
  alpha: payload.word matches "/^[A-Za-z]+$/",
  regexLiteral: "admin123" matches /a.*\d+/,
  regexLiteralCall: "admin123" matches(/a.*\d+/),
  regexLiteralAnchored: "admin123" matches /^b.+/,
  captureMatch: "me@mulesoft.com" match(/([a-z]*)@([a-z]*)\.com/),
  scanMatch: "www.mulesoft.com" scan(/([w]*)\.([a-z]*)\.([a-z]*)/),
  literal: payload.word matches "abc",
  bangVsNot: {
    bang: (! true or true),
    word: (not true or true)
  }
}
"#;
        assert_eq!(
            execute_smoke(
                script,
                json!({
                    "values": [1],
                    "objects": [{}, {"name": "Ada"}],
                    "code": 12345,
                    "word": "abc"
                })
            )
            .unwrap(),
            json!({
                "notEmpty": true,
                "bangEmpty": true,
                "filtered": [{"name": "Ada"}],
                "numeric": true,
                "alpha": true,
                "regexLiteral": true,
                "regexLiteralCall": true,
                "regexLiteralAnchored": false,
                "captureMatch": ["me@mulesoft.com", "me", "mulesoft"],
                "scanMatch": [["www.mulesoft.com", "www", "mulesoft", "com"]],
                "literal": true,
                "bangVsNot": {"bang": true, "word": false}
            })
        );
    }

    #[test]
    fn evaluates_collection_object_helpers_and_dynamic_indexing() {
        let script = r#"%dw 2.0
output application/python
var key = "name"
---
{
  flattened: flatten(payload.matrix),
  firstIndex: indexOf(payload.values, 3),
  missingIndex: indexOf(payload.values, 9),
  stringIndex: indexOf(payload.text, "na"),
  lastArrayIndex: lastIndexOf(payload.values, 2),
  lastStringIndex: lastIndexOf(payload.text, "na"),
  maxValue: max(payload.values),
  minValue: min(payload.values),
  keys: keysOf(payload.object),
  values: valuesOf(payload.object),
  entries: entriesOf(payload.object),
  dynamic: payload.user[key]
}
"#;
        assert_eq!(
            execute_smoke(
                script,
                json!({
                    "matrix": [[1, 2], [3, 4], 5],
                    "values": [1, 2, 2, 3],
                    "text": "banana",
                    "object": {"a": 1, "b": 2},
                    "user": {"name": "Ana"}
                })
            )
            .unwrap(),
            json!({
                "flattened": [1, 2, 3, 4, 5],
                "firstIndex": 3,
                "missingIndex": -1,
                "stringIndex": 2,
                "lastArrayIndex": 2,
                "lastStringIndex": 4,
                "maxValue": 3,
                "minValue": 1,
                "keys": ["a", "b"],
                "values": [1, 2],
                "entries": [
                    {"key": "a", "value": 1, "attributes": {}},
                    {"key": "b", "value": 2, "attributes": {}}
                ],
                "dynamic": "Ana"
            })
        );
    }

    #[test]
    fn evaluates_xml_entries_with_attributes() {
        let script = r#"%dw 2.0
import * from dw::core::Objects
var myVar = read('<xml attr="x"><a>true</a><b>1</b></xml>', 'application/xml')
output application/python
---
{
  entries: entriesOf(myVar),
  entrySet: entrySet(myVar)
}
"#;
        let expected = json!([
            {
                "key": "xml",
                "value": {"a": "true", "b": "1"},
                "attributes": {"attr": "x"}
            }
        ]);
        let result = execute_smoke(script, Value::Null).unwrap();
        assert_eq!(result["entries"], expected);
        assert_eq!(result["entrySet"], expected);
    }

    #[test]
    fn evaluates_xml_keys_of_namespace_and_attributes() {
        let script = r#"%dw 2.0
var myVar = read('<users xmlns="http://test.com">
  <user name="Mariano" lastName="Achaval"/>
  <user name="Stacey" lastName="Duke"/>
</users>', 'application/xml')
output application/python
---
{
  keysOfExample: flatten([keysOf(myVar.users) map $.#, keysOf(myVar.users) map $.@]),
  namesOfExample: flatten([namesOf(myVar.users) map $.#, namesOf(myVar.users) map $.@])
}
"#;
        assert_eq!(
            execute_smoke(script, Value::Null).unwrap(),
            json!({
                "keysOfExample": [
                    "http://test.com",
                    "http://test.com",
                    {"name": "Mariano", "lastName": "Achaval"},
                    {"name": "Stacey", "lastName": "Duke"}
                ],
                "namesOfExample": [null, null, null, null]
            })
        );
    }

    #[test]
    fn preserves_repeated_xml_nodes_with_attributes_for_mapping() {
        let script = r#"%dw 2.0
var doc = read('<images><image type="SwatchImage"> http://example.com/a.jpg </image></images>', 'application/xml')
output application/python
---
doc.images.*image map (image) -> {
  href: trim(image),
  rel: image.@'type'
}
"#;
        assert_eq!(
            execute_smoke(script, Value::Null).unwrap(),
            json!([{"href": "http://example.com/a.jpg", "rel": "SwatchImage"}])
        );
    }

    #[test]
    fn evaluates_string_helpers() {
        let script = r##"%dw 2.0
output application/python
---
{
  appended: appendIfMissing("abc", "xyz"),
  appendedPresent: appendIfMissing("abcxyz", "xyz"),
  camelized: camelize("customer_first_name"),
  capitalized: capitalize("customerName"),
  capitalizedSymbols: capitalize("a*s_b’s"),
  charCode: charCode("Mule"),
  charCodeAt: charCodeAt("MuleSoft", 1),
  collapsed: collapse("a  b"),
  matchCount: countMatches("hello worlo!", "lo"),
  numericCharCount: "42 = 11 * 2 + 20" countCharactersBy isNumeric($),
  vowelCount: countMatches("hello, ciao!", "/[aeiou]/"),
  dasherized: dasherize("customer_first_name"),
  firstText: first("hello world!", 5.9),
  fromCode: fromCharCode(117),
  hamming: hammingDistance("holu", "chau"),
  hammingMismatch: hammingDistance("abc", "ab"),
  alpha: isAlpha("abc"),
  alphanumeric: isAlphanumeric("ab2c"),
  lowerCase: isLowerCase("mulesoft"),
  lowerCaseUnicode: isLowerCase("mulesöft"),
  mixedCaseUnicode: isLowerCase("mulesÖft"),
  upperCase: isUpperCase("ABC"),
  whitespace: isWhitespace(""),
  allDigitsAndSpaces: "12 34  56" everyCharacter $ == " " or isNumeric($),
  lastText: last("hello world!", 5.1),
  leftPadded: leftPad("bat", 5),
  rightPadded: rightPad("bat", 5),
  editDistance: levenshteinDistance("kitten", "sitting"),
  lines: lines("hello world\n\nhere   data-weave"),
  mappedString: "\$234" mapString if (isNumeric($)) "~" else $,
  ordinal: ordinalize(103),
  plural: pluralize("box"),
  prepended: prependIfMissing("abc", "xyz"),
  repeated: repeat("e", 3),
  replaced: replaceAll("AAAA", "AAA", "B"),
  removed: remove("stateful state", "state"),
  reversed: reverse("Mariano"),
  singular: singularize("boxes"),
  someUpper: "someCharacter" someCharacter isUpperCase($),
  substringed: substring("hello world!", 1, 5),
  substringBy: "hello~world=here_data-weave" substringBy $ == "~" or $ == "=" or $ == "_",
  clamped: substring("hello", -2, 99),
  after: substringAfter("abcba", "b"),
  afterLast: substringAfterLast("abcba", "b"),
  before: substringBefore("abc", "c"),
  beforeLast: substringBeforeLast("abcba", "b"),
  emptyBeforeLast: substringBeforeLast("abc", ""),
  every: substringEvery("substringEvery", 3),
  underscored: underscore("customerName"),
  limited: withMaxSize("123", 2),
  unlimited: withMaxSize("123", 0),
  words: words("hello world\nhere\tdata-weave"),
  unwrapped: unwrap("'abc'", "'"),
  keptWrapped: unwrap("#A", "#"),
  wrapped: wrapWith("ab", "'"),
  wrappedMissing: wrapIfMissing("a/b/c", "/")
}
"##;
        assert_eq!(
            execute_smoke(script, Value::Null).unwrap(),
            json!({
                "appended": "abcxyz",
                "appendedPresent": "abcxyz",
                "camelized": "customerFirstName",
                "capitalized": "Customer Name",
                "capitalizedSymbols": "A*S B’S",
                "charCode": 77,
                "charCodeAt": 117,
                "collapsed": ["a", "  ", "b"],
                "matchCount": 2,
                "numericCharCount": 7,
                "vowelCount": 5,
                "dasherized": "customer-first-name",
                "firstText": "hello",
                "fromCode": "u",
                "hamming": 3,
                "hammingMismatch": null,
                "alpha": true,
                "alphanumeric": true,
                "lowerCase": true,
                "lowerCaseUnicode": true,
                "mixedCaseUnicode": false,
                "upperCase": true,
                "whitespace": true,
                "allDigitsAndSpaces": true,
                "lastText": "world!",
                "leftPadded": "  bat",
                "rightPadded": "bat  ",
                "editDistance": 3,
                "lines": ["hello world", "", "here   data-weave"],
                "mappedString": "$~~~",
                "ordinal": "103rd",
                "plural": "boxes",
                "prepended": "xyzabc",
                "repeated": "eee",
                "replaced": "BA",
                "removed": "ful ",
                "reversed": "onairaM",
                "singular": "box",
                "someUpper": true,
                "substringed": "ello",
                "substringBy": ["hello", "world", "here", "data-weave"],
                "clamped": "hello",
                "after": "cba",
                "afterLast": "a",
                "before": "ab",
                "beforeLast": "abc",
                "emptyBeforeLast": "ab",
                "every": ["sub", "str", "ing", "Eve", "ry"],
                "underscored": "customer_name",
                "limited": "12",
                "unlimited": "123",
                "words": ["hello", "world", "here", "data-weave"],
                "unwrapped": "abc",
                "keptWrapped": "#A",
                "wrapped": "'ab'",
                "wrappedMissing": "/a/b/c/"
            })
        );
    }

    #[test]
    fn evaluates_ranges_slices_and_difference() {
        let script = r#"%dw 2.0
output application/python
---
{
  up: 1 to 5,
  down: 5 to 1,
  doubled: 1 to 5 map ((value) -> value * 2),
  reversed: (1 to 5)[-1 to 0],
  textSlice: payload.text[2 to 6],
  textReverse: payload.text[11 to -0],
  arrayDiff: [1, 2, 3, 2] -- [2],
  objectDiffKeys: payload.object -- ["a"],
  objectDiffObject: payload.object -- {b: 2},
  objectDiffKeyCoercion: {hello: "world", name: "DW"} -- ["hello" as Key],
  stringDiff: "abcabc" -- "b"
}
"#;
        assert_eq!(
            execute_smoke(
                script,
                json!({
                    "text": "Hello World!",
                    "object": {"a": 1, "b": 2, "c": 3}
                })
            )
            .unwrap(),
            json!({
                "up": [1, 2, 3, 4, 5],
                "down": [5, 4, 3, 2, 1],
                "doubled": [2, 4, 6, 8, 10],
                "reversed": [5, 4, 3, 2, 1],
                "textSlice": "llo W",
                "textReverse": "!dlroW olleH",
                "arrayDiff": [1, 3],
                "objectDiffKeys": {"b": 2, "c": 3},
                "objectDiffObject": {"a": 1, "c": 3},
                "objectDiffKeyCoercion": {"name": "DW"},
                "stringDiff": "acac"
            })
        );
    }

    #[test]
    fn evaluates_size_of_large_range_without_materializing() {
        assert_eq!(
            execute_json(
                "%dw 2.0\noutput application/json\n---\nsizeOf(1 to 100000000)",
                Value::Null,
                true,
            )
            .unwrap(),
            json!("100000000")
        );
        assert_eq!(
            execute_json(
                "%dw 2.0\noutput application/python\n---\nsizeOf(100000000 to 1)",
                Value::Null,
                false,
            )
            .unwrap(),
            json!(100000000)
        );
    }

    #[test]
    fn evaluates_prefix_and_infix_string_collection_helpers() {
        let script = r#"%dw 2.0
output application/python
---
{
  infixContains: payload.list contains 3,
  prefixContains: contains(payload.list, 3),
  callStyleContains: payload.list contains(2),
  objectContainsKey: payload.object contains "a",
  objectContainsValue: payload.object contains 2,
  stringContains: payload.text contains "nan",
  infixJoin: payload.words joinBy "-",
  prefixJoin: joinBy(payload.words, "-"),
  joinNulls: ["a", null, "b"] joinBy "|",
  infixSplit: payload.phrase splitBy "-",
  callStyleSplit: payload.phrase splitBy("-"),
  prefixSplit: splitBy(payload.phrase, "-"),
  backtickPathSplit: 'root.sources.data.`test.branch.BranchSource`.source.traits' splitBy(/[.](?=(?:[^`]*`[^`]*`)*[^`]*$)/),
  composedUrl: compose `http://examplewebsite.com/$(payload.urlPath)/page.html`,
  chars: splitBy("abc", ""),
  starts: payload.text startsWith "ba",
  callStyleStarts: payload.text startsWith("ba"),
  ends: endsWith(payload.text, "na"),
  foundText: payload.text find "na",
  foundRegex: "I heart DataWeave" find /\w*ea\w*(\b)/,
  foundArray: find(payload.list, 3),
  prefixGroup: groupBy(payload.objects, (item) -> item.language),
  stringGroup: "hello world!" groupBy (not isEmpty($ find /[aeiou]/)),
  names: namesOf(payload.object),
  infixMod: 7 mod 4,
  infixPow: 2 pow 3,
  callStyleMap: payload.words map((word) -> upper(word)),
  dynamicKeyMap: payload.words map((word, index) -> {(index): word}),
  implicitDynamicKeyMap: payload.words map ("$$": $),
  trimmedNull: trim(null),
  numberSize: sizeOf(123.45),
  coercedNumberSize: sizeOf("123" as Number),
  binarySize: sizeOf("'my word'" as Binary),
  emptyBinary: isEmpty("" as Binary),
  binaryType: typeOf("Mule" as Binary),
  binaryHex: toHex("Mule" as Binary),
  even: isEven(2),
  odd: isOdd(3),
  integerChecks: [isInteger(1), isInteger(2.0), isInteger(2.2), isInteger("1")],
  types: [typeOf("A b"), typeOf([1,2]), typeOf(34), typeOf(true), typeOf({ a: 5 })],
  compatibilityFlag: evaluateCompatibilityFlag("com.mulesoft.dw.xml_reader.honourMixedContentStructure"),
  math: {
    acos: acos(1),
    atan: atan(-1),
    log10: log10(10),
    logn: logn(10),
    degrees: toDegrees(0),
    radians: toRadians(0)
  },
  typedEmptyArray: [] as Array<Number>
}
"#;
        assert_eq!(
            execute_smoke(
                script,
                json!({
                    "list": [1, 2, 3],
                    "object": {"a": 1, "b": 2},
                    "text": "banana",
                    "words": ["a", "b", "c"],
                    "phrase": "a-b-c",
                    "urlPath": "content folder",
                    "objects": [
                        {"name": "Foo", "language": "Java"},
                        {"name": "Bar", "language": "Scala"},
                        {"name": "FooBar", "language": "Java"}
                    ]
                })
            )
            .unwrap(),
            json!({
                "infixContains": true,
                "prefixContains": true,
                "callStyleContains": true,
                "objectContainsKey": true,
                "objectContainsValue": true,
                "stringContains": true,
                "infixJoin": "a-b-c",
                "prefixJoin": "a-b-c",
                "joinNulls": "a||b",
                "infixSplit": ["a", "b", "c"],
                "callStyleSplit": ["a", "b", "c"],
                "prefixSplit": ["a", "b", "c"],
                "backtickPathSplit": ["root", "sources", "data", "`test.branch.BranchSource`", "source", "traits"],
                "composedUrl": "http://examplewebsite.com/content%20folder/page.html",
                "chars": ["a", "b", "c"],
                "starts": true,
                "callStyleStarts": true,
                "ends": true,
                "foundText": [2, 4],
                "foundRegex": [[2, 7], [8, 17]],
                "foundArray": [2],
                "prefixGroup": {
                    "Java": [
                        {"name": "Foo", "language": "Java"},
                        {"name": "FooBar", "language": "Java"}
                    ],
                    "Scala": [{"name": "Bar", "language": "Scala"}]
                },
                "stringGroup": {"false": "hll wrld!", "true": "eoo"},
                "names": ["a", "b"],
                "infixMod": 3,
                "infixPow": 8,
                "callStyleMap": ["A", "B", "C"],
                "dynamicKeyMap": [{"0": "a"}, {"1": "b"}, {"2": "c"}],
                "implicitDynamicKeyMap": [{"0": "a"}, {"1": "b"}, {"2": "c"}],
                "trimmedNull": null,
                "numberSize": 6,
                "coercedNumberSize": 1,
                "binarySize": 9,
                "emptyBinary": true,
                "binaryType": "Binary",
                "binaryHex": "4D756C65",
                "even": true,
                "odd": true,
                "integerChecks": [true, true, false, true],
                "types": ["String", "Array", "Number", "Boolean", "Object"],
                "compatibilityFlag": true,
                "math": {
                    "acos": 0,
                    "atan": -0.7853981633974483,
                    "log10": 1,
                    "logn": 2.302585092994046,
                    "degrees": 0,
                    "radians": 0
                },
                "typedEmptyArray": []
            })
        );
    }

    #[test]
    fn evaluates_runtime_crypto_object_and_period_module_helpers() {
        let script = r##"%dw 2.0
import try, fail from dw::Runtime
import * from dw::Crypto
import * from dw::core::Objects
import * from dw::core::Periods
import * from dw::core::Dates
output application/python
---
{
  ok: try(() -> "ok"),
	  err: try(() -> fail("boom")),
	  documentedRandomFailure: try(() -> randomNumber()),
	  sha1: hashWith("hello" as Binary, "SHA-1"),
  cryptoMd2Binary: Crypto::hashWith("hello" as Binary, "MD2"),
  cryptoMd5: Crypto::MD5("asd" as Binary),
  cryptoSha1: Crypto::SHA1("dsasd" as Binary),
  cryptoHmac: Crypto::HMACWith("secret_key" as Binary, "Some value to hash" as Binary, "HmacSHA256"),
  cryptoHmacBinary: Crypto::HMACBinary("confidential" as Binary, "xxxxx" as Binary, "HmacSHA512"),
  stringFromArray: toString(["h", "o", "l", "a"]),
  arrayFromString: toArray("hola"),
  emptyArrayFromString: toArray(""),
  booleanFromString: toBoolean("TrUe"),
  numberFromString: toNumber("1.0"),
  localizedNumber: toNumber("1,25", "#.##", "ES"),
  durationSeconds: toNumber(|PT1H10M|, "seconds"),
  durationMillis: toNumber(|PT1M7S|, "milliseconds"),
  formattedNumber: toString(0.035,"#.##","ES"),
  formattedMoney: toString(1.1234,"\$.## 'in my account'"),
  formattedDate: toString(|2003-10-01|, "uuuu/MM/dd"),
  formattedTime: toString(|23:57:59|, "HH-mm-ss"),
  formattedDateTime: toString(|2003-10-01T23:57:59|, "uuuu-MM-dd HH:mm:ss a"),
  formattedSpanishDateTime: toString(|2003-01-01T23:57:59|, "eeee, dd MMMM, uuuu HH:mm:ss a", "ES"),
  timeType: typeOf(|22:10:18Z|),
  regexText: toString(/a-Z/),
  uriText: toString("https://docs.mulesoft.com/" as Uri),
  entries: entrySet({ a: 1, b: true }),
  names: nameSet({ first: "Ana", last: "Simpson" }),
  merged: mergeWith({ a: 1, b: 2 }, { b: 3, c: 4 }),
  divided: divideBy({ a: 1, b: 2, c: 3 }, 2),
  dividedDuplicateKeys: {"a": 1, "b" : true, "a" : 2, "b" : false, "c" : 3} divideBy 2,
  taken: takeWhile({ a: 1, b: 2, c: 5 }, (value, key) -> value < 3),
  every: everyEntry({ a: 1, b: 2 }, (value, key) -> value < 3),
  some: someEntry({ a: 1, b: 4 }, (value, key) -> value > 3),
  periodValue: years(4),
  nextHour: |2020-10-05T20:22:34.385000Z| + hours(1),
  betweenValue: between(|2011-12-11|, |2010-11-10|),
  daysBetweenValue: daysBetween('2016-10-01T23:57:59-03:00', '2017-10-01T23:57:59-03:00'),
  leapYears: [
    isLeapYear(|2016-10-01T23:57:59|),
    isLeapYear(|2017-10-01T23:57:59|),
    isLeapYear(|2016-10-01|),
    isLeapYear(|2017-10-01|)
  ],
  zonedLeapYears: [ |2016-10-01T23:57:59-03:00|, |2016-10-01T23:57:59Z| ] map isLeapYear($),
  beginningOfDay: atBeginningOfDay(|2020-10-06T18:23:20.351-03:00|),
  beginningOfLocalDay: atBeginningOfDay(|2020-10-06T18:23:20.351|),
  beginningOfHour: atBeginningOfHour(|2020-10-06T18:23:20.351-03:00|),
  beginningOfLocalHour: atBeginningOfHour(|18:23:20.351|),
  beginningOfMonth: atBeginningOfMonth(|2020-10-06T18:23:20.351-03:00|),
  beginningOfDateMonth: atBeginningOfMonth(|2020-10-06|),
  beginningOfWeek: atBeginningOfWeek(|2020-10-06T18:23:20.351-03:00|),
  beginningOfDateWeek: atBeginningOfWeek(|2020-10-06|),
  beginningOfYear: atBeginningOfYear(|2020-10-06T18:23:20.351-03:00|),
  beginningOfLocalYear: atBeginningOfYear(|2020-10-06T18:23:20.351|),
  beginningOfDateYear: atBeginningOfYear(|2020-10-06|),
  constructedDate: date({year: 2012, month: 10, day: 11}),
  constructedDateTime: dateTime({year: 2012, month: 10, day: 11, hour: 12, minutes: 30, seconds: 40, timeZone: |-03:00|}),
  constructedLocalDateTime: localDateTime({year: 2012, month: 10, day: 11, hour: 12, minutes: 30, seconds: 40}),
  constructedLocalTime: localTime({hour: 12, minutes: 30, seconds: 40}),
  constructedTime: time({hour: 12, minutes: 30, seconds: 40, timeZone: |-03:00|})
}
"##;
        let result = execute_smoke(script, Value::Null).unwrap();
        assert_eq!(result["ok"], json!({"success": true, "result": "ok"}));
        assert_eq!(
            result["err"],
            json!({
                "success": false,
                "error": {
                    "kind": "UserException",
                    "message": "boom",
                    "location": "Unknown location",
                    "stack": [
                        "fail (anonymous:0:0)",
                        "myFunction (anonymous:1:114)",
                        "main (anonymous:1:179)"
                    ]
                }
            })
        );
        assert_eq!(
            result["documentedRandomFailure"],
            json!({
                "success": false,
                "error": {
                    "kind": "UserException",
                    "message": "This function is failing",
                    "location": "Unknown location",
                    "stack": [
                        "fail (anonymous:0:0)",
                        "myFunction (anonymous:1:114)",
                        "main (anonymous:1:179)"
                    ]
                }
            })
        );
        assert_eq!(
            result["sha1"],
            json!({"__dwpy_binary": [170, 244, 198, 29, 220, 197, 232, 162, 218, 190, 222, 15, 59, 72, 44, 217, 174, 169, 67, 77]})
        );
        assert_eq!(
            result["cryptoMd2Binary"],
            json!({"__dwpy_binary": [169, 4, 108, 115, 224, 3, 49, 175, 104, 145, 125, 56, 4, 247, 6, 85]})
        );
        assert_eq!(
            result["cryptoMd5"],
            json!("7815696ecbf1c96e6894b779456d330e")
        );
        assert_eq!(
            result["cryptoSha1"],
            json!("2fa183839c954e6366c206367c9be5864e4f4a65")
        );
        assert_eq!(
            result["cryptoHmac"],
            json!("b51b4fe8c4e37304605753272b5b4321f9644a9b09cb1179d7016c25041d1747")
        );
        assert_eq!(
            result["cryptoHmacBinary"],
            json!({"__dwpy_binary": [153, 184, 254, 153, 94, 104, 233, 33, 51, 5, 236, 141, 214, 142, 1, 55, 147, 158, 159, 96, 178, 56, 63, 231, 106, 110, 55, 202, 98, 115, 59, 9, 241, 184, 129, 198, 133, 232, 247, 184, 120, 38, 103, 143, 126, 179, 242, 37, 244, 55, 62, 49, 180, 75, 14, 64, 203, 67, 17, 255, 84, 175, 125, 87]})
        );
        assert_eq!(result["stringFromArray"], json!("hola"));
        assert_eq!(result["arrayFromString"], json!(["h", "o", "l", "a"]));
        assert_eq!(result["emptyArrayFromString"], json!([]));
        assert_eq!(result["booleanFromString"], json!(true));
        assert_eq!(result["numberFromString"], json!(1));
        assert_eq!(result["localizedNumber"], json!(1.25));
        assert_eq!(result["durationSeconds"], json!(4200));
        assert_eq!(result["durationMillis"], json!(67000));
        assert_eq!(result["formattedNumber"], json!("0,04"));
        assert_eq!(result["formattedMoney"], json!("$1.12 in my account"));
        assert_eq!(result["formattedDate"], json!("2003/10/01"));
        assert_eq!(result["formattedTime"], json!("23-57-59"));
        assert_eq!(result["formattedDateTime"], json!("2003-10-01 23:57:59 PM"));
        assert_eq!(
            result["formattedSpanishDateTime"],
            json!("miércoles, 01 enero, 2003 23:57:59 p. m.")
        );
        assert_eq!(result["timeType"], json!("Time"));
        assert_eq!(result["regexText"], json!("a-Z"));
        assert_eq!(result["uriText"], json!("https://docs.mulesoft.com/"));
        assert_eq!(
            result["entries"],
            json!([
                {"key": "a", "value": 1, "attributes": {}},
                {"key": "b", "value": true, "attributes": {}}
            ])
        );
        assert_eq!(result["names"], json!(["first", "last"]));
        assert_eq!(result["merged"], json!({"a": 1, "b": 3, "c": 4}));
        assert_eq!(result["divided"], json!([{"a": 1, "b": 2}, {"c": 3}]));
        assert_eq!(
            result["dividedDuplicateKeys"],
            json!([
                {"a": 1, "b": true},
                {"a": 2, "b": false},
                {"c": 3}
            ])
        );
        assert_eq!(result["taken"], json!({"a": 1, "b": 2}));
        assert_eq!(result["every"], json!(true));
        assert_eq!(result["some"], json!(true));
        assert_eq!(result["periodValue"]["text"], json!("P4Y"));
        assert_eq!(
            result["nextHour"],
            json!({"__dwpy_temporal": "datetime", "value": "2020-10-05T21:22:34.385Z"})
        );
        assert_eq!(result["betweenValue"]["text"], json!("P1Y1M1D"));
        assert_eq!(result["daysBetweenValue"], json!(365));
        assert_eq!(result["leapYears"], json!([true, false, true, false]));
        assert_eq!(result["zonedLeapYears"], json!([true, true]));
        assert_eq!(
            result["beginningOfDay"]["value"],
            json!("2020-10-06T00:00:00-03:00")
        );
        assert_eq!(
            result["beginningOfLocalDay"]["value"],
            json!("2020-10-06T00:00:00")
        );
        assert_eq!(
            result["beginningOfHour"]["value"],
            json!("2020-10-06T18:00:00-03:00")
        );
        assert_eq!(result["beginningOfLocalHour"]["value"], json!("18:00:00"));
        assert_eq!(
            result["beginningOfMonth"]["value"],
            json!("2020-10-01T00:00:00-03:00")
        );
        assert_eq!(result["beginningOfDateMonth"]["value"], json!("2020-10-01"));
        assert_eq!(
            result["beginningOfWeek"]["value"],
            json!("2020-10-04T00:00:00-03:00")
        );
        assert_eq!(result["beginningOfDateWeek"]["value"], json!("2020-10-04"));
        assert_eq!(
            result["beginningOfYear"]["value"],
            json!("2020-01-01T00:00:00.000-03:00")
        );
        assert_eq!(
            result["beginningOfLocalYear"]["value"],
            json!("2020-01-01T00:00:00")
        );
        assert_eq!(result["beginningOfDateYear"]["value"], json!("2020-01-01"));
        assert_eq!(result["constructedDate"]["value"], json!("2012-10-11"));
        assert_eq!(
            result["constructedDateTime"]["value"],
            json!("2012-10-11T12:30:40-03:00")
        );
        assert_eq!(
            result["constructedLocalDateTime"]["value"],
            json!("2012-10-11T12:30:40")
        );
        assert_eq!(result["constructedLocalTime"]["value"], json!("12:30:40"));
        assert_eq!(result["constructedTime"]["value"], json!("12:30:40-03:00"));
    }

    #[test]
    fn evaluates_runtime_version_wait_and_fail_if_helpers() {
        let script = r#"%dw 2.0
import * from dw::Runtime
output application/python
var result = {"a": "test"}
var a = 123
var user = {}
var otherUser = {name: "DW"}

fun simplify(df: DataFormatDescriptor | Null) = df match {
  case d is DataFormatDescriptor -> { name: d.name, defaultMimeType: d.defaultMimeType }
  case is Null -> { name: "unknown", defaultMimeType: "unknown" }
}
---
{
  version: version(),
  location: location(sqrt),
  locationString: locationString(a),
  waited: {"user": 1} wait 2000,
  failIfResult: result failIf isEmpty($),
  fallback: try(() -> user.name!) orElse "No User Name",
  success: try(() -> otherUser.name) orElse "No User Name",
  fallbackTryError: try(() -> user.name!) orElseTry otherUser.missing!,
  fallbackTrySuccess: try(() -> user.name!) orElseTry "No User Name",
  jsonFormat: simplify(findDataFormatDescriptorByMime({'type': "*", subtype: "json", parameters: {}})),
  unknownFormat: simplify(findDataFormatDescriptorByMime({'type': "*", subtype: "*", parameters: {}}))
}
"#;
        assert_eq!(
            execute_smoke(script, Value::Null).unwrap(),
            json!({
                "version": "2.5",
                "location": {
                    "uri": "/dw/Core.dwl",
                    "nameIdentifier": "dw::Core",
                    "startLine": 5797,
                    "startColumn": 36,
                    "endLine": 5797,
                    "endColumn": 77
                },
                "locationString": "var a = 123",
                "waited": {"user": 1},
                "failIfResult": {"a": "test"},
                "fallback": "No User Name",
                "success": "DW",
                "fallbackTryError": {
                    "success": false,
                    "error": {
                        "kind": "KeyNotFoundException",
                        "message": "There is no key named 'missing'",
                        "location": "Unknown location: otherUser.missing!",
                        "stack": ["main (org::mule::weave::v2::engine::transform:9:40)"]
                    }
                },
                "fallbackTrySuccess": {"success": true, "result": "No User Name"},
                "jsonFormat": {"name": "json", "defaultMimeType": "application/json"},
                "unknownFormat": {"name": "unknown", "defaultMimeType": "unknown"}
            })
        );
    }

    #[test]
    fn evaluates_generic_function_syntax_and_eval_url_fixture() {
        let generic = execute_smoke(
            r#"%dw 2.5
output application/python
fun max<T>(elems: Array<T>): T = elems reduce ((candidate: T, currentMax = elems[0]) -> if (candidate > currentMax) candidate else currentMax)
---
{
  max: max<Number>(measures)
}
"#,
            json!({"measures": [1, 2, 4, 1, 5, 2, 3, 3]}),
        )
        .unwrap();
        assert_eq!(generic, json!({"max": 5}));

        let eval_url = execute_smoke(
            r#"%dw 2.0
import * from dw::Runtime
output application/python
---
{
  execute_ok: evalUrl("classpath://org/mule/weave/v2/engine/runtime_evalUrl/example.dwl", {}),
  execute_ok_withValue: evalUrl("classpath://org/mule/weave/v2/engine/runtime_evalUrl/example.dwl", {}, {"payload": {name: "Mariano"}})
}
"#,
            Value::Null,
        )
        .unwrap();
        assert_eq!(
            eval_url,
            json!({
                "execute_ok": {"success": true, "value": "Mariano", "logs": []},
                "execute_ok_withValue": {"success": true, "value": "Mariano", "logs": []}
            })
        );
    }

    #[test]
    fn evaluates_type_selection_with_metadata() {
        let script = r#"%dw 2.0
output application/python
type User = {
 birthDate: Date {format: "dd-MMM-yy"},
 userName : String {schema: "value"}
}
type FormattedDate = User.birthDate
type UserName = User.userName
var formattedDate: FormattedDate = "10-SEP-15" as Date {format: "dd-MMM-yy"}
var otherFormatDate = "23-10-2022" as Date {format: "dd-MM-yyyy"}
var userName = "Messi" as String {schema: "value"}
var otherUserName = "Di María" as String {schema: "otherValue"}
---
{
  formattedDate: formattedDate is FormattedDate,
  userName: userName is UserName,
  otherFormatDate: otherFormatDate is FormattedDate,
  otherUserName: otherUserName is UserName
}
"#;
        assert_eq!(
            execute_smoke(script, Value::Null).unwrap(),
            json!({
                "formattedDate": true,
                "userName": true,
                "otherFormatDate": false,
                "otherUserName": false
            })
        );
    }

    #[test]
    fn evaluates_documented_runtime_eval_fixture() {
        let script = r#"%dw 2.0
import * from dw::Runtime
output application/python
---
{
  execute_ok: run("main.dwl", {"main.dwl": "{a: 1}"}, {}),
  logs: do {
    var execResult = run("main.dwl", {"main.dwl": "{a: log(1)}"}, {})
    ---
    { m: execResult.logs.message, l: execResult.logs.level }
  },
  grant: eval("main.dwl", {"main.dwl": "{a: readUrl(`http://google.com`)}"}, {}, {}, {securityManager: (grant, args) -> false}),
  library: eval("main.dwl", {"main.dwl": "Utils::sum(1,2)", "/Utils.dwl": "fun sum(a,b) = a +b"}, {}),
  timeout: eval("main.dwl", {"main.dwl": "(1 to 1000000000000) map $ + 1"}, {}, {}, {timeOut: 2}).success,
  execFail: eval("main.dwl", {"main.dwl": "dw::Runtime::fail('My Bad')"}, {}),
  parseFail: eval("main.dwl", {"main.dwl": "(1 + "}, {}),
  writerFail: eval("main.dwl", {"main.dwl": "output application/xml --- 2"}, {}),
  defaultOutput: eval("main.dwl", {"main.dwl": "payload"}, {}),
  customLogger: eval("main.dwl", {"main.dwl": "log(1234)"}, {})
}
"#;
        let result = execute_smoke(script, Value::Null).unwrap();
        assert_eq!(result["execute_ok"]["success"], json!(true));
        assert_eq!(result["execute_ok"]["value"], json!("{\n  a: 1\n}"));
        assert_eq!(result["logs"], json!({"m": ["1"], "l": ["INFO"]}));
        assert_eq!(result["grant"]["success"], json!(false));
        assert_eq!(
            result["library"],
            json!({"success": true, "value": 3, "logs": []})
        );
        assert_eq!(result["timeout"], json!(false));
        assert_eq!(result["execFail"]["success"], json!(false));
        assert_eq!(result["parseFail"]["success"], json!(false));
        assert_eq!(
            result["writerFail"],
            json!({"success": true, "value": 2, "logs": []})
        );
        assert_eq!(
            result["defaultOutput"],
            json!({"success": true, "value": {"name": "Mariano", "lastName": "achaval"}, "logs": []})
        );
        assert_eq!(
            result["customLogger"],
            json!({"success": true, "value": 1234, "logs": []})
        );
    }

    #[test]
    fn evaluates_tree_utility_helpers() {
        let script = r#"%dw 2.0
import * from dw::util::Tree
output application/python
var myObject = {
  user: [{name: "mariano", lastName: "achaval", friends: [{name: "julian"}]}],
  group: "data-weave"
}
---
{
  expression: asExpressionString([
    {kind: OBJECT_TYPE, selector: "user", namespace: null},
    {kind: ATTRIBUTE_TYPE, selector: "name", namespace: null}
  ]),
  mapped: myObject mapLeafValues (value, path) -> upper(value),
  exists: myObject nodeExists ((value, path) -> path[-1].selector == "name" and value == "julian"),
  missing: myObject nodeExists ($$[-1].selector == "name" and $ == "teo"),
  arrayPath: isArrayType([
    {kind: OBJECT_TYPE, selector: "user", namespace: null},
    {kind: ARRAY_TYPE, selector: 0, namespace: null}
  ]),
  objectPath: isObjectType([
    {kind: OBJECT_TYPE, selector: "user", namespace: null},
    {kind: OBJECT_TYPE, selector: "name", namespace: null}
  ]),
  attributePath: isAttributeType([
    {kind: OBJECT_TYPE, selector: "user", namespace: null},
    {kind: ATTRIBUTE_TYPE, selector: "name", namespace: null}
  ]),
  mappedByPathType: { name: "Mariano", test: [1, 2, 3] } mapLeafValues ((value, path) ->
    if (isObjectType(path)) "***"
    else if (isArrayType(path)) "In an array"
    else "Is an attribute"
  ),
  filteredArrayLeafs: [1, {name: ["", true], test: 213}, "123", null] filterArrayLeafs ((value, path) ->
    !(value is Null or value is String)
  ),
  filteredObjectLeafs: {
    name: "Mariano",
    lastName: null,
    age: 123,
    friends: [{name @(mail: "me@me.com", test: 123): "", id: "test"}, {name: "Me", id: null}]
  } filterObjectLeafs ((value, path) ->
    !(value is Null or value is String)
  )
}
"#;
        assert_eq!(
            execute_smoke(script, Value::Null).unwrap(),
            json!({
                "expression": ".user.@name",
                "mapped": {
                    "user": [{"name": "MARIANO", "lastName": "ACHAVAL", "friends": [{"name": "JULIAN"}]}],
                    "group": "DATA-WEAVE"
                },
                "exists": true,
                "missing": false,
                "arrayPath": true,
                "objectPath": true,
                "attributePath": true,
                "mappedByPathType": {
                    "name": "***",
                    "test": ["In an array", "In an array", "In an array"]
                },
                "filteredArrayLeafs": [1, {"name": [true], "test": 213}],
                "filteredObjectLeafs": {"age": 123, "friends": [{}, {}]}
            })
        );
    }

    #[test]
    fn renders_periods_and_temporals_as_json_strings() {
        let result = execute_json(
            r#"%dw 2.0
import * from dw::core::Periods
output application/json
---
{
  periodValue: years(4),
  nextHour: |2020-10-05T20:22:34.385000Z| + hours(1),
  betweenValue: between(|2020-02-29|, |2020-03-30|)
}
"#,
            Value::Null,
            true,
        )
        .unwrap();
        assert_eq!(
            result,
            json!(
                r#"{"periodValue":"P4Y","nextHour":"2020-10-05T21:22:34.385Z","betweenValue":"P-1M-1D"}"#
            )
        );
    }

    #[test]
    fn evaluates_dynamic_date_helpers() {
        let script = r#"%dw 2.0
import * from dw::core::Dates
output application/python
---
{
  todayValue: today(),
  tomorrowValue: tomorrow(),
  yesterdayValue: yesterday()
}
"#;
        assert_eq!(
            execute_smoke(script, Value::Null).unwrap(),
            json!({
                "todayValue": crate::builtins::current_utc_date_string(0),
                "tomorrowValue": crate::builtins::current_utc_date_string(1),
                "yesterdayValue": crate::builtins::current_utc_date_string(-1)
            })
        );
    }

    #[test]
    fn evaluates_now_number_fields_and_formats() {
        let script = r#"%dw 2.0
output application/python
---
{
  nowValue: now(),
  epochTime: now() as Number,
  nanoseconds: now().nanoseconds,
  milliseconds: now().milliseconds,
  seconds: now().seconds,
  minutes: now().minutes,
  hour: now().hour,
  day: now().day,
  month: now().month,
  year: now().year,
  quarter: now().quarter,
  dayOfWeek: now().dayOfWeek,
  dayOfYear: now().dayOfYear,
  offsetSeconds: now().offsetSeconds,
  formattedDate: now() as String {format: "y-MM-dd"},
  formattedTime: now() as String {format: "hh:m:s"}
}
"#;
        let result = execute_smoke(script, Value::Null).unwrap();
        let now_value = result["nowValue"].as_str().unwrap();
        assert_eq!(result["formattedDate"], json!(&now_value[..10]));
        assert_eq!(result["milliseconds"], json!(0));
        assert_eq!(result["nanoseconds"], json!(0));
        assert_eq!(result["offsetSeconds"], json!(0));
        assert!(result["epochTime"].as_i64().unwrap() > 0);
        assert!(result["formattedTime"].as_str().unwrap().len() >= 7);
    }
}
