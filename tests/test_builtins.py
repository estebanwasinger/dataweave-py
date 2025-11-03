import json
import logging

import pytest

from dwpy import builtins
from dwpy.runtime import DataWeaveRuntime


def test_concat_operator_handles_arrays_and_objects():
    assert builtins.binary_concat([1, 2], [3]) == [1, 2, 3]
    assert builtins.binary_concat({"a": 1}, {"b": 2}) == {"a": 1, "b": 2}
    assert builtins.binary_concat("foo", "bar") == "foobar"


def test_diff_operator_removes_array_and_object_entries():
    assert builtins.binary_diff([1, 2, 3], [2]) == [1, 3]
    assert builtins.binary_diff({"a": 1, "b": 2}, {"b": 2}) == {"a": 1}


def test_numeric_builtins_behaviour():
    assert builtins.builtin_abs(-5) == 5
    assert builtins.builtin_avg([1, 2, 3]) == pytest.approx(2.0)
    assert builtins.builtin_ceil(2.1) == 3
    assert builtins.builtin_floor(2.9) == 2
    assert builtins.builtin_round(2.4) == 2
    assert builtins.builtin_sum([1, 2, 3]) == 6
    assert builtins.builtin_is_decimal(2.5)
    assert not builtins.builtin_is_decimal(2)
    assert builtins.builtin_is_integer(2.0)
    assert builtins.builtin_is_even(4)
    assert builtins.builtin_is_odd(3)
    assert builtins.builtin_is_leap_year("2020-01-01T00:00:00Z")
    assert builtins.builtin_is_leap_year("|2016-10-01|")
    assert not builtins.builtin_is_leap_year("2019-01-01")
    assert builtins.builtin_pow(2, 3) == 8
    assert builtins.builtin_mod(7, 4) == 3
    assert builtins.builtin_random_int(10) < 10
    assert 0 <= builtins.builtin_random() < 1
    assert builtins.builtin_days_between("2016-10-01T23:57:59-03:00", "2017-10-01T23:57:59-03:00") == 365


def test_string_helpers():
    assert builtins.builtin_contains("mule", "ul")
    assert builtins.builtin_startswith("mulesoft", "mule")
    assert builtins.builtin_endswith("mulesoft", "soft")
    assert builtins.builtin_trim("  hi  ") == "hi"
    assert builtins.CORE_FUNCTIONS["upper"]("dw") == "DW"
    assert builtins.builtin_lower("DW") == "dw"
    assert builtins.builtin_is_blank("   ")
    assert not builtins.builtin_is_blank("dw")


def test_joinby_and_keys_values():
    assert builtins.builtin_joinby(["a", "b", "c"], "-") == "a-b-c"
    assert builtins.builtin_keys_of({"a": 1}) == ["a"]
    assert builtins.builtin_values_of({"a": 1}) == [1]


def test_size_and_empty_helpers():
    assert builtins.builtin_is_empty([])
    assert not builtins.builtin_is_empty([1])
    assert builtins.builtin_size_of([1, 2, 3]) == 3
    assert builtins.builtin_size_of("abc") == 3


def test_collection_transforms():
    distinct = builtins.builtin_distinct_by([1, 2, 2, 3], lambda value, index=None: value)
    assert distinct == [1, 2, 3]
    assert builtins.builtin_flatten([[1, 2], [3]]) == [1, 2, 3]
    flatmapped = builtins.builtin_flat_map([[1, 2], [3, 4]], lambda value, index=None: [v * 2 for v in value])
    assert flatmapped == [2, 4, 6, 8]
    assert builtins.builtin_index_of(["a", "b"], "b") == 1
    assert builtins.builtin_max([1, 4, 2]) == 4
    assert builtins.builtin_min([1, 4, 2]) == 1


def test_object_and_string_utilities():
    obj = {"a": 1, "b": 2, "c": 3}
    filtered = builtins.builtin_filter_object(obj, lambda value, key=None, index=None: value > 1)
    assert filtered == {"b": 2, "c": 3}
    grouped = builtins.builtin_group_by(["apple", "apricot", "banana"], lambda value, index=None: value[0])
    assert grouped == {"a": ["apple", "apricot"], "b": ["banana"]}
    ordered = builtins.builtin_order_by([3, 1, 2], lambda value, index=None: value)
    assert ordered == [1, 2, 3]
    assert builtins.builtin_find("banana", "na") == [2, 4]
    assert builtins.builtin_split_by("a-b-c", "-") == ["a", "b", "c"]
    entries = builtins.builtin_entries_of({"key": "value"})
    assert entries == [{"key": "key", "value": "value", "attributes": {}}]
    assert builtins.builtin_last_index_of([1, 2, 1], 1) == 2
    assert builtins.builtin_last_index_of("banana", "na") == 4
    assert builtins.builtin_match("mulesoft", "/(mule)(soft)/") == ["mulesoft", "mule", "soft"]
    assert builtins.builtin_matches("mulesoft", "/^mule.*/") is True
    assert builtins.builtin_matches("mulesoft", "/^soft/") is False


def test_array_by_functions():
    values = [{"a": 1}, {"a": 3}, {"a": 2}]
    assert builtins.builtin_max_by(values, lambda item, index=None: item["a"]) == {"a": 3}
    assert builtins.builtin_min_by(values, lambda item, index=None: item["a"]) == {"a": 1}
    obj = {"a": 1, "b": 2}
    assert builtins.builtin_pluck(obj, lambda value, key=None, index=None: {key: value}) == [{"a": 1}, {"b": 2}]


def test_logging_helpers(caplog):
    caplog.set_level(logging.DEBUG)
    assert builtins.builtin_log("prefix", "value") == "value"
    assert builtins.builtin_log_info("info", "value") == "value"
    assert builtins.builtin_log_debug("debug", "value") == "value"
    assert builtins.builtin_log_warn("warn", "value") == "value"
    assert builtins.builtin_log_error("error", "value") == "value"
    logged_messages = [record.message for record in caplog.records]
    assert any("prefix" in message for message in logged_messages)


def test_runtime_executes_fixture_script(tmp_path):
    runtime = DataWeaveRuntime()
    script = (tmp_path / "script.dwl")
    script.write_text(
        """%dw 2.0
output application/json
var numbers = payload.values -- [3]
---
{
  concatenated: payload.values ++ [4],
  filtered: payload.values filter (value) -> value > 1,
  upperName: upper(payload.name),
  sumValues: sum(payload.values),
  flattened: flatten([[payload.values[0]], payload.values -- [1]]),
  maxValue: max(payload.values),
  minValue: min(payload.values)
}
"""
    )
    payload = {
        "name": "dw",
        "values": [1, 2, 3],
        "meta": {"id": 1},
    }

    result = runtime.execute(script.read_text(), payload=payload)
    assert result["concatenated"] == [1, 2, 3, 4]
    assert result["filtered"] == [2, 3]
    assert result["upperName"] == "DW"
    assert result["sumValues"] == 6
    assert result["flattened"] == [1, 2, 3]
    assert result["maxValue"] == 3
    assert result["minValue"] == 1
