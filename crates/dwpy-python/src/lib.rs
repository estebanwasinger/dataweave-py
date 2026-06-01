use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyDict, PyFloat, PyList, PyModule};
use std::sync::Mutex;

#[pyclass]
struct RustDataWeaveRuntime {
    legacy_runtime: Option<Py<PyAny>>,
    allow_legacy_fallback: bool,
    last_engine: Mutex<String>,
}

#[pymethods]
impl RustDataWeaveRuntime {
    #[new]
    #[pyo3(signature = (*, enable_module_imports = true, allow_legacy_fallback = true))]
    fn new(
        py: Python<'_>,
        enable_module_imports: bool,
        allow_legacy_fallback: bool,
    ) -> PyResult<Self> {
        let legacy_runtime = if allow_legacy_fallback {
            let module = PyModule::import(py, "dwpy._python_runtime")?;
            let runtime_cls = module.getattr("DataWeaveRuntime")?;
            let kwargs = PyDict::new(py);
            kwargs.set_item("enable_module_imports", enable_module_imports)?;
            Some(runtime_cls.call((), Some(&kwargs))?.unbind())
        } else {
            None
        };
        Ok(Self {
            legacy_runtime,
            allow_legacy_fallback,
            last_engine: Mutex::new("not-run".to_string()),
        })
    }

    #[pyo3(signature = (
        script_source,
        payload,
        vars = None,
        *,
        payload_format = None,
        payload_format_options = None,
        render_output = true
    ))]
    fn execute(
        &self,
        py: Python<'_>,
        script_source: &str,
        payload: Py<PyAny>,
        vars: Option<Py<PyAny>>,
        payload_format: Option<&str>,
        payload_format_options: Option<Py<PyAny>>,
        render_output: bool,
    ) -> PyResult<Py<PyAny>> {
        let payload_format_options_json = payload_format_options
            .as_ref()
            .map(|options| py_to_json(py, options.bind(py)))
            .transpose();
        let payload_format_options_json = match payload_format_options_json {
            Ok(value) => value,
            Err(err) => {
                return self.unsupported_or_legacy(
                    py,
                    script_source,
                    payload,
                    vars,
                    payload_format,
                    payload_format_options,
                    render_output,
                    Some(err),
                );
            }
        };

        if is_supported_payload_request(payload_format, payload_format_options_json.as_ref())
            && is_core_candidate(script_source)
        {
            if let Ok(payload_json) = py_to_json(py, payload.bind(py)) {
                let payload_json = match dwpy_core::parse_payload_format_with_options(
                    payload_json,
                    payload_format,
                    payload_format_options_json.as_ref(),
                ) {
                    Ok(value) => value,
                    Err(err) => {
                        return self.unsupported_or_legacy(
                            py,
                            script_source,
                            payload,
                            vars,
                            payload_format,
                            payload_format_options,
                            render_output,
                            Some(dataweave_evaluation_error(py, &err.to_string())?),
                        );
                    }
                };
                let rust_result = if let Some(vars) = vars.as_ref() {
                    py_to_json(py, vars.bind(py)).and_then(|vars_json| {
                        dwpy_core::execute_json_with_vars(
                            script_source,
                            payload_json,
                            vars_json,
                            render_output,
                        )
                        .map_err(|err| rust_core_error(py, err, script_source))
                    })
                } else {
                    dwpy_core::execute_json(script_source, payload_json, render_output)
                        .map_err(|err| rust_core_error(py, err, script_source))
                };

                match rust_result {
                    Ok(result) => {
                        self.set_last_engine("rust-core");
                        return json_to_py(py, result);
                    }
                    Err(err) => {
                        return self.unsupported_or_legacy(
                            py,
                            script_source,
                            payload,
                            vars,
                            payload_format,
                            payload_format_options,
                            render_output,
                            Some(err),
                        );
                    }
                }
            }
        }

        self.unsupported_or_legacy(
            py,
            script_source,
            payload,
            vars,
            payload_format,
            payload_format_options,
            render_output,
            None,
        )
    }

    fn capabilities(&self) -> Vec<&'static str> {
        dwpy_core::engine_capabilities()
    }

    fn last_execution_engine(&self) -> String {
        self.last_engine
            .lock()
            .map(|value| value.clone())
            .unwrap_or_else(|_| "unknown".to_string())
    }

    fn execute_smoke_json(&self, script_source: &str, payload_json: &str) -> PyResult<String> {
        let payload: serde_json::Value = serde_json::from_str(payload_json)
            .map_err(|err| pyo3::exceptions::PyValueError::new_err(err.to_string()))?;
        let result = dwpy_core::execute_smoke(script_source, payload)
            .map_err(|err| pyo3::exceptions::PyNotImplementedError::new_err(err.to_string()))?;
        serde_json::to_string(&result)
            .map_err(|err| pyo3::exceptions::PyValueError::new_err(err.to_string()))
    }

    #[pyo3(signature = (script_source, payload = None, vars = None))]
    fn infer_type_descriptor(
        &self,
        py: Python<'_>,
        script_source: &str,
        payload: Option<Py<PyAny>>,
        vars: Option<Py<PyAny>>,
    ) -> PyResult<Py<PyAny>> {
        let payload_json = payload
            .as_ref()
            .map(|value| py_to_json(py, value.bind(py)))
            .transpose()?;
        let vars_json = vars
            .as_ref()
            .map(|value| py_to_json(py, value.bind(py)))
            .transpose()?;
        let result = dwpy_core::infer_type_descriptor(script_source, payload_json, vars_json)
            .map_err(|err| pyo3::exceptions::PyNotImplementedError::new_err(err.to_string()))?;
        json_to_py(py, result)
    }
}

impl RustDataWeaveRuntime {
    fn set_last_engine(&self, value: &str) {
        if let Ok(mut last_engine) = self.last_engine.lock() {
            *last_engine = value.to_string();
        }
    }

    fn execute_legacy(
        &self,
        py: Python<'_>,
        script_source: &str,
        payload: Py<PyAny>,
        vars: Option<Py<PyAny>>,
        payload_format: Option<&str>,
        payload_format_options: Option<Py<PyAny>>,
        render_output: bool,
    ) -> PyResult<Py<PyAny>> {
        let Some(legacy_runtime) = &self.legacy_runtime else {
            return Err(pyo3::exceptions::PyNotImplementedError::new_err(
                "Rust backend does not support this DataWeave feature and legacy fallback is disabled",
            ));
        };
        let kwargs = PyDict::new(py);
        if let Some(vars) = vars {
            kwargs.set_item("vars", vars)?;
        }
        if let Some(payload_format) = payload_format {
            kwargs.set_item("payload_format", payload_format)?;
        }
        if let Some(payload_format_options) = payload_format_options {
            kwargs.set_item("payload_format_options", payload_format_options)?;
        }
        kwargs.set_item("render_output", render_output)?;

        let result = legacy_runtime.bind(py).call_method(
            "execute",
            (script_source, payload),
            Some(&kwargs),
        )?;
        self.set_last_engine("python-legacy");
        Ok(result.unbind())
    }

    fn unsupported_or_legacy(
        &self,
        py: Python<'_>,
        script_source: &str,
        payload: Py<PyAny>,
        vars: Option<Py<PyAny>>,
        payload_format: Option<&str>,
        payload_format_options: Option<Py<PyAny>>,
        render_output: bool,
        rust_error: Option<PyErr>,
    ) -> PyResult<Py<PyAny>> {
        if self.allow_legacy_fallback {
            return self.execute_legacy(
                py,
                script_source,
                payload,
                vars,
                payload_format,
                payload_format_options,
                render_output,
            );
        }
        self.set_last_engine("rust-unsupported");
        Err(rust_error.unwrap_or_else(|| {
            pyo3::exceptions::PyNotImplementedError::new_err(
                "Rust backend does not support this DataWeave feature and legacy fallback is disabled",
            )
        }))
    }
}

fn py_to_json(py: Python<'_>, value: &Bound<'_, PyAny>) -> PyResult<serde_json::Value> {
    let json = PyModule::import(py, "json")?;
    let bridge = PyModule::import(py, "dwpy._rust_bridge")?;
    let kwargs = PyDict::new(py);
    kwargs.set_item("default", bridge.getattr("json_default")?)?;
    let encoded: String = json
        .call_method("dumps", (value,), Some(&kwargs))?
        .extract()?;
    serde_json::from_str(&encoded)
        .map_err(|err| pyo3::exceptions::PyValueError::new_err(err.to_string()))
}

fn json_to_py(py: Python<'_>, value: serde_json::Value) -> PyResult<Py<PyAny>> {
    if contains_special_marker(&value) {
        return json_to_py_preserving_dw_objects(py, value);
    }
    json_to_py_plain(py, value)
}

fn json_to_py_plain(py: Python<'_>, value: serde_json::Value) -> PyResult<Py<PyAny>> {
    let json = PyModule::import(py, "json")?;
    let encoded = serde_json::to_string(&value)
        .map_err(|err| pyo3::exceptions::PyValueError::new_err(err.to_string()))?;
    Ok(json.call_method1("loads", (encoded,))?.unbind())
}

fn contains_special_marker(value: &serde_json::Value) -> bool {
    match value {
        serde_json::Value::Array(items) => items.iter().any(contains_special_marker),
        serde_json::Value::Object(map) => {
            map.contains_key("__dwpy_object_pairs")
                || map.contains_key("__dwpy_binary")
                || map.contains_key("__dwpy_nonfinite")
                || map.contains_key("__dwpy_temporal")
                || map.contains_key("__dwpy_period")
                || map.values().any(contains_special_marker)
        }
        _ => false,
    }
}

fn json_to_py_preserving_dw_objects(
    py: Python<'_>,
    value: serde_json::Value,
) -> PyResult<Py<PyAny>> {
    match value {
        serde_json::Value::Array(items) => {
            let output = PyList::empty(py);
            for item in items {
                output.append(json_to_py_preserving_dw_objects(py, item)?)?;
            }
            Ok(output.unbind().into())
        }
        serde_json::Value::Object(map)
            if map.len() == 1 && map.contains_key("__dwpy_object_pairs") =>
        {
            let formats = PyModule::import(py, "dwpy.formats")?;
            let object = formats.getattr("DWObject")?.call0()?;
            let pairs = map
                .get("__dwpy_object_pairs")
                .and_then(serde_json::Value::as_array)
                .ok_or_else(|| {
                    pyo3::exceptions::PyValueError::new_err("invalid DWObject pair marker")
                })?;
            for pair in pairs {
                let pair = pair.as_object().ok_or_else(|| {
                    pyo3::exceptions::PyValueError::new_err("invalid DWObject pair entry")
                })?;
                let key = pair
                    .get("key")
                    .and_then(serde_json::Value::as_str)
                    .ok_or_else(|| {
                        pyo3::exceptions::PyValueError::new_err("invalid DWObject pair key")
                    })?;
                let value = pair
                    .get("value")
                    .cloned()
                    .unwrap_or(serde_json::Value::Null);
                object.call_method1("add", (key, json_to_py_preserving_dw_objects(py, value)?))?;
            }
            Ok(object.unbind())
        }
        serde_json::Value::Object(map) if map.len() == 1 && map.contains_key("__dwpy_binary") => {
            let bytes = map
                .get("__dwpy_binary")
                .and_then(serde_json::Value::as_array)
                .ok_or_else(|| pyo3::exceptions::PyValueError::new_err("invalid binary marker"))?
                .iter()
                .map(|item| {
                    let byte = item.as_u64().ok_or_else(|| {
                        pyo3::exceptions::PyValueError::new_err("invalid binary byte")
                    })?;
                    if byte > 255 {
                        return Err(pyo3::exceptions::PyValueError::new_err(
                            "invalid binary byte",
                        ));
                    }
                    Ok(byte as u8)
                })
                .collect::<PyResult<Vec<_>>>()?;
            Ok(PyBytes::new(py, &bytes).unbind().into())
        }
        serde_json::Value::Object(map)
            if map.len() == 1 && map.contains_key("__dwpy_nonfinite") =>
        {
            let value = map
                .get("__dwpy_nonfinite")
                .and_then(serde_json::Value::as_str)
                .ok_or_else(|| {
                    pyo3::exceptions::PyValueError::new_err("invalid nonfinite marker")
                })?;
            let value = match value {
                "nan" => f64::NAN,
                "inf" => f64::INFINITY,
                "-inf" => f64::NEG_INFINITY,
                _ => {
                    return Err(pyo3::exceptions::PyValueError::new_err(
                        "invalid nonfinite value",
                    ))
                }
            };
            Ok(PyFloat::new(py, value).unbind().into())
        }
        serde_json::Value::Object(map) if map.contains_key("__dwpy_temporal") => {
            let kind = map
                .get("__dwpy_temporal")
                .and_then(serde_json::Value::as_str)
                .ok_or_else(|| {
                    pyo3::exceptions::PyValueError::new_err("invalid temporal marker")
                })?;
            let value = map
                .get("value")
                .and_then(serde_json::Value::as_str)
                .ok_or_else(|| pyo3::exceptions::PyValueError::new_err("invalid temporal value"))?;
            let datetime = PyModule::import(py, "datetime")?;
            let object = match kind {
                "datetime" => datetime
                    .getattr("datetime")?
                    .call_method1("fromisoformat", (value.replace('Z', "+00:00"),))?,
                "date" => datetime
                    .getattr("date")?
                    .call_method1("fromisoformat", (value,))?,
                "time" => datetime
                    .getattr("time")?
                    .call_method1("fromisoformat", (value.replace('Z', "+00:00"),))?,
                _ => {
                    return Err(pyo3::exceptions::PyValueError::new_err(
                        "invalid temporal kind",
                    ))
                }
            };
            Ok(object.unbind())
        }
        serde_json::Value::Object(map) if map.contains_key("__dwpy_period") => {
            let value = map
                .get("text")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("");
            Ok(value.into_pyobject(py)?.unbind().into())
        }
        serde_json::Value::Object(map) => {
            let output = PyDict::new(py);
            for (key, value) in map {
                output.set_item(key, json_to_py_preserving_dw_objects(py, value)?)?;
            }
            Ok(output.unbind().into())
        }
        other => json_to_py_plain(py, other),
    }
}

fn dataweave_evaluation_error(py: Python<'_>, message: &str) -> PyResult<PyErr> {
    let module = PyModule::import(py, "dwpy._python_runtime")?;
    let error_cls = module.getattr("DataWeaveEvaluationError")?;
    Ok(PyErr::from_value(error_cls.call1((message,))?))
}

fn rust_core_error(py: Python<'_>, err: dwpy_core::DwError, script_source: &str) -> PyErr {
    let raw_message = err.to_string();
    if raw_message.starts_with("DataWeave parse error:") {
        return dataweave_parse_error(py, &parse_error_message(&raw_message, script_source))
            .unwrap_or_else(|_| pyo3::exceptions::PyValueError::new_err(raw_message));
    }
    let message = rust_core_error_message(&raw_message, script_source);
    dataweave_evaluation_error(py, &message)
        .unwrap_or_else(|_| pyo3::exceptions::PyNotImplementedError::new_err(message))
}

fn rust_core_error_message(message: &str, script_source: &str) -> String {
    if message.contains("expected number, got")
        || (message.contains("cannot coerce string") && message.contains("to Number"))
    {
        return "You called the function '+' with these arguments, but it expects one of these combinations:\n(Number, Number)\n\nLocation:\nmain (line: 1, column: 1)".to_string();
    }
    if let Some((name, line, column, line_text)) = unresolved_infix_location(script_source) {
        return format!(
            "Unable to resolve reference of `{name}`.\n\n{line}| {line_text}\n\nLocation:\nmain (line: {line}, column: {column})"
        );
    }
    message.to_string()
}

fn dataweave_parse_error(py: Python<'_>, message: &str) -> PyResult<PyErr> {
    let module = PyModule::import(py, "dwpy.parser")?;
    let error_cls = module.getattr("ParseError")?;
    Ok(PyErr::from_value(error_cls.call1((message,))?))
}

fn parse_error_message(message: &str, script_source: &str) -> String {
    let (line, column) = script_source
        .find(",,")
        .map(|index| line_column(script_source, index + 1))
        .unwrap_or_else(|| line_column(script_source, script_source.len()));
    format!("{message} (line: {line}, column: {column})")
}

fn unresolved_infix_location(script_source: &str) -> Option<(String, usize, usize, String)> {
    for (line_index, line) in script_source.lines().enumerate() {
        let Some(close_brace) = line.find('}') else {
            continue;
        };
        let after_brace = &line[close_brace + 1..];
        let trimmed = after_brace.trim_start();
        let leading_spaces = after_brace.len() - trimmed.len();
        let name = trimmed
            .chars()
            .take_while(|ch| ch.is_ascii_alphanumeric() || *ch == '_')
            .collect::<String>();
        if name.is_empty() {
            continue;
        }
        let after_name = trimmed[name.len()..].trim_start();
        if !after_name.starts_with('{') {
            continue;
        }
        return Some((
            name,
            line_index + 1,
            close_brace + 1 + leading_spaces + 1,
            line.to_string(),
        ));
    }
    None
}

fn line_column(source: &str, byte_index: usize) -> (usize, usize) {
    let mut line = 1usize;
    let mut column = 1usize;
    for (index, ch) in source.char_indices() {
        if index >= byte_index {
            break;
        }
        if ch == '\n' {
            line += 1;
            column = 1;
        } else {
            column += 1;
        }
    }
    (line, column)
}

fn is_core_candidate(script_source: &str) -> bool {
    let output_line = script_source
        .lines()
        .map(str::trim)
        .find_map(|line| line.strip_prefix("output ").map(str::trim));

    let output = output_line.and_then(|line| line.split_whitespace().next());

    match output {
        None | Some("application/python") => true,
        Some(value)
            if value == "json" || value == "application/json" || value.ends_with("+json") =>
        {
            let Some(line) = output_line else {
                return false;
            };
            is_supported_json_output_line(line)
        }
        Some("application/csv") => {
            let Some(line) = output_line else {
                return false;
            };
            is_supported_csv_output_line(line)
        }
        Some("application/xml") => {
            let Some(line) = output_line else {
                return false;
            };
            is_supported_xml_output_line(line)
        }
        Some("yaml")
        | Some("yml")
        | Some("application/yaml")
        | Some("application/x-yaml")
        | Some("text/yaml")
        | Some("text/x-yaml") => {
            let Some(line) = output_line else {
                return false;
            };
            is_supported_yaml_output_line(line)
        }
        Some("text/plain") | Some("plain") => true,
        Some("markdown") | Some("md") | Some("text/markdown") | Some("text/x-markdown") => {
            let Some(line) = output_line else {
                return false;
            };
            is_supported_markdown_output_line(line)
        }
        _ => false,
    }
}

fn is_supported_payload_request(
    payload_format: Option<&str>,
    payload_format_options: Option<&serde_json::Value>,
) -> bool {
    if !is_supported_payload_options(payload_format, payload_format_options) {
        return false;
    }
    match payload_format {
        None => true,
        Some("csv") | Some("application/csv") | Some("text/csv") => true,
        value => matches!(
            value,
            Some("json")
                | Some("application/json")
                | Some("xml")
                | Some("application/xml")
                | Some("text/xml")
                | Some("yaml")
                | Some("yml")
                | Some("application/yaml")
                | Some("application/x-yaml")
                | Some("text/yaml")
                | Some("text/x-yaml")
                | Some("csv")
                | Some("application/csv")
                | Some("text/csv")
                | Some("markdown")
                | Some("md")
                | Some("text/markdown")
                | Some("text/x-markdown")
        ),
    }
}

fn is_supported_payload_options(
    payload_format: Option<&str>,
    payload_format_options: Option<&serde_json::Value>,
) -> bool {
    let Some(options) = payload_format_options else {
        return true;
    };
    let Some(map) = options.as_object() else {
        return false;
    };
    match payload_format {
        Some("csv") | Some("application/csv") | Some("text/csv") => map
            .keys()
            .all(|key| matches!(key.as_str(), "separator" | "quote" | "header")),
        Some("markdown") | Some("md") | Some("text/markdown") | Some("text/x-markdown") => {
            map.keys().all(|key| key == "header")
        }
        _ => map.is_empty(),
    }
}

fn is_supported_json_output_line(line: &str) -> bool {
    let mut tokens = line.split_whitespace();
    let Some(mime) = tokens.next() else {
        return false;
    };
    if !(mime == "json" || mime == "application/json" || mime.ends_with("+json")) {
        return false;
    }
    let mut tokens = tokens.collect::<Vec<_>>();
    if tokens.first() == Some(&"with") && matches!(tokens.get(1), Some(&"json" | &"binary")) {
        tokens.drain(..2);
    }
    parse_output_option_tokens(tokens)
        .is_some_and(|options| options.into_iter().all(is_supported_json_output_option))
}

fn parse_output_option_tokens(tokens: Vec<&str>) -> Option<Vec<(&str, &str)>> {
    let mut options = Vec::new();
    let mut index = 0usize;
    while index < tokens.len() {
        let token = tokens[index];
        if let Some((key, value)) = token.split_once('=') {
            if value.is_empty() {
                let value = *tokens.get(index + 1)?;
                options.push((key, value));
                index += 2;
            } else {
                options.push((key, value));
                index += 1;
            }
            continue;
        }
        if tokens.get(index + 1) == Some(&"=") {
            let value = *tokens.get(index + 2)?;
            options.push((token, value));
            index += 3;
            continue;
        }
        return None;
    }
    Some(options)
}

fn is_supported_json_output_option((key, value): (&str, &str)) -> bool {
    let value = value.trim_matches('"');
    if key == "indent" && (value == "false" || value.parse::<usize>().is_ok()) {
        return true;
    }
    matches!(key, "ensure_ascii" | "sort_keys" | "duplicateKeyAsArray")
        && matches!(value, "true" | "false" | "True" | "False")
}

fn is_supported_csv_output_line(line: &str) -> bool {
    let mut tokens = line.split_whitespace();
    if tokens.next() != Some("application/csv") {
        return false;
    }
    for token in tokens {
        if token.starts_with("separator=")
            || token.starts_with("quote=")
            || token.starts_with("header=")
            || token.starts_with("columns=")
        {
            continue;
        }
        return false;
    }
    true
}

fn is_supported_xml_output_line(line: &str) -> bool {
    let mut tokens = line.split_whitespace();
    if tokens.next() != Some("application/xml") {
        return false;
    }
    for token in tokens {
        if token.starts_with("root=") || token.starts_with("inlineCloseOn=") {
            continue;
        }
        return false;
    }
    true
}

fn is_supported_yaml_output_line(line: &str) -> bool {
    let mut tokens = line.split_whitespace();
    matches!(
        tokens.next(),
        Some(
            "yaml"
                | "yml"
                | "application/yaml"
                | "application/x-yaml"
                | "text/yaml"
                | "text/x-yaml"
        )
    ) && tokens
        .all(|token| token.starts_with("skipNullOn=") || token.starts_with("writeDeclaration="))
}

fn is_supported_markdown_output_line(line: &str) -> bool {
    let mut tokens = line.split_whitespace();
    if !matches!(
        tokens.next(),
        Some("markdown" | "md" | "text/markdown" | "text/x-markdown")
    ) {
        return false;
    }
    tokens.all(|token| token.starts_with("header=") || token.starts_with("columns="))
}

#[pymodule]
fn _dwpy_rust(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_class::<RustDataWeaveRuntime>()?;
    Ok(())
}
