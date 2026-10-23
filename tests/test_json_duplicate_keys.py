import json

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
