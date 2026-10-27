//! Choose invocation syntax from tolerant source tokens, without evaluating code.
use crate::language::Signature;
use crate::syntax::{
    editor_tokens, find_matching_delimiter, is_identifier, split_top_level, split_top_level_char,
};

pub(super) fn after_value(
    source: &str,
    position: usize,
    infix_names: &std::collections::BTreeSet<String>,
) -> bool {
    let tokens = editor_tokens(source);
    let Some(index) = tokens.iter().rposition(|t| t.end <= position) else {
        return false;
    };
    let token = tokens[index];
    if token.literal {
        return true;
    }
    if !token.text.is_empty() && token.text.bytes().all(|c| c.is_ascii_digit()) {
        return true;
    }
    if token.text == ")" {
        let (matching, _) = crate::language::groups(&tokens);
        // An if condition or using binding list is followed by a new expression.
        if matching[index]
            .and_then(|open| open.checked_sub(1))
            .is_some_and(|i| matches!(tokens[i].text, "if" | "using"))
        {
            return false;
        }
        return true;
    }
    if matches!(token.text, "]" | "}") {
        return true;
    }
    if !(is_identifier(token.text) || matches!(token.text, "$" | "$$" | "$$$")) {
        return false;
    }
    // A selector named like an operator still completes an operand.
    if index > 0 && matches!(tokens[index - 1].text, "." | "::") {
        return true;
    }
    !matches!(
        token.text,
        "if" | "else"
            | "case"
            | "not"
            | "do"
            | "using"
            | "fun"
            | "var"
            | "import"
            | "output"
            | "input"
            | "type"
    ) && !crate::function_catalog::operators()
        .iter()
        .any(|op| op == token.text)
        && !crate::BINARY_BUILTIN_OPERATORS.contains(&token.text)
        && !crate::NUMERIC_BUILTIN_OPERATORS.contains(&token.text)
        && !crate::COLLECTION_OPERATORS.contains(&token.text)
        && !infix_names.contains(token.text)
}

pub(super) fn has_rhs(suffix: &str) -> bool {
    editor_tokens(suffix)
        .first()
        .is_some_and(|t| !matches!(t.text, ")" | "]" | "}" | "," | ";"))
}

pub(super) fn escape(name: &str) -> String {
    name.replace('\\', "\\\\")
        .replace('$', "\\$")
        .replace('}', "\\}")
}

pub(super) fn infix_snippet(signature: &Signature) -> String {
    let rhs = &signature.parameters[1];
    if let Some((_, annotation)) = split_top_level_char(rhs, ':') {
        let annotation = annotation.trim();
        if let Some(close) = find_matching_delimiter(annotation, 0, '(', ')') {
            if annotation[close + 1..].trim_start().starts_with("->") {
                let parameters = split_top_level(&annotation[1..close], ',')
                    .into_iter()
                    .enumerate()
                    .filter(|(_, p)| !p.trim().is_empty())
                    .map(|(i, p)| {
                        split_top_level_char(p, ':')
                            .map(|(name, _)| name.trim().to_owned())
                            .filter(|name| is_identifier(name))
                            .unwrap_or_else(|| format!("argument{}", i + 1))
                    })
                    .collect::<Vec<_>>()
                    .join(", ");
                return format!("{} (({parameters}) -> ${{1}})", signature.name);
            }
        }
    }
    let name = signature
        .parameter_names()
        .get(1)
        .cloned()
        .unwrap_or_else(|| "other".into());
    format!("{} ${{1:{}}}", signature.name, escape(&name))
}
