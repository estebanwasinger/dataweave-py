use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use wasm_bindgen::prelude::*;

/// Query capabilities from the actual engine build, never a client-side function list.
#[wasm_bindgen]
pub fn get_dataweave_function_catalog() -> Result<String, JsValue> {
    serde_json::to_string(dwpy_core::function_catalog::functions())
        .map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Source/input language service; reads input formats but never evaluates the script.
#[wasm_bindgen]
pub fn complete_dataweave_request(request_json: &str) -> Result<String, JsValue> {
    #[derive(serde::Deserialize)]
    struct Request {
        script: String,
        #[serde(default)]
        module_sources: BTreeMap<String, String>,
        line: usize,
        column: usize,
        #[serde(default)]
        payload: serde_json::Value,
        payload_format: Option<String>,
        payload_format_options: Option<serde_json::Value>,
        #[serde(default)]
        vars: serde_json::Value,
        #[serde(default)]
        attributes: serde_json::Value,
    }
    let request: Request = serde_json::from_value(decode_request_context(request_json)?)
        .map_err(|err| JsValue::from_str(&err.to_string()))?;
    let items = dwpy_core::completion::complete_fields(
        &request.script,
        request.line,
        request.column,
        |root| match root {
            "payload" => dwpy_core::parse_native_payload_format_with_options(
                request.payload.clone().into(),
                request.payload_format.as_deref(),
                request
                    .payload_format_options
                    .clone()
                    .map(dwpy_core::DwValue::from)
                    .as_ref(),
            )
            .ok(),
            "vars" => Some(request.vars.clone().into()),
            "attributes" => Some(request.attributes.clone().into()),
            _ => None,
        },
    )
    .unwrap_or_else(|| {
        dwpy_core::completion::complete_with_modules(
            &request.script,
            request.line,
            request.column,
            &request.module_sources,
        )
    });
    serde_json::to_string(&serde_json::json!({ "items": items }))
        .map_err(|err| JsValue::from_str(&err.to_string()))
}

#[wasm_bindgen]
pub fn run_dataweave_smoke(script_source: &str, payload_json: &str) -> Result<String, JsValue> {
    let payload = decode_json_input(payload_json)?;
    let result = dwpy_core::execute_smoke(script_source, payload)
        .map_err(|err| JsValue::from_str(&err.to_string()))?;
    serde_json::to_string(&result).map_err(|err| JsValue::from_str(&err.to_string()))
}

#[wasm_bindgen]
pub fn run_dataweave_request(request_json: &str) -> Result<String, JsValue> {
    run_request(request_json, dwpy_core::Loggers::default())
}

/// Opt-in envelope; preserves logs even when evaluation fails. The original
/// entry point retains its result-only/throwing contract.
#[wasm_bindgen]
pub fn run_dataweave_request_with_logs(request_json: &str) -> Result<String, JsValue> {
    let captured = Arc::new(Mutex::new(CapturedLogs::default()));
    let sink = captured.clone();
    let mut loggers = dwpy_core::Loggers::default();
    loggers.push(move |entry: &dwpy_core::LogEntry| {
        sink.lock().unwrap().push(entry);
    });
    let result = run_request(request_json, loggers);
    let captured = captured.lock().unwrap();
    let mut envelope = serde_json::json!({
        "logs": captured.entries,
        "logs_truncated": captured.truncated,
    });
    match result {
        Ok(result) => {
            envelope["result"] =
                serde_json::from_str(&result).map_err(|err| JsValue::from_str(&err.to_string()))?
        }
        Err(error) => {
            envelope["error"] = serde_json::json!(error
                .as_string()
                .unwrap_or_else(|| "DataWeave execution failed".into()))
        }
    }
    serde_json::to_string(&envelope).map_err(|err| JsValue::from_str(&err.to_string()))
}

#[derive(Default)]
struct CapturedLogs {
    entries: Vec<dwpy_core::LogEntry>,
    bytes: usize,
    truncated: bool,
}

impl CapturedLogs {
    fn push(&mut self, entry: &dwpy_core::LogEntry) {
        // Bound browser/Node captures by count, total bytes, and message size.
        if self.entries.len() >= 1000 || self.bytes >= 1024 * 1024 {
            self.truncated = true;
            return;
        }
        let mut entry = entry.clone();
        self.truncated |= truncate_log_text(&mut entry.message, 16 * 1024);
        if let Some(prefix) = &mut entry.prefix {
            self.truncated |= truncate_log_text(prefix, 1024);
        }
        let bytes = entry.message.len() + entry.prefix.as_ref().map_or(0, String::len);
        if self.bytes + bytes > 1024 * 1024 {
            self.truncated = true;
            return;
        }
        self.bytes += bytes;
        self.entries.push(entry);
    }
}

fn truncate_log_text(text: &mut String, max_bytes: usize) -> bool {
    if text.len() <= max_bytes {
        return false;
    }
    let mut end = max_bytes;
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    text.truncate(end);
    text.push('…');
    true
}

fn run_request(request_json: &str, loggers: dwpy_core::Loggers) -> Result<String, JsValue> {
    let request = decode_request_context(request_json)?;
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

    let payload = if payload_format.is_some()
        && request
            .get("typed_transport")
            .and_then(serde_json::Value::as_bool)
            == Some(true)
    {
        dwpy_core::DwValue::from(payload).to_transport_json()
    } else {
        payload
    };

    let vars = request
        .get("vars")
        .cloned()
        .unwrap_or_else(|| serde_json::Value::Object(serde_json::Map::new()));
    let attributes = request
        .get("attributes")
        .cloned()
        .unwrap_or_else(|| serde_json::Value::Object(serde_json::Map::new()));
    let bindings = request.get("bindings").cloned();
    let properties = request
        .get("properties")
        .map(properties_to_map)
        .transpose()
        .map_err(|err| JsValue::from_str(&err))?;
    let (result, trace) = dwpy_core::with_execution_trace(script_source, || {
        if request
            .get("typed_transport")
            .and_then(serde_json::Value::as_bool)
            == Some(true)
        {
            dwpy_core::execute_transport_with_bindings_and_loggers(
                script_source,
                payload,
                Some(vars),
                Some(attributes),
                properties,
                bindings,
                render_output,
                loggers,
            )
        } else {
            dwpy_core::ExecutionContext::new(Some(vars), Some(attributes), properties).and_then(
                |mut context| {
                    if let Some(bindings) = bindings {
                        context = context.with_bindings(bindings.into())?;
                    }
                    context.loggers = loggers;
                    dwpy_core::execute_json_with_context(
                        script_source,
                        payload,
                        context,
                        render_output,
                    )
                },
            )
        }
    });
    let result = result.map_err(|err| {
        let mut message = match (&err, &trace) {
            (dwpy_core::DwError::UnsupportedFeature(message), Some(_))
                if message.starts_with("Unable to evaluate expression `") =>
            {
                message.clone()
            }
            _ => wasm_error_message(err, script_source),
        };
        if let Some(trace) = trace {
            message.push_str(&trace);
        }
        JsValue::from_str(&message)
    })?;

    serde_json::to_string(&result).map_err(|err| JsValue::from_str(&err.to_string()))
}

#[wasm_bindgen]
pub fn analyze_dataweave_request(request_json: &str) -> Result<String, JsValue> {
    let request = decode_request_context(request_json)?;
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
    let bindings = request.get("bindings").cloned();

    let typed = request
        .get("typed_transport")
        .and_then(serde_json::Value::as_bool)
        == Some(true);
    let decode = |value: Option<serde_json::Value>| -> Result<_, dwpy_core::DwError> {
        value
            .map(|value| {
                if typed {
                    dwpy_core::DwValue::from_transport_json(value)
                } else {
                    Ok(value.into())
                }
            })
            .transpose()
    };
    let result = decode(payload)
        .and_then(|payload| {
            dwpy_core::analyze_native_expression_with_bindings(
                expression,
                payload,
                decode(vars)?,
                decode(attributes)?,
                decode(bindings)?,
            )
        })
        .map(|value| value.to_json())
        .map_err(|err| JsValue::from_str(&wasm_error_message(err, expression)))?;
    serde_json::to_string(&result).map_err(|err| JsValue::from_str(&err.to_string()))
}

fn decode_json_input(text: &str) -> Result<serde_json::Value, JsValue> {
    dwpy_core::parse_payload_format(
        serde_json::Value::String(text.to_string()),
        Some("application/json"),
    )
    .map_err(|err| JsValue::from_str(&err.to_string()))
}

fn decode_request_context(text: &str) -> Result<serde_json::Value, JsValue> {
    let mut request: serde_json::Value =
        serde_json::from_str(text).map_err(|err| JsValue::from_str(&err.to_string()))?;
    if let Some(fields) = request.as_object_mut() {
        // Keep the envelope's existing validation and last-key behavior, but
        // decode execution data from its original text before keys can be lost.
        let raw_fields: BTreeMap<String, Box<serde_json::value::RawValue>> =
            serde_json::from_str(text).map_err(|err| JsValue::from_str(&err.to_string()))?;
        for name in ["payload", "vars", "attributes", "bindings"] {
            if let Some(raw) = raw_fields.get(name) {
                fields.insert(name.to_string(), decode_json_input(raw.get())?);
            }
        }
    }
    Ok(request)
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
        _ => dwpy_core::execution_error_message(&err),
    }
}

fn normalized_script_error_message(message: &str, script_source: &str) -> String {
    if let Some(expression) = message
        .strip_prefix("Unable to evaluate expression `")
        .and_then(|rest| rest.split_once('`').map(|(expression, _)| expression))
    {
        if let Some(index) = dwpy_core::unique_expression_offset(script_source, expression) {
            let before = &script_source[..index];
            let line = before.bytes().filter(|byte| *byte == b'\n').count() + 1;
            let column = before.rsplit('\n').next().unwrap_or("").chars().count() + 1;
            let line_text = script_source.lines().nth(line - 1).unwrap_or("");
            return format!("{message}\n\n{line}| {line_text}\n{}\nLocation:\nmain (line: {line}, column: {column})", underline(line, column, expression.chars().count()));
        }
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

#[cfg(test)]
mod tests {
    use super::normalized_script_error_message;

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
    #[cfg_attr(not(target_arch = "wasm32"), test)]
    fn preserves_numeric_type_errors() {
        let message = normalized_script_error_message(
            "expected number, got Array [Number(1), Number(2)]",
            "%dw 2.0\noutput application/json\n---\n(1 to 10) * 2",
        );

        assert!(!message.contains("Rust evaluator"));
        assert_eq!(message, "expected number, got Array [Number(1), Number(2)]");
    }

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
    #[cfg_attr(not(target_arch = "wasm32"), test)]
    fn preserves_unrecognized_errors_without_guessing_the_cause() {
        let message = normalized_script_error_message(
            "some internal unresolved expression",
            "%dw 2.0\n---\n{a: 1} unknownInfix {b: 2}",
        );

        assert_eq!(message, "some internal unresolved expression");
    }

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
    #[cfg_attr(not(target_arch = "wasm32"), test)]
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

    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test::wasm_bindgen_test)]
    #[cfg_attr(not(target_arch = "wasm32"), test)]
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

/// DAP messages over a host-selected transport. Keep the adapter alive between
/// dispatch/poll calls; each poll yields after a bounded continuation budget.
#[wasm_bindgen]
pub struct DataWeaveDebugAdapter {
    inner: dwpy_core::dap::DebugAdapter,
}
#[wasm_bindgen]
impl DataWeaveDebugAdapter {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Self {
        Self {
            inner: dwpy_core::dap::DebugAdapter::new(),
        }
    }
    pub fn dispatch(&mut self, request_json: &str) -> Result<String, JsValue> {
        let request =
            serde_json::from_str(request_json).map_err(|e| JsValue::from_str(&e.to_string()))?;
        serde_json::to_string(&self.inner.dispatch(request))
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }
    pub fn poll(&mut self, fuel: u32) -> Result<String, JsValue> {
        serde_json::to_string(&self.inner.poll(fuel as usize))
            .map_err(|e| JsValue::from_str(&e.to_string()))
    }
    pub fn is_running(&self) -> bool {
        self.inner.is_running()
    }
}
impl Default for DataWeaveDebugAdapter {
    fn default() -> Self {
        Self::new()
    }
}
