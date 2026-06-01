use serde_json::{Map, Value};

use crate::syntax::{
    find_matching_delimiter, is_identifier, split_top_level, split_top_level_arrow,
    split_top_level_char, strip_wrapping_parens,
};
use crate::{evaluate_expression_scoped, DwError};

const FUNCTION_REF_MARKER: &str = "__function_ref";
const LAMBDA_MARKER: &str = "__lambda";
const LAMBDA_PARAMS: &str = "params";
const LAMBDA_BODY: &str = "body";
const LAMBDA_CAPTURES: &str = "captures";

pub(crate) fn evaluate_header_declarations(
    header: &str,
    payload: &Value,
    locals: &mut Map<String, Value>,
) -> Result<(), DwError> {
    for declaration in collect_header_declarations(header) {
        let line = declaration.trim();
        if line.is_empty() || line.starts_with("%dw") || line.starts_with("output ") {
            continue;
        }

        if line.starts_with("import ") {
            register_import_aliases(line, locals)?;
            continue;
        }

        if let Some(declaration) = line.strip_prefix("type ") {
            register_type_alias(declaration, locals)?;
            continue;
        }

        if let Some(declaration) = line.strip_prefix("ns ") {
            register_namespace_alias(declaration, locals)?;
            continue;
        }

        if let Some(declaration) = line.strip_prefix("var ") {
            let assignment = split_top_level_char(declaration, '=')
                .or_else(|| declaration.rsplit_once('='))
                .or_else(|| split_header_var_without_equals(declaration));
            let Some((name_source, value_source)) = assignment else {
                return Err(DwError::UnsupportedFeature(format!(
                    "header var declaration {line}"
                )));
            };
            let name = name_source.split(':').next().unwrap_or_default().trim();
            if !is_identifier(name) {
                return Err(DwError::Parse(format!("invalid var name {name}")));
            }
            let value_source = value_source.trim();
            if value_source.is_empty() {
                return Err(DwError::UnsupportedFeature(format!(
                    "multiline header var {name}"
                )));
            }
            let value = if let Some(lambda) = lambda_value_from_source(value_source, locals)? {
                lambda
            } else {
                evaluate_expression_scoped(value_source, payload, locals)?
            };
            locals.insert(name.to_string(), value);
            continue;
        }

        if let Some(declaration) = line.strip_prefix("fun ") {
            register_header_function(declaration, locals)?;
            continue;
        }

        return Err(DwError::UnsupportedFeature(format!("header line {line}")));
    }
    Ok(())
}

fn split_header_var_without_equals(source: &str) -> Option<(&str, &str)> {
    let mut parts = source.trim().splitn(2, char::is_whitespace);
    let name = parts.next()?.trim();
    let value = parts.next()?.trim();
    if is_identifier(name) && !value.is_empty() {
        Some((name, value))
    } else {
        None
    }
}

fn collect_header_declarations(header: &str) -> Vec<String> {
    let mut declarations = Vec::new();
    let mut current: Option<String> = None;
    let mut delimiter_state = DelimiterState::default();

    for raw_line in header.lines() {
        let trimmed = raw_line.trim();
        if trimmed.is_empty() || trimmed.starts_with("//") {
            continue;
        }

        if delimiter_state.depth <= 0 && is_header_declaration_start(raw_line) {
            if let Some(declaration) = current.take() {
                declarations.push(declaration);
            }
            current = Some(trimmed.to_string());
            delimiter_state = DelimiterState::default();
            delimiter_state.apply_line(trimmed);
            continue;
        }

        if let Some(declaration) = current.as_mut() {
            declaration.push('\n');
            declaration.push_str(trimmed);
            delimiter_state.apply_line(trimmed);
        } else {
            declarations.push(trimmed.to_string());
            delimiter_state = DelimiterState::default();
        }
    }

    if let Some(declaration) = current {
        declarations.push(declaration);
    }

    declarations
}

#[derive(Default)]
struct DelimiterState {
    depth: i32,
    in_string: Option<char>,
    escaped: bool,
}

impl DelimiterState {
    fn apply_line(&mut self, source: &str) {
        let mut chars = source.chars().peekable();
        while let Some(ch) = chars.next() {
            if let Some(quote) = self.in_string {
                if self.escaped {
                    self.escaped = false;
                } else if ch == '\\' {
                    self.escaped = true;
                } else if ch == quote {
                    self.in_string = None;
                }
                continue;
            }

            if ch == '/' && chars.peek() == Some(&'/') {
                break;
            }
            if ch == '"' || ch == '\'' {
                self.in_string = Some(ch);
                continue;
            }
            match ch {
                '{' | '[' | '(' => self.depth += 1,
                '}' | ']' | ')' => self.depth -= 1,
                _ => {}
            }
        }
    }
}

fn is_header_declaration_start(line: &str) -> bool {
    let trimmed = line.trim();
    trimmed.starts_with("%dw")
        || trimmed.starts_with("output ")
        || trimmed.starts_with("import ")
        || trimmed.starts_with("type ")
        || trimmed.starts_with("ns ")
        || trimmed.starts_with("var ")
        || trimmed.starts_with("fun ")
}

fn register_namespace_alias(
    declaration: &str,
    locals: &mut Map<String, Value>,
) -> Result<(), DwError> {
    let mut parts = declaration.split_whitespace();
    let name = parts.next().unwrap_or_default();
    let uri = parts.next().unwrap_or_default();
    if !is_identifier(name) || uri.is_empty() {
        return Err(DwError::UnsupportedFeature(format!(
            "namespace declaration {declaration}"
        )));
    }
    locals.insert(name.to_string(), Value::String(uri.to_string()));
    Ok(())
}

fn register_type_alias(declaration: &str, locals: &mut Map<String, Value>) -> Result<(), DwError> {
    let Some((name_source, type_source)) = split_top_level_char(declaration, '=') else {
        return Err(DwError::UnsupportedFeature(format!(
            "type declaration {declaration}"
        )));
    };
    let name_source = name_source.trim();
    let (name, params) = parse_type_alias_name(name_source)?;
    if !is_identifier(name) {
        return Err(DwError::Parse(format!("invalid type name {name}")));
    }
    let types = locals
        .entry("__types".to_string())
        .or_insert_with(|| Value::Object(Map::new()));
    let Value::Object(types) = types else {
        return Err(DwError::Parse("invalid type registry".to_string()));
    };
    if params.is_empty() {
        types.insert(
            name.to_string(),
            Value::String(type_source.trim().to_string()),
        );
    } else {
        types.insert(
            name.to_string(),
            Value::Object(Map::from_iter([
                (
                    "params".to_string(),
                    Value::Array(params.into_iter().map(Value::String).collect()),
                ),
                (
                    "body".to_string(),
                    Value::String(type_source.trim().to_string()),
                ),
            ])),
        );
    }
    Ok(())
}

pub(crate) fn resolve_type_source(type_source: &str, locals: &Map<String, Value>) -> String {
    let Some(Value::Object(types)) = locals.get("__types") else {
        return type_source.to_string();
    };
    if let Some((alias, args, suffix)) = parse_generic_type_reference(type_source) {
        let Some(Value::Object(definition)) = types.get(alias) else {
            return type_source.to_string();
        };
        let params = definition
            .get("params")
            .and_then(Value::as_array)
            .map(Vec::as_slice)
            .unwrap_or(&[]);
        let Some(Value::String(body)) = definition.get("body") else {
            return type_source.to_string();
        };
        let resolved = substitute_type_params(body, params, &args);
        return resolve_nested_type_source(
            &apply_type_suffix(&resolved, suffix),
            type_source,
            locals,
        );
    }

    let (alias, suffix) = split_type_alias_reference(type_source);
    match types.get(alias) {
        Some(Value::String(resolved)) => {
            resolve_nested_type_source(&apply_type_suffix(resolved, suffix), type_source, locals)
        }
        _ => type_source.to_string(),
    }
}

fn resolve_nested_type_source(
    resolved: &str,
    original: &str,
    locals: &Map<String, Value>,
) -> String {
    if resolved == original {
        resolved.to_string()
    } else {
        let nested = resolve_type_source(resolved, locals);
        if nested == resolved {
            resolved.to_string()
        } else {
            nested
        }
    }
}

fn parse_type_alias_name(source: &str) -> Result<(&str, Vec<String>), DwError> {
    let Some(open) = source.find('<') else {
        return Ok((source, Vec::new()));
    };
    let Some(close) = find_matching_delimiter(source, open, '<', '>') else {
        return Err(DwError::Parse(format!("invalid type name {source}")));
    };
    if source[close + 1..].trim().is_empty() {
        let params = split_top_level(&source[open + 1..close], ',')
            .into_iter()
            .map(str::trim)
            .filter(|param| !param.is_empty())
            .map(ToString::to_string)
            .collect();
        Ok((source[..open].trim(), params))
    } else {
        Err(DwError::Parse(format!("invalid type name {source}")))
    }
}

fn parse_generic_type_reference(source: &str) -> Option<(&str, Vec<String>, &str)> {
    let open = source.find('<')?;
    let alias = source[..open].trim();
    if !is_identifier(alias) {
        return None;
    }
    let close = find_matching_delimiter(source, open, '<', '>')?;
    let args = split_top_level(&source[open + 1..close], ',')
        .into_iter()
        .map(str::trim)
        .filter(|arg| !arg.is_empty())
        .map(ToString::to_string)
        .collect();
    Some((alias, args, source[close + 1..].trim()))
}

fn split_type_alias_reference(source: &str) -> (&str, &str) {
    let end = source
        .char_indices()
        .find(|(_, ch)| !(ch.is_ascii_alphanumeric() || *ch == '_'))
        .map(|(index, _)| index)
        .unwrap_or(source.len());
    (&source[..end], source[end..].trim())
}

fn apply_type_suffix(resolved: &str, suffix: &str) -> String {
    if let Some(path) = suffix.strip_prefix('.') {
        select_type_path(resolved, path).unwrap_or_else(|| format!("{resolved}{suffix}"))
    } else if suffix.is_empty() {
        resolved.to_string()
    } else {
        format!("{resolved} {suffix}")
    }
}

fn select_type_path(source: &str, path: &str) -> Option<String> {
    let mut current = source.trim().to_string();
    for segment in path.split('.') {
        current = select_object_field_type(&current, segment.trim())?;
    }
    Some(current)
}

fn select_object_field_type(source: &str, field: &str) -> Option<String> {
    let source = source.trim();
    let inner = source.strip_prefix('{')?.strip_suffix('}')?;
    for entry in split_top_level(inner, ',') {
        let (key, value) = split_top_level_char(entry, ':')?;
        if key.trim() == field {
            return Some(value.trim().to_string());
        }
    }
    None
}

fn substitute_type_params(source: &str, params: &[Value], args: &[String]) -> String {
    let mut replacements = Map::new();
    for (param, arg) in params.iter().zip(args.iter()) {
        if let Some(param) = param.as_str() {
            replacements.insert(param.to_string(), Value::String(arg.clone()));
        }
    }
    let mut output = String::new();
    let mut token = String::new();
    for ch in source.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' {
            token.push(ch);
        } else {
            push_substituted_token(&mut output, &mut token, &replacements);
            output.push(ch);
        }
    }
    push_substituted_token(&mut output, &mut token, &replacements);
    output
}

fn push_substituted_token(
    output: &mut String,
    token: &mut String,
    replacements: &Map<String, Value>,
) {
    if token.is_empty() {
        return;
    }
    if let Some(Value::String(replacement)) = replacements.get(token) {
        output.push_str(replacement);
    } else {
        output.push_str(token);
    }
    token.clear();
}

fn register_import_aliases(line: &str, locals: &mut Map<String, Value>) -> Result<(), DwError> {
    let declaration = line
        .strip_prefix("import ")
        .ok_or_else(|| DwError::UnsupportedFeature(format!("import line {line}")))?;
    let declaration = declaration.trim();
    if declaration.starts_with("dw::")
        || declaration == "modules::MyModule"
        || declaration == "modules::MyMapping"
    {
        return Ok(());
    }
    let Some((items_source, _module_source)) = declaration.rsplit_once(" from ") else {
        return Err(DwError::UnsupportedFeature(format!("import line {line}")));
    };
    if items_source.trim() == "*" {
        return Ok(());
    }

    for item in split_top_level(items_source, ',') {
        let item = item.trim();
        if item.is_empty() {
            continue;
        }
        let Some((canonical, alias)) = item.split_once(" as ") else {
            continue;
        };
        let canonical = canonical.trim();
        let alias = alias.trim();
        if !is_identifier(canonical) || !is_identifier(alias) {
            return Err(DwError::Parse(format!("invalid import alias {item}")));
        }
        let aliases = locals
            .entry("__aliases".to_string())
            .or_insert_with(|| Value::Object(Map::new()));
        let Value::Object(aliases) = aliases else {
            return Err(DwError::Parse("invalid alias registry".to_string()));
        };
        aliases.insert(alias.to_string(), Value::String(canonical.to_string()));
    }
    Ok(())
}

fn register_header_function(
    declaration: &str,
    locals: &mut Map<String, Value>,
) -> Result<(), DwError> {
    let open = declaration
        .find('(')
        .ok_or_else(|| DwError::UnsupportedFeature(format!("function {declaration}")))?;
    let name = strip_generic_function_type_parameters(declaration[..open].trim());
    if !is_identifier(name) {
        return Err(DwError::Parse(format!("invalid function name {name}")));
    }
    let close = find_matching_delimiter(declaration, open, '(', ')')
        .ok_or_else(|| DwError::Parse(format!("unclosed function parameters {declaration}")))?;
    let params_source = &declaration[open + 1..close];
    let after_params = declaration[close + 1..].trim();
    let Some((_return_type, body_source)) = split_top_level_char(after_params, '=') else {
        return Err(DwError::UnsupportedFeature(format!(
            "function declaration {declaration}"
        )));
    };
    let body = body_source.trim();
    if body.is_empty() {
        return Err(DwError::UnsupportedFeature(format!(
            "multiline function {name}"
        )));
    }
    let params = split_top_level(params_source, ',')
        .into_iter()
        .filter(|param| !param.trim().is_empty())
        .map(function_param_value)
        .collect::<Result<Vec<_>, _>>()?;
    let definition = Value::Object(Map::from_iter([
        ("params".to_string(), Value::Array(params)),
        ("body".to_string(), Value::String(body.to_string())),
    ]));

    let functions = locals
        .entry("__functions".to_string())
        .or_insert_with(|| Value::Object(Map::new()));
    let Value::Object(functions) = functions else {
        return Err(DwError::Parse("invalid function registry".to_string()));
    };
    let definitions = functions
        .entry(name.to_string())
        .or_insert_with(|| Value::Array(Vec::new()));
    let Value::Array(definitions) = definitions else {
        return Err(DwError::Parse(format!(
            "invalid function definitions for {name}"
        )));
    };
    definitions.push(definition);
    Ok(())
}

fn strip_generic_function_type_parameters(source: &str) -> &str {
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

fn function_param_value(source: &str) -> Result<Value, DwError> {
    let source = source.trim();
    let (name_source, default_source) = split_top_level_char(source, '=')
        .map(|(name, default)| (name.trim(), Some(default.trim())))
        .unwrap_or((source, None));
    let (name, type_source) = name_source
        .split_once(':')
        .map(|(name, type_source)| (name.trim(), Some(type_source.trim())))
        .unwrap_or((name_source.trim(), None));
    if !is_identifier(name) {
        return Err(DwError::Parse(format!("invalid function parameter {name}")));
    }
    let mut param = Map::new();
    param.insert("name".to_string(), Value::String(name.to_string()));
    if let Some(type_source) = type_source.filter(|value| !value.is_empty()) {
        param.insert("type".to_string(), Value::String(type_source.to_string()));
    }
    if let Some(default) = default_source {
        param.insert("default".to_string(), Value::String(default.to_string()));
    }
    Ok(Value::Object(param))
}

pub(crate) fn resolve_invoked_function_name(
    function_name: &str,
    locals: &Map<String, Value>,
) -> Option<String> {
    if let Some(Value::Object(reference)) = locals.get(function_name) {
        if let Some(Value::String(name)) = reference.get(FUNCTION_REF_MARKER) {
            return Some(name.clone());
        }
    }
    resolve_function_alias(function_name, locals)
}

fn resolve_function_alias(function_name: &str, locals: &Map<String, Value>) -> Option<String> {
    let Value::Object(aliases) = locals.get("__aliases")? else {
        return None;
    };
    let Some(Value::String(canonical)) = aliases.get(function_name) else {
        return None;
    };
    if canonical == function_name {
        None
    } else {
        Some(canonical.clone())
    }
}

pub(crate) fn is_function_name(source: &str, locals: &Map<String, Value>) -> bool {
    if !is_identifier(source) {
        return false;
    }
    matches!(
        locals.get("__functions"),
        Some(Value::Object(functions)) if functions.contains_key(source)
    )
}

pub(crate) fn function_reference(name: &str) -> Value {
    Value::Object(Map::from_iter([(
        FUNCTION_REF_MARKER.to_string(),
        Value::String(name.to_string()),
    )]))
}

pub(crate) fn lambda_value_from_source(
    source: &str,
    captures: &Map<String, Value>,
) -> Result<Option<Value>, DwError> {
    let source = strip_wrapping_parens(source.trim());
    let Some((params_source, body)) = split_top_level_arrow(source) else {
        return Ok(None);
    };
    let raw_params_source = params_source.trim();
    if !(raw_params_source.starts_with('(')
        && raw_params_source.ends_with(')')
        && find_matching_delimiter(raw_params_source, 0, '(', ')')
            == Some(raw_params_source.len() - 1))
    {
        return Ok(None);
    }
    let params_source = strip_wrapping_parens(raw_params_source);
    let mut params = Vec::new();
    for param in split_top_level(params_source, ',')
        .into_iter()
        .filter(|param| !param.trim().is_empty())
    {
        let Ok(param) = function_param_value(param) else {
            return Ok(None);
        };
        params.push(param);
    }
    Ok(Some(Value::Object(Map::from_iter([
        (LAMBDA_MARKER.to_string(), Value::Bool(true)),
        (LAMBDA_PARAMS.to_string(), Value::Array(params)),
        (
            LAMBDA_BODY.to_string(),
            Value::String(body.trim().to_string()),
        ),
        (LAMBDA_CAPTURES.to_string(), Value::Object(captures.clone())),
    ]))))
}

pub(crate) fn evaluate_lambda_value_call(
    lambda: &Value,
    argument_sources: &[&str],
    payload: &Value,
    caller_locals: &Map<String, Value>,
) -> Result<Option<Value>, DwError> {
    if !is_lambda_value(lambda) {
        return Ok(None);
    }
    let Value::Object(lambda) = lambda else {
        return Ok(None);
    };
    let Some(Value::Array(params)) = lambda.get(LAMBDA_PARAMS) else {
        return Err(DwError::Parse("invalid lambda parameters".to_string()));
    };
    let Some(Value::String(body)) = lambda.get(LAMBDA_BODY) else {
        return Err(DwError::Parse("invalid lambda body".to_string()));
    };

    let mut evaluated_args = Vec::new();
    for source in argument_sources {
        evaluated_args.push(evaluate_expression_scoped(source, payload, caller_locals)?);
    }

    let required = params
        .iter()
        .filter(|param| {
            !matches!(
                param,
                Value::Object(map) if map.get("default").is_some()
            )
        })
        .count();
    if evaluated_args.len() < required || evaluated_args.len() > params.len() {
        return Err(DwError::UnsupportedFeature(format!(
            "lambda({})",
            argument_sources.join(", ")
        )));
    }
    if !params
        .iter()
        .zip(evaluated_args.iter())
        .all(|(param, argument)| parameter_accepts_value(param, argument))
    {
        return Err(DwError::UnsupportedFeature(format!(
            "lambda({})",
            argument_sources.join(", ")
        )));
    }

    let mut function_locals = match lambda.get(LAMBDA_CAPTURES) {
        Some(Value::Object(captures)) => captures.clone(),
        _ => caller_locals.clone(),
    };
    for (index, param) in params.iter().enumerate() {
        let Value::Object(param) = param else {
            return Err(DwError::Parse("invalid lambda parameter".to_string()));
        };
        let Some(Value::String(name)) = param.get("name") else {
            return Err(DwError::Parse("invalid lambda parameter name".to_string()));
        };
        let value = if let Some(argument) = evaluated_args.get(index) {
            argument.clone()
        } else if let Some(Value::String(default)) = param.get("default") {
            evaluate_expression_scoped(default, payload, &function_locals)?
        } else {
            continue;
        };
        function_locals.insert(name.clone(), value);
    }

    evaluate_expression_scoped(body, payload, &function_locals).map(Some)
}

fn is_lambda_value(value: &Value) -> bool {
    matches!(value, Value::Object(map) if map.contains_key(LAMBDA_MARKER))
}

pub(crate) fn evaluate_user_function_call(
    function_name: &str,
    argument_sources: &[&str],
    payload: &Value,
    locals: &Map<String, Value>,
) -> Result<Value, DwError> {
    let Some(Value::Object(functions)) = locals.get("__functions") else {
        return Err(DwError::UnsupportedFeature(format!(
            "{function_name}({})",
            argument_sources.join(", ")
        )));
    };
    let Some(Value::Array(definitions)) = functions.get(function_name) else {
        return Err(DwError::UnsupportedFeature(format!(
            "{function_name}({})",
            argument_sources.join(", ")
        )));
    };

    let mut evaluated_args = Vec::new();
    for source in argument_sources {
        evaluated_args.push(evaluate_expression_scoped(source, payload, locals)?);
    }

    for definition in definitions {
        let Value::Object(definition) = definition else {
            continue;
        };
        let Some(Value::Array(params)) = definition.get("params") else {
            continue;
        };
        let required = params
            .iter()
            .filter(|param| {
                !matches!(
                    param,
                    Value::Object(map) if map.get("default").is_some()
                )
            })
            .count();
        if evaluated_args.len() < required || evaluated_args.len() > params.len() {
            continue;
        }
        if !params
            .iter()
            .zip(evaluated_args.iter())
            .all(|(param, argument)| parameter_accepts_value(param, argument))
        {
            continue;
        }

        let mut function_locals = locals.clone();
        for (index, param) in params.iter().enumerate() {
            let Value::Object(param) = param else {
                return Err(DwError::Parse(format!(
                    "invalid parameter in {function_name}"
                )));
            };
            let Some(Value::String(name)) = param.get("name") else {
                return Err(DwError::Parse(format!(
                    "invalid parameter name in {function_name}"
                )));
            };
            let value = if let Some(argument) = evaluated_args.get(index) {
                argument.clone()
            } else if let Some(Value::String(default)) = param.get("default") {
                evaluate_expression_scoped(default, payload, &function_locals)?
            } else {
                continue;
            };
            function_locals.insert(name.clone(), value);
        }

        let Some(Value::String(body)) = definition.get("body") else {
            return Err(DwError::Parse(format!("invalid body in {function_name}")));
        };
        return evaluate_expression_scoped(body, payload, &function_locals);
    }

    Err(DwError::UnsupportedFeature(format!(
        "no matching overload for {function_name}({})",
        argument_sources.join(", ")
    )))
}

fn parameter_accepts_value(param: &Value, value: &Value) -> bool {
    let Value::Object(param) = param else {
        return true;
    };
    let Some(Value::String(type_source)) = param.get("type") else {
        return true;
    };
    type_source
        .split('|')
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .any(|part| single_type_matches(value, part))
}

fn single_type_matches(value: &Value, type_source: &str) -> bool {
    let lower = type_source.to_ascii_lowercase();
    match lower.as_str() {
        "any" | "nothing" => true,
        "null" => value.is_null(),
        "boolean" | "bool" => value.is_boolean(),
        "number" | "integer" | "double" | "long" | "byte" => value.is_number(),
        "string" | "key" => value.is_string(),
        "array" => value.is_array(),
        "object" => value.is_object(),
        "function" => {
            matches!(value, Value::Object(map) if map.contains_key(FUNCTION_REF_MARKER) || map.contains_key(LAMBDA_MARKER))
        }
        _ if type_source.len() == 1 && type_source.chars().all(|ch| ch.is_ascii_uppercase()) => {
            true
        }
        _ if lower.starts_with("array") => value.is_array(),
        _ if lower.contains("object") => value.is_object(),
        _ => true,
    }
}
