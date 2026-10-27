//! Tolerant lexical bindings for editing. No expression is evaluated and scope
//! boundaries are found by the shared lexer, including incomplete source.
use crate::declarations::header_declaration_spans;
use crate::syntax::{
    editor_tokens, find_matching_delimiter, is_identifier, mask_comments, split_top_level,
    split_top_level_char, visit_top_level, CodeToken,
};
use std::collections::{BTreeMap, BTreeSet};
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
    inputs: &mut BTreeSet<String>,
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
            annotations.extend(signature_annotations(source, f.range.start..f.body.start));
            if f.has_body {
                expressions.push(f.body.clone());
            }
            let raw_signature = &source[f.range.start..f.body.start];
            if let Some(open) = raw_signature.find('(') {
                if let Some(close) = find_matching_delimiter(raw_signature, open, '(', ')') {
                    for parameter in split_top_level(&raw_signature[open + 1..close], ',') {
                        if let Some((name, binding)) =
                            parameter_binding(parameter, "Function parameter")
                        {
                            let start = parameter.as_ptr() as usize - source.as_ptr() as usize
                                + parameter.len();
                            nested.push(Scope {
                                range: start..f.body.start,
                                header: start..start,
                                bindings: BTreeMap::from([(name, binding)]),
                            });
                        }
                    }
                }
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
        if let Some(name) = prefix.strip_prefix("input ").and_then(parameter_name) {
            annotations.push(span.clone());
            if name != "payload" {
                inputs.insert(format!("vars.{name}"));
            }
        }
        if prefix.starts_with("import ")
            || prefix.starts_with("output ")
            || prefix.starts_with("ns ")
            || prefix.starts_with("%dw")
        {
            annotations.push(span.clone());
        }
        if let Some(name) = prefix
            .strip_prefix("type ")
            .or_else(|| prefix.strip_prefix("ns "))
            .and_then(|s| {
                editor_tokens(s)
                    .first()
                    .filter(|t| is_identifier(t.text))
                    .map(|t| t.text.to_owned())
            })
        {
            scope
                .bindings
                .insert(name, Binding::Variable("Type or namespace"));
        }
        if let Some(import) = prefix.strip_prefix("import ") {
            if let Some((items, _)) = crate::syntax::split_top_level_keyword(import, "from") {
                for item in split_top_level(items, ',') {
                    let (_, alias) = crate::syntax::split_top_level_keyword(item.trim(), "as")
                        .unwrap_or((item.trim(), item.trim()));
                    if is_identifier(alias) {
                        scope
                            .bindings
                            .insert(alias.into(), Binding::Variable("Imported symbol"));
                    }
                }
            }
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
fn is_binding_parameter(source: &str) -> bool {
    let Some(name) = parameter_name(source) else {
        return false;
    };
    let offset = name.as_ptr() as usize - source.as_ptr() as usize + name.len();
    let tail = source[offset..].trim();
    tail.is_empty() || tail.starts_with(':') || tail.starts_with('=')
}

fn signature_annotations(source: &str, range: Range<usize>) -> Vec<Range<usize>> {
    let raw = &source[range.clone()];
    let Some(open) = raw.find('(') else {
        return vec![range];
    };
    let Some(close) = find_matching_delimiter(raw, open, '(', ')') else {
        return vec![range];
    };
    let mut annotations = vec![];
    let mut start = range.start;
    for parameter in split_top_level(&raw[open + 1..close], ',') {
        if let Some((_, default)) = split_top_level_char(parameter, '=') {
            let value_start = default.as_ptr() as usize - source.as_ptr() as usize;
            annotations.push(start..value_start);
            start = value_start + default.len();
        }
    }
    annotations.push(start..range.end);
    annotations
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
pub(crate) struct ScopeIndex {
    scopes: Vec<Scope>,
    pub(crate) annotations: Vec<Range<usize>>,
    pub(crate) inputs: BTreeSet<String>,
}

impl ScopeIndex {
    pub(crate) fn new(source: &str, module_sources: &BTreeMap<String, String>) -> Self {
        let tokens = editor_tokens(source);
        let (matching, parent) = groups(&tokens);
        let boundary = crate::parse_script_boundary_span(source);
        let header_end = boundary.map_or_else(
            || {
                if source.trim_start().starts_with("output ") {
                    source.find('\n').unwrap_or(source.len())
                } else {
                    source.len()
                }
            },
            |s| s.0,
        );
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
        let mut inputs = BTreeSet::new();
        for scope in &mut scopes {
            expressions.push(scope.header.end..scope.range.end);
            read_header(
                source,
                scope,
                &mut functions,
                module_sources,
                &mut signature_ranges,
                &mut expressions,
                &mut inputs,
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
            if open > 0 && matches!(tokens[open - 1].text, "if" | "when") {
                continue;
            }
            let parameter_source = &source[tokens[open].end..tokens[i - 1].start];
            let parameters = split_top_level(parameter_source, ',');
            // Function types `(T) -> R` in declarations are not lambda definitions.
            if parameters.iter().any(|p| !is_binding_parameter(p)) {
                continue;
            }
            signature_ranges.extend(signature_annotations(source, tokens[open].start..t.end));
            let end = expression_end(
                &tokens,
                &matching,
                &parent,
                i,
                i + 1,
                &expressions,
                source.len(),
            );
            for parameter in &parameters {
                if let Some((name, binding)) = parameter_binding(parameter, "Lambda parameter") {
                    let end_of_parameter =
                        parameter.as_ptr() as usize - source.as_ptr() as usize + parameter.len();
                    scopes.push(Scope {
                        range: end_of_parameter..t.start,
                        header: t.start..t.start,
                        bindings: BTreeMap::from([(name, binding)]),
                    });
                }
            }
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
            for assignment in split_top_level(&source[tokens[i + 1].end..tokens[close].start], ',')
            {
                if let Some((name, value)) = split_top_level_char(assignment, '=') {
                    let start = assignment.as_ptr() as usize - source.as_ptr() as usize;
                    signature_ranges.push(start..start + name.len());
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
            for assignment in split_top_level(&source[tokens[i + 1].end..tokens[close].start], ',')
            {
                if let Some((name, _)) = split_top_level_char(assignment, '=') {
                    if let Some(name) = parameter_name(name) {
                        let start = assignment.as_ptr() as usize - source.as_ptr() as usize
                            + assignment.len();
                        scopes.push(Scope {
                            range: start..tokens[close].end,
                            header: start..start,
                            bindings: BTreeMap::from([(
                                name.into(),
                                Binding::Variable("Using variable"),
                            )]),
                        });
                    }
                }
            }
            scopes.push(Scope {
                range: tokens[close].end..end,
                header: tokens[close].end..tokens[close].end,
                bindings,
            });
        }
        for (i, token) in tokens.iter().enumerate() {
            if token.literal || token.text != "case" {
                continue;
            }
            let Some(open) = parent[i].filter(|open| tokens[*open].text == "{") else {
                continue;
            };
            // A case is syntax only in a match/update block.
            if open == 0 || !matches!(tokens[open - 1].text, "match" | "update") {
                continue;
            }
            let end_index = (i + 1..tokens.len())
                .find(|j| {
                    parent[*j] == Some(open)
                        && !tokens[*j].literal
                        && (tokens[*j].text == "case"
                            || (tokens[*j].text == "else"
                                && tokens.get(*j + 1).is_some_and(|t| t.text == "->")))
                })
                .or(matching[open])
                .unwrap_or(tokens.len());
            let Some(arrow) =
                (i + 1..end_index).find(|j| parent[*j] == Some(open) && tokens[*j].text == "->")
            else {
                continue;
            };
            signature_ranges.push(token.start..token.end);
            let first = if tokens.get(i + 1).is_some_and(|t| t.text == "var") {
                signature_ranges.push(tokens[i + 1].start..tokens[i + 1].end);
                i + 2
            } else {
                i + 1
            };
            if tokens.get(first + 1).is_some_and(|t| t.text == "at") {
                signature_ranges.push(tokens[first + 1].start..tokens[first + 1].end);
            }
            if let Some(guard) = (first + 1..arrow).find(|j| {
                parent[*j] == Some(open) && !tokens[*j].literal && tokens[*j].text == "when"
            }) {
                signature_ranges.push(tokens[guard].start..tokens[guard].end);
            }
            let Some(binding) = tokens.get(first).filter(|t| {
                !t.literal
                    && is_identifier(t.text)
                    && !matches!(t.text, "is" | "true" | "false" | "null")
            }) else {
                continue;
            };
            signature_ranges.push(binding.start..binding.end);
            scopes.push(Scope {
                range: binding.end..tokens.get(end_index).map_or(source.len(), |t| t.start),
                header: binding.end..binding.end,
                bindings: BTreeMap::from([(
                    binding.text.into(),
                    Binding::Variable("Case binding"),
                )]),
            });
        }
        scopes.sort_by(|a, b| {
            (b.range.end - b.range.start)
                .cmp(&(a.range.end - a.range.start))
                .then(a.range.start.cmp(&b.range.start))
        });
        Self {
            scopes,
            annotations: signature_ranges,
            inputs,
        }
    }

    pub(crate) fn is_bound(&self, name: &str, cursor: usize) -> bool {
        self.scopes
            .iter()
            .any(|scope| scope.range.contains(&cursor) && scope.bindings.contains_key(name))
    }

    fn visible(&self, cursor: usize) -> BTreeMap<String, Binding> {
        let mut visible = BTreeMap::new();
        for scope in &self.scopes {
            if cursor >= scope.range.start && cursor <= scope.range.end {
                visible.extend(scope.bindings.clone());
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
                            Binding::Function(
                                signatures.iter().map(|s| s.renamed(&name)).collect(),
                            ),
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
}

pub(crate) fn visible_bindings(
    source: &str,
    cursor: usize,
    module_sources: &BTreeMap<String, String>,
) -> BTreeMap<String, Binding> {
    ScopeIndex::new(source, module_sources).visible(cursor)
}
