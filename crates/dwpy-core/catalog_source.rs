//! Discover the callable contract from Rust syntax; shared by engine builds and coverage tests.
use std::collections::{BTreeMap, BTreeSet};
use syn::{visit::Visit, BinOp, Expr, Item, Lit, Pat};

fn names(pattern: &Pat, out: &mut Vec<String>) {
    match pattern {
        Pat::Lit(p) => {
            if let Lit::Str(s) = &p.lit {
                out.push(s.value());
            } else {
                panic!("builtin dispatch names must be string literals");
            }
        }
        Pat::Or(p) => {
            for p in &p.cases {
                names(p, out);
            }
        }
        Pat::Wild(_) => (),
        _ => panic!("unhandled builtin dispatch name pattern"),
    }
}
fn number(expr: &Expr, arity: usize) -> Option<usize> {
    match expr {
        Expr::Path(p) if p.path.is_ident("arity") => Some(arity),
        Expr::Lit(p) => {
            if let Lit::Int(n) = &p.lit {
                n.base10_parse().ok()
            } else {
                None
            }
        }
        Expr::Paren(p) => number(&p.expr, arity),
        Expr::Reference(p) => number(&p.expr, arity),
        _ => None,
    }
}
fn accepts(expr: &Expr, arity: usize) -> bool {
    match expr {
        Expr::Binary(b) => match &b.op {
            BinOp::And(_) => accepts(&b.left, arity) && accepts(&b.right, arity),
            BinOp::Or(_) => accepts(&b.left, arity) || accepts(&b.right, arity),
            BinOp::Eq(_) => {
                number(&b.left, arity).expect("arity guard operand")
                    == number(&b.right, arity).expect("arity guard operand")
            }
            _ => panic!("unhandled arity guard"),
        },
        Expr::MethodCall(m) if m.method == "contains" => {
            let Expr::Paren(p) = m.receiver.as_ref() else {
                panic!("range guard")
            };
            let Expr::Range(r) = p.expr.as_ref() else {
                panic!("range guard")
            };
            let start = number(r.start.as_ref().unwrap(), arity).unwrap();
            let end = number(r.end.as_ref().unwrap(), arity).unwrap();
            arity >= start
                && if matches!(r.limits, syn::RangeLimits::Closed(_)) {
                    arity <= end
                } else {
                    arity < end
                }
        }
        // This predicate restricts accepted value types, not arity.
        Expr::Call(_) => true,
        _ => panic!("unhandled arity guard"),
    }
}
#[derive(Default)]
struct Dispatch(BTreeMap<String, BTreeSet<usize>>);
impl<'ast> Visit<'ast> for Dispatch {
    fn visit_expr_match(&mut self, node: &'ast syn::ExprMatch) {
        if matches!(node.expr.as_ref(), Expr::Path(p) if p.path.is_ident("function_name")) {
            for arm in &node.arms {
                let mut ns = vec![];
                names(&arm.pat, &mut ns);
                for name in ns {
                    struct GuardNumbers(usize);
                    impl<'a> Visit<'a> for GuardNumbers {
                        fn visit_lit_int(&mut self, value: &'a syn::LitInt) {
                            self.0 = self.0.max(value.base10_parse::<usize>().unwrap());
                        }
                    }
                    let arities: Vec<_> = if let Some((_, guard)) = &arm.guard {
                        let mut max = GuardNumbers(0);
                        max.visit_expr(guard);
                        (0..=max.0).filter(|n| accepts(guard, *n)).collect()
                    } else {
                        // Unguarded variadic dispatch delegates to a helper whose
                        // slice patterns are the argument contract (currently JWT).
                        struct HelperArities(BTreeSet<usize>);
                        impl<'a> Visit<'a> for HelperArities {
                            fn visit_pat_slice(&mut self, p: &'a syn::PatSlice) {
                                if !p.elems.iter().any(|p| matches!(p, Pat::Rest(_))) {
                                    self.0.insert(p.elems.len());
                                }
                            }
                        }
                        struct Calls(Vec<(String, String)>);
                        impl<'a> Visit<'a> for Calls {
                            fn visit_expr_call(&mut self, c: &'a syn::ExprCall) {
                                if let Expr::Path(p) = c.func.as_ref() {
                                    let names: Vec<_> = p
                                        .path
                                        .segments
                                        .iter()
                                        .map(|s| s.ident.to_string())
                                        .collect();
                                    if names.len() == 3 && names[0] == "crate" {
                                        self.0.push((names[1].clone(), names[2].clone()));
                                    }
                                }
                                syn::visit::visit_expr_call(self, c);
                            }
                        }
                        let mut calls = Calls(vec![]);
                        calls.visit_expr(&arm.body);
                        let mut found = HelperArities(BTreeSet::new());
                        for (module, function) in calls.0 {
                            let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                                .join("src")
                                .join(format!("{module}.rs"));
                            if let Ok(source) = std::fs::read_to_string(path) {
                                for item in syn::parse_file(&source).unwrap().items {
                                    if let Item::Fn(f) = item {
                                        if f.sig.ident == function {
                                            found.visit_block(&f.block);
                                        }
                                    }
                                }
                            }
                        }
                        assert!(!found.0.is_empty(), "builtin {name} needs an explicit arity guard or a helper argument-pattern contract");
                        found.0.into_iter().collect()
                    };
                    self.0.entry(name).or_default().extend(arities);
                }
            }
            // Only the dispatch match, not inner switches on the chosen name.
        } else {
            syn::visit::visit_expr_match(self, node);
        }
    }
}
pub fn extract(calls: &str, operators: &str) -> BTreeMap<String, BTreeSet<usize>> {
    let mut dispatch = Dispatch::default();
    let file = syn::parse_file(calls).unwrap();
    for item in file.items {
        if let Item::Fn(f) = item {
            if f.sig.ident == "evaluate_function_call" {
                dispatch.visit_block(&f.block);
            }
        }
    }
    let file = syn::parse_file(operators).unwrap();
    for item in file.items {
        if let Item::Const(c) = item {
            if c.ident == "BINARY_BUILTIN_OPERATORS" || c.ident == "COLLECTION_OPERATORS" {
                let Expr::Reference(r) = c.expr.as_ref() else {
                    panic!("operator list")
                };
                let Expr::Array(a) = r.expr.as_ref() else {
                    panic!("operator list")
                };
                for e in &a.elems {
                    let Expr::Lit(e) = e else {
                        panic!("operator name")
                    };
                    let Lit::Str(s) = &e.lit else {
                        panic!("operator name")
                    };
                    dispatch.0.entry(s.value()).or_default().insert(2);
                }
            }
        }
    }
    dispatch.0
}

/// Operators handled by the expression evaluator do not pass through function
/// dispatch. Discover their keyword operands from the actual parser calls too.
pub fn keyword_operators(source: &str) -> BTreeSet<String> {
    #[derive(Default)]
    struct Operators(BTreeSet<String>);
    impl<'ast> Visit<'ast> for Operators {
        fn visit_expr_call(&mut self, call: &'ast syn::ExprCall) {
            if let Expr::Path(path) = call.func.as_ref() {
                let name = path.path.segments.last().unwrap().ident.to_string();
                if matches!(
                    name.as_str(),
                    "split_top_level_keyword"
                        | "split_top_level_keyword_operator"
                        | "split_top_level_keyword_or_call_operator"
                ) {
                    fn collect(expr: &Expr, names: &mut BTreeSet<String>) {
                        match expr {
                            Expr::Lit(lit) => {
                                if let Lit::Str(value) = &lit.lit {
                                    names.insert(value.value());
                                } else {
                                    panic!("keyword operator must be a string");
                                }
                            }
                            Expr::Reference(r) => collect(&r.expr, names),
                            Expr::Array(a) => {
                                for expr in &a.elems {
                                    collect(expr, names);
                                }
                            }
                            // Variable operands iterate the callable-operator
                            // lists, already discovered by `extract` above.
                            Expr::Path(_) => (),
                            _ => panic!("unhandled keyword operator declaration"),
                        }
                    }
                    collect(
                        call.args.iter().nth(1).expect("keyword operand"),
                        &mut self.0,
                    );
                }
            }
            syn::visit::visit_expr_call(self, call);
        }
    }
    let mut operators = Operators::default();
    for item in syn::parse_file(source).unwrap().items {
        if let Item::Fn(function) = item {
            if function.sig.ident == "evaluate_expression_scoped"
                || function.sig.ident == "evaluate_expression_inner"
            {
                operators.visit_block(&function.block);
            }
        }
    }
    operators.0
}

/// Read the existing runtime's Python literal export registry. This deliberately
/// lexes Python comments and triple/single/double quoted strings separately from
/// DataWeave, so examples and comments cannot become module declarations.
pub fn python_exports(source: &str) -> BTreeMap<String, BTreeSet<String>> {
    #[derive(Debug)]
    enum Token<'a> {
        Word(&'a str, usize),
        String(String),
        Symbol(u8),
    }
    let bytes = source.as_bytes();
    let mut tokens = vec![];
    let mut i = 0;
    while i < bytes.len() {
        let start = i;
        match bytes[i] {
            b'#' => {
                while i < bytes.len() && bytes[i] != b'\n' {
                    i += 1;
                }
            }
            b'\'' | b'"' => {
                let quote = bytes[i];
                let width = if bytes[i..].starts_with(&[quote, quote, quote]) {
                    3
                } else {
                    1
                };
                i += width;
                let content = i;
                while i < bytes.len()
                    && !bytes[i..]
                        .get(..width)
                        .is_some_and(|part| part.iter().all(|c| *c == quote))
                {
                    if bytes[i] == b'\\' {
                        i += 1;
                    }
                    i += 1;
                }
                let end = i.min(bytes.len());
                i = (i + width).min(bytes.len());
                if width == 1 {
                    let value = if quote == b'"' {
                        serde_json::from_str::<String>(&source[start..i]).ok()
                    } else {
                        Some(source[content..end].to_string())
                    };
                    if let Some(value) = value {
                        tokens.push(Token::String(value));
                    }
                }
            }
            c if c.is_ascii_alphabetic() || c == b'_' => {
                i += 1;
                while i < bytes.len() && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
                    i += 1;
                }
                tokens.push(Token::Word(&source[start..i], start));
            }
            c if c.is_ascii_whitespace() => i += 1,
            c => {
                tokens.push(Token::Symbol(c));
                i += 1;
            }
        }
    }
    let registry = tokens.iter().position(|t| matches!(t, Token::Word("MODULE_EXPORTS", start) if *start == 0 || bytes[*start - 1] == b'\n'))
        .expect("runtime MODULE_EXPORTS assignment");
    let assignment = tokens
        .iter()
        .enumerate()
        .skip(registry)
        .find(|(_, t)| matches!(t, Token::Symbol(b'=')))
        .unwrap()
        .0;
    let mut cursor = assignment + 1;
    assert!(
        matches!(tokens.get(cursor), Some(Token::Symbol(b'{'))),
        "MODULE_EXPORTS must be a literal registry"
    );
    cursor += 1;
    let mut result = BTreeMap::new();
    while !matches!(tokens.get(cursor), Some(Token::Symbol(b'}'))) {
        if matches!(tokens.get(cursor), Some(Token::Symbol(b','))) {
            cursor += 1;
            continue;
        }
        let Some(Token::String(module)) = tokens.get(cursor) else {
            panic!("module export name must be a string literal")
        };
        let module = module.clone();
        cursor += 1;
        assert!(matches!(tokens.get(cursor), Some(Token::Symbol(b':'))));
        cursor += 1;
        assert!(matches!(tokens.get(cursor), Some(Token::Symbol(b'['))));
        cursor += 1;
        let mut names = BTreeSet::new();
        while !matches!(tokens.get(cursor), Some(Token::Symbol(b']'))) {
            match tokens.get(cursor) {
                Some(Token::String(name)) => {
                    names.insert(name.clone());
                }
                Some(Token::Symbol(b',')) => (),
                _ => panic!("module export names must be string literals"),
            }
            cursor += 1;
        }
        result.insert(module, names);
        cursor += 1;
    }
    result
}
