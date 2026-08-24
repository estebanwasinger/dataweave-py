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
