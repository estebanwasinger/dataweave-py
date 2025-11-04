import json
from pathlib import Path

import pytest

import dwpy.parser as parser
from dwpy.runtime import DataWeaveRuntime


FIXTURES_DIR = Path(__file__).parent / "fixtures"


def test_executes_basic_object_transformation():
    script = """%dw 2.0
output application/json
---
{
  id: payload.orderId,
  status: upper(payload.status default "pending")
}
"""
    payload = {
        "orderId": "A123",
    }

    runtime = DataWeaveRuntime()
    result = runtime.execute(script, payload)

    assert result == {
        "id": "A123",
        "status": "PENDING",
    }


def test_header_var_declarations_are_evaluated_before_body():
    script = """%dw 2.0
output application/json
var greeting = upper("hello")
    var summary = greeting
---
{
  message: summary
}
"""

    runtime = DataWeaveRuntime()
    result = runtime.execute(script, payload={})

    assert result == {
        "message": "HELLO",
    }


def test_header_var_with_concatenation_operator():
    script = """%dw 2.0
output application/json
var greeting = upper("hello")
var summary = greeting ++ " WORLD"
---
{
  message: summary
}
"""

    runtime = DataWeaveRuntime()
    result = runtime.execute(script, payload={})

    assert result == {
        "message": "HELLO WORLD",
    }


def test_header_import_directive_is_tolerated():
    script = """%dw 2.0
output application/json
import * from dw::core::Strings
var greeting = upper("hello")
---
{
  message: greeting
}
"""

    runtime = DataWeaveRuntime()
    result = runtime.execute(script, payload={})

    assert result["message"] == "HELLO"


def test_transformation_with_payload_and_vars_defaults():
    script = """%dw 2.0
output application/json
var captured = (vars.requestTime default now())
---
{
  id: payload.orderId,
  status: upper(payload.status default "pending"),
  generatedAt: captured
}
"""
    payload = {
        "orderId": "A123",
        "status": "confirmed",
    }

    runtime = DataWeaveRuntime()
    result = runtime.execute(
        script,
        payload=payload,
        vars={"requestTime": "2024-05-05T12:00:00Z"},
    )

    assert result["id"] == "A123"
    assert result["status"] == "CONFIRMED"
    assert result["generatedAt"] == "2024-05-05T12:00:00Z"

    fallback = runtime.execute(script, payload=payload, vars={})
    assert fallback["generatedAt"].endswith("Z")


def test_reduction_over_items_for_total():
    script = """%dw 2.0
output application/json
---
{
  total: (payload.items default [])
            reduce ((item, acc = 0) -> acc + item.price * (item.quantity default 1))
}
"""
    payload = {
        "items": [
            {"price": 15.5, "quantity": 2},
            {"price": 9.99},
        ]
    }

    runtime = DataWeaveRuntime()
    result = runtime.execute(script, payload=payload)

    assert result["total"] == pytest.approx(40.99, rel=1e-9)


def test_map_over_items_for_projection():
    script = """%dw 2.0
output application/json
---
{
  projected: (payload.items default []) map ((item) -> item.price)
}
"""
    payload = {
        "items": [
            {"price": 5},
            {"price": 7},
        ]
    }

    runtime = DataWeaveRuntime()
    result = runtime.execute(script, payload=payload)

    assert result["projected"] == [5, 7]


def test_filter_and_if_expression():
    script = """%dw 2.0
output application/json
---
{
  filtered: (payload.items default [])
              filter ((item) -> item.category == "book"),
  discountApplied: if ((payload.discount default 0) > 0) "YES" else "NO"
}
"""
    payload = {
        "items": [
            {"category": "book", "price": 10},
            {"category": "video", "price": 20},
            {"category": "book", "price": 5},
        ],
        "discount": 5,
    }

    runtime = DataWeaveRuntime()
    result = runtime.execute(script, payload=payload)

    assert len(result["filtered"]) == 2
    assert result["filtered"][0]["category"] == "book"
    assert result["discountApplied"] == "YES"

    no_discount = runtime.execute(
        script,
        payload={**payload, "discount": 0},
    )

    assert no_discount["discountApplied"] == "NO"


def test_comments_are_ignored():
    script = """%dw 2.0
// header comment
output application/json
---
{
  // inline comment
  id: payload.orderId, /* block comment */
  status: payload.status default "unknown"
}
"""
    payload = {
        "orderId": "Z9",
    }

    runtime = DataWeaveRuntime()
    result = runtime.execute(script, payload=payload)

    assert result["id"] == "Z9"
    assert result["status"] == "unknown"


def test_match_expression_chooses_case():
    script = """%dw 2.0
output application/json
---
{
  normalized: payload.status match {
    case "confirmed" -> "CONFIRMED",
    case "pending" -> "PENDING",
    else -> "UNKNOWN"
  }
}
"""
    payload = {"status": "confirmed"}

    runtime = DataWeaveRuntime()
    result = runtime.execute(script, payload=payload)

    assert result["normalized"] == "CONFIRMED"

    other = runtime.execute(script, payload={"status": "missing"})
    assert other["normalized"] == "UNKNOWN"


def test_match_expression_with_binding_and_guard():
    script = """%dw 2.0
output application/json
---
{
  bucket: payload.total match {
    case var value when value > 100 -> "large",
    case var value -> "small"
  }
}
"""
    runtime = DataWeaveRuntime()

    result_large = runtime.execute(script, payload={"total": 150})
    assert result_large["bucket"] == "large"

    result_small = runtime.execute(script, payload={"total": 40})
    assert result_small["bucket"] == "small"


def test_index_selector_and_header_reference():
    script = """%dw 2.0
output application/json
var x = payload.values[0]
---
{
  id: payload.orderId,
  status: upper(payload.status default "pending"),
  values: payload.values map (value) -> value * x
}
"""
    payload = {
        "orderId": "IDX-1",
        "status": "confirmed",
        "values": [2, 3],
    }

    runtime = DataWeaveRuntime()
    result = runtime.execute(script, payload=payload)

    assert result["id"] == "IDX-1"
    assert result["status"] == "CONFIRMED"
    assert result["values"] == [4, 6]


def test_null_safe_selectors_fall_back_to_default():
    script = """%dw 2.0
output application/json
var city = payload.user?.address?.city default "UNKNOWN"
---
{
  city: city
}
"""

    runtime = DataWeaveRuntime()
    result = runtime.execute(script, payload={})
    assert result["city"] == "UNKNOWN"

    result_with_city = runtime.execute(
        script,
        payload={"user": {"address": {"city": "Madrid"}}},
    )
    assert result_with_city["city"] == "Madrid"


def test_parse_error_reports_line_and_column():
    script = """%dw 2.0
output application/json
---
{
  id: payload.orderId,,
}
"""
    runtime = DataWeaveRuntime()
    with pytest.raises(parser.ParseError) as exc:
        runtime.execute(script, payload={})

    message = str(exc.value)
    assert "line" in message
    assert "column" in message


def test_fixture_parity_sample_script():
    script_path = FIXTURES_DIR / "sample_script.dwl"
    payload_path = FIXTURES_DIR / "sample_input.json"
    expected_path = FIXTURES_DIR / "sample_expected.json"

    script_source = script_path.read_text()
    payload = json.loads(payload_path.read_text())
    expected = json.loads(expected_path.read_text())

    runtime = DataWeaveRuntime()
    result = runtime.execute(
        script_source,
        payload=payload,
        vars={"requestTime": "2024-05-05T12:00:00Z"},
    )

    assert result["id"] == expected["id"]
    assert result["status"] == expected["status"]
    assert result["values"] == expected["values"]
    assert result["normalizedStatus"] == expected["normalizedStatus"]
    assert result["city"] == expected["city"]
    assert result["reference"] == expected["reference"]
    assert result["generatedAt"] == expected["generatedAt"]
    assert result["total"] == pytest.approx(expected["total"], rel=1e-9)


def test_distinct_flatmap_and_index_helpers():
    script = """%dw 2.0
output application/json
---
{
  distinct: payload.values distinctBy (value) -> value,
  flatmapped: payload.matrix flatMap (row, index) -> row,
  flattened: flatten(payload.matrix),
  firstIndex: indexOf(payload.values, 3),
  maxValue: max(payload.values),
  minValue: min(payload.values),
  filteredObj: filterObject(payload.object, (value, key) -> value > 1),
  grouped: groupBy(payload.values, (value) -> if (value <= 2) "low" else "high"),
  ordered: orderBy(payload.valuesDescending, (value) -> value),
  found: find(payload.text, "na"),
  split: splitBy(payload.phrase, "-")
}
"""
    payload = {
        "values": [1, 2, 2, 3],
        "matrix": [[1, 2], [3, 4]],
        "object": {"a": 1, "b": 2, "c": 3},
        "valuesDescending": [3, 2, 1],
        "text": "banana",
        "phrase": "a-b-c",
    }

    runtime = DataWeaveRuntime()
    result = runtime.execute(script, payload=payload)

    assert result["distinct"] == [1, 2, 3]
    assert result["flatmapped"] == [1, 2, 3, 4]
    assert result["flattened"] == [1, 2, 3, 4]
    assert result["firstIndex"] == 3
    assert result["maxValue"] == 3
    assert result["minValue"] == 1
    assert result["filteredObj"] == {"b": 2, "c": 3}
    assert result["grouped"] == {"low": [1, 2, 2], "high": [3]}
    assert result["ordered"] == [1, 2, 3]
    assert result["found"] == [2, 4]
    assert result["split"] == ["a", "b", "c"]


def test_infix_and_prefix_function_calls():
    script = """%dw 2.0
output application/json
---
{
  infixContains: payload.list contains 3,
  prefixContains: contains(payload.list, 3),
  infixJoin: payload.words joinBy "-",
  prefixJoin: joinBy(payload.words, "-"),
  infixSplit: payload.phrase splitBy "-",
  prefixSplit: splitBy(payload.phrase, "-"),
  infixGroup: payload.objects groupBy (item) -> item.language,
  prefixGroup: groupBy(payload.objects, (item) -> item.language)
}
"""
    payload = {
        "list": [1, 2, 3],
        "words": ["a", "b", "c"],
        "phrase": "a-b-c",
        "objects": [
            {"name": "Foo", "language": "Java"},
            {"name": "Bar", "language": "Scala"},
            {"name": "FooBar", "language": "Java"},
        ],
    }

    runtime = DataWeaveRuntime()
    result = runtime.execute(script, payload=payload)

    assert result["infixContains"] is True
    assert result["prefixContains"] is True
    assert result["infixJoin"] == "a-b-c"
    assert result["prefixJoin"] == "a-b-c"
    assert result["infixSplit"] == ["a", "b", "c"]
    assert result["prefixSplit"] == ["a", "b", "c"]
    assert result["infixGroup"] == {
        "Java": [
            {"name": "Foo", "language": "Java"},
            {"name": "FooBar", "language": "Java"},
        ],
        "Scala": [{"name": "Bar", "language": "Scala"}],
    }
    assert result["prefixGroup"] == result["infixGroup"]


def test_random_functions_available():
    runtime = DataWeaveRuntime()
    result = runtime.execute(
        "%dw 2.0\noutput application/json\n---\n{ price: randomInt(1000), ratio: random() }",
        {}
    )
    assert 0 <= result["price"] < 1000
    assert 0 <= result["ratio"] < 1


def test_numeric_range_to_operator():
    runtime = DataWeaveRuntime()
    result = runtime.execute(
        "%dw 2.0\noutput application/json\n---\n{ up: 1 to 5, down: 5 to 1 }",
        {}
    )
    assert result["up"] == [1, 2, 3, 4, 5]
    assert result["down"] == [5, 4, 3, 2, 1]


def test_numeric_range_with_legacy_lambda_syntax():
    runtime = DataWeaveRuntime()
    result = runtime.execute(
        "%dw 2.0\noutput application/json\n---\n1 to 5 map ((value) -> value * 2)",
        {}
    )
    assert result == [2, 4, 6, 8, 10]


def test_import_star_from_strings_module():
    runtime = DataWeaveRuntime()
    script = """%dw 2.0
output application/json
import * from dw::core::Strings
---
upper(payload.name)
"""
    result = runtime.execute(script, {"name": "dw"})
    assert result == "DW"


def test_import_named_function_with_alias():
    runtime = DataWeaveRuntime()
    script = """%dw 2.0
output application/json
import trim as tidy from dw::core::Strings
---
tidy(payload.value)
"""
    result = runtime.execute(script, {"value": "  hello  "})
    assert result == "hello"


def test_import_from_module_file():
    runtime = DataWeaveRuntime()
    script = """%dw 2.0
output application/json
import keysOf, valuesOf from dw::core::Objects
---
{
  keys: keysOf(payload.obj),
  values: valuesOf(payload.obj)
}
"""
    payload = {"obj": {"a": 1, "b": 2}}
    result = runtime.execute(script, payload)
    assert result == {"keys": ["a", "b"], "values": [1, 2]}


def test_body_only_script_without_header():
    runtime = DataWeaveRuntime()
    result = runtime.execute("payload.name", {"name": "hello"})
    assert result == "hello"


def test_header_defined_function_invocation():
    runtime = DataWeaveRuntime()
    script = """%dw 2.0
output application/json
fun toUpper(aString) = upper(aString)
---
toUpper(\"h\" ++ \"el\" ++ lower(\"LO\"))
"""
    result = runtime.execute(script, {})
    assert result == "HELLO"


def test_map_over_range_generates_objects():
    runtime = DataWeaveRuntime()
    script = """%dw 2.0
output application/json
---
1 to 10 map {
  "hi": "Esteban"
}
"""
    result = runtime.execute(script, {})
    assert result == [{"hi": "Esteban"}] * 10
