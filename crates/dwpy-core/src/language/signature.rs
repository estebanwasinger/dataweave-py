use crate::declarations::{header_declaration_spans, strip_generic_function_type_parameters};
use crate::syntax::{
    editor_tokens, find_matching_delimiter, is_identifier, mask_comments, split_top_level,
    visit_top_level,
};
use serde::{Deserialize, Serialize};
use std::ops::Range;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Signature {
    pub name: String,
    pub parameters: Vec<String>,
    #[serde(default)]
    pub return_type: Option<String>,
    #[serde(default)]
    pub documentation: Option<String>,
    /// Filled from the active runtime's operator contract, retained by aliases.
    #[serde(skip)]
    pub(crate) infix: bool,
}
impl Signature {
    pub fn label(&self) -> String {
        let mut label = format!("{}({})", self.name, self.parameters.join(", "));
        if let Some(result) = &self.return_type {
            label.push_str(&format!(": {result}"));
        }
        label
    }
    pub fn parameter_names(&self) -> Vec<String> {
        self.parameters
            .iter()
            .filter_map(|p| parameter_name(p).map(str::to_owned))
            .collect()
    }
    pub fn renamed(&self, name: &str) -> Self {
        let mut s = self.clone();
        s.name = name.into();
        s.infix = false;
        s
    }
}

pub(crate) fn parameter_name(parameter: &str) -> Option<&str> {
    let tokens = editor_tokens(parameter);
    let mut i = 0;
    while tokens.get(i).is_some_and(|t| t.text == "@") {
        i += 2;
        if tokens.get(i).is_some_and(|t| t.text == "(") {
            let end = find_matching_delimiter(parameter, tokens[i].start, '(', ')')?;
            while tokens.get(i).is_some_and(|t| t.start <= end) {
                i += 1;
            }
        }
    }
    let token = tokens.get(i)?;
    is_identifier(token.text).then_some(token.text)
}

#[derive(Clone, Debug)]
pub(crate) struct Function {
    pub signature: Signature,
    pub range: Range<usize>,
    pub body: Range<usize>,
    pub has_body: bool,
}

pub(crate) fn function(source: &str, range: Range<usize>) -> Option<Function> {
    let raw = &source[range.clone()];
    let trimmed = raw.trim_start();
    let start = range.start + raw.len() - trimmed.len();
    let rest = trimmed.strip_prefix("fun")?;
    if !rest.starts_with(char::is_whitespace) {
        return None;
    }
    let rest_start = start + 3;
    let mut open = None;
    let mut equals = None;
    visit_top_level(rest, |i, ch| {
        if open.is_none() && ch == '(' {
            open = Some(i);
        }
        if equals.is_none() && ch == '=' {
            equals = Some(i);
        }
    });
    let open = open?;
    let close = find_matching_delimiter(rest, open, '(', ')')?;
    let end = equals.unwrap_or(close + 1);
    if end < close {
        return None;
    }
    let clean = mask_comments(&rest[..end]).ok()?;
    let name = strip_generic_function_type_parameters(clean[..open].trim());
    if !is_identifier(name) {
        return None;
    }
    let parameters = split_top_level(&clean[open + 1..close], ',')
        .into_iter()
        .filter(|p| !p.trim().is_empty())
        .map(|p| {
            let name = parameter_name(p).unwrap_or("");
            let start = if name.is_empty() {
                0
            } else {
                name.as_ptr() as usize - p.as_ptr() as usize
            };
            p[start..].split_whitespace().collect::<Vec<_>>().join(" ")
        })
        .collect();
    let result = clean[close + 1..]
        .trim()
        .strip_prefix(':')
        .map(|t| t.split_whitespace().collect::<Vec<_>>().join(" "));
    Some(Function {
        signature: Signature {
            name: name.into(),
            parameters,
            return_type: result,
            documentation: None,
            infix: false,
        },
        range: start..range.end,
        has_body: equals.is_some(),
        body: rest_start + equals.map_or(rest.len(), |i| i + 1)..range.end,
    })
}

/// Export declarations only; nested implementation helpers are excluded.
pub fn module_signatures(source: &str) -> Vec<Signature> {
    header_declaration_spans(source)
        .into_iter()
        .filter_map(|r| function(source, r).map(|f| f.signature))
        .collect()
}
