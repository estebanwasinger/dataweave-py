//! Tolerant lexical bindings for editing. No expression is evaluated and scope
//! boundaries are found by the shared lexer, including incomplete source.
use crate::declarations::header_declaration_spans;
use crate::syntax::{
    editor_tokens, find_matching_delimiter, is_identifier, mask_comments, split_top_level,
    split_top_level_char, visit_top_level, CodeToken,
};
use std::collections::BTreeMap;
use std::ops::Range;

mod signature;
use signature::{function, parameter_name};
pub use signature::{module_signatures, Signature};

#[derive(Clone, Debug)]
pub(crate) enum Binding {
    Function(Vec<Signature>),
    Variable(&'static str),
    Reference(String),
}
#[derive(Clone, Debug)]
struct Scope {
    range: Range<usize>,
    header: Range<usize>,
    bindings: BTreeMap<String, Binding>,
}

pub(crate) fn groups(tokens: &[CodeToken<'_>]) -> (Vec<Option<usize>>, Vec<Option<usize>>) {
    let mut matching = vec![None; tokens.len()];
    let mut parent = vec![None; tokens.len()];
    let mut stack: Vec<usize> = vec![];
    for (i, t) in tokens.iter().enumerate() {
        parent[i] = stack.last().copied();
        if t.literal {
            continue;
        }
        match t.text {
            "(" | "[" | "{" => stack.push(i),
            ")" | "]" | "}" => {
                if let Some(open) = stack.last().copied() {
                    if matches!(
                        (tokens[open].text, t.text),
                        ("(", ")") | ("[", "]") | ("{", "}")
                    ) {
                        stack.pop();
                        matching[open] = Some(i);
                        matching[i] = Some(open);
                    }
                }
            }
            _ => (),
        }
    }
    (matching, parent)
}

fn insert_function(bindings: &mut BTreeMap<String, Binding>, signature: Signature) {
    let name = signature.name.clone();
    match bindings.get_mut(&name) {
        Some(Binding::Function(signatures)) => {
            if !signatures.contains(&signature) {
                signatures.push(signature);
            }
        }
        _ => {
            bindings.insert(name, Binding::Function(vec![signature]));
        }
    }
}

fn read_header(
    source: &str,
    scope: &mut Scope,
    nested: &mut Vec<Scope>,
    module_sources: &BTreeMap<String, String>,
    annotations: &mut Vec<Range<usize>>,
    expressions: &mut Vec<Range<usize>>,
) {
    let header = &source[scope.header.clone()];
    for span in header_declaration_spans(header) {
        let span = scope.header.start + span.start..scope.header.start + span.end;
        if let Some(f) = function(source, span.clone()) {
            let bindings = f
                .signature
                .parameters
                .iter()
                .filter_map(|p| parameter_binding(p, "Function parameter"))
                .collect();
            annotations.push(f.range.start..f.body.start);
            if f.has_body {
                expressions.push(f.body.clone());
            }
            if f.has_body {
                nested.push(Scope {
                    range: f.body.clone(),
                    header: f.range.start..f.body.start,
                    bindings,
                });
            }
            insert_function(&mut scope.bindings, f.signature);
            continue;
        }
        let raw = &source[span.clone()];
        // Remove comments from the declaration prefix, even with an unfinished initializer.
        let mut eq = None;
        visit_top_level(raw, |i, c| {
            if eq.is_none() && c == '=' {
                eq = Some(i);
            }
        });
        let prefix = mask_comments(&raw[..eq.unwrap_or(raw.len())]).unwrap_or_default();
        let prefix = prefix.trim();
        if prefix.starts_with("var ") || prefix.starts_with("input ") || prefix.starts_with("type ")
        {
            let end = if prefix.starts_with("type ") {
                raw.len()
            } else {
                eq.unwrap_or(raw.len())
            };
            let start = raw.as_ptr() as usize - source.as_ptr() as usize;
            annotations.push(start..start + end);
            if let Some(eq) = eq {
                if !prefix.starts_with("type ") {
                    expressions.push(start + eq + 1..span.end);
                }
            }
        }
        if let Some(import) = prefix.strip_prefix("import ") {
            for (name, signatures) in
                crate::function_catalog::imported_signatures(import, module_sources)
            {
                scope.bindings.insert(name, Binding::Function(signatures));
            }
        } else if let Some(var) = prefix
            .strip_prefix("var ")
            .or_else(|| prefix.strip_prefix("input "))
        {
            if let Some(name) = parameter_name(var) {
                let value = eq.map(|i| raw[i + 1..].trim()).unwrap_or("");
                let lambda = lambda_signature(value, name).or_else(|| function_type(var, name));
                if let Some(signature) = lambda {
                    insert_function(&mut scope.bindings, signature);
                } else if is_qualified_identifier(value) {
                    scope
                        .bindings
                        .insert(name.into(), Binding::Reference(value.into()));
                } else {
                    scope
                        .bindings
                        .insert(name.into(), Binding::Variable("Script variable"));
                }
            }
        }
    }
}
fn is_qualified_identifier(s: &str) -> bool {
    !s.is_empty() && s.split("::").all(is_identifier)
}
fn lambda_signature(source: &str, name: &str) -> Option<Signature> {
    let trimmed = crate::syntax::strip_wrapping_parens(source.trim());
    let close = find_matching_delimiter(trimmed, 0, '(', ')')?;
    if !trimmed[close + 1..].trim_start().starts_with("->") {
        return None;
    }
    let parameters = split_top_level(&trimmed[1..close], ',')
        .into_iter()
        .filter(|s| !s.trim().is_empty())
        .map(|s| s.trim().into())
        .collect();
    Some(Signature {
        name: name.into(),
        parameters,
        return_type: None,
        documentation: None,
        infix: false,
    })
}

fn function_type(parameter: &str, name: &str) -> Option<Signature> {
    let (_, annotation) = split_top_level_char(parameter, ':')?;
    let annotation = split_top_level_char(annotation, '=')
        .map_or(annotation, |p| p.0)
        .trim();
    let mut signature = lambda_signature(annotation, name)?;
    let close = find_matching_delimiter(annotation, 0, '(', ')')?;
    signature.return_type = annotation[close + 1..]
        .trim()
        .strip_prefix("->")
        .map(|s| s.trim().into());
    for (index, parameter) in signature.parameters.iter_mut().enumerate() {
        if split_top_level_char(parameter, ':').is_none() {
            *parameter = format!(
                "{}: {}",
                if index == 0 {
                    "value".into()
                } else {
                    format!("argument{}", index + 1)
                },
                parameter
            );
        }
    }
    Some(signature)
}
fn parameter_binding(parameter: &str, detail: &'static str) -> Option<(String, Binding)> {
    let name = parameter_name(parameter)?;
    let binding = function_type(parameter, name)
        .map_or(Binding::Variable(detail), |s| Binding::Function(vec![s]));
    Some((name.into(), binding))
}
pub(crate) fn expression_end(
    tokens: &[CodeToken<'_>],
    matching: &[Option<usize>],
    parent: &[Option<usize>],
    anchor: usize,
    first: usize,
    expressions: &[Range<usize>],
    source_len: usize,
) -> usize {
    let end = tokens
        .iter()
        .enumerate()
        .skip(first)
        .find(|(j, n)| {
            (parent[*j] == parent[anchor] && matches!(n.text, "," | ";"))
                || (Some(*j) == parent[anchor].and_then(|p| matching[p]))
        })
        .map_or(source_len, |(_, n)| n.start);
    expressions
        .iter()
        .filter(|r| r.contains(&tokens[anchor].start))
        .map(|r| r.end)
        .fold(end, usize::min)
}

/// Bindings visible at a source byte position, outer scopes first. Function
/// overloads are grouped per scope so shadowing never mixes unrelated overloads.
pub(crate) fn visible_bindings(
    source: &str,
    cursor: usize,
    module_sources: &BTreeMap<String, String>,
) -> BTreeMap<String, Binding> {
    let tokens = editor_tokens(source);
    let (matching, parent) = groups(&tokens);
    let boundary = crate::parse_script_boundary_span(source);
    let header_end = boundary.map_or(source.len(), |s| s.0);
    let mut scopes = vec![Scope {
        range: 0..source.len(),
        header: 0..header_end,
        bindings: BTreeMap::new(),
    }];
    // Each do block owns a header and a body, including a missing closing brace.
    for (i, t) in tokens.iter().enumerate() {
        if t.literal || t.text != "do" || tokens.get(i + 1).is_none_or(|t| t.text != "{") {
            continue;
        }
        let open = i + 1;
        let end = matching[open].map_or(source.len(), |n| tokens[n].start);
        let start = tokens[open].end;
        let delimiter = tokens
            .iter()
            .enumerate()
            .skip(open + 1)
            .take_while(|(_, t)| t.start < end)
            .find(|(j, t)| t.text == "---" && parent[*j] == Some(open))
            .map(|(_, t)| t.start);
        scopes.push(Scope {
            range: start..end,
            header: start..delimiter.unwrap_or(end),
            bindings: BTreeMap::new(),
        });
    }
    let mut functions = vec![];
    let mut signature_ranges = vec![];
    let mut expressions = vec![];
    for scope in &mut scopes {
        expressions.push(scope.header.end..scope.range.end);
        read_header(
            source,
            scope,
            &mut functions,
            module_sources,
            &mut signature_ranges,
            &mut expressions,
        );
    }
    scopes.extend(functions);
    // A lambda's body ends at the containing expression's comma/closing group.
    for (i, t) in tokens.iter().enumerate() {
        if t.literal
            || t.text != "->"
            || i == 0
            || tokens[i - 1].text != ")"
            || signature_ranges.iter().any(|r| r.contains(&t.start))
        {
            continue;
        }
        let Some(open) = matching[i - 1] else {
            continue;
        };
        let parameter_source = &source[tokens[open].end..tokens[i - 1].start];
        let parameters = split_top_level(parameter_source, ',');
        // Function types `(T) -> R` in declarations are not lambda definitions.
        if parameters.iter().any(|p| parameter_name(p).is_none()) {
            continue;
        }
        let end = expression_end(
            &tokens,
            &matching,
            &parent,
            i,
            i + 1,
            &expressions,
            source.len(),
        );
        let bindings = parameters
            .into_iter()
            .filter_map(|p| parameter_binding(p, "Lambda parameter"))
            .collect();
        scopes.push(Scope {
            range: t.end..end,
            header: t.end..t.end,
            bindings,
        });
    }
    for (i, t) in tokens.iter().enumerate() {
        if t.literal || t.text != "using" || tokens.get(i + 1).is_none_or(|t| t.text != "(") {
            continue;
        }
        let Some(close) = matching[i + 1] else {
            continue;
        };
        let mut bindings = BTreeMap::new();
        for assignment in split_top_level(&source[tokens[i + 1].end..tokens[close].start], ',') {
            if let Some((name, value)) = split_top_level_char(assignment, '=') {
                if let Some(name) = parameter_name(name) {
                    let binding = lambda_signature(value, name).map_or_else(
                        || {
                            if is_qualified_identifier(value.trim()) {
                                Binding::Reference(value.trim().into())
                            } else {
                                Binding::Variable("Using variable")
                            }
                        },
                        |s| Binding::Function(vec![s]),
                    );
                    bindings.insert(name.into(), binding);
                }
            }
        }
        let end = expression_end(
            &tokens,
            &matching,
            &parent,
            i,
            close + 1,
            &expressions,
            source.len(),
        );
        scopes.push(Scope {
            range: tokens[close].end..end,
            header: tokens[close].end..tokens[close].end,
            bindings,
        });
    }
    scopes.sort_by(|a, b| {
        (b.range.end - b.range.start)
            .cmp(&(a.range.end - a.range.start))
            .then(a.range.start.cmp(&b.range.start))
    });
    let mut visible = BTreeMap::new();
    for scope in scopes {
        if cursor >= scope.range.start && cursor <= scope.range.end {
            visible.extend(scope.bindings);
        }
    }
    // Function-valued aliases are resolved without evaluating initializers.
    for _ in 0..visible.len().min(32) {
        let mut changed = false;
        for (name, binding) in visible.clone() {
            if let Binding::Reference(target) = binding {
                let signatures = match visible.get(&target) {
                    Some(Binding::Function(s)) => s.clone(),
                    Some(Binding::Reference(_)) => continue,
                    Some(_) => vec![],
                    None => crate::function_catalog::signatures(&target),
                };
                if !signatures.is_empty() {
                    visible.insert(
                        name.clone(),
                        Binding::Function(signatures.iter().map(|s| s.renamed(&name)).collect()),
                    );
                    changed = true;
                }
            }
        }
        if !changed {
            break;
        }
    }
    visible
}
