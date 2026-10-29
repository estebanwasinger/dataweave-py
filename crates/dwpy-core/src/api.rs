//! Stable JSON-facing adapters. Only this boundary knows the wire encoding.
use crate::{CompiledScript, DwError, DwValue, ExecutionContext, ExecutionOptions};
use serde_json::Value;

impl CompiledScript {
    pub fn execute(
        &self,
        payload: Value,
        vars: Option<Value>,
        options: &ExecutionOptions,
    ) -> Result<Value, DwError> {
        self.execute_value(DwValue::from(payload), vars.map(DwValue::from), options)
            .map(|value| value.to_json())
    }
    pub fn execute_with_context(
        &self,
        payload: Value,
        context: ExecutionContext,
        options: &ExecutionOptions,
    ) -> Result<Value, DwError> {
        self.execute_value_with_context(DwValue::from(payload), context, options)
            .map(|value| value.to_json())
    }
    pub fn execute_to_writer<W: std::io::Write>(
        &self,
        payload: Value,
        vars: Option<Value>,
        options: &ExecutionOptions,
        writer: &mut W,
    ) -> Result<(), DwError> {
        self.execute_value_to_writer(payload.into(), vars.map(DwValue::from), options, writer)
    }
    pub fn execute_to_writer_with_context<W: std::io::Write>(
        &self,
        payload: Value,
        context: ExecutionContext,
        options: &ExecutionOptions,
        writer: &mut W,
    ) -> Result<(), DwError> {
        self.execute_value_to_writer_with_context(payload.into(), context, options, writer)
    }
}

pub fn execute_smoke(script: &str, payload: Value) -> Result<Value, DwError> {
    execute_json(script, payload, false)
}
pub fn execute_json(script: &str, payload: Value, render_output: bool) -> Result<Value, DwError> {
    crate::execute_value(script, DwValue::from(payload), render_output).map(|value| value.to_json())
}
pub fn execute_json_with_vars(
    script: &str,
    payload: Value,
    vars: Value,
    render_output: bool,
) -> Result<Value, DwError> {
    crate::execute_value_with_vars(script, payload.into(), vars.into(), render_output)
        .map(|value| value.to_json())
}
pub fn execute_json_with_context(
    script: &str,
    payload: Value,
    context: ExecutionContext,
    render_output: bool,
) -> Result<Value, DwError> {
    crate::execute_value_with_context(script, payload.into(), context, render_output)
        .map(|value| value.to_json())
}

/// Explicit typed-wire adapter for Python and WASM bridges. Ordinary JSON APIs
/// never reinterpret marker-shaped user objects as temporal scalars.
pub fn execute_transport_with_context(
    script: &str,
    payload: Value,
    vars: Option<Value>,
    attributes: Option<Value>,
    properties: Option<std::collections::BTreeMap<String, String>>,
    render_output: bool,
) -> Result<Value, DwError> {
    execute_transport_with_loggers(
        script,
        payload,
        vars,
        attributes,
        properties,
        render_output,
        crate::Loggers::default(),
    )
}

/// Typed-wire execution with host-provided sinks, without changing the result.
pub fn execute_transport_with_loggers(
    script: &str,
    payload: Value,
    vars: Option<Value>,
    attributes: Option<Value>,
    properties: Option<std::collections::BTreeMap<String, String>>,
    render_output: bool,
    loggers: crate::Loggers,
) -> Result<Value, DwError> {
    execute_transport_with_bindings_and_loggers(
        script,
        payload,
        vars,
        attributes,
        properties,
        None,
        render_output,
        loggers,
    )
}

pub fn execute_transport_with_bindings_and_loggers(
    script: &str,
    payload: Value,
    vars: Option<Value>,
    attributes: Option<Value>,
    properties: Option<std::collections::BTreeMap<String, String>>,
    bindings: Option<Value>,
    render_output: bool,
    loggers: crate::Loggers,
) -> Result<Value, DwError> {
    execute_transport_with_output_mime_type(
        script,
        payload,
        vars,
        attributes,
        properties,
        bindings,
        render_output,
        loggers,
        None,
    )
}

pub fn execute_transport_with_output_mime_type(
    script: &str,
    payload: Value,
    vars: Option<Value>,
    attributes: Option<Value>,
    properties: Option<std::collections::BTreeMap<String, String>>,
    bindings: Option<Value>,
    render_output: bool,
    loggers: crate::Loggers,
    output_mime_type: Option<&str>,
) -> Result<Value, DwError> {
    let mut context = ExecutionContext::from_values(
        vars.map(DwValue::from_transport_json).transpose()?,
        attributes.map(DwValue::from_transport_json).transpose()?,
        properties,
    )?;
    if let Some(bindings) = bindings {
        context = context.with_bindings(DwValue::from_transport_json(bindings)?)?;
    }
    context.loggers = loggers;
    crate::compile_with_output_mime_type(
        script,
        if render_output {
            output_mime_type
        } else {
            None
        },
    )?
    .execute_value_with_context(
        DwValue::from_transport_json(payload)?,
        context,
        &ExecutionOptions {
            render_output,
            ..ExecutionOptions::default()
        },
    )
    .map(|value| value.to_transport_json())
}

pub fn parse_payload_format(payload: Value, format: Option<&str>) -> Result<Value, DwError> {
    parse_payload_format_with_options(payload, format, None)
}
pub fn parse_payload_format_with_options(
    payload: Value,
    format: Option<&str>,
    options: Option<&Value>,
) -> Result<Value, DwError> {
    // No format means the bridge payload has not been decoded yet.
    if format.is_none() {
        return Ok(payload);
    }
    let options = options.cloned().map(DwValue::from);
    crate::parse_native_payload_format_with_options(payload.into(), format, options.as_ref())
        .map(|value| value.to_json())
}
pub fn infer_type_descriptor(
    script: &str,
    payload: Option<Value>,
    vars: Option<Value>,
) -> Result<Value, DwError> {
    infer_type_descriptor_with_context(script, payload, vars, None)
}
pub fn infer_type_descriptor_with_context(
    script: &str,
    payload: Option<Value>,
    vars: Option<Value>,
    attributes: Option<Value>,
) -> Result<Value, DwError> {
    crate::infer_native_type_descriptor_with_context(
        script,
        payload.map(DwValue::from),
        vars.map(DwValue::from),
        attributes.map(DwValue::from),
    )
    .map(|value| value.to_json())
}
pub fn analyze_expression(
    script: &str,
    payload: Option<Value>,
    vars: Option<Value>,
) -> Result<Value, DwError> {
    analyze_expression_with_context(script, payload, vars, None)
}
pub fn analyze_expression_with_context(
    script: &str,
    payload: Option<Value>,
    vars: Option<Value>,
    attributes: Option<Value>,
) -> Result<Value, DwError> {
    crate::analyze_native_expression_with_context(
        script,
        payload.map(DwValue::from),
        vars.map(DwValue::from),
        attributes.map(DwValue::from),
    )
    .map(|value| value.to_json())
}

pub fn analyze_transport_expression_with_context(
    script: &str,
    payload: Option<Value>,
    vars: Option<Value>,
    attributes: Option<Value>,
) -> Result<Value, DwError> {
    crate::analyze_native_expression_with_context(
        script,
        payload.map(DwValue::from_transport_json).transpose()?,
        vars.map(DwValue::from_transport_json).transpose()?,
        attributes.map(DwValue::from_transport_json).transpose()?,
    )
    .map(|value| value.to_json())
}
