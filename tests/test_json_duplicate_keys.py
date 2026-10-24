import json
import xml.etree.ElementTree as ET

import pytest

from dwpy.formats import DWObject, FormatRegistry
from dwpy.runtime import DataWeaveEvaluationError, DataWeaveRuntime

PAYLOAD = '''{
  "test": {
    "test2": {"array": 1},
    "test2": {"array": 2},
    "test2": {"array": 3}
  }
}'''
EXPECTED = [{"array": 1}, {"array": 2}, {"array": 3}]


@pytest.fixture(params=["python", "rust"])
def runtime(request):
    return DataWeaveRuntime(backend=request.param)


def test_json_payload_multi_value_selector_preserves_duplicate_keys(runtime):
    result = runtime.execute(
        "%dw 2.0\noutput application/json\n---\npayload.test.*test2",
        PAYLOAD,
        payload_format="application/json",
    )
    assert json.loads(result) == EXPECTED


def test_read_json_preserves_duplicate_keys_in_nested_expression(runtime):
    result = runtime.execute(
        '''%dw 2.0
output application/json
---
(read(payload, "application/json").test.*test2) map $.array
''',
        PAYLOAD,
    )
    assert json.loads(result) == [1, 2, 3]


def test_single_value_selector_uses_first_duplicate_entry(runtime):
    assert runtime.execute(
        "payload.test.test2", PAYLOAD, payload_format="json"
    ) == {"array": 1}


def test_json_numeric_values_still_decode_as_numbers(runtime):
    assert runtime.execute(
        "payload.*a", '{"a":1.25,"a":-2.5,"a":1e3}', payload_format="json"
    ) == [1.25, -2.5, 1000]


def test_json_duplicates_inside_arrays_preserve_nulls_arrays_and_key_order(runtime):
    result = runtime.execute(
        '''%dw 2.0
output application/json
---
payload map ((item) -> item.*a)
''',
        '[{"a":null,"other":0,"a":[1,2],"\\u0061":false}, {"a":"last"}]',
        payload_format="json",
    )
    assert json.loads(result) == [[None, [1, 2], False], ["last"]]


def test_duplicate_json_input_round_trips_without_losing_entries(runtime):
    result = runtime.execute(
        "%dw 2.0\noutput application/json\n---\npayload",
        PAYLOAD,
        payload_format="json",
    )
    assert json.loads(result, object_pairs_hook=list) == json.loads(
        PAYLOAD, object_pairs_hook=list
    )


@pytest.mark.parametrize("expression", ["payload.test", "[payload.test]"])
@pytest.mark.parametrize("options", ["", "indent=2", "indent=4 sort_keys=true"])
def test_parent_object_output_preserves_duplicate_fields(runtime, expression, options):
    result = runtime.execute(
        f"%dw 2.0\noutput application/json {options}\n---\n{expression}",
        PAYLOAD, payload_format="application/json",
    )
    expected = '{"test2":{"array":1},"test2":{"array":2},"test2":{"array":3}}'
    if expression.startswith("["):
        expected = f"[{expected}]"
    assert json.loads(result, object_pairs_hook=list) == json.loads(
        expected, object_pairs_hook=list
    )


def test_duplicate_json_keys_render_as_repeated_xml_elements(runtime):
    result = runtime.execute(
        "%dw 2.0\noutput application/xml\n---\n{root: payload.test}",
        PAYLOAD,
        payload_format="json",
    )
    assert ET.tostring(ET.fromstring(result), encoding="unicode") == (
        "<root><test2><array>1</array></test2>"
        "<test2><array>2</array></test2>"
        "<test2><array>3</array></test2></root>"
    )


def test_rust_xml_writer_supports_implicit_root_object():
    runtime = DataWeaveRuntime(backend="rust")
    result = runtime.execute(
        "%dw 2.0\noutput application/xml\n---\nroot: payload.test",
        PAYLOAD,
        payload_format="json",
    )
    assert result == (
        "<root><test2><array>1</array></test2>"
        "<test2><array>2</array></test2>"
        "<test2><array>3</array></test2></root>"
    )
    assert runtime._rust_runtime is not None
    assert runtime._rust_runtime.last_execution_engine() == "rust-core"


@pytest.mark.parametrize(
    "expression",
    [
        '{root: {test2: {array: 1}, test2: {array: 2}, test2: {array: 3}}}',
        '{root: read(payload, "application/json").test}',
    ],
)
def test_duplicate_keys_render_as_xml_from_literals_and_read(runtime, expression):
    result = runtime.execute(
        f"%dw 2.0\noutput application/xml\n---\n{expression}", PAYLOAD
    )
    root = ET.fromstring(result)
    assert root.tag == "root"
    assert [child.tag for child in root] == ["test2"] * 3
    assert [child.findtext("array") for child in root] == ["1", "2", "3"]


def test_duplicate_xml_children_preserve_order_nested_values_and_attributes(runtime):
    result = runtime.execute(
        "%dw 2.0\noutput application/xml\n---\n{root: payload}",
        '''{"@id":"a&b", "item":{"@code":"x\\\"y", "#text":"<one>"},
            "between":null, "item":{"part":1,"part":2}, "last":true}''',
        payload_format="json",
    )
    root = ET.fromstring(result)
    assert root.attrib == {"id": "a&b"}
    assert [child.tag for child in root] == ["item", "between", "item", "last"]
    assert root[0].attrib == {"code": 'x"y'}
    assert root[0].text == "<one>"
    assert root[1].text is None
    assert [(child.tag, child.text) for child in root[2]] == [("part", "1"), ("part", "2")]
    assert root[3].text is not None
    assert root[3].text.lower() == "true"


@pytest.mark.parametrize("directive", ["application/xml", 'application/xml root="root"'])
def test_duplicate_xml_output_at_document_level_uses_wrapper(runtime, directive):
    result = runtime.execute(
        f"%dw 2.0\noutput {directive}\n---\npayload.test",
        PAYLOAD,
        payload_format="json",
    )
    root = ET.fromstring(result)
    assert root.tag == "root"
    assert [child.tag for child in root] == ["test2"] * 3
    assert [child.findtext("array") for child in root] == ["1", "2", "3"]


def test_preparsed_duplicate_json_object_survives_runtime_bridge(runtime):
    payload = FormatRegistry.read(PAYLOAD, "json", {})
    assert isinstance(payload["test"], DWObject)
    assert runtime.execute("payload.test.*test2", payload) == EXPECTED


def test_ndjson_uses_same_duplicate_preserving_reader(runtime):
    result = runtime.execute(
        "payload map ((item) -> item.*a)",
        '{"a":1,"a":2}\n{"a":3,"a":4}',
        payload_format="application/x-ndjson",
    )
    assert result == [[1, 2], [3, 4]]


@pytest.mark.parametrize("payload", ['{"a":1,"a":}', '{"a":1,"a":2} trailing'])
def test_duplicate_preserving_reader_still_rejects_invalid_json(runtime, payload):
    with pytest.raises(DataWeaveEvaluationError, match="JSON|json"):
        runtime.execute("payload.*a", payload, payload_format="json")
