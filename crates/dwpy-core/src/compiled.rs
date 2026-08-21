use std::collections::HashSet;
use std::io::Write;

use serde_json::{Map, Value};

use crate::json::{render_json_value, JsonOutputOptions};
use crate::literals::{parse_literal, parse_string_literal};
use crate::operators::number_value;
use crate::syntax::{
    split_top_level_arrow, split_top_level_char, split_top_level_keyword,
    split_top_level_keyword_or_call_operator, split_top_level_operator, strip_wrapping_parens,
};
use crate::{evaluate_expression_scoped, is_truthy, number_result, stable_marker, DwError};

const LAZY_OPERATORS: &[&str] = &[
    "map",
    "filter",
    "flatMap",
    "distinctBy",
    "takeWhile",
    "dropWhile",
];
const TERMINAL_OPERATORS: &[&str] = &[
    "some",
    "every",
    "firstWith",
    "indexWhere",
    "countBy",
    "sumBy",
];

#[derive(Clone)]
pub(crate) enum CompiledBody {
    Sequence(CompiledSequence),
    Reduce {
        sequence: CompiledSequence,
        reducer: CompiledReducer,
        default_source: Option<String>,
    },
    Terminal {
        sequence: CompiledSequence,
        operation: TerminalOperation,
        lambda: CompiledLambda,
    },
}

#[derive(Clone)]
pub(crate) struct CompiledSequence {
    source: SequenceSource,
    operations: Vec<SequenceOperation>,
}

#[derive(Clone)]
enum SequenceSource {
    Range { start: String, end: String },
    Expression(String),
}

#[derive(Clone)]
enum SequenceOperation {
    Map(CompiledLambda),
    Filter(CompiledLambda),
    FlatMap(CompiledLambda),
    DistinctBy(CompiledLambda),
    TakeWhile(CompiledLambda),
    DropWhile(CompiledLambda),
}

#[derive(Clone, Copy)]
pub(crate) enum TerminalOperation {
    Some,
    Every,
    FirstWith,
    IndexWhere,
    CountBy,
    SumBy,
}

#[derive(Clone)]
pub(crate) struct CompiledLambda {
    expression: FastExpr,
}

#[derive(Clone)]
pub(crate) struct CompiledReducer {
    expression: FastExpr,
}

#[derive(Clone)]
enum FastExpr {
    Literal(FastValue),
    Item,
    Secondary,
    Array(Vec<FastExpr>),
    Path {
        root: FastRoot,
        fields: Vec<String>,
    },
    Binary {
        left: Box<FastExpr>,
        operation: FastBinary,
        right: Box<FastExpr>,
    },
}

#[derive(Clone, Copy)]
enum FastRoot {
    Item,
    Secondary,
}

#[derive(Clone, Copy)]
enum FastBinary {
    Add,
    Subtract,
    Multiply,
    Divide,
    Equal,
    NotEqual,
    Greater,
    GreaterEqual,
    Less,
    LessEqual,
    And,
    Or,
}

#[derive(Clone, Debug)]
enum FastValue {
    Integer(i64),
    Number(f64),
    Json(Value),
}

impl FastValue {
    fn from_json(value: Value) -> Result<Self, DwError> {
        if value.is_number() {
            if let Some(value) = value.as_i64() {
                return Ok(Self::Integer(value));
            }
            return Ok(Self::Number(number_value(&value)?));
        }
        Ok(Self::Json(value))
    }

    fn to_json(&self) -> Result<Value, DwError> {
        match self {
            Self::Integer(value) => Ok(Value::Number((*value).into())),
            Self::Number(value) => number_result(*value),
            Self::Json(value) => Ok(value.clone()),
        }
    }

    fn into_json(self) -> Result<Value, DwError> {
        match self {
            Self::Integer(value) => Ok(Value::Number(value.into())),
            Self::Number(value) => number_result(value),
            Self::Json(value) => Ok(value),
        }
    }

    fn number(&self) -> Result<f64, DwError> {
        match self {
            Self::Integer(value) => Ok(*value as f64),
            Self::Number(value) => Ok(*value),
            Self::Json(value) => number_value(value),
        }
    }

    fn truthy(&self) -> Result<bool, DwError> {
        Ok(match self {
            Self::Integer(_) | Self::Number(_) => true,
            Self::Json(value) => is_truthy(value),
        })
    }

    fn stable_key(&self) -> Result<String, DwError> {
        match self {
            Self::Integer(value) => Ok(value.to_string()),
            Self::Number(value) => Ok(value.to_string()),
            Self::Json(value) => Ok(stable_marker(value)),
        }
    }
}

pub(crate) fn compile_body(source: &str) -> Option<CompiledBody> {
    let source = strip_wrapping_parens(source.trim());
    if let Some((left, operator, reducer_source)) =
        split_top_level_keyword_or_call_operator(source, &["reduce"])
    {
        if operator == "reduce" {
            let sequence = compile_sequence(left, true)?;
            let (reducer, default_source) = compile_reducer(reducer_source)?;
            return Some(CompiledBody::Reduce {
                sequence,
                reducer,
                default_source,
            });
        }
    }
    if let Some((left, operator, lambda_source)) =
        split_top_level_keyword_or_call_operator(source, TERMINAL_OPERATORS)
    {
        let sequence = compile_sequence(left, true)?;
        let lambda = compile_lambda(lambda_source)?;
        let operation = match operator {
            "some" => TerminalOperation::Some,
            "every" => TerminalOperation::Every,
            "firstWith" => TerminalOperation::FirstWith,
            "indexWhere" => TerminalOperation::IndexWhere,
            "countBy" => TerminalOperation::CountBy,
            "sumBy" => TerminalOperation::SumBy,
            _ => return None,
        };
        return Some(CompiledBody::Terminal {
            sequence,
            operation,
            lambda,
        });
    }
    compile_sequence(source, false).map(CompiledBody::Sequence)
}

fn compile_sequence(source: &str, allow_expression_source: bool) -> Option<CompiledSequence> {
    let source = strip_wrapping_parens(source.trim());
    if let Some((left, operator, lambda_source)) =
        split_top_level_keyword_or_call_operator(source, LAZY_OPERATORS)
    {
        let mut sequence = compile_sequence(left, true)?;
        let lambda = compile_lambda(lambda_source)?;
        sequence.operations.push(match operator {
            "map" => SequenceOperation::Map(lambda),
            "filter" => SequenceOperation::Filter(lambda),
            "flatMap" => SequenceOperation::FlatMap(lambda),
            "distinctBy" => SequenceOperation::DistinctBy(lambda),
            "takeWhile" => SequenceOperation::TakeWhile(lambda),
            "dropWhile" => SequenceOperation::DropWhile(lambda),
            _ => return None,
        });
        return Some(sequence);
    }
    if let Some((start, end)) = split_top_level_keyword(source, "to") {
        return Some(CompiledSequence {
            source: SequenceSource::Range {
                start: start.trim().to_string(),
                end: end.trim().to_string(),
            },
            operations: Vec::new(),
        });
    }
    allow_expression_source.then(|| CompiledSequence {
        source: SequenceSource::Expression(source.to_string()),
        operations: Vec::new(),
    })
}

fn compile_reducer(source: &str) -> Option<(CompiledReducer, Option<String>)> {
    let source = strip_wrapping_parens(source.trim());
    let (parameters_source, body) = split_top_level_arrow(source)?;
    let parameters_source = strip_wrapping_parens(parameters_source.trim());
    let parameters = crate::syntax::split_top_level(parameters_source, ',');
    let first = parameters.first()?.trim();
    let second = parameters.get(1)?.trim();
    let first_name = first.split(':').next()?.trim();
    let (second_name, default_source) =
        if let Some((name, default)) = split_top_level_char(second, '=') {
            (
                name.split(':').next()?.trim(),
                Some(default.trim().to_string()),
            )
        } else {
            (second.split(':').next()?.trim(), None)
        };
    let expression = compile_fast_expr(body.trim(), first_name, second_name)?;
    Some((CompiledReducer { expression }, default_source))
}

fn compile_lambda(source: &str) -> Option<CompiledLambda> {
    let source = strip_wrapping_parens(source.trim());
    let (parameters_source, body) = split_top_level_arrow(source)
        .map(|(parameters, body)| (strip_wrapping_parens(parameters.trim()), body.trim()))
        .unwrap_or(("", source));
    let parameters = crate::syntax::split_top_level(parameters_source, ',');
    let first_name = parameters
        .first()
        .map(|parameter| parameter.split(':').next().unwrap_or("$").trim())
        .filter(|name| !name.is_empty())
        .unwrap_or("$");
    let implicit_secondary = "$$";
    let second_name = parameters
        .get(1)
        .map(|parameter| {
            parameter
                .split([':', '='])
                .next()
                .unwrap_or(implicit_secondary)
                .trim()
        })
        .filter(|name| !name.is_empty())
        .unwrap_or(implicit_secondary);
    Some(CompiledLambda {
        expression: compile_fast_expr(body, first_name, second_name)?,
    })
}

fn compile_fast_expr(source: &str, item_name: &str, secondary_name: &str) -> Option<FastExpr> {
    let source = strip_wrapping_parens(source.trim());
    if let Some(value) = parse_literal(source).ok().flatten() {
        return FastValue::from_json(value).ok().map(FastExpr::Literal);
    }
    if let Some(value) = parse_string_literal(source).ok().flatten() {
        return Some(FastExpr::Literal(FastValue::Json(Value::String(value))));
    }
    if source == item_name || source == "$" {
        return Some(FastExpr::Item);
    }
    if source == secondary_name || source == "$$" {
        return Some(FastExpr::Secondary);
    }
    if let Some(inner) = source
        .strip_prefix('[')
        .and_then(|value| value.strip_suffix(']'))
    {
        if inner.trim().is_empty() {
            return Some(FastExpr::Array(Vec::new()));
        }
        let items = crate::syntax::split_top_level(inner, ',')
            .into_iter()
            .map(|item| compile_fast_expr(item, item_name, secondary_name))
            .collect::<Option<Vec<_>>>()?;
        return Some(FastExpr::Array(items));
    }
    if let Some(path) = compile_path(source, item_name, secondary_name) {
        return Some(path);
    }
    if let Some((left, right)) = split_top_level_keyword(source, "or") {
        return compile_binary(left, FastBinary::Or, right, item_name, secondary_name);
    }
    if let Some((left, right)) = split_top_level_keyword(source, "and") {
        return compile_binary(left, FastBinary::And, right, item_name, secondary_name);
    }
    if let Some((left, operation, right)) =
        split_top_level_operator(source, &["==", "!=", ">=", "<=", ">", "<"])
    {
        let operation = match operation {
            "==" => FastBinary::Equal,
            "!=" => FastBinary::NotEqual,
            ">" => FastBinary::Greater,
            ">=" => FastBinary::GreaterEqual,
            "<" => FastBinary::Less,
            "<=" => FastBinary::LessEqual,
            _ => return None,
        };
        return compile_binary(left, operation, right, item_name, secondary_name);
    }
    if let Some((left, operation, right)) = split_top_level_operator(source, &["+", "-"]) {
        let operation = if operation == "+" {
            FastBinary::Add
        } else {
            FastBinary::Subtract
        };
        return compile_binary(left, operation, right, item_name, secondary_name);
    }
    if let Some((left, operation, right)) = split_top_level_operator(source, &["*", "/"]) {
        let operation = if operation == "*" {
            FastBinary::Multiply
        } else {
            FastBinary::Divide
        };
        return compile_binary(left, operation, right, item_name, secondary_name);
    }
    None
}

fn compile_binary(
    left: &str,
    operation: FastBinary,
    right: &str,
    item_name: &str,
    secondary_name: &str,
) -> Option<FastExpr> {
    Some(FastExpr::Binary {
        left: Box::new(compile_fast_expr(left, item_name, secondary_name)?),
        operation,
        right: Box::new(compile_fast_expr(right, item_name, secondary_name)?),
    })
}

fn compile_path(source: &str, item_name: &str, secondary_name: &str) -> Option<FastExpr> {
    let (root, remainder) = if let Some(remainder) = source.strip_prefix(&format!("{item_name}.")) {
        (FastRoot::Item, remainder)
    } else if let Some(remainder) = source.strip_prefix("$.") {
        (FastRoot::Item, remainder)
    } else if let Some(remainder) = source.strip_prefix(&format!("{secondary_name}.")) {
        (FastRoot::Secondary, remainder)
    } else if let Some(remainder) = source.strip_prefix("$$.") {
        (FastRoot::Secondary, remainder)
    } else {
        return None;
    };
    let fields = remainder
        .split('.')
        .filter(|field| !field.is_empty())
        .map(str::to_string)
        .collect::<Vec<_>>();
    if fields.is_empty()
        || fields.iter().any(|field| {
            !field
                .chars()
                .all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
        })
    {
        return None;
    }
    Some(FastExpr::Path { root, fields })
}

impl CompiledBody {
    pub(crate) fn evaluate(
        &self,
        payload: &Value,
        locals: &Map<String, Value>,
        max_materialized_bytes: usize,
    ) -> Result<Option<Value>, DwError> {
        match self {
            Self::Sequence(sequence) => {
                sequence.materialize(payload, locals, max_materialized_bytes)
            }
            Self::Reduce {
                sequence,
                reducer,
                default_source,
            } => {
                let default = default_source
                    .as_deref()
                    .map(|source| evaluate_expression_scoped(source, payload, locals))
                    .transpose()?
                    .map(FastValue::from_json)
                    .transpose()?;
                sequence.reduce(payload, locals, reducer, default)
            }
            Self::Terminal {
                sequence,
                operation,
                lambda,
            } => sequence.evaluate_terminal(payload, locals, *operation, lambda),
        }
    }

    pub(crate) fn write_json<W: Write>(
        &self,
        payload: &Value,
        locals: &Map<String, Value>,
        options: JsonOutputOptions,
        writer: &mut W,
    ) -> Result<bool, DwError> {
        let Self::Sequence(sequence) = self else {
            return Ok(false);
        };
        let mut first = true;
        let valid = sequence.for_each(payload, locals, |value, _| {
            if first {
                writer
                    .write_all(b"[")
                    .map_err(|error| DwError::Output(error.to_string()))?;
            }
            if options.indent.is_some() {
                if !first {
                    writer
                        .write_all(b",")
                        .map_err(|error| DwError::Output(error.to_string()))?;
                }
                writer
                    .write_all(b"\n")
                    .map_err(|error| DwError::Output(error.to_string()))?;
                write_indent(writer, options.indent.unwrap_or(2))?;
            } else if !first {
                writer
                    .write_all(b",")
                    .map_err(|error| DwError::Output(error.to_string()))?;
            }
            first = false;
            let rendered = render_json_value(&value.to_json()?, options)?;
            if let Some(indent) = options.indent {
                let continuation = format!("\n{}", " ".repeat(indent));
                let rendered = rendered.replace('\n', &continuation);
                writer
                    .write_all(rendered.as_bytes())
                    .map_err(|error| DwError::Output(error.to_string()))
            } else {
                writer
                    .write_all(rendered.as_bytes())
                    .map_err(|error| DwError::Output(error.to_string()))
            }
        })?;
        if !valid {
            return Ok(false);
        }
        if first {
            writer
                .write_all(b"[")
                .map_err(|error| DwError::Output(error.to_string()))?;
        }
        if options.indent.is_some() && !first {
            writer
                .write_all(b"\n")
                .map_err(|error| DwError::Output(error.to_string()))?;
        }
        writer
            .write_all(b"]")
            .map_err(|error| DwError::Output(error.to_string()))?;
        Ok(true)
    }

    pub(crate) fn write_ndjson<W: Write>(
        &self,
        payload: &Value,
        locals: &Map<String, Value>,
        directive: &str,
        writer: &mut W,
    ) -> Result<bool, DwError> {
        let Self::Sequence(sequence) = self else {
            return Ok(false);
        };
        let valid = sequence.for_each(payload, locals, |value, _| {
            let rendered =
                crate::ndjson::render_ndjson_record_output(&value.to_json()?, directive)?;
            writer
                .write_all(rendered.as_bytes())
                .map_err(|error| DwError::Output(error.to_string()))
        })?;
        Ok(valid)
    }

    pub(crate) fn write_csv<W: Write>(
        &self,
        payload: &Value,
        locals: &Map<String, Value>,
        directive: &str,
        writer: &mut W,
    ) -> Result<bool, DwError> {
        let Self::Sequence(sequence) = self else {
            return Ok(false);
        };
        let mut csv = crate::csv::CsvRecordWriter::new(directive);
        let valid = sequence.for_each(payload, locals, |value, _| {
            csv.write_record(&value.to_json()?, writer)
        })?;
        Ok(valid)
    }
}

fn write_indent<W: Write>(writer: &mut W, count: usize) -> Result<(), DwError> {
    const SPACES: &[u8; 64] = b"                                                                ";
    let mut remaining = count;
    while remaining > 0 {
        let chunk = remaining.min(SPACES.len());
        writer
            .write_all(&SPACES[..chunk])
            .map_err(|error| DwError::Output(error.to_string()))?;
        remaining -= chunk;
    }
    Ok(())
}

impl CompiledSequence {
    fn for_each<F>(
        &self,
        payload: &Value,
        locals: &Map<String, Value>,
        mut visitor: F,
    ) -> Result<bool, DwError>
    where
        F: FnMut(FastValue, usize) -> Result<(), DwError>,
    {
        self.for_each_while(payload, locals, true, |value, index| {
            visitor(value, index)?;
            Ok(true)
        })
    }

    fn for_each_while<F>(
        &self,
        payload: &Value,
        locals: &Map<String, Value>,
        allow_string_source: bool,
        mut visitor: F,
    ) -> Result<bool, DwError>
    where
        F: FnMut(FastValue, usize) -> Result<bool, DwError>,
    {
        let mut source_index = 0usize;
        let mut distinct = self
            .operations
            .iter()
            .map(|operation| match operation {
                SequenceOperation::DistinctBy(_) => Some(HashSet::new()),
                _ => None,
            })
            .collect::<Vec<_>>();
        let mut operation_indices = vec![0usize; self.operations.len()];
        let mut take_finished = vec![false; self.operations.len()];
        let mut drop_finished = vec![false; self.operations.len()];
        let mut process_source_item = |source_value: FastValue| -> Result<bool, DwError> {
            let mut values = vec![source_value];
            for (operation_index, operation) in self.operations.iter().enumerate() {
                let mut next_values = Vec::new();
                for value in values {
                    let operation_item_index = operation_indices[operation_index];
                    operation_indices[operation_index] = operation_item_index.saturating_add(1);
                    match operation {
                        SequenceOperation::Map(lambda) => {
                            next_values.push(lambda.expression.evaluate(
                                &value,
                                &FastValue::Integer(operation_item_index as i64),
                            )?)
                        }
                        SequenceOperation::Filter(lambda) => {
                            let selected = lambda.expression.evaluate(
                                &value,
                                &FastValue::Integer(operation_item_index as i64),
                            )?;
                            if selected.truthy()? {
                                next_values.push(value);
                            }
                        }
                        SequenceOperation::FlatMap(lambda) => {
                            let mapped = lambda.expression.evaluate(
                                &value,
                                &FastValue::Integer(operation_item_index as i64),
                            )?;
                            match mapped.into_json()? {
                                Value::Array(items) => {
                                    for item in items {
                                        next_values.push(FastValue::from_json(item)?);
                                    }
                                }
                                Value::Null => {}
                                other => next_values.push(FastValue::from_json(other)?),
                            }
                        }
                        SequenceOperation::DistinctBy(lambda) => {
                            let key = lambda.expression.evaluate(
                                &value,
                                &FastValue::Integer(operation_item_index as i64),
                            )?;
                            let seen = distinct[operation_index].as_mut().expect("distinct state");
                            if seen.insert(key.stable_key()?) {
                                next_values.push(value);
                            }
                        }
                        SequenceOperation::TakeWhile(lambda) => {
                            if take_finished[operation_index] {
                                continue;
                            }
                            let selected = lambda.expression.evaluate(
                                &value,
                                &FastValue::Integer(operation_item_index as i64),
                            )?;
                            if selected.truthy()? {
                                next_values.push(value);
                            } else {
                                take_finished[operation_index] = true;
                            }
                        }
                        SequenceOperation::DropWhile(lambda) => {
                            if drop_finished[operation_index] {
                                next_values.push(value);
                                continue;
                            }
                            let selected = lambda.expression.evaluate(
                                &value,
                                &FastValue::Integer(operation_item_index as i64),
                            )?;
                            if !selected.truthy()? {
                                drop_finished[operation_index] = true;
                                next_values.push(value);
                            }
                        }
                    }
                }
                values = next_values;
                if values.is_empty() {
                    break;
                }
            }
            for value in values {
                if !visitor(value, source_index)? {
                    return Ok(false);
                }
                source_index += 1;
            }
            if take_finished.iter().any(|finished| *finished) {
                return Ok(false);
            }
            Ok(true)
        };

        match &self.source {
            SequenceSource::Range { start, end } => {
                let start_value = evaluate_expression_scoped(start, payload, locals)?;
                let end_value = evaluate_expression_scoped(end, payload, locals)?;
                let start = number_value(&start_value)? as i64;
                let end = number_value(&end_value)? as i64;
                let step = if end >= start { 1i64 } else { -1i64 };
                let mut current = start;
                loop {
                    if !process_source_item(FastValue::Integer(current))? || current == end {
                        break;
                    }
                    current = current.checked_add(step).ok_or_else(|| {
                        DwError::UnsupportedFeature(format!(
                            "range from {start} to {end} overflows i64"
                        ))
                    })?;
                }
                Ok(true)
            }
            SequenceSource::Expression(source) => {
                match evaluate_expression_scoped(source, payload, locals)? {
                    Value::Array(items) => {
                        for item in items {
                            if !process_source_item(FastValue::from_json(item)?)? {
                                break;
                            }
                        }
                    }
                    Value::String(text) => {
                        if !allow_string_source {
                            return Ok(false);
                        }
                        for character in text.chars() {
                            if !process_source_item(FastValue::Json(Value::String(
                                character.to_string(),
                            )))? {
                                break;
                            }
                        }
                    }
                    _ => return Ok(false),
                }
                Ok(true)
            }
        }
    }

    fn evaluate_terminal(
        &self,
        payload: &Value,
        locals: &Map<String, Value>,
        operation: TerminalOperation,
        lambda: &CompiledLambda,
    ) -> Result<Option<Value>, DwError> {
        let mut boolean = matches!(operation, TerminalOperation::Every);
        let mut selected = Value::Null;
        let mut count = 0i64;
        let mut sum = 0.0f64;
        let mut saw_item = false;
        let valid = self.for_each_while(payload, locals, false, |item, index| {
            saw_item = true;
            let mapped = lambda
                .expression
                .evaluate(&item, &FastValue::Integer(index as i64))?;
            match operation {
                TerminalOperation::Some => {
                    if mapped.truthy()? {
                        boolean = true;
                        return Ok(false);
                    }
                }
                TerminalOperation::Every => {
                    if !mapped.truthy()? {
                        boolean = false;
                        return Ok(false);
                    }
                }
                TerminalOperation::FirstWith => {
                    if mapped.truthy()? {
                        selected = item.into_json()?;
                        return Ok(false);
                    }
                }
                TerminalOperation::IndexWhere => {
                    if mapped.truthy()? {
                        selected = Value::Number((index as i64).into());
                        return Ok(false);
                    }
                }
                TerminalOperation::CountBy => {
                    if mapped.truthy()? {
                        count += 1;
                    }
                }
                TerminalOperation::SumBy => sum += mapped.number()?,
            }
            Ok(true)
        })?;
        if !valid {
            return Ok(None);
        }
        let value = match operation {
            TerminalOperation::Some => Value::Bool(boolean),
            TerminalOperation::Every => Value::Bool(saw_item && boolean),
            TerminalOperation::FirstWith => selected,
            TerminalOperation::IndexWhere => {
                if selected.is_null() {
                    Value::Number((-1).into())
                } else {
                    selected
                }
            }
            TerminalOperation::CountBy => Value::Number(count.into()),
            TerminalOperation::SumBy => number_result(sum)?,
        };
        Ok(Some(value))
    }

    fn reduce(
        &self,
        payload: &Value,
        locals: &Map<String, Value>,
        reducer: &CompiledReducer,
        default: Option<FastValue>,
    ) -> Result<Option<Value>, DwError> {
        let mut accumulator = default;
        let valid = self.for_each(payload, locals, |item, _| {
            accumulator = Some(match accumulator.take() {
                Some(accumulator) => reducer.expression.evaluate(&item, &accumulator)?,
                None => item,
            });
            Ok(())
        })?;
        if !valid {
            return Ok(None);
        }
        accumulator
            .map(FastValue::into_json)
            .transpose()
            .map(|value| value.unwrap_or(Value::Null))
            .map(Some)
    }

    fn materialize(
        &self,
        payload: &Value,
        locals: &Map<String, Value>,
        max_materialized_bytes: usize,
    ) -> Result<Option<Value>, DwError> {
        let mut items = Vec::new();
        let mut estimated_bytes = std::mem::size_of::<Vec<Value>>();
        let valid = self.for_each(payload, locals, |item, index| {
            let value = item.into_json()?;
            estimated_bytes = estimated_bytes.saturating_add(estimate_json_bytes(&value));
            if estimated_bytes > max_materialized_bytes {
                return Err(DwError::ResourceLimit {
                    operation: "materializing lazy sequence".to_string(),
                    limit_bytes: max_materialized_bytes,
                    estimated_bytes,
                    item_index: index,
                });
            }
            items.push(value);
            Ok(())
        })?;
        if valid {
            Ok(Some(Value::Array(items)))
        } else {
            Ok(None)
        }
    }
}

impl FastExpr {
    fn evaluate(&self, item: &FastValue, secondary: &FastValue) -> Result<FastValue, DwError> {
        match self {
            Self::Literal(value) => Ok(value.clone()),
            Self::Item => Ok(item.clone()),
            Self::Secondary => Ok(secondary.clone()),
            Self::Array(items) => items
                .iter()
                .map(|expression| expression.evaluate(item, secondary)?.into_json())
                .collect::<Result<Vec<_>, _>>()
                .map(Value::Array)
                .map(FastValue::Json),
            Self::Path { root, fields } => {
                let root = match root {
                    FastRoot::Item => item,
                    FastRoot::Secondary => secondary,
                };
                let mut value = root.to_json()?;
                for field in fields {
                    value = match value {
                        Value::Object(map) => map.get(field).cloned().unwrap_or(Value::Null),
                        _ => Value::Null,
                    };
                }
                FastValue::from_json(value)
            }
            Self::Binary {
                left,
                operation,
                right,
            } => {
                if matches!(operation, FastBinary::And) {
                    let left = left.evaluate(item, secondary)?;
                    if !left.truthy()? {
                        return Ok(FastValue::Json(Value::Bool(false)));
                    }
                    return Ok(FastValue::Json(Value::Bool(
                        right.evaluate(item, secondary)?.truthy()?,
                    )));
                }
                if matches!(operation, FastBinary::Or) {
                    let left = left.evaluate(item, secondary)?;
                    if left.truthy()? {
                        return Ok(FastValue::Json(Value::Bool(true)));
                    }
                    return Ok(FastValue::Json(Value::Bool(
                        right.evaluate(item, secondary)?.truthy()?,
                    )));
                }
                let left = left.evaluate(item, secondary)?;
                let right = right.evaluate(item, secondary)?;
                match operation {
                    FastBinary::Add => fast_number_result(left.number()? + right.number()?),
                    FastBinary::Subtract => fast_number_result(left.number()? - right.number()?),
                    FastBinary::Multiply => fast_number_result(left.number()? * right.number()?),
                    FastBinary::Divide => fast_number_result(left.number()? / right.number()?),
                    FastBinary::Equal | FastBinary::NotEqual => {
                        let equal = left.to_json()? == right.to_json()?;
                        Ok(FastValue::Json(Value::Bool(
                            if matches!(operation, FastBinary::Equal) {
                                equal
                            } else {
                                !equal
                            },
                        )))
                    }
                    FastBinary::Greater
                    | FastBinary::GreaterEqual
                    | FastBinary::Less
                    | FastBinary::LessEqual => {
                        let left = left.number()?;
                        let right = right.number()?;
                        let result = match operation {
                            FastBinary::Greater => left > right,
                            FastBinary::GreaterEqual => left >= right,
                            FastBinary::Less => left < right,
                            FastBinary::LessEqual => left <= right,
                            _ => unreachable!(),
                        };
                        Ok(FastValue::Json(Value::Bool(result)))
                    }
                    FastBinary::And | FastBinary::Or => unreachable!(),
                }
            }
        }
    }
}

fn fast_number_result(value: f64) -> Result<FastValue, DwError> {
    if value.is_finite() {
        Ok(FastValue::Number(value))
    } else {
        Err(DwError::InvalidJson(value.to_string()))
    }
}

fn estimate_json_bytes(value: &Value) -> usize {
    std::mem::size_of::<Value>()
        + match value {
            Value::String(text) => text.len(),
            Value::Array(items) => items.iter().map(estimate_json_bytes).sum(),
            Value::Object(map) => map
                .iter()
                .map(|(key, value)| key.len() + estimate_json_bytes(value))
                .sum(),
            _ => 0,
        }
}
