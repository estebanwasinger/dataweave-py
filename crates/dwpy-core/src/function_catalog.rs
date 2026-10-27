//! Shared supported-function contract used by runtime imports and language tools.
use crate::language::Signature;
use crate::syntax::{is_identifier, mask_comments, split_top_level, split_top_level_keyword};
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Function {
    pub name: String,
    pub arities: Vec<usize>,
    pub modules: Vec<String>,
    pub signatures: Vec<Signature>,
}
#[derive(Deserialize)]
struct Catalog {
    functions: Vec<Function>,
    operators: Vec<String>,
}
fn catalog() -> &'static Catalog {
    static CATALOG: OnceLock<Catalog> = OnceLock::new();
    CATALOG.get_or_init(|| {
        let mut catalog: Catalog = serde_json::from_str(include_str!(concat!(
            env!("OUT_DIR"),
            "/function_catalog.json"
        )))
        .expect("validated function catalog");
        for function in &mut catalog.functions {
            let infix = crate::BINARY_BUILTIN_OPERATORS.contains(&function.name.as_str())
                || crate::NUMERIC_BUILTIN_OPERATORS.contains(&function.name.as_str())
                || crate::COLLECTION_OPERATORS.contains(&function.name.as_str())
                || catalog.operators.contains(&function.name);
            for signature in &mut function.signatures {
                signature.infix = infix && signature.parameters.len() == 2;
            }
        }
        catalog
    })
}
pub fn functions() -> &'static [Function] {
    &catalog().functions
}
pub fn operators() -> &'static [String] {
    &catalog().operators
}
pub fn signatures(name: &str) -> Vec<Signature> {
    functions()
        .iter()
        .find(|f| f.name == name)
        .map(|f| f.signatures.clone())
        .unwrap_or_default()
}
pub(crate) fn imported_signature(signature: &Signature, name: &str) -> Signature {
    let mut renamed = signature.renamed(name);
    // The runtime resolves imported aliases for collection operators. Scalar
    // binary operators are recognized only by their canonical spelling.
    renamed.infix = signature.infix
        && (name == signature.name
            || crate::COLLECTION_OPERATORS.contains(&signature.name.as_str()));
    renamed
}
pub fn module_exports(module: &str) -> std::collections::BTreeMap<String, String> {
    let mut result = std::collections::BTreeMap::new();
    for function in functions()
        .iter()
        .filter(|f| f.modules.iter().any(|m| m == module))
    {
        let name = function.name.rsplit("::").next().unwrap();
        // Prefer a specialized namespaced implementation when it exists.
        if !result.contains_key(name) || function.name.contains("::") {
            result.insert(name.into(), function.name.clone());
        }
    }
    result
}
pub fn resolve_qualified(name: &str) -> Option<String> {
    let (module, name) = name.rsplit_once("::")?;
    for function in functions() {
        if function.name.rsplit("::").next() == Some(name)
            && function
                .modules
                .iter()
                .any(|m| m == module || m.rsplit("::").next() == Some(module))
        {
            return module_exports(
                function
                    .modules
                    .iter()
                    .find(|m| m.as_str() == module || m.rsplit("::").next() == Some(module))?,
            )
            .get(name)
            .cloned();
        }
    }
    None
}
/// Same import binding calculation is used by execution and completion.
fn import_bindings_with(
    source: &str,
    exports_for: impl Fn(&str) -> std::collections::BTreeMap<String, String>,
) -> std::collections::BTreeMap<String, String> {
    let clean = mask_comments(source).unwrap_or_default();
    let source = clean.trim();
    let mut bindings = std::collections::BTreeMap::new();
    if let Some((items, module)) = split_top_level_keyword(source, "from") {
        let exports = exports_for(module.trim());
        for item in split_top_level(items, ',') {
            let (name, alias) =
                split_top_level_keyword(item.trim(), "as").unwrap_or((item.trim(), item.trim()));
            if name == "*" {
                bindings.extend(exports.clone());
            } else if is_identifier(alias) {
                if let Some(target) = exports.get(name) {
                    bindings.insert(alias.into(), target.clone());
                }
            }
        }
    } else {
        let (module, alias) = split_top_level_keyword(source, "as")
            .unwrap_or((source, source.rsplit("::").next().unwrap_or(source)));
        if is_identifier(alias) {
            for (name, target) in exports_for(module) {
                bindings.insert(format!("{alias}::{name}"), target.clone());
                bindings.insert(format!("{module}::{name}"), target);
            }
        }
    }
    bindings
}

pub fn import_bindings(source: &str) -> std::collections::BTreeMap<String, String> {
    import_bindings_with(source, module_exports)
}

/// Module source is supplied by the host, so document/module edits appear on the
/// next request without mutating the engine's builtin registry.
pub fn imported_signatures(
    source: &str,
    module_sources: &std::collections::BTreeMap<String, String>,
) -> std::collections::BTreeMap<String, Vec<Signature>> {
    let mut external = std::collections::BTreeMap::new();
    for (module, source) in module_sources {
        for s in crate::language::module_signatures(source) {
            external
                .entry(format!("{module}::{}", s.name))
                .or_insert_with(Vec::new)
                .push(s);
        }
    }
    let bindings = import_bindings_with(source, |module| {
        let mut exports = module_exports(module);
        for name in external
            .keys()
            .filter_map(|n: &String| n.strip_prefix(&format!("{module}::")))
        {
            exports.insert(name.into(), format!("{module}::{name}"));
        }
        exports
    });
    bindings
        .into_iter()
        .filter_map(|(name, target)| {
            let signatures = external
                .get(&target)
                .cloned()
                .unwrap_or_else(|| signatures(&target));
            (!signatures.is_empty()).then(|| {
                (
                    name.clone(),
                    signatures
                        .iter()
                        .map(|s| imported_signature(s, &name))
                        .collect(),
                )
            })
        })
        .collect()
}
