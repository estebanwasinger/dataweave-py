//! Source-only editor completions. Never executes initializers or function bodies.
//!
//! Header recovery is independent of body parsing so partially written expressions
//! do not hide declarations. Syntax boundaries come from the runtime's lexer.
use crate::syntax::is_identifier;
use crate::{DwMap, DwValue};
use serde::Serialize;
mod references;

/// Complete selectors using parsed runtime values, without evaluating user code.
/// A lookup is requested only for a recognized context root and selector path.
pub fn complete_fields(
    source: &str,
    line: usize,
    column: usize,
    lookup: impl FnOnce(&str) -> Option<DwValue>,
) -> Option<Vec<CompletionItem>> {
    use references::Projection;
    let before = &source[..position_offset(source, line, column)];
    let dot = before.rfind('.')?;
    let partial = &before[dot + 1..];
    if !partial
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '@'))
    {
        return None;
    }
    let (reference, label) = references::resolve(source, dot)?;
    if reference.projections.len() > 64 {
        return Some(vec![]);
    }
    let Some(value) = lookup(&reference.root) else {
        return Some(vec![]);
    };
    let mut values = vec![value];
    for projection in reference.projections {
        let mut next = vec![];
        if matches!(projection, Projection::Array) {
            values = vec![DwValue::Array(values)];
            continue;
        }
        for value in values {
            if next.len() >= 10_000 {
                break;
            }
            match &projection {
                Projection::Selector(segment) => {
                    // Preserve native XML nodes, field ordering and duplicates.
                    if let Ok(value) = crate::selectors::select_path_segment(&value, segment, false)
                    {
                        next.push(value);
                    }
                }
                Projection::ArrayItems | Projection::ObjectValues => {
                    let value = crate::selectors::unwrap_metadata_value(&value).unwrap_or(value);
                    let remaining = 10_000 - next.len();
                    if matches!(projection, Projection::ArrayItems) {
                        if let Some(items) = crate::xml::xml_list_items(&value) {
                            next.extend(items.iter().take(remaining).cloned());
                            continue;
                        }
                    }
                    match (&projection, value) {
                        (Projection::ArrayItems, DwValue::Array(items)) => {
                            next.extend(items.into_iter().take(remaining))
                        }
                        (Projection::ObjectValues, value) => {
                            if let Some(pairs) = crate::selectors::duplicate_object_pairs(&value) {
                                next.extend(pairs.into_iter().map(|(_, v)| v).take(remaining));
                            } else if let DwValue::Object(fields) = value {
                                next.extend(fields.into_iter().map(|(_, v)| v).take(remaining));
                            }
                        }
                        _ => (),
                    }
                }
                Projection::Array => unreachable!(),
            }
        }
        values = next;
    }
    let mut keys = DwMap::<String, ()>::new();
    let mut budget = 10_000;
    for value in values {
        collect_field_names(&value, &mut keys, &mut budget, 0);
    }
    let prefix = partial.to_lowercase();
    Some(
        keys.keys()
            .filter(|key| key.to_lowercase().starts_with(&prefix))
            .map(|key| {
                let insert_text =
                    if is_identifier(key) || key.strip_prefix('@').is_some_and(is_identifier) {
                        key.clone()
                    } else {
                        serde_json::to_string(key).unwrap()
                    };
                CompletionItem {
                    label: key.clone(),
                    kind: "field",
                    insert_text,
                    insert_text_format: "plain",
                    detail: format!("{label} field"),
                    sort_text: format!("1_{key}"),
                }
            })
            .collect(),
    )
}

fn collect_field_names(
    value: &DwValue,
    keys: &mut DwMap<String, ()>,
    budget: &mut usize,
    depth: usize,
) {
    use crate::selectors::{duplicate_object_pairs, unwrap_metadata_value};
    if *budget == 0 || depth > 64 {
        return;
    }
    *budget -= 1;
    if let Some(value) = unwrap_metadata_value(value) {
        collect_field_names(&value, keys, budget, depth + 1);
    } else if let Some(values) = crate::xml::xml_list_items(value) {
        for value in values {
            if *budget == 0 {
                break;
            }
            collect_field_names(value, keys, budget, depth + 1);
        }
    } else if let Some(pairs) = duplicate_object_pairs(value) {
        for (key, _) in pairs {
            keys.insert(key, ());
        }
    } else {
        match value {
            DwValue::Object(fields) => {
                for key in fields.keys() {
                    keys.insert(key.clone(), ());
                }
            }
            DwValue::Array(values) => {
                for value in values {
                    if *budget == 0 {
                        break;
                    }
                    collect_field_names(value, keys, budget, depth + 1);
                }
            }
            _ => (),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CompletionItem {
    pub label: String,
    pub kind: &'static str,
    pub insert_text: String,
    pub insert_text_format: &'static str,
    pub detail: String,
    pub sort_text: String,
}

/// Zero-based UTF-16 positions, as used by LSP and Monaco.
pub fn complete(source: &str, line: usize, column: usize) -> Vec<CompletionItem> {
    complete_with_modules(source, line, column, &Default::default())
}
pub fn complete_with_modules(
    source: &str,
    line: usize,
    column: usize,
    module_sources: &std::collections::BTreeMap<String, String>,
) -> Vec<CompletionItem> {
    use crate::language::Binding;
    let cursor = position_offset(source, line, column);
    let before = &source[..cursor];
    let prefix = before
        .rsplit(|c: char| !c.is_ascii_alphanumeric() && !matches!(c, '_' | '$' | ':'))
        .next()
        .unwrap_or("");
    let context = &source[..cursor - prefix.len()];
    if context.trim_end().ends_with('.') {
        return vec![];
    }
    let following_word = source[cursor..]
        .find(|c: char| !c.is_ascii_alphanumeric() && c != '_' && c != '$')
        .map_or(source.len(), |end| cursor + end);
    let has_arguments = source[following_word..].trim_start().starts_with('(');
    let mut bindings = std::collections::BTreeMap::new();
    let mut ranks = std::collections::BTreeMap::new();
    for f in crate::function_catalog::functions() {
        bindings.insert(f.name.clone(), Binding::Function(f.signatures.clone()));
        ranks.insert(f.name.clone(), "4");
    }
    // Fully qualified module access does not require a wildcard import.
    if let Some((module, _)) = prefix.rsplit_once("::") {
        let modules: std::collections::BTreeSet<_> = crate::function_catalog::functions()
            .iter()
            .flat_map(|f| &f.modules)
            .filter(|m| m.as_str() == module || m.rsplit("::").next() == Some(module))
            .collect();
        for m in modules {
            for (name, target) in crate::function_catalog::module_exports(m) {
                let label = format!("{module}::{name}");
                let signatures = crate::function_catalog::signatures(&target)
                    .iter()
                    .map(|s| s.renamed(&label))
                    .collect();
                bindings.insert(label.clone(), Binding::Function(signatures));
                ranks.insert(label, "3");
            }
        }
    }
    for name in ["payload", "attributes", "vars"] {
        bindings.insert(name.into(), Binding::Variable("Runtime context"));
        ranks.insert(name.into(), "2");
    }
    for (name, binding) in crate::language::visible_bindings(source, cursor, module_sources) {
        let rank = match &binding {
            Binding::Function(s) if s.iter().any(|s| s.documentation.is_some()) => "3",
            Binding::Variable("Function parameter" | "Lambda parameter") => "0",
            _ => "1",
        };
        bindings.insert(name.clone(), binding);
        ranks.insert(name, rank);
    }
    let lower = prefix.to_lowercase();
    let mut items = vec![];
    for name in crate::function_catalog::operators() {
        if name.to_lowercase().starts_with(&lower) && !bindings.contains_key(name) {
            items.push(CompletionItem {
                label: name.clone(),
                kind: "keyword",
                insert_text: name.clone(),
                insert_text_format: "plain",
                detail: "DataWeave operator".into(),
                sort_text: format!("4_{name}"),
            });
        }
    }
    for (name, binding) in bindings {
        if !name.to_lowercase().starts_with(&lower) {
            continue;
        }
        let rank = ranks[&name];
        match binding {
            Binding::Function(signatures) => {
                for signature in signatures {
                    let arguments = signature
                        .parameter_names()
                        .iter()
                        .enumerate()
                        .map(|(i, name)| format!("${{{}:{}}}", i + 1, name))
                        .collect::<Vec<_>>()
                        .join(", ");
                    items.push(CompletionItem {
                        label: name.clone(),
                        kind: "function",
                        insert_text: if has_arguments {
                            name.clone()
                        } else {
                            format!("{name}({arguments})")
                        },
                        insert_text_format: if has_arguments { "plain" } else { "snippet" },
                        detail: signature.label(),
                        sort_text: format!("{rank}_{name}_{:04}", signature.parameters.len()),
                    });
                }
            }
            Binding::Variable(detail) => items.push(variable(&name, detail, rank)),
            Binding::Reference(_) => items.push(variable(&name, "Script variable", rank)),
        }
    }
    items.sort_by(|a, b| a.sort_text.cmp(&b.sort_text).then(a.detail.cmp(&b.detail)));
    let mut seen = std::collections::BTreeSet::new();
    items.retain(|i| seen.insert((i.label.clone(), i.detail.clone())));
    items
}

fn variable(name: &str, detail: &str, rank: &str) -> CompletionItem {
    CompletionItem {
        label: name.into(),
        kind: "variable",
        insert_text: name.into(),
        insert_text_format: "plain",
        detail: detail.into(),
        sort_text: format!("{rank}_{name}"),
    }
}

fn position_offset(source: &str, line: usize, column: usize) -> usize {
    let mut start = 0;
    for _ in 0..line {
        let Some(end) = source[start..].find('\n') else {
            return source.len();
        };
        start += end + 1;
    }
    let mut units = 0;
    for (offset, ch) in source[start..].char_indices() {
        if matches!(ch, '\r' | '\n') || units + ch.len_utf16() > column {
            return start + offset;
        }
        units += ch.len_utf16();
    }
    source.len()
}
