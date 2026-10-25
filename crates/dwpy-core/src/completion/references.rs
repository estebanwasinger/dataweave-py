//! Resolve selector references to input projections, without running callbacks.
use crate::selectors::{parse_path_segments, PathSegment};
use crate::syntax::{editor_tokens, is_identifier, split_top_level, CodeToken};
use std::ops::Range;

#[derive(Clone)]
pub(super) enum Projection {
    Selector(PathSegment),
    ArrayItems,
    ObjectValues,
    Array,
}
pub(super) struct Reference {
    pub root: String,
    pub projections: Vec<Projection>,
}
struct Callback {
    input: Range<usize>,
    body: Range<usize>,
    parameters: Vec<String>,
    operator: String,
    arrow: Option<usize>,
}
struct Analysis<'a> {
    source: &'a str,
    tokens: Vec<CodeToken<'a>>,
    matching: Vec<Option<usize>>,
    parent: Vec<Option<usize>>,
    limits: Vec<Range<usize>>,
    callbacks: Vec<Callback>,
}

fn item_projection(operator: &str) -> Option<Projection> {
    match operator {
        "map" | "filter" | "flatMap" | "reduce" | "distinctBy" | "groupBy" | "orderBy" | "some"
        | "every" | "takeWhile" | "dropWhile" => Some(Projection::ArrayItems),
        "mapObject" | "filterObject" | "pluck" => Some(Projection::ObjectValues),
        _ => None,
    }
}

impl<'a> Analysis<'a> {
    fn new(source: &'a str) -> Self {
        let tokens = editor_tokens(source);
        let (matching, parent) = crate::language::groups(&tokens);
        let mut limits = vec![];
        let header_end = crate::parse_script_boundary_span(source).map_or(0, |s| s.0);
        let mut headers = vec![0..header_end];
        for (i, t) in tokens.iter().enumerate() {
            if !t.literal && t.text == "do" && tokens.get(i + 1).is_some_and(|t| t.text == "{") {
                let open = i + 1;
                let end = matching[open].map_or(source.len(), |j| tokens[j].start);
                if let Some(delimiter) = tokens
                    .iter()
                    .enumerate()
                    .skip(open + 1)
                    .take_while(|(_, t)| t.start < end)
                    .find(|(j, t)| parent[*j] == Some(open) && t.text == "---")
                {
                    headers.push(tokens[open].end..delimiter.1.start);
                }
            }
        }
        for header in headers {
            for span in crate::declarations::header_declaration_spans(&source[header.clone()]) {
                limits.push(header.start + span.start..header.start + span.end);
            }
        }
        let mut this = Self {
            source,
            tokens,
            matching,
            parent,
            limits,
            callbacks: vec![],
        };
        for i in 0..this.tokens.len() {
            if this.callbacks.len() >= 256 {
                break;
            }
            if let Some(callback) = this.callback(i) {
                this.callbacks.push(callback);
            }
        }
        // An unrelated lambda shadows callback references too. Do not infer its
        // parameters from a preceding collection or leak bindings to siblings.
        for (i, t) in this.tokens.iter().enumerate() {
            if t.literal || t.text != "->" || this.callbacks.iter().any(|c| c.arrow == Some(i)) {
                continue;
            }
            if let Some(parameters) = this.parameters_before(i) {
                this.callbacks.push(Callback {
                    input: 0..0,
                    body: t.end..this.end(i, i + 1),
                    parameters,
                    operator: String::new(),
                    arrow: Some(i),
                });
            }
        }
        this
    }

    fn end(&self, anchor: usize, first: usize) -> usize {
        crate::language::expression_end(
            &self.tokens,
            &self.matching,
            &self.parent,
            anchor,
            first,
            &self.limits,
            self.source.len(),
        )
    }
    fn parameters_before(&self, arrow: usize) -> Option<Vec<String>> {
        let last = arrow.checked_sub(1)?;
        let text = if self.tokens[last].text == ")" {
            let open = self.matching[last]?;
            &self.source[self.tokens[open].end..self.tokens[last].start]
        } else {
            self.tokens[last].text
        };
        split_top_level(text, ',')
            .into_iter()
            .map(|s| {
                let tokens = editor_tokens(s);
                let name = tokens.first()?.text;
                (is_identifier(name) || matches!(name, "$" | "$$" | "$$$")).then(|| name.into())
            })
            .collect()
    }
    fn operand_start(&self, operator: usize) -> usize {
        let level = self.parent[operator];
        let mut i = operator;
        while let Some(previous) = i.checked_sub(1) {
            let t = self.tokens[previous];
            if Some(previous) == level {
                break;
            }
            if self.parent[previous] == level
                && !t.literal
                && matches!(t.text, "," | ";" | ":" | "=" | "->" | "---")
            {
                break;
            }
            if matches!(t.text, ")" | "]" | "}") && !t.literal {
                if let Some(open) = self.matching[previous] {
                    i = open;
                    continue;
                }
            }
            i = previous;
        }
        // Header declaration boundaries also stop an operand spanning lines.
        let limit = self
            .limits
            .iter()
            .filter(|r| r.contains(&self.tokens[operator].start))
            .map(|r| r.start)
            .max()
            .unwrap_or(0);
        self.tokens.get(i).map_or(limit, |t| t.start.max(limit))
    }
    fn callback(&self, i: usize) -> Option<Callback> {
        let token = self.tokens[i];
        if token.literal || item_projection(token.text).is_none() {
            return None;
        }
        if i > 0 && matches!(self.tokens[i - 1].text, "." | "::" | "fun") {
            return None;
        }
        // A local callable with this name has unknown callback semantics.
        if crate::language::visible_bindings(self.source, token.start, &Default::default())
            .get(token.text)
            .is_some_and(|b| match b {
                crate::language::Binding::Function(s) => {
                    s.iter().any(|s| s.documentation.is_none())
                }
                _ => true,
            })
        {
            return None;
        }
        let first = i + 1;
        let mut end = self.end(i, first);
        let start = self.operand_start(i);
        let call_form = start == token.start;
        let (input, rhs_start) = if call_form {
            if self.tokens.get(first)?.text != "(" {
                return None;
            }
            let comma = (first + 1..self.tokens.len())
                .find(|j| self.parent[*j] == Some(first) && self.tokens[*j].text == ",")?;
            end = self.matching[first].map_or(end, |j| self.tokens[j].start);
            (self.tokens[first].end..self.tokens[comma].start, comma + 1)
        } else {
            (start..token.start, first)
        };
        let mut body_first = rhs_start;
        // Peel callback wrappers, preserving a `(parameters) -> body` pair.
        while self.tokens.get(body_first).is_some_and(|t| t.text == "(") {
            if let Some(close) = self.matching[body_first] {
                if self.tokens.get(close + 1).is_some_and(|t| t.text == "->") {
                    break;
                }
                end = end.min(self.tokens[close].start);
            }
            body_first += 1;
        }
        let arrow = if self.tokens.get(body_first).is_some_and(|t| t.text == "(") {
            self.matching[body_first].map(|j| j + 1)
        } else {
            Some(body_first + 1)
        }
        .filter(|j| self.tokens.get(*j).is_some_and(|t| t.text == "->"));
        let parameters = arrow
            .and_then(|j| self.parameters_before(j))
            .unwrap_or_default();
        let body_start = arrow.map_or_else(
            || self.tokens.get(body_first).map_or(end, |t| t.start),
            |j| self.tokens[j].end,
        );
        if arrow.is_none() && !call_form {
            // Implicit pipelines associate to the left; an explicit lambda's
            // body may itself contain a collection pipeline.
            if let Some(next) = self
                .tokens
                .iter()
                .enumerate()
                .skip(i + 1)
                .take_while(|(_, t)| t.start < end)
                .find(|(j, t)| {
                    self.parent[*j] == self.parent[i]
                        && !t.literal
                        && item_projection(t.text).is_some()
                        && (*j == 0 || !matches!(self.tokens[*j - 1].text, "." | "::"))
                })
            {
                end = next.1.start;
            }
        }
        (body_start <= end).then(|| Callback {
            input,
            body: body_start..end,
            parameters,
            operator: token.text.into(),
            arrow,
        })
    }

    fn reference(&self, name: &str, position: usize, depth: usize) -> Option<Reference> {
        if depth > 32 {
            return None;
        }
        if !matches!(name, "$" | "$$" | "$$$") {
            if let Some(binding) =
                crate::language::visible_bindings(self.source, position, &Default::default())
                    .get(name)
            {
                if !matches!(
                    binding,
                    crate::language::Binding::Variable("Lambda parameter")
                ) {
                    return None;
                }
            }
        }
        let mut scopes: Vec<_> = self
            .callbacks
            .iter()
            .filter(|c| c.body.start <= position && position < c.body.end)
            .collect();
        scopes.sort_by_key(|c| (c.body.len(), std::cmp::Reverse(c.body.start)));
        for scope in scopes {
            let parameter = scope.parameters.iter().position(|p| p == name);
            if name == "$" || parameter == Some(0) {
                let mut input = self.expression(scope.input.clone(), depth + 1)?;
                input.projections.push(item_projection(&scope.operator)?);
                return Some(input);
            }
            if matches!(name, "$$" | "$$$") || parameter.is_some() {
                return None;
            }
        }
        matches!(name, "payload" | "vars" | "attributes").then(|| Reference {
            root: name.into(),
            projections: vec![],
        })
    }
    fn expression(&self, range: Range<usize>, depth: usize) -> Option<Reference> {
        if depth > 32 || range.is_empty() {
            return None;
        }
        let mut indices: Vec<_> = self
            .tokens
            .iter()
            .enumerate()
            .filter(|(_, t)| range.start <= t.start && t.end <= range.end)
            .map(|(i, _)| i)
            .collect();
        while indices.first().is_some_and(|i| self.tokens[*i].text == "(")
            && self.matching[*indices.first()?] == indices.last().copied()
        {
            indices.remove(0);
            indices.pop();
        }
        let first = *indices.first()?;
        let last = *indices.last()?;
        let start = self.tokens[first].start;
        let end = self.tokens[last].end;
        // Preserve shapes through filtering/ordering and through selector-only
        // map projections. Never infer transformed output by running a mapper.
        if let Some(callback) = self
            .callbacks
            .iter()
            .filter(|c| c.input.start == start && c.input.end < end && c.body.end <= end)
            .max_by_key(|c| c.input.end)
        {
            return match callback.operator.as_str() {
                "filter" | "distinctBy" | "orderBy" | "takeWhile" | "dropWhile" => {
                    self.expression(callback.input.clone(), depth + 1)
                }
                "map" | "flatMap" => {
                    let mut result = self.expression(callback.body.clone(), depth + 1)?;
                    if callback.operator == "flatMap" {
                        result.projections.push(Projection::ArrayItems);
                    }
                    result.projections.push(Projection::Array);
                    Some(result)
                }
                _ => None,
            };
        }
        let name = self.tokens[first].text;
        if self.tokens[first].literal
            || !(is_identifier(name) || matches!(name, "$" | "$$" | "$$$"))
        {
            return None;
        }
        let tail = &self.source[self.tokens[first].end..end];
        let segments = parse_path_segments(tail).ok()?;
        let mut reference = self.reference(name, start, depth + 1)?;
        reference
            .projections
            .extend(segments.into_iter().map(Projection::Selector));
        Some(reference)
    }
}

pub(super) fn resolve(source: &str, dot: usize) -> Option<(Reference, String)> {
    let analysis = Analysis::new(source);
    for (i, token) in analysis
        .tokens
        .iter()
        .enumerate()
        .rev()
        .filter(|(_, t)| t.end <= dot)
    {
        if token.literal || !(is_identifier(token.text) || matches!(token.text, "$" | "$$" | "$$$"))
        {
            continue;
        }
        if i > 0 && matches!(analysis.tokens[i - 1].text, "." | "::" | "@") {
            continue;
        }
        let end = source[..dot].strip_suffix('?').map_or(dot, str::len);
        if let Some(reference) = analysis.expression(token.start..end, 0) {
            return Some((reference, token.text.into()));
        }
    }
    None
}
