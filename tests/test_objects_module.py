from dwpy.runtime import DataWeaveRuntime


def _execute(script: str):
    runtime = DataWeaveRuntime()
    return runtime.execute(script, payload={})


def test_entry_set_returns_key_value_descriptions():
    script = """%dw 2.0
import entrySet from dw::core::Objects
output application/json
---
entrySet({ a: 1, b: true })
"""
    result = _execute(script)
    assert result == [
        {"key": "a", "value": 1, "attributes": {}},
        {"key": "b", "value": True, "attributes": {}},
    ]


def test_name_key_value_sets_match_expectations():
    script = """%dw 2.0
import nameSet, keySet, valueSet from dw::core::Objects
output application/json
---
{
  names: nameSet({ first: "Ana", last: "Simpson" }),
  keys: keySet({ x: 1, y: 2 }),
  values: valueSet({ x: 1, y: 2 })
}
"""
    result = _execute(script)
    assert result["names"] == ["first", "last"]
    assert result["keys"] == ["x", "y"]
    assert result["values"] == [1, 2]


def test_merge_with_preserves_source_when_target_missing():
    script = """%dw 2.0
import mergeWith from dw::core::Objects
output application/json
---
{
  merged: mergeWith({ a: 1, b: 2 }, { b: 3, c: 4 }),
  leftNull: mergeWith(null, { x: 1 }),
  rightNull: mergeWith({ y: 2 }, null)
}
"""
    result = _execute(script)
    assert result["merged"] == {"a": 1, "b": 3, "c": 4}
    assert result["leftNull"] == {"x": 1}
    assert result["rightNull"] == {"y": 2}


def test_divide_by_groups_entries():
    script = """%dw 2.0
import divideBy from dw::core::Objects
output application/json
---
divideBy({ a: 1, b: 2, c: 3, d: 4, e: 5 }, 2)
"""
    result = _execute(script)
    assert result == [
        {"a": 1, "b": 2},
        {"c": 3, "d": 4},
        {"e": 5},
    ]


def test_take_while_stops_when_condition_fails():
    script = """%dw 2.0
import takeWhile from dw::core::Objects
output application/json
---
takeWhile({ a: 1, b: 2, c: 5, d: 1 }, (value, key) -> value < 3)
"""
    result = _execute(script)
    assert result == {"a": 1, "b": 2}


def test_every_entry_handles_truthy_and_null_inputs():
    script = """%dw 2.0
import everyEntry from dw::core::Objects
output application/json
---
{
  matches: everyEntry({ a: 1, b: 2 }, (value, key) -> value < 3),
  fails: everyEntry({ a: 1, b: 4 }, (value, key) -> value < 3),
  nullValue: everyEntry(null, (value, key) -> value < 3)
}
"""
    result = _execute(script)
    assert result["matches"] is True
    assert result["fails"] is False
    assert result["nullValue"] is True


def test_some_entry_detects_any_matching_entry():
    script = """%dw 2.0
import someEntry from dw::core::Objects
output application/json
---
{
  found: someEntry({ a: 1, b: 4 }, (value, key) -> value > 3),
  missing: someEntry({ a: 1, b: 2 }, (value, key) -> value > 3),
  nullValue: someEntry(null, (value, key) -> value > 3)
}
"""
    result = _execute(script)
    assert result["found"] is True
    assert result["missing"] is False
    assert result["nullValue"] is False
