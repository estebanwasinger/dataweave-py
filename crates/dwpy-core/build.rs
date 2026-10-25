//! Discover functions automatically on every engine build. The only inputs are
//! runtime dispatch and module declarations; there is no maintained name catalog.
mod catalog_source;
#[allow(dead_code)]
#[path = "src/declarations.rs"]
mod declarations;
#[allow(dead_code)]
#[path = "src/language/signature.rs"]
mod signature;
#[allow(dead_code)]
#[path = "src/syntax.rs"]
mod syntax;
use serde_json::json;
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
};
#[derive(Debug)]
#[allow(dead_code)]
pub enum DwError {
    Parse(String),
}

fn modules(path: &Path, root: &Path, out: &mut BTreeMap<String, Vec<signature::Signature>>) {
    for entry in std::fs::read_dir(path).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            modules(&path, root, out);
        } else if path.extension().is_some_and(|s| s == "dwl") {
            let name = path
                .strip_prefix(root)
                .unwrap()
                .with_extension("")
                .to_string_lossy()
                .replace(std::path::MAIN_SEPARATOR, "::");
            out.insert(
                name,
                signature::module_signatures(&std::fs::read_to_string(path).unwrap()),
            );
        }
    }
}
fn main() {
    for path in [
        "src",
        "src/calls.rs",
        "src/jwt.rs",
        "src/lib.rs",
        "../../dwpy/modules",
        "../../dwpy/builtins.py",
        "src/syntax.rs",
        "src/declarations.rs",
        "src/language/signature.rs",
        "catalog_source.rs",
    ] {
        println!("cargo:rerun-if-changed={path}");
    }
    let dispatch = catalog_source::extract(
        &std::fs::read_to_string("src/calls.rs").unwrap(),
        &std::fs::read_to_string("src/lib.rs").unwrap(),
    );
    let operators =
        catalog_source::keyword_operators(&std::fs::read_to_string("src/lib.rs").unwrap());
    let root = Path::new("../../dwpy/modules");
    let mut metadata = BTreeMap::new();
    modules(root, root, &mut metadata);
    let mut exports =
        catalog_source::python_exports(&std::fs::read_to_string("../../dwpy/builtins.py").unwrap());
    for (module, signatures) in &metadata {
        exports
            .entry(module.clone())
            .or_default()
            .extend(signatures.iter().map(|s| s.name.clone()));
    }
    // Qualified dispatch entries declare their namespace directly, including new modules.
    for name in dispatch.keys() {
        if let Some((module, name)) = name.rsplit_once("::") {
            let module = if module.starts_with("dw::") {
                module.to_string()
            } else {
                metadata
                    .keys()
                    .find(|m| m.rsplit("::").next() == Some(module))
                    .cloned()
                    .unwrap_or_else(|| module.to_string())
            };
            exports.entry(module).or_default().insert(name.to_string());
        }
    }
    let mut functions = vec![];
    for (name, arities) in &dispatch {
        let short = name.rsplit("::").next().unwrap();
        let memberships: Vec<_> = exports
            .iter()
            .filter(|(module, names)| {
                names.contains(short)
                    && (!name.contains("::")
                        || name.rsplit_once("::").is_some_and(|(namespace, _)| {
                            namespace == module.as_str()
                                || module.rsplit("::").next() == Some(namespace)
                        }))
            })
            .map(|(m, _)| m.clone())
            .collect();
        let mut signatures = vec![];
        let mut seen = BTreeSet::new();
        for arity in arities {
            let candidates: Vec<_> = memberships
                .iter()
                .flat_map(|m| metadata.get(m).into_iter().flatten())
                .filter(|s| s.name == short && s.parameters.len() == *arity)
                .collect();
            if candidates.is_empty() {
                let names = [
                    "value",
                    "other",
                    "options",
                    "configuration",
                    "context",
                    "argument6",
                    "argument7",
                    "argument8",
                ];
                let parameters = (0..*arity)
                    .map(|i| {
                        names
                            .get(i)
                            .map_or_else(|| format!("argument{}", i + 1), |s| s.to_string())
                    })
                    .collect::<Vec<_>>();
                signatures.push(json!({"name":name,"parameters":parameters,"returnType":null,"documentation":"Supported by the active Rust engine."}));
            } else {
                for candidate in candidates {
                    let mut signature = candidate.renamed(name);
                    signature
                        .documentation
                        .get_or_insert_with(|| "Supported by the active Rust engine.".into());
                    let key = (signature.parameters.clone(), signature.return_type.clone());
                    if seen.insert(key) {
                        signatures.push(serde_json::to_value(signature).unwrap());
                    }
                }
            }
        }
        functions.push(
            json!({"name":name,"arities":arities,"modules":memberships,"signatures":signatures}),
        );
    }
    let output = Path::new(&std::env::var("OUT_DIR").unwrap()).join("function_catalog.json");
    std::fs::write(
        output,
        serde_json::to_string(&json!({"version":1,"functions":functions,"operators":operators}))
            .unwrap(),
    )
    .unwrap();
}
