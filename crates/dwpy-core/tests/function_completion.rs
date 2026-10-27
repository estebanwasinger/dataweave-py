use dwpy_core::{
    completion::{complete, complete_with_modules, CompletionItem},
    function_catalog,
};
use std::collections::{BTreeMap, BTreeSet};
#[allow(dead_code)]
#[path = "../catalog_source.rs"]
mod catalog_source;
fn at(source: &str) -> Vec<CompletionItem> {
    let offset = source.find("<cursor>").unwrap();
    let line = source[..offset].bytes().filter(|b| *b == b'\n').count();
    let column = source[..offset]
        .rsplit('\n')
        .next()
        .unwrap()
        .encode_utf16()
        .count();
    complete(&source.replace("<cursor>", ""), line, column)
}
fn named<'a>(items: &'a [CompletionItem], name: &str) -> Vec<&'a CompletionItem> {
    items.iter().filter(|i| i.label == name).collect()
}
#[test]
fn invocation_syntax_follows_expression_position() {
    for source in [
        "{name: up<cursor>}",
        "{name:up<cursor>}",
        "[up<cursor>]",
        "if (true) up<cursor> else null",
        "null default up<cursor>",
        "payload map up<cursor>",
        "payload + up<cursor>",
        "((item) -> up<cursor>)",
    ] {
        let items = at(source);
        assert!(
            named(&items, "upper")
                .iter()
                .all(|i| i.insert_text.starts_with("upper(")),
            "{source}"
        );
        assert!(!named(&items, "upper").is_empty(), "{source}");
    }
    for source in [
        "ma<cursor>",
        "{values: ma<cursor>}",
        "payload map ma<cursor>",
        "if (true) ma<cursor> else []",
    ] {
        let items = at(source);
        assert!(
            named(&items, "map")
                .iter()
                .all(|i| i.insert_text.starts_with("map(")),
            "{source}"
        );
    }
    for source in [
        "{values: array ma<cursor>}",
        "payload.items ma<cursor>",
        "[1, 2] ma<cursor>",
        "({x: 1}) ma<cursor>",
        "getItems() ma<cursor>",
        "payload /* ignored: + */\n ma<cursor>",
        "payload.map ma<cursor>",
        "payload map ((item) -> item.children ma<cursor>)",
    ] {
        let items = at(source);
        assert!(
            named(&items, "map")
                .iter()
                .all(|i| i.insert_text == "map ((item, index) -> ${1})"),
            "{source}"
        );
        assert_eq!(
            items[0].label, "map",
            "infix overloads should rank first: {source}"
        );
    }
    let items = at("[1,2] jo<cursor>");
    assert!(named(&items, "joinBy")
        .iter()
        .all(|i| i.insert_text == "joinBy ${1:separator}"));
    assert!(named(&at("2 po<cursor>"), "pow")
        .iter()
        .all(|i| i.insert_text.starts_with("pow ${1:")));
    assert!(named(&at("payload th<cursor>"), "then")
        .iter()
        .all(|i| i.insert_text.contains(") -> ${1})")));
    for source in [
        "payload ma<cursor> ((item) -> item.name)",
        "payload ma<cursor> $.name",
        "ma<cursor>(payload, $)",
    ] {
        assert!(
            named(&at(source), "map")
                .iter()
                .all(|i| i.insert_text == "map" && i.insert_text_format == "plain"),
            "{source}"
        );
    }
}

#[test]
fn imported_infix_aliases_preserve_style_but_local_functions_do_not_inherit_it() {
    let items = at("%dw 2.0\nvar each = map\n---\npayload ea<cursor>");
    assert!(named(&items, "each")
        .iter()
        .all(|i| i.insert_text.starts_with("each(")));
    let items = at("%dw 2.0\nimport joinBy as join from dw::Core\n---\npayload joi<cursor>");
    assert!(named(&items, "join")
        .iter()
        .all(|i| i.insert_text.starts_with("join(")));
    let items = at("%dw 2.0\nimport map as each from dw::Core\n---\npayload ea<cursor>");
    assert!(named(&items, "each")
        .iter()
        .all(|i| i.insert_text == "each ((item, index) -> ${1})"));
    assert!(!named(&items, "each").is_empty());
    let items = at("%dw 2.0\nimport map as each from dw::Core\n---\npayload each ma<cursor>");
    assert!(named(&items, "map")
        .iter()
        .all(|i| i.insert_text.starts_with("map(")));
    let items = at("%dw 2.0\nimport dw::Core as C\n---\npayload C::ma<cursor>");
    assert!(named(&items, "C::map")
        .iter()
        .all(|i| i.insert_text.starts_with("C::map ((")));
    let items = at("%dw 2.0\nfun map(a, b) = a + b\n---\npayload ma<cursor>");
    assert_eq!(named(&items, "map")[0].insert_text, "map(${1:a}, ${2:b})");
}
#[test]
fn every_runtime_name_and_arity_is_discovered_without_a_catalog_update() {
    let expected = catalog_source::extract(
        include_str!("../src/calls.rs"),
        include_str!("../src/lib.rs"),
    );
    let actual: BTreeMap<_, BTreeSet<_>> = function_catalog::functions()
        .iter()
        .map(|f| (f.name.clone(), f.arities.iter().copied().collect()))
        .collect();
    assert_eq!(expected, actual);
    let items = complete("", 0, 0);
    for f in function_catalog::functions() {
        let offered = named(&items, &f.name);
        assert!(!offered.is_empty(), "{}", f.name);
        assert_eq!(
            f.signatures
                .iter()
                .map(|s| s.parameters.len())
                .collect::<BTreeSet<_>>(),
            f.arities.iter().copied().collect()
        );
        for item in offered {
            assert_eq!(item.kind, "function");
        }
    }
}
#[test]
fn a_new_dispatch_arm_is_discovered_automatically() {
    let source = r#"fn evaluate_function_call() { match function_name { "brandNewFunction" if arity == 13 => 42, _ => 0 } }"#;
    let discovered = catalog_source::extract(source, "");
    assert_eq!(discovered["brandNewFunction"], BTreeSet::from([13]));
}
#[test]
fn evaluator_operators_are_discovered_without_function_dispatch_entries() {
    let operators = catalog_source::keyword_operators(include_str!("../src/lib.rs"));
    assert!(operators.contains("update"));
    assert_eq!(
        operators,
        function_catalog::operators().iter().cloned().collect()
    );
    let added = catalog_source::keyword_operators(
        r#"fn evaluate_expression_scoped() {
        split_top_level_keyword(source, "brandNewOperator");
        split_top_level_keyword_or_call_operator(source, &["anotherOperator"]);
    }"#,
    );
    assert_eq!(
        added,
        BTreeSet::from(["brandNewOperator".into(), "anotherOperator".into()])
    );
    for source in [
        "%dw 2.0\n---\nup<cursor>",
        "%dw 2.0\n---\n{numA: 2} up<cursor>",
    ] {
        let items = at(source);
        let update = named(&items, "update");
        assert_eq!(update.len(), 1);
        assert_eq!(update[0].kind, "keyword");
        assert_eq!(update[0].insert_text, "update");
        assert_eq!(update[0].insert_text_format, "plain");
    }
    let items = at("%dw 2.0\nfun update(value) = value\n---\nup<cursor>");
    assert_eq!(named(&items, "update").len(), 1);
    assert_eq!(named(&items, "update")[0].kind, "function");
    assert!(named(&at("Text::up<cursor>"), "update").is_empty());
}
#[test]
fn imports_select_aliases_wildcards_and_namespaces_share_runtime_resolution() {
    for (script, label, expression, expected) in [
        (
            "%dw 2.0\nimport upper as shout from dw::core::Strings\n---\nsho<cursor>",
            "shout",
            r#"shout("hi")"#,
            serde_json::json!("HI"),
        ),
        (
            "%dw 2.0\nimport dw::core::Strings as Text\n---\nText::up<cursor>",
            "Text::upper",
            r#"Text::upper("hi")"#,
            serde_json::json!("HI"),
        ),
        (
            "%dw 2.0\nimport dw::util::Math\n---\nMath::si<cursor>",
            "Math::sin",
            "Math::sin(0)",
            serde_json::json!(0),
        ),
        (
            "%dw 2.0\nimport * from dw::core::Arrays\n---\nsumB<cursor>",
            "sumBy",
            "sumBy([1,2], (n) -> n)",
            serde_json::json!(3),
        ),
    ] {
        let items = at(script);
        assert!(!named(&items, label).is_empty(), "{script}");
        let source = script.split("---").next().unwrap().to_string() + "---\n" + expression;
        let result = dwpy_core::execute_smoke(&source, serde_json::Value::Null).unwrap();
        assert_eq!(result, expected, "{expression}");
    }
    assert!(!named(
        &at("dw::core::Strings::up<cursor>"),
        "dw::core::Strings::upper"
    )
    .is_empty());
}
#[test]
fn nested_functions_are_visible_only_in_their_lexical_scope() {
    let inside = at(
        "%dw 2.0\nfun outer(x) = do {\n fun inner(y) = y + x\n ---\n inn<cursor>\n}\n---\nouter(1)",
    );
    assert_eq!(named(&inside, "inner")[0].insert_text, "inner(${1:y})");
    let outside = at(
        "%dw 2.0\nfun outer(x) = do {\n fun inner(y) = y + x\n ---\n inner(1)\n}\n---\ninn<cursor>",
    );
    assert!(named(&outside, "inner").is_empty());
    let sibling = at("[do { fun privateFn(n) = n\n---\nprivateFn(1) }, do { ---\npri<cursor> }]");
    assert!(named(&sibling, "privateFn").is_empty());
}
#[test]
fn closures_recursion_overloads_and_incomplete_do_blocks_remain_available() {
    let items = at("%dw 2.0\nfun helper(a,b) = a + b\n---\ndo {\n fun helper(n: Number) = helper(n - 1)\n fun helper(s: String) = s\n ---\n do { --- hel<cursor>");
    let helpers = named(&items, "helper");
    assert_eq!(helpers.len(), 2);
    assert!(helpers
        .iter()
        .all(|i| i.insert_text == "helper(${1:n})" || i.insert_text == "helper(${1:s})"));
}
#[test]
fn function_valued_variables_and_aliases_are_suggested_without_execution() {
    let items = at("%dw 2.0\nvar double = ((n) -> n * 2)\nvar twice = double\nvar message = log(\"do not execute\")\n---\ntwi<cursor>");
    assert_eq!(named(&items, "twice")[0].insert_text, "twice(${1:n})");
    let items = at("%dw 2.0\n---\npayload map ((item) -> do {\n var apply = (value, factor = 2) -> value * factor\n ---\n app<cursor>\n})");
    assert_eq!(
        named(&items, "apply")[0].insert_text,
        "apply(${1:value}, ${2:factor})"
    );
}
#[test]
fn parameters_shadow_functions_and_do_not_leak_to_siblings_or_signature_types() {
    let items = at("%dw 2.0\nfun f(map: (Number) -> Number) = ma<cursor>\n---\nnull");
    assert_eq!(named(&items, "map").len(), 1);
    assert_eq!(named(&items, "map")[0].insert_text, "map(${1:value})");
    let items = at("%dw 2.0\n---\n[(privateCallback) -> privateCallback, pri<cursor>]");
    assert!(named(&items, "privateCallback").is_empty());
    let items = at("%dw 2.0\nfun f(x): (T) -> Number = (n) -> n\n---\nT<cursor>");
    assert!(named(&items, "T").is_empty());
}
#[test]
fn comments_literals_and_unicode_do_not_invent_or_hide_scope_bindings() {
    let items = at("%dw 2.0\n// fun fake(n) = n\nvar text = \"fun alsoFake(n) = n\"\n---\n{ emoji: \"😀\", value: do {\n fun réel(n) = n\n fun real(n) = n\n ---\n rea<cursor>\n}}");
    assert_eq!(named(&items, "real").len(), 1);
    assert!(named(&items, "fake").is_empty());
    let items = at("%dw 2.0\n---\n\"Result: $(do { fun nested(n) = n\n---\nnes<cursor>})\"");
    assert_eq!(named(&items, "nested").len(), 1);
}
#[test]
fn imported_module_edits_are_discovered_on_each_request() {
    let script = "%dw 2.0\nimport * from custom::Tools\n---\nnew";
    let mut modules = BTreeMap::from([(
        "custom::Tools".into(),
        "%dw 2.0\nfun oldFunction(n) = n\n---\nnull".into(),
    )]);
    assert!(named(
        &complete_with_modules(script, 3, 3, &modules),
        "newFunction"
    )
    .is_empty());
    modules.insert(
        "custom::Tools".into(),
        "%dw 2.0\nfun newFunction(first, second) = first\n---\nnull".into(),
    );
    assert_eq!(
        named(
            &complete_with_modules(script, 3, 3, &modules),
            "newFunction"
        )[0]
        .insert_text,
        "newFunction(${1:first}, ${2:second})"
    );
}

#[test]
fn header_lambdas_and_using_bindings_end_with_their_expression() {
    let outside =
        at("%dw 2.0\nvar transform = (privateParameter) -> privateParameter\n---\npri<cursor>");
    assert!(named(&outside, "privateParameter").is_empty());
    let inside = at("%dw 2.0\nvar data = using (converter = (n) -> n * 2) conv<cursor>\n---\nnull");
    assert_eq!(
        named(&inside, "converter")[0].insert_text,
        "converter(${1:n})"
    );
    let outside =
        at("%dw 2.0\nvar data = using (converter = (n) -> n * 2) converter(4)\n---\nconv<cursor>");
    assert!(named(&outside, "converter").is_empty());
    let outside = at("%dw 2.0\ntype Callback = (TypeParameter) -> Number\nvar callback: (OtherType) -> Number = (n) -> n\n---\nType<cursor>");
    assert!(named(&outside, "TypeParameter").is_empty());
}

#[test]
fn runtime_module_registry_discovery_ignores_python_comments_and_examples() {
    let source = r###"
# MODULE_EXPORTS: Dict = {"fake": ["comment"]}
DOC = """
MODULE_EXPORTS = {"fake": ["example"]}
"""
MODULE_EXPORTS: Dict[str, List[str]] = {
    'custom::Tools': ['freshFunction', "otherFunction"], # new exports are live inputs
}
"###;
    let modules = catalog_source::python_exports(source);
    assert_eq!(modules.len(), 1);
    assert_eq!(
        modules["custom::Tools"],
        BTreeSet::from(["freshFunction".into(), "otherFunction".into()])
    );
}
