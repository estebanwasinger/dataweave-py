//! Static reference analysis over the shared lexer and lexical scope index.
//! Literal contents are opaque except for interpolation expressions emitted by
//! the lexer. No header initializer, callback, or function is executed here.
use crate::language::ScopeIndex;
use crate::syntax::{editor_tokens, is_identifier, CodeToken};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Default)]
pub(crate) struct References {
    pub paths: BTreeSet<String>,
    pub wildcards: BTreeSet<String>,
}

pub(crate) fn validate(source: &str) -> Result<(), crate::DwError> {
    crate::syntax::mask_comments(source)?;
    let tokens = editor_tokens(source);
    let mut stack = vec![];
    for token in &tokens {
        if token.literal {
            continue;
        }
        match token.text {
            "(" => stack.push(")"),
            "[" => stack.push("]"),
            "{" => stack.push("}"),
            ")" | "]" | "}" if stack.pop() != Some(token.text) => {
                return Err(crate::DwError::Parse(format!("unexpected {}", token.text)))
            }
            _ => (),
        }
    }
    if let Some(close) = stack.pop() {
        return Err(crate::DwError::Parse(format!("expected {close}")));
    }
    if crate::script::split_script(source).body.trim().is_empty() {
        return Err(crate::DwError::Parse("empty expression".into()));
    }
    let descendant = tokens.len() >= 2
        && tokens[tokens.len() - 2].text == "."
        && tokens.last().is_some_and(|t| matches!(t.text, "." | "*"));
    if !descendant
        && tokens.last().is_some_and(|t| {
            !t.literal
                && matches!(
                    t.text,
                    "+" | "-" | "*" | "/" | "=" | "->" | ":" | "::" | "."
                )
        })
    {
        return Err(crate::DwError::Parse("missing expression".into()));
    }
    Ok(())
}

pub(crate) fn references(source: &str, payload: Option<&crate::DwValue>) -> References {
    let tokens = editor_tokens(source);
    let (matching, parent) = crate::language::groups(&tokens);
    let scopes = ScopeIndex::new(source, &BTreeMap::new());
    let mut ignored = tokens
        .iter()
        .map(|t| scopes.annotations.iter().any(|r| r.contains(&t.start)))
        .collect::<Vec<_>>();
    // Types are syntax after coercion/type checks and in literal type arguments.
    for (i, token) in tokens.iter().enumerate() {
        if token.literal || ignored[i] {
            continue;
        }
        if matches!(token.text, "as" | "is") && !selector_component(&tokens, i) {
            let end = type_end(&tokens, &matching, i + 1);
            ignored[i + 1..end].fill(true);
        }
        if is_identifier(token.text) && tokens.get(i + 1).is_some_and(|t| t.text == "<") {
            let end = type_end(&tokens, &matching, i);
            if tokens.get(end).is_some_and(|t| t.text == "(") {
                ignored[i + 1..end].fill(true);
            }
        }
        if tokens.get(i + 1).is_some_and(|t| t.text == "(") && type_function(token.text) {
            if let Some(close) = matching[i + 1] {
                let first = tokens.get(i + 2);
                if first.is_some_and(|t| {
                    t.text.chars().next().is_some_and(char::is_uppercase) || t.text == "("
                }) {
                    ignored[i + 2..close].fill(true);
                }
            }
        }
    }
    let operators = crate::function_catalog::operators();
    let mut operator_roles = vec![false; tokens.len()];
    for (i, token) in tokens.iter().enumerate() {
        if !token.literal
            && !ignored[i]
            && (operators.iter().any(|op| op == token.text)
                || crate::BINARY_BUILTIN_OPERATORS.contains(&token.text)
                || crate::COLLECTION_OPERATORS.contains(&token.text)
                || matches!(token.text, "mod" | "pow" | "match"))
            && !selector_component(&tokens, i)
            && i > 0
            && !operator_roles[i - 1]
            && ends_operand(&tokens, i - 1)
            && tokens.get(i + 1).is_some_and(starts_operand)
        {
            operator_roles[i] = true;
        }
    }
    let mut result = References {
        paths: scopes.inputs.clone(),
        ..References::default()
    };
    let mut i = 0;
    while i < tokens.len() {
        let token = tokens[i];
        if token.literal
            || ignored[i]
            || operator_roles[i]
            || !is_identifier(token.text)
            || token.text.starts_with('$')
            || keyword(token.text)
            || selector_component(&tokens, i)
            || object_key(&tokens, &matching, i)
            || (token.text == "compose" && tokens.get(i + 1).is_some_and(starts_operand))
            // Execution reads payload from its runtime input, including when
            // the script declares its format with `input payload ...`.
            || (token.text != "payload" && scopes.is_bound(token.text, token.start))
        {
            i += 1;
            continue;
        }
        // Numeric exponents are part of numbers, never variable names.
        if i > 0
            && tokens[i - 1].end == token.start
            && tokens[i - 1]
                .text
                .chars()
                .last()
                .is_some_and(|c| c.is_ascii_digit())
        {
            i += 1;
            continue;
        }
        let known_data = payload.is_some_and(|p| p.get(token.text).is_some());
        let call_argument = parent[i].is_some_and(|open| {
            open > 0 && tokens[open].text == "(" && is_identifier(tokens[open - 1].text)
        });
        let callback = i > 0
            && operator_roles[i - 1]
            && crate::COLLECTION_OPERATORS.contains(&tokens[i - 1].text);
        let initializer = i > 0
            && tokens[i - 1].text == "="
            && tokens.get(i + 1).is_none_or(|next| {
                matches!(next.text, "," | ")" | "---")
                    || source[token.end..next.start].contains('\n')
            });
        if !known_data
            && (call_argument || callback || initializer)
            && crate::function_catalog::functions()
                .iter()
                .any(|f| f.name == token.text)
        {
            i += 1;
            continue;
        }
        let mut end = i + 1;
        let mut qualified = false;
        while tokens.get(end).is_some_and(|t| t.text == "::")
            && tokens.get(end + 1).is_some_and(|t| is_identifier(t.text))
        {
            qualified = true;
            end += 2;
        }
        if qualified {
            i = end;
            continue;
        }
        let mut path = token.text.to_owned();
        let mut dynamic = false;
        while tokens
            .get(end)
            .is_some_and(|t| t.text == "." || t.text == "?")
        {
            let mut next = end;
            if tokens[next].text == "?" {
                next += 1;
            }
            if tokens.get(next).is_none_or(|t| t.text != ".") {
                break;
            }
            next += 1;
            if tokens.get(next).is_some_and(|t| t.text == ".") {
                dynamic = true;
                next += 1;
            }
            let attribute = tokens.get(next).is_some_and(|t| t.text == "@");
            if tokens.get(next).is_some_and(|t| t.text == "^") {
                dynamic = true;
            }
            if tokens
                .get(next)
                .is_some_and(|t| matches!(t.text, "*" | "&" | "@" | "^"))
            {
                next += 1;
            }
            let Some(component) = tokens.get(next) else {
                dynamic = true;
                break;
            };
            let name = if !component.literal && is_identifier(component.text) {
                Some(component.text.to_owned())
            } else if component.literal
                && matches!(component.text.chars().next(), Some('\'' | '"' | '`'))
                && !component.text.contains("$(")
            {
                crate::literals::parse_string_literal(component.text)
                    .ok()
                    .flatten()
            } else {
                None
            };
            let Some(mut name) = name else {
                dynamic = true;
                break;
            };
            // A namespace qualifier belongs to the selector, not to a data root.
            if tokens.get(next + 1).is_some_and(|t| t.text == "#") {
                if let Some(leaf) = tokens.get(next + 2) {
                    if leaf.literal && leaf.text.contains("$(") {
                        dynamic = true;
                        break;
                    }
                    name = if leaf.literal {
                        crate::literals::parse_string_literal(leaf.text)
                            .ok()
                            .flatten()
                            .unwrap_or_else(|| leaf.text.into())
                    } else {
                        leaf.text.into()
                    };
                    next += 2;
                }
            }
            if name.contains('.') {
                dynamic = true;
            }
            if !dynamic {
                path.push('.');
                if attribute {
                    path.push('@');
                }
                path.push_str(&name);
            }
            end = next + 1;
        }
        let call = tokens.get(end).is_some_and(|t| t.text == "(");
        if end == i + 1 && call {
            i += 1;
            continue;
        }
        if dynamic || tokens.get(end).is_some_and(|t| t.text == "[") {
            result.wildcards.insert(path.clone());
        }
        result.paths.insert(path);
        // Interpolation tokens inside quoted selector components are not consumed
        // above; the main walk will visit them and record their own inputs.
        i = end;
    }
    result
}

fn object_key(tokens: &[CodeToken<'_>], matching: &[Option<usize>], i: usize) -> bool {
    let next = tokens.get(i + 1).map(|t| t.text);
    matches!(next, Some(":" | "#"))
        || (next == Some("@")
            && tokens.get(i + 2).is_some_and(|t| t.text == "(")
            && matching[i + 2]
                .and_then(|close| tokens.get(close + 1))
                .is_some_and(|t| t.text == ":"))
}

fn selector_component(tokens: &[CodeToken<'_>], i: usize) -> bool {
    if i == 0 {
        return false;
    }
    let previous = tokens[i - 1].text;
    matches!(previous, "." | "::" | "#")
        || (matches!(previous, "*" | "&" | "@" | "^") && i > 1 && tokens[i - 2].text == ".")
}

fn ends_operand(tokens: &[CodeToken<'_>], i: usize) -> bool {
    let t = tokens[i];
    t.literal
        || t.text.starts_with('$')
        || matches!(t.text, ")" | "]" | "}" | "true" | "false" | "null")
        || (is_identifier(t.text) && (!keyword(t.text) || selector_component(tokens, i)))
        || t.text.chars().next().is_some_and(|c| c.is_ascii_digit())
}
fn starts_operand(t: &CodeToken<'_>) -> bool {
    t.literal
        || t.text.starts_with('$')
        || matches!(
            t.text,
            "(" | "[" | "{" | "-" | "!" | "if" | "do" | "using" | "not" | "true" | "false" | "null"
        )
        || (is_identifier(t.text) && !keyword(t.text))
        || t.text.chars().next().is_some_and(|c| c.is_ascii_digit())
}
fn keyword(name: &str) -> bool {
    matches!(
        name,
        "and"
            | "as"
            | "case"
            | "default"
            | "do"
            | "else"
            | "false"
            | "fun"
            | "if"
            | "import"
            | "input"
            | "is"
            | "not"
            | "null"
            | "or"
            | "output"
            | "true"
            | "type"
            | "using"
            | "var"
            | "OBJECT_TYPE"
            | "ATTRIBUTE_TYPE"
            | "ARRAY_TYPE"
            | "VALUE_TYPE"
    )
}
fn type_function(name: &str) -> bool {
    crate::function_catalog::functions().iter().any(|f| {
        f.name == name
            && ((name.starts_with("is") && name.ends_with("Type"))
                || matches!(
                    name,
                    "arrayItem"
                        | "baseTypeOf"
                        | "functionParamTypes"
                        | "functionReturnType"
                        | "intersectionItems"
                        | "literalValueOf"
                        | "metadataOf"
                        | "nameOf"
                        | "unionItems"
                ))
    })
}
fn type_atom_end(tokens: &[CodeToken<'_>], matching: &[Option<usize>], start: usize) -> usize {
    let Some(first) = tokens.get(start) else {
        return start;
    };
    let mut end = if matches!(first.text, "{" | "[" | "(") {
        matching[start].map_or(start + 1, |close| close + 1)
    } else {
        start + 1
    };
    if tokens.get(end).is_some_and(|t| t.text == "<") {
        let mut depth = 0;
        while end < tokens.len() {
            match tokens[end].text {
                "<" => depth += 1,
                ">" => depth -= 1,
                _ => (),
            }
            end += 1;
            if depth == 0 {
                break;
            }
        }
    }
    while tokens.get(end).is_some_and(|t| t.text == ".")
        && tokens.get(end + 1).is_some_and(|t| is_identifier(t.text))
    {
        end += 2;
    }
    if tokens.get(end).is_some_and(|t| t.text == "{") {
        end = matching[end].map_or(end, |close| close + 1);
    }
    end
}

pub(crate) fn type_end(
    tokens: &[CodeToken<'_>],
    matching: &[Option<usize>],
    start: usize,
) -> usize {
    let mut end = type_atom_end(tokens, matching, start);
    while tokens.get(end).is_some_and(|t| matches!(t.text, "|" | "&"))
        && tokens.get(end + 1).is_some()
    {
        end = type_atom_end(tokens, matching, end + 1);
    }
    end
}
