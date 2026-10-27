//! Opt-in execution tracing for host-facing diagnostics. It observes the real
//! evaluation and never retries an expression or executes user code again.
use std::cell::RefCell;

use crate::DwError;

/// Present the underlying execution error without backend implementation details.
pub fn execution_error_message(error: &DwError) -> String {
    match error {
        DwError::UnsupportedFeature(message) => message.clone(),
        DwError::InvalidJson(message)
            if matches!(message.as_str(), "inf" | "-inf" | "NaN" | "nan") =>
        {
            format!("Cannot represent the non-finite number {message} as JSON")
        }
        _ => error.to_string(),
    }
}

#[derive(Clone)]
struct Frame {
    source: String,
    name: Option<String>,
    source_address: usize,
    parent: Option<(usize, usize)>,
}

#[derive(Default)]
struct Trace {
    active: bool,
    stack: Vec<Frame>,
    failure: Option<(String, Vec<Frame>)>,
}

thread_local! {
    static TRACE: RefCell<Trace> = RefCell::new(Trace::default());
}

pub(crate) fn frame<T>(
    source: &str,
    name: Option<&str>,
    run: impl FnOnce() -> Result<T, DwError>,
) -> Result<T, DwError> {
    let enabled = TRACE.with(|trace| {
        let mut trace = trace.borrow_mut();
        if !trace.active {
            return false;
        }
        if trace.stack.is_empty() {
            trace.failure = None;
        }
        let source_address = source.as_ptr() as usize;
        let source_end = source_address.saturating_add(source.len());
        let parent = trace
            .stack
            .iter()
            .enumerate()
            .rev()
            .find_map(|(index, frame)| {
                let parent_end = frame.source_address.saturating_add(frame.source.len());
                (source_address >= frame.source_address && source_end <= parent_end)
                    .then(|| (index, source_address - frame.source_address))
            });
        trace.stack.push(Frame {
            source: source.to_owned(),
            name: name.map(str::to_owned),
            source_address,
            parent,
        });
        true
    });
    let result = run();
    if enabled {
        TRACE.with(|trace| {
            let mut trace = trace.borrow_mut();
            if let Err(err) = &result {
                let key = err.to_string();
                if trace
                    .failure
                    .as_ref()
                    .is_none_or(|(previous, _)| previous != &key)
                {
                    trace.failure = Some((key, trace.stack.clone()));
                }
            } else if trace.failure.as_ref().is_some_and(|(_, frames)| {
                frames.len() >= trace.stack.len()
                    && frames
                        .iter()
                        .zip(&trace.stack)
                        .all(|(a, b)| a.source == b.source && a.name == b.name)
            }) {
                // A containing expression handled the error, so it cannot be
                // the trace for a later failure in the same execution.
                trace.failure = None;
            }
            trace.stack.pop();
        });
    }
    result
}

/// Execute once and return a source-backed trace for an error, when available.
pub fn with_execution_trace<T>(
    script: &str,
    run: impl FnOnce() -> Result<T, DwError>,
) -> (Result<T, DwError>, Option<String>) {
    let previous = TRACE.with(|trace| {
        std::mem::replace(
            &mut *trace.borrow_mut(),
            Trace {
                active: true,
                ..Trace::default()
            },
        )
    });
    let result = run();
    let trace = TRACE.with(|trace| std::mem::replace(&mut *trace.borrow_mut(), previous));
    let diagnostic = match (&result, trace.failure) {
        (Err(err), Some((key, frames))) if key == err.to_string() => format_trace(script, &frames),
        (Err(DwError::Parse(message)), None) if message == "empty expression" => {
            let line = script.bytes().filter(|byte| *byte == b'\n').count() + 1;
            let text = script.rsplit('\n').next().unwrap_or("");
            let column = text.chars().count() + 1;
            let gutter = format!("{line}| ");
            Some(format!(
                "\n\n{gutter}{text}\n{}^\nTrace:\n  at main::main (line: {line}, column: {column})",
                " ".repeat(gutter.chars().count() + column - 1)
            ))
        }
        _ => None,
    };
    (result, diagnostic)
}

/// Find an expression only when its position can be determined unambiguously.
pub fn unique_expression_offset(script: &str, expression: &str) -> Option<usize> {
    if expression.is_empty() {
        return None;
    }
    // The parser masks comments with spaces while retaining byte positions.
    // Match that same representation so commented code neither creates a
    // false duplicate nor makes an executable expression unlocatable.
    let masked = crate::syntax::mask_comments(script).ok();
    let script = masked.as_deref().unwrap_or(script);
    if let Some((_, end)) = crate::parse_script_boundary_span(script) {
        let body = &script[end..];
        if body.trim() == expression {
            return Some(end + body.len() - body.trim_start().len());
        }
    }
    let is_word = |c: char| c.is_alphanumeric() || c == '_' || c == '$';
    let first_word = expression.chars().next().is_some_and(is_word);
    let last_word = expression.chars().next_back().is_some_and(is_word);
    let mut matches = script.match_indices(expression).filter(|(index, _)| {
        let before = script[..*index].chars().next_back();
        let after = script[index + expression.len()..].chars().next();
        !(first_word && before.is_some_and(is_word) || last_word && after.is_some_and(is_word))
    });
    let (index, _) = matches.next()?;
    matches.next().is_none().then_some(index)
}

fn format_trace(script: &str, frames: &[Frame]) -> Option<String> {
    let mut located = Vec::new();
    let header_scope = frames.first().and_then(|frame| {
        if frame.name.as_deref() == Some("<body>") {
            return None;
        }
        frame.name.as_ref()?;
        let start = unique_expression_offset(script, &frame.source)?;
        Some((start, &script[start..start + frame.source.len()]))
    });
    let mut starts = Vec::with_capacity(frames.len());
    for frame in frames {
        let start = frame
            .parent
            .and_then(|(parent, offset)| {
                starts
                    .get(parent)
                    .copied()
                    .flatten()
                    .map(|start| start + offset)
            })
            .or_else(|| {
                header_scope
                    .and_then(|(start, scope)| {
                        unique_expression_offset(scope, &frame.source).map(|offset| start + offset)
                    })
                    .or_else(|| unique_expression_offset(script, &frame.source))
            });
        starts.push(start);
    }
    for (frame, start) in frames.iter().zip(starts).rev() {
        let Some(start) = start else {
            continue;
        };
        let (name, offset, width) =
            if matches!(frame.name.as_deref(), Some("<body>" | "<declaration>")) {
                ("main".to_string(), 0, 1)
            } else if let Some(name) = &frame.name {
                let Some(offset) = frame.source.find(&format!("var {name}")).map(|at| at + 4)
                else {
                    continue;
                };
                (name.clone(), offset, name.len())
            } else {
                let expression = crate::syntax::strip_wrapping_parens(frame.source.trim());
                let Some(expression_offset) = frame.source.find(expression) else {
                    continue;
                };
                if let Some((left, _)) = crate::syntax::split_top_level_keyword(expression, "to") {
                    let Some(operator_offset) = expression[left.len()..].find("to") else {
                        continue;
                    };
                    (
                        "main".to_string(),
                        expression_offset + left.len() + operator_offset,
                        2,
                    )
                } else if let Some((left, operator, _)) = crate::syntax::split_top_level_operator(
                    expression,
                    &["++", "--", "+", "-", "*", "/"],
                ) {
                    let Some(operator_offset) = expression[left.len()..].find(operator) else {
                        continue;
                    };
                    (
                        operator.to_string(),
                        expression_offset + left.len() + operator_offset,
                        operator.len(),
                    )
                } else if let Some((left, _, _)) =
                    crate::syntax::split_top_level_keyword_operator(expression, &["as"])
                {
                    let Some(operator_offset) = expression[left.len()..].find("as") else {
                        continue;
                    };
                    (
                        "as".to_string(),
                        expression_offset + left.len() + operator_offset,
                        2,
                    )
                } else if let Some((name, _)) = crate::literals::parse_call_args(expression) {
                    (name.to_owned(), expression_offset, name.len())
                } else {
                    ("main".to_string(), expression_offset, 1)
                }
            };
        let before = &script[..start + offset];
        let line = before.bytes().filter(|b| *b == b'\n').count() + 1;
        let column = before.rsplit('\n').next().unwrap_or("").chars().count() + 1;
        if located
            .iter()
            .any(|(n, l, c, _): &(String, usize, usize, usize)| {
                n == &name && *l == line && *c == column
            })
        {
            continue;
        }
        located.push((name, line, column, width));
    }
    let (_, line, column, width) = located.first()?;
    let line_text = script.lines().nth(line - 1)?;
    let gutter = format!("{line}| ");
    let mut output = format!(
        "\n\n{gutter}{line_text}\n{}{}\nTrace:",
        " ".repeat(gutter.chars().count() + column - 1),
        "^".repeat(*width)
    );
    for (name, line, column, _) in located.iter().take(12) {
        output.push_str(&format!(
            "\n  at main::{name} (line: {line}, column: {column})"
        ));
    }
    Some(output)
}
