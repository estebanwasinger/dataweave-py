from __future__ import annotations

import json
from pathlib import Path

from dwpy.lsp.engine import DataWeaveLanguageEngine


def _cursor(script_with_marker: str) -> tuple[str, int, int]:
    marker = "<cursor>"
    offset = script_with_marker.index(marker)
    script = script_with_marker.replace(marker, "")
    line = script[:offset].count("\n")
    line_start = script.rfind("\n", 0, offset)
    column = offset if line_start == -1 else offset - line_start - 1
    return script, line, column


def _labels(items: list) -> set[str]:
    return {item.label for item in items}


def test_completion_includes_local_imported_and_builtin_symbols() -> None:
    engine = DataWeaveLanguageEngine()
    script, line, column = _cursor(
        """%dw 2.0
import * from dw::core::Strings
fun localName(value) = value
---
<cursor>
"""
    )
    items = engine.complete(
        script=script,
        line=line,
        column=column,
        payload={"name": "mule"},
        vars={"source": "test"},
    )
    labels = _labels(items)

    assert "localName" in labels
    assert "camelize" in labels
    assert "upper" in labels
    assert "payload" in labels
    assert "vars" in labels


def test_completion_includes_attributes_and_nested_fields() -> None:
    engine = DataWeaveLanguageEngine()
    script, line, column = _cursor("attributes.<cursor>")
    items = engine.complete(
        script=script,
        line=line,
        column=column,
        attributes={"headers": {"x-request-id": "abc"}, "queryParams": {"page": "1"}},
    )
    assert {item.label for item in items} == {"headers", "queryParams"}

    nested_script, line, column = _cursor("attributes.headers.<cursor>")
    nested_items = engine.complete(
        script=nested_script,
        line=line,
        column=column,
        attributes={"headers": {"x-request-id": "abc"}},
    )
    assert {item.label for item in nested_items} == {"x-request-id"}


def test_property_signature_help_and_property_key_completion() -> None:
    engine = DataWeaveLanguageEngine()
    script, line, column = _cursor('p("<cursor>')
    items = engine.complete(
        script=script,
        line=line,
        column=column,
        properties={"env": "dev", "app.config.value": "yes"},
    )
    assert {item.label for item in items} == {"env", "app.config.value"}

    signature = engine.signature_help(
        script='Mule::p("',
        line=0,
        column=len('Mule::p("'),
        properties={"env": "dev"},
    )
    assert signature is not None
    assert signature.signatures[0].label == "p(propertyName) -> String | Null"


def test_context_sidecars_load_independently_and_ignore_invalid_files(tmp_path: Path) -> None:
    script_path = tmp_path / "transform.dwl"
    script_path.write_text("%dw 2.0\n---\nattributes.headers", encoding="utf-8")
    script_path.with_name("transform.dwl.attributes.json").write_text(
        '{"headers":{"requestId":"abc"}}', encoding="utf-8"
    )
    script_path.with_name("transform.dwl.properties.json").write_text(
        '{"env":"dev"}', encoding="utf-8"
    )
    engine = DataWeaveLanguageEngine()
    script, line, column = _cursor("attributes.headers.<cursor>")
    assert {
        item.label
        for item in engine.complete(
            script=script,
            line=line,
            column=column,
            document_path=str(script_path),
        )
    } == {"requestId"}

    script_path.with_name("transform.dwl.attributes.json").write_text(
        "not-json", encoding="utf-8"
    )
    script_path.with_name("transform.dwl.vars.json").write_text(
        '{"name":"Ana"}', encoding="utf-8"
    )
    fallback_script, line, column = _cursor("vars.<cursor>")
    assert {
        item.label
        for item in engine.complete(
            script=fallback_script,
            line=line,
            column=column,
            document_path=str(script_path),
        )
    } == {"name"}


def test_completion_uses_lambda_snippet_for_group_by() -> None:
    engine = DataWeaveLanguageEngine()
    script, line, column = _cursor(
        """%dw 2.0
---
payload group<cursor>
"""
    )
    items = engine.complete(
        script=script,
        line=line,
        column=column,
        payload=[{"name": "mule"}],
        vars={},
    )
    group_by = next(item for item in items if item.label == "groupBy")
    assert group_by.insert_text == "groupBy ((value, key) -> ${1})"
    assert group_by.insert_text_format == "snippet"


def test_completion_uses_lambda_snippets_for_higher_order_operators() -> None:
    engine = DataWeaveLanguageEngine()
    script, line, column = _cursor(
        """%dw 2.0
---
payload <cursor>
"""
    )
    items = engine.complete(
        script=script,
        line=line,
        column=column,
        payload=[{"name": "mule"}],
        vars={},
    )
    by_label = {item.label: item for item in items}

    assert by_label["map"].insert_text == "map ((value, index) -> ${1})"
    assert by_label["filter"].insert_text == "filter ((value, index) -> ${1})"
    assert by_label["reduce"].insert_text == "reduce ((value, accumulator = ${1}) -> ${2})"
    assert by_label["orderBy"].insert_text == "orderBy ((value, index) -> ${1})"
    assert by_label["groupBy"].insert_text == "groupBy ((value, key) -> ${1})"
    assert by_label["map"].insert_text_format == "snippet"
    assert by_label["reduce"].insert_text_format == "snippet"


def test_completion_suggests_payload_fields() -> None:
    engine = DataWeaveLanguageEngine()
    script, line, column = _cursor(
        """%dw 2.0
---
payload.user.<cursor>
"""
    )
    items = engine.complete(
        script=script,
        line=line,
        column=column,
        payload={"user": {"name": "Mule", "age": 12}},
        vars={},
    )
    labels = _labels(items)
    assert "name" in labels
    assert "age" in labels


def test_completion_suggests_array_member_functions_for_array_field() -> None:
    engine = DataWeaveLanguageEngine()
    script, line, column = _cursor(
        """%dw 2.0
---
payload.defaultRoles.<cursor>
"""
    )
    items = engine.complete(
        script=script,
        line=line,
        column=column,
        payload={"defaultRoles": ["role-a", "role-b"]},
        vars={},
    )
    labels = _labels(items)
    assert "filter" in labels
    assert "joinBy" in labels


def test_completion_suggests_string_member_functions_for_array_index() -> None:
    engine = DataWeaveLanguageEngine()
    script, line, column = _cursor(
        """%dw 2.0
---
payload.defaultRoles[0].<cursor>
"""
    )
    items = engine.complete(
        script=script,
        line=line,
        column=column,
        payload={"defaultRoles": ["role-a", "role-b"]},
        vars={},
    )
    labels = _labels(items)
    assert "upper" in labels
    assert "trim" in labels


def test_completion_uses_sidecar_payload_and_vars(tmp_path: Path) -> None:
    engine = DataWeaveLanguageEngine()
    script_with_cursor = """%dw 2.0
---
payload.customer.<cursor>
"""
    script, line, column = _cursor(script_with_cursor)

    script_path = tmp_path / "sample.dwl"
    script_path.write_text(script, encoding="utf-8")
    (tmp_path / "sample.dwl.payload.json").write_text(
        json.dumps({"customer": {"id": "c-1", "tier": "gold"}}), encoding="utf-8"
    )
    (tmp_path / "sample.dwl.vars.json").write_text(
        json.dumps({"tenant": {"region": "us-east"}}), encoding="utf-8"
    )

    items = engine.complete(
        script=script,
        line=line,
        column=column,
        document_path=str(script_path),
    )
    labels = _labels(items)
    assert "id" in labels
    assert "tier" in labels


def test_completion_infers_lambda_parameter_structure() -> None:
    engine = DataWeaveLanguageEngine()
    script, line, column = _cursor(
        """%dw 2.0
---
payload.items map ((item, index) -> item.<cursor>)
"""
    )
    items = engine.complete(
        script=script,
        line=line,
        column=column,
        payload={"items": [{"name": "foo", "price": 10}]},
        vars={},
    )
    labels = _labels(items)
    assert "name" in labels
    assert "price" in labels


def test_completion_infers_array_element_type_inside_lambda() -> None:
    engine = DataWeaveLanguageEngine()
    script, line, column = _cursor(
        """%dw 2.0
---
payload.defaultRoles map ((item) -> item.<cursor>)
"""
    )
    items = engine.complete(
        script=script,
        line=line,
        column=column,
        payload={"defaultRoles": ["role-a", "role-b"]},
        vars={},
    )
    labels = _labels(items)
    assert "upper" in labels
    assert "trim" in labels


def test_hover_reports_property_chain_type() -> None:
    engine = DataWeaveLanguageEngine()
    script, line, column = _cursor(
        """%dw 2.0
---
payload.user.<cursor>name
"""
    )
    hover = engine.hover(
        script=script,
        line=line,
        column=column,
        payload={"user": {"name": "mule"}},
        vars={},
    )
    assert hover is not None
    assert "payload.user.name" in hover.contents
    assert "String" in hover.contents


def test_lsp_field_resolution_can_use_rust_type_descriptors(monkeypatch) -> None:
    import dwpy.lsp.engine as lsp_engine

    def fail_python_property_resolution(*args, **kwargs):  # noqa: ANN002, ANN003
        raise AssertionError("Python property resolver should not be needed")

    monkeypatch.setattr(lsp_engine, "_resolve_property_type", fail_python_property_resolution)

    engine = DataWeaveLanguageEngine()
    script, line, column = _cursor(
        """%dw 2.0
---
payload.user.<cursor>
"""
    )
    items = engine.complete(
        script=script,
        line=line,
        column=column,
        payload={"user": {"name": "mule", "age": 12}},
        vars={},
    )

    labels = _labels(items)
    assert "name" in labels
    assert "age" in labels


def test_lsp_index_resolution_can_use_rust_type_descriptors(monkeypatch) -> None:
    import dwpy.lsp.engine as lsp_engine

    def fail_python_index_resolution(*args, **kwargs):  # noqa: ANN002, ANN003
        raise AssertionError("Python index resolver should not be needed")

    monkeypatch.setattr(lsp_engine, "_resolve_index_type", fail_python_index_resolution)

    engine = DataWeaveLanguageEngine()
    script, line, column = _cursor(
        """%dw 2.0
---
payload.items[0].<cursor>
"""
    )
    items = engine.complete(
        script=script,
        line=line,
        column=column,
        payload={"items": [{"name": "mule", "price": 12}]},
        vars={},
    )

    labels = _labels(items)
    assert "name" in labels
    assert "price" in labels


def test_hover_reports_function_signature() -> None:
    engine = DataWeaveLanguageEngine()
    script, line, column = _cursor(
        """%dw 2.0
---
<cursor>upper(payload.name)
"""
    )
    hover = engine.hover(script=script, line=line, column=column, payload={"name": "mule"}, vars={})
    assert hover is not None
    assert "upper(" in hover.contents


def test_signature_help_tracks_active_parameter() -> None:
    engine = DataWeaveLanguageEngine()
    script, line, column = _cursor(
        """%dw 2.0
---
replaceAll(payload.text, "a", <cursor>)
"""
    )
    signature_help = engine.signature_help(
        script=script,
        line=line,
        column=column,
        payload={"text": "banana"},
        vars={},
    )
    assert signature_help is not None
    assert signature_help.active_parameter == 2
    assert signature_help.signatures[0].label.startswith("replaceAll(")


def test_completion_is_resilient_to_incomplete_scripts() -> None:
    engine = DataWeaveLanguageEngine()
    script, line, column = _cursor(
        """%dw 2.0
var user = payload.customer
---
user.<cursor>
{ invalid
"""
    )
    items = engine.complete(
        script=script,
        line=line,
        column=column,
        payload={"customer": {"id": "1"}},
        vars={},
    )
    assert isinstance(items, list)


def test_completion_preserves_typed_and_arity_overloads_with_valid_snippets() -> None:
    engine = DataWeaveLanguageEngine()
    script, line, column = _cursor('''%dw 2.0
output application/json
fun multiply(number, multiplier : Number) = number * multiplier * 2
fun multiply(number, multiplier : String) = number * multiplier as Number
fun multiply(number) = multiply(number,number)
---
{ value: m<cursor>
''')
    items = [i for i in engine.complete(script=script, line=line, column=column) if i.label == "multiply"]
    assert len(items) == 3
    assert all(i.kind == "function" for i in items)
    assert {i.detail.replace(" : ", ": ") for i in items} == {"multiply(number)", "multiply(number, multiplier: Number)", "multiply(number, multiplier: String)"}
    assert {i.insert_text for i in items} == {"multiply(${1:number})", "multiply(${1:number}, ${2:multiplier})"}


def test_local_function_shadows_builtin_and_uses_its_own_signature() -> None:
    engine = DataWeaveLanguageEngine()
    script, line, column = _cursor("%dw 2.0\nfun map(custom: Number) = custom\n---\nma<cursor>")
    items = [i for i in engine.complete(script=script, line=line, column=column) if i.label == "map"]
    assert len(items) == 1
    assert items[0].insert_text == "map(${1:custom})"


def test_header_recovery_ignores_delimiters_in_comments_and_strings() -> None:
    script, line, column = _cursor('''%dw 2.0
/*
---
*/
var marker = "---"
fun multiply(number) = number
---
{ value: m<cursor>
''')
    items = DataWeaveLanguageEngine().complete(script=script, line=line, column=column)
    multiply = next(i for i in items if i.label == "multiply")
    assert multiply.kind == "function"
    assert multiply.insert_text == "multiply(${1:number})"


def test_completion_uses_utf16_columns_and_keeps_existing_call_arguments() -> None:
    script, line, column = _cursor('%dw 2.0\r\nfun multiply(number) = number\r\n---\r\n{emoji: "😀", result: mu<cursor>ltiply(4)}')
    # LSP positions count surrogate pairs as two UTF-16 units.
    column += 1
    items = DataWeaveLanguageEngine().complete(script=script, line=line, column=column)
    multiply = next(i for i in items if i.label == "multiply")
    assert multiply.insert_text == "multiply"
    assert multiply.insert_text_format == "plain"
    assert multiply.replacement_range == (line, column - 2, line, column + len("ltiply"))


def test_native_catalog_covers_every_runtime_function() -> None:
    from dwpy._dwpy_rust import function_catalog_json
    items = DataWeaveLanguageEngine().complete(script="", line=0, column=0)
    names = {i.label for i in items if i.kind == "function"}
    assert {f["name"] for f in json.loads(function_catalog_json())} <= names
    assert {"flatMap", "mapObject", "filterObject", "logInfo", "today"} <= names


def test_nested_functions_and_lambda_variables_are_resolved_live() -> None:
    engine = DataWeaveLanguageEngine()
    script, line, column = _cursor("%dw 2.0\n---\ndo {\n fun helper(first, second) = first\n ---\n hel<cursor> }")
    helper = next(i for i in engine.complete(script=script, line=line, column=column) if i.label == "helper")
    assert helper.insert_text == "helper(${1:first}, ${2:second})"
    script, line, column = _cursor("%dw 2.0\n---\n[do { fun helper(n) = n\n---\nhelper(1) }, hel<cursor>]")
    assert "helper" not in _labels(engine.complete(script=script, line=line, column=column))


def test_module_file_edits_and_added_functions_appear_without_restarting_engine(tmp_path: Path) -> None:
    module = tmp_path / "tools" / "Math.dwl"
    module.parent.mkdir()
    module.write_text("%dw 2.0\nfun oldFunction(n) = n\n---\nnull")
    engine = DataWeaveLanguageEngine(module_base_path=tmp_path)
    script, line, column = _cursor("%dw 2.0\nimport tools::Math as Tools\n---\nTools::new<cursor>")
    assert "Tools::newFunction" not in _labels(engine.complete(script=script, line=line, column=column))
    module.write_text("%dw 2.0\nfun newFunction(first, second) = first\n---\nnull")
    items = engine.complete(script=script, line=line, column=column)
    item = next(i for i in items if i.label == "Tools::newFunction")
    assert item.insert_text == "Tools::newFunction(${1:first}, ${2:second})"


def test_qualified_completion_replaces_namespace_and_preserves_arguments() -> None:
    script, line, column = _cursor('%dw 2.0\nimport dw::core::Strings as Text\n---\n{emoji: "😀", value: Text::up<cursor>per("hello")}')
    column += 1  # UTF-16 surrogate pair before the identifier.
    item = next(i for i in DataWeaveLanguageEngine().complete(
        script=script, line=line, column=column,
    ) if i.label == "Text::upper")
    assert item.replacement_range == (line, column - len("Text::up"), line, column + len("per"))
    assert item.insert_text == "Text::upper"
    assert item.insert_text_format == "plain"


def test_evaluator_operators_are_suggested_once_without_call_arguments() -> None:
    engine = DataWeaveLanguageEngine()
    for source, label in [("{numA: 2} up<cursor>", "update"), ("payload de<cursor>", "default")]:
        script, line, column = _cursor("%dw 2.0\n---\n" + source)
        items = [i for i in engine.complete(script=script, line=line, column=column) if i.label == label]
        assert len(items) == 1
        assert items[0].kind == "keyword"
        assert items[0].insert_text == label
        assert items[0].insert_text_format == "plain"


def test_implicit_callback_selectors_complete_payload_element_fields() -> None:
    engine = DataWeaveLanguageEngine()
    payload = [{"name": "Esteban", "lastName": "Wasinger"}, {"email": "e@example.test"}]
    for source in ["payload map $.<cursor>", "payload filter $.<cursor>", "payload map ((item) -> item.<cursor>)"]:
        script, line, column = _cursor("%dw 2.0\n---\n" + source)
        items = engine.complete(script=script, line=line, column=column, payload=payload)
        assert _labels(items) == {"name", "lastName", "email"}
        assert all(item.kind == "field" for item in items)
    script, line, column = _cursor("payload map $.la<cursor>stName")
    item = engine.complete(script=script, line=line, column=column, payload=payload)[0]
    assert item.insert_text == "lastName"
    assert item.replacement_range == (line, column - 2, line, column + len("stName"))


def test_nested_implicit_callbacks_rebind_dollar_and_refresh_after_input_changes() -> None:
    engine = DataWeaveLanguageEngine()
    script, line, column = _cursor("payload map ($.children map $.<cursor>)")
    items = engine.complete(script=script, line=line, column=column, payload=[{"parentOnly": 1, "children": [{"childOnly": 2}]}])
    assert _labels(items) == {"childOnly"}
    items = engine.complete(script=script, line=line, column=column, payload=[{"children": [{"newField": 2}]}])
    assert _labels(items) == {"newField"}
    for source in ["$.<cursor>", "payload map $$.<cursor>", "[payload map $.name, $.<cursor>]"]:
        script, line, column = _cursor(source)
        assert engine.complete(script=script, line=line, column=column, payload=[{"name": "E"}]) == []
