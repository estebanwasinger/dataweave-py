from __future__ import annotations

import json
from pathlib import Path

import pandas as pd
import pytest

from dwpy.runtime import DataWeaveRuntime
from dwpy.type_inference import infer_script_type
from dwpy.typesystem import ArrayType, ObjectType, STRING


def test_default_runtime_uses_rust_backend_bridge() -> None:
    runtime = DataWeaveRuntime()

    assert runtime.active_backend == "rust"
    assert "rust-core-evaluator" in runtime.capabilities()
    assert runtime.execute("payload.name", {"name": "dw"}) == "dw"
    assert runtime._rust_runtime.last_execution_engine() == "rust-core"


def test_python_backend_remains_available_for_parity() -> None:
    runtime = DataWeaveRuntime(backend="python")

    assert runtime.active_backend == "python"
    assert runtime.execute("payload.name", {"name": "legacy"}) == "legacy"


def test_auto_backend_falls_back_only_when_rust_extension_is_unavailable(monkeypatch: pytest.MonkeyPatch) -> None:
    import dwpy.runtime as runtime_module

    def fail_to_create_rust_runtime(
        enable_module_imports: bool,  # noqa: ARG001
        *,
        allow_legacy_fallback: bool,  # noqa: ARG001
    ):
        raise ImportError("simulated missing extension")

    monkeypatch.setattr(
        runtime_module.DataWeaveRuntime,
        "_new_rust_runtime",
        staticmethod(fail_to_create_rust_runtime),
    )

    runtime = runtime_module.DataWeaveRuntime(backend="auto")

    assert runtime.active_backend == "python"
    assert runtime.execute("payload.name", {"name": "fallback"}) == "fallback"


def test_explicit_rust_backend_is_strict_for_unsupported_features() -> None:
    strict_runtime = DataWeaveRuntime(backend="rust")
    script = "%dw 2.0\noutput application/python\n---\npayload[0].city\n"

    with pytest.raises(NotImplementedError):
        strict_runtime.execute(
            script,
            "name,city\nAna,Lisbon",
            payload_format="csv",
            payload_format_options={"skipEmptyLines": True},
        )

    auto_runtime = DataWeaveRuntime(backend="auto")
    assert (
        auto_runtime.execute(
            script,
            "name,city\nAna,Lisbon",
            payload_format="csv",
            payload_format_options={"skipEmptyLines": True},
        )
        == "Lisbon"
    )
    assert auto_runtime._rust_runtime.last_execution_engine() == "python-legacy"


def test_rust_bridge_preserves_python_value_roundtrip() -> None:
    runtime = DataWeaveRuntime()
    script = """%dw 2.0
output application/python
---
{
  name: payload.user.name,
  tags: payload.user.tags,
  active: payload.user.active
}
"""

    assert runtime.execute(
        script,
        {"user": {"name": "Ana", "tags": ["a", "b"], "active": True}},
    ) == {"name": "Ana", "tags": ["a", "b"], "active": True}
    assert runtime._rust_runtime.last_execution_engine() == "rust-core"


def test_rust_bridge_normalizes_dataframe_inputs_in_core() -> None:
    runtime = DataWeaveRuntime(backend="rust")
    script = """%dw 2.0
output application/python
---
{
  payloadCities: payload map ((item) -> item.city),
  varNames: vars.source map ((item) -> upper(item.name))
}
"""
    payload = pd.DataFrame(
        [
            {"id": 1, "city": "London"},
            {"id": 2, "city": "Berlin"},
        ]
    )
    vars_source = pd.DataFrame(
        [
            {"name": "alice"},
            {"name": "Bob"},
        ]
    )

    assert runtime.execute(script, payload, vars={"source": vars_source}) == {
        "payloadCities": ["London", "Berlin"],
        "varNames": ["ALICE", "BOB"],
    }
    assert runtime._rust_runtime.last_execution_engine() == "rust-core"


def test_rust_bridge_executes_json_indent_output_in_core() -> None:
    runtime = DataWeaveRuntime()
    script = """%dw 2.0
output application/json indent=2
---
{
  name: payload.name
}
"""

    result = runtime.execute(
        script,
        {"name": "legacy"},
    )
    assert json.loads(result) == {"name": "legacy"}
    assert result.count("\n") > 1
    assert runtime._rust_runtime.last_execution_engine() == "rust-core"


def test_rust_bridge_executes_json_writer_options_in_core() -> None:
    runtime = DataWeaveRuntime(backend="rust")
    script = """%dw 2.0
output application/json indent=4 sort_keys=true ensure_ascii=true
---
{z: payload.word, a: "café"}
"""

    result = runtime.execute(script, {"word": "niño"})

    assert result == '{\n    "a": "caf\\u00e9",\n    "z": "ni\\u00f1o"\n}'
    assert runtime._rust_runtime.last_execution_engine() == "rust-core"


def test_rust_bridge_ignores_comments_in_core() -> None:
    runtime = DataWeaveRuntime(backend="rust")
    script = """%dw 2.0
// header comment
output application/json
/*
---
*/
var greeting = "DataWeave"
---
{
  // body comment
  id: payload.orderId, /* inline block comment */
  greeting: greeting,
  literal: "not // a comment"
}
"""

    result = runtime.execute(script, {"orderId": "Z9"}, render_output=False)

    assert result == {
        "id": "Z9",
        "greeting": "DataWeave",
        "literal": "not // a comment",
    }
    assert runtime._rust_runtime.last_execution_engine() == "rust-core"


def test_rust_bridge_executes_operators_and_indexing_in_core() -> None:
    runtime = DataWeaveRuntime()
    script = """%dw 2.0
output application/python
---
{
  total: payload.price * payload.quantity + 2,
  label: "Order " ++ payload.id,
  first: payload.items[0],
  eligible: if (payload.price * payload.quantity >= 30 and payload.enabled == true) "yes" else "no"
}
"""

    assert runtime.execute(
        script,
        {
            "price": 10,
            "quantity": 3,
            "id": "A-1",
            "enabled": True,
            "items": ["first", "second"],
        },
    ) == {
        "total": 32,
        "label": "Order A-1",
        "first": "first",
        "eligible": "yes",
    }
    assert runtime._rust_runtime.last_execution_engine() == "rust-core"


def test_rust_bridge_executes_primitive_coercions_in_core() -> None:
    runtime = DataWeaveRuntime()
    script = """%dw 2.0
output application/python
---
{
  fromString: payload.price as Number,
  fromBool: true as Number,
  toString: (payload.count + 1) as String,
  trueString: "true" as Boolean,
  falseNumber: 0 as Boolean,
  nullString: null as String,
  nowDateSize: sizeOf(now() as Date),
  mapped: payload.values map (($ as String) ++ "-x")
}
"""

    assert runtime.execute(script, {"price": "12.5", "count": 2, "values": [1, 2, 3]}) == {
        "fromString": 12.5,
        "fromBool": 1,
        "toString": "3",
        "trueString": True,
        "falseNumber": False,
        "nullString": None,
        "nowDateSize": 10,
        "mapped": ["1-x", "2-x", "3-x"],
    }
    assert runtime._rust_runtime.last_execution_engine() == "rust-core"


def test_rust_bridge_executes_string_interpolation_in_core() -> None:
    runtime = DataWeaveRuntime()
    script = """%dw 2.0
output application/json
var suffix = upper(vars.suffix)
---
{
  message: "Hello $(payload.user.name), total: $(payload.price * payload.quantity) $(suffix)",
  nested: "Result: $((payload.a + payload.b) * 2)",
  defaulted: "Guest: $(payload.missing default 'anonymous')",
  nullValue: "Value: $(payload.missing)"
}
"""

    result = runtime.execute(
        script,
        {
            "user": {"name": "Ada"},
            "price": 10,
            "quantity": 3,
            "a": 5,
            "b": 3,
        },
        vars={"suffix": "ok"},
    )

    assert json.loads(result) == {
        "message": "Hello Ada, total: 30 OK",
        "nested": "Result: 16",
        "defaulted": "Guest: anonymous",
        "nullValue": "Value: ",
    }
    assert runtime._rust_runtime.last_execution_engine() == "rust-core"


def test_rust_bridge_executes_simple_header_functions_in_core() -> None:
    runtime = DataWeaveRuntime()
    script = """%dw 2.0
output application/json
fun lineTotal(item) = item.qty * item.price
fun label(name, prefix = "Item") = prefix ++ ": " ++ upper(name)
fun normalize(value): Number = value as Number
---
{
  total: sum(payload.items map ((item) -> lineTotal(item))),
  labels: payload.items map ((item) -> label(item.name)),
  custom: label("special", "Custom"),
  normalized: normalize("12.5")
}
"""

    result = runtime.execute(
        script,
        {
            "items": [
                {"name": "apple", "qty": 2, "price": 3},
                {"name": "pear", "qty": 1, "price": 4},
            ]
        },
    )

    assert json.loads(result) == {
        "total": 10,
        "labels": ["Item: APPLE", "Item: PEAR"],
        "custom": "Custom: SPECIAL",
        "normalized": 12.5,
    }
    assert runtime._rust_runtime.last_execution_engine() == "rust-core"


def test_rust_bridge_executes_function_references_in_core() -> None:
    runtime = DataWeaveRuntime()
    script = """%dw 2.0
output application/json
fun applyEach(f, arr) = arr map ((item) -> f(item))
fun up(s) = upper(s)
---
applyEach(up, payload.names)
"""

    result = runtime.execute(script, {"names": ["a", "b"]})

    assert json.loads(result) == ["A", "B"]
    assert runtime._rust_runtime.last_execution_engine() == "rust-core"


def test_rust_bridge_executes_supported_import_aliases_in_core() -> None:
    runtime = DataWeaveRuntime()
    script = """%dw 2.0
output application/json
import trim as tidy from dw::core::Strings
import keysOf, valuesOf from dw::core::Objects
---
{
  cleaned: tidy(payload.value),
  keys: keysOf(payload.object),
  values: valuesOf(payload.object)
}
"""

    result = runtime.execute(script, {"value": "  hello  ", "object": {"a": 1, "b": 2}})

    assert json.loads(result) == {
        "cleaned": "hello",
        "keys": ["a", "b"],
        "values": [1, 2],
    }
    assert runtime._rust_runtime.last_execution_engine() == "rust-core"


def test_rust_bridge_executes_null_safe_selectors_in_core() -> None:
    runtime = DataWeaveRuntime()
    script = """%dw 2.0
output application/json
var city = payload.user?.address?.city default "UNKNOWN"
---
{
  city: city,
  quoted: payload.user?."value 2" default "UNKNOWN"
}
"""

    missing = runtime.execute(script, {})
    assert json.loads(missing) == {"city": "UNKNOWN", "quoted": "UNKNOWN"}
    assert runtime._rust_runtime.last_execution_engine() == "rust-core"

    present = runtime.execute(script, {"user": {"address": {"city": "Madrid"}, "value 2": "s"}})
    assert json.loads(present) == {"city": "Madrid", "quoted": "s"}
    assert runtime._rust_runtime.last_execution_engine() == "rust-core"


def test_rust_bridge_executes_match_expressions_in_core() -> None:
    runtime = DataWeaveRuntime()
    script = """%dw 2.0
output application/json
---
{
  normalized: payload.status match {
    case "confirmed" -> "CONFIRMED",
    case "pending" -> "PENDING",
    else -> "UNKNOWN"
  },
  bucket: payload.total match {
    case var value when value > 100 -> "large",
    case var value -> "small"
  },
  boolCase: payload.flag match {
    case true -> 1,
    case false -> 0
  }
}
"""

    result = runtime.execute(script, {"status": "confirmed", "total": 150, "flag": True})
    assert json.loads(result) == {
        "normalized": "CONFIRMED",
        "bucket": "large",
        "boolCase": 1,
    }
    assert runtime._rust_runtime.last_execution_engine() == "rust-core"

    other = runtime.execute(script, {"status": "other", "total": 40, "flag": False})
    assert json.loads(other) == {
        "normalized": "UNKNOWN",
        "bucket": "small",
        "boolCase": 0,
    }
    assert runtime._rust_runtime.last_execution_engine() == "rust-core"


def test_rust_bridge_executes_representative_fixture_in_core() -> None:
    runtime = DataWeaveRuntime()
    script = Path("tests/fixtures/sample_script.dwl").read_text()

    result = runtime.execute(
        script,
        {
            "orderId": "A-1",
            "status": "confirmed",
            "items": [{"price": 3, "quantity": 2}, {"price": 4}],
            "values": [1, 2],
            "user": {"address": {"city": "Madrid"}},
        },
        vars={"requestTime": "2024-01-01T00:00:00Z"},
    )

    assert json.loads(result) == {
        "id": "A-1",
        "status": "CONFIRMED",
        "total": 10,
        "values": [2, 4],
        "normalizedStatus": "CONFIRMED",
        "city": "Madrid",
        "reference": 1,
        "generatedAt": "2024-01-01T00:00:00Z",
    }
    assert runtime._rust_runtime.last_execution_engine() == "rust-core"


def test_rust_bridge_executes_numeric_and_empty_helpers_in_core() -> None:
    runtime = DataWeaveRuntime()
    script = """%dw 2.0
output application/json
---
{
  sumValues: sum(payload.values),
  avgValues: avg(payload.values),
  rounded: round(payload.roundMe),
  ceilValue: ceil(payload.decimal),
  floorValue: floor(payload.decimal),
  absValue: abs(payload.negative),
  emptyArray: isEmpty([]),
  emptyObject: isEmpty({}),
  emptyString: isEmpty(""),
  blankNull: isBlank(null),
  blankSpaces: isBlank("   "),
  numericDigits: isNumeric("12345"),
  numericLetters: isNumeric("12a"),
  decimalValue: isDecimal("12.5"),
  integerValue: isDecimal(12)
}
"""

    result = runtime.execute(
        script,
        {
            "values": [4, 6],
            "roundMe": 2.5,
            "decimal": 2.2,
            "negative": -7,
        },
    )

    assert json.loads(result) == {
        "sumValues": 10,
        "avgValues": 5,
        "rounded": 2,
        "ceilValue": 3,
        "floorValue": 2,
        "absValue": 7,
        "emptyArray": True,
        "emptyObject": True,
        "emptyString": True,
        "blankNull": True,
        "blankSpaces": True,
        "numericDigits": True,
        "numericLetters": False,
        "decimalValue": True,
        "integerValue": False,
    }
    assert runtime._rust_runtime.last_execution_engine() == "rust-core"


def test_rust_bridge_executes_negation_and_simple_matches_in_core() -> None:
    runtime = DataWeaveRuntime()
    script = """%dw 2.0
output application/json
---
{
  notEmpty: not isEmpty(payload.values),
  bangEmpty: !isEmpty(payload.values),
  negativeAmount: -payload.amount,
  filtered: payload.objects filter (!isEmpty($)),
  numeric: (payload.code as String) matches "/^[0-9]+$/",
  alpha: payload.word matches "/^[A-Za-z]+$/",
  literal: payload.word matches "abc"
}
"""

    result = runtime.execute(
        script,
        {
            "values": [1],
            "objects": [{}, {"name": "Ada"}],
            "amount": 7,
            "code": 12345,
            "word": "abc",
        },
    )

    assert json.loads(result) == {
        "notEmpty": True,
        "bangEmpty": True,
        "negativeAmount": -7,
        "filtered": [{"name": "Ada"}],
        "numeric": True,
        "alpha": True,
        "literal": True,
    }
    assert runtime._rust_runtime.last_execution_engine() == "rust-core"


def test_rust_bridge_executes_collection_object_helpers_in_core() -> None:
    runtime = DataWeaveRuntime()
    script = """%dw 2.0
output application/json
var key = "name"
---
{
  flattened: flatten(payload.matrix),
  firstIndex: indexOf(payload.values, 3),
  missingIndex: indexOf(payload.values, 9),
  stringIndex: indexOf(payload.text, "na"),
  lastArrayIndex: lastIndexOf(payload.values, 2),
  lastStringIndex: lastIndexOf(payload.text, "na"),
  maxValue: max(payload.values),
  minValue: min(payload.values),
  keys: keysOf(payload.object),
  values: valuesOf(payload.object),
  entries: entriesOf(payload.object),
  dynamic: payload.user[key]
}
"""

    result = runtime.execute(
        script,
        {
            "matrix": [[1, 2], [3, 4], 5],
            "values": [1, 2, 2, 3],
            "text": "banana",
            "object": {"a": 1, "b": 2},
            "user": {"name": "Ana"},
        },
    )

    assert json.loads(result) == {
        "flattened": [1, 2, 3, 4, 5],
        "firstIndex": 3,
        "missingIndex": -1,
        "stringIndex": 2,
        "lastArrayIndex": 2,
        "lastStringIndex": 4,
        "maxValue": 3,
        "minValue": 1,
        "keys": ["a", "b"],
        "values": [1, 2],
        "entries": [
            {"key": "a", "value": 1, "attributes": {}},
            {"key": "b", "value": 2, "attributes": {}},
        ],
        "dynamic": "Ana",
    }
    assert runtime._rust_runtime.last_execution_engine() == "rust-core"


def test_rust_bridge_executes_string_helpers_in_core() -> None:
    runtime = DataWeaveRuntime()
    script = """%dw 2.0
output application/json
---
{
  appended: appendIfMissing("abc", "xyz"),
  camelized: camelize("customer_first_name"),
  capitalized: capitalize("customerName"),
  charCode: charCode("Mule"),
  charCodeAt: charCodeAt("MuleSoft", 1),
  collapsed: collapse("a  b"),
  matchCount: countMatches("hello worlo!", "lo"),
  vowelCount: countMatches("hello, ciao!", "/[aeiou]/"),
  dasherized: dasherize("customer_first_name"),
  firstText: first("hello world!", 5.9),
  fromCode: fromCharCode(117),
  hamming: hammingDistance("holu", "chau"),
  alpha: isAlpha("abc"),
  alphanumeric: isAlphanumeric("ab2c"),
  lowerCase: isLowerCase("mulesoft"),
  upperCase: isUpperCase("ABC"),
  whitespace: isWhitespace(""),
  lastText: last("hello world!", 5.1),
  leftPadded: leftPad("bat", 5),
  rightPadded: rightPad("bat", 5),
  editDistance: levenshteinDistance("kitten", "sitting"),
  lines: lines("hello world\\n\\nhere   data-weave"),
  ordinal: ordinalize(103),
  plural: pluralize("box"),
  prepended: prependIfMissing("abc", "xyz"),
  repeated: repeat("e", 3),
  replaced: replaceAll("AAAA", "AAA", "B"),
  removed: remove("stateful state", "state"),
  reversed: reverse("Mariano"),
  singular: singularize("boxes"),
  substringed: substring("hello world!", 1, 5),
  after: substringAfter("abcba", "b"),
  afterLast: substringAfterLast("abcba", "b"),
  before: substringBefore("abc", "c"),
  beforeLast: substringBeforeLast("abcba", "b"),
  every: substringEvery("substringEvery", 3),
  underscored: underscore("customerName"),
  limited: withMaxSize("123", 2),
  words: words("hello world\\nhere\\tdata-weave"),
  unwrapped: unwrap("'abc'", "'"),
  wrapped: wrapWith("ab", "'"),
  wrappedMissing: wrapIfMissing("a/b/c", "/")
}
"""

    result = runtime.execute(script, {})

    assert json.loads(result) == {
        "appended": "abcxyz",
        "camelized": "customerFirstName",
        "capitalized": "Customer Name",
        "charCode": 77,
        "charCodeAt": 117,
        "collapsed": ["a", "  ", "b"],
        "matchCount": 2,
        "vowelCount": 5,
        "dasherized": "customer-first-name",
        "firstText": "hello",
        "fromCode": "u",
        "hamming": 3,
        "alpha": True,
        "alphanumeric": True,
        "lowerCase": True,
        "upperCase": True,
        "whitespace": True,
        "lastText": "world!",
        "leftPadded": "  bat",
        "rightPadded": "bat  ",
        "editDistance": 3,
        "lines": ["hello world", "", "here   data-weave"],
        "ordinal": "103rd",
        "plural": "boxes",
        "prepended": "xyzabc",
        "repeated": "eee",
        "replaced": "BA",
        "removed": "ful ",
        "reversed": "onairaM",
        "singular": "box",
        "substringed": "ello",
        "after": "cba",
        "afterLast": "a",
        "before": "ab",
        "beforeLast": "abc",
        "every": ["sub", "str", "ing", "Eve", "ry"],
        "underscored": "customer_name",
        "limited": "12",
        "words": ["hello", "world", "here", "data-weave"],
        "unwrapped": "abc",
        "wrapped": "'ab'",
        "wrappedMissing": "/a/b/c/",
    }
    assert runtime._rust_runtime.last_execution_engine() == "rust-core"


def test_rust_bridge_executes_ranges_slices_and_difference_in_core() -> None:
    runtime = DataWeaveRuntime()
    script = """%dw 2.0
output application/json
---
{
  up: 1 to 5,
  down: 5 to 1,
  doubled: 1 to 5 map ((value) -> value * 2),
  reversed: (1 to 5)[-1 to 0],
  textSlice: payload.text[2 to 6],
  textReverse: payload.text[11 to -0],
  arrayDiff: [1, 2, 3, 2] -- [2],
  objectDiffKeys: payload.object -- ["a"],
  objectDiffObject: payload.object -- {b: 2},
  stringDiff: "abcabc" -- "b",
  arrayPlusArray: [2] + [2],
  arrayPlusScalar: [2] + 2
}
"""

    result = runtime.execute(
        script,
        {"text": "Hello World!", "object": {"a": 1, "b": 2, "c": 3}},
    )

    assert json.loads(result) == {
        "up": [1, 2, 3, 4, 5],
        "down": [5, 4, 3, 2, 1],
        "doubled": [2, 4, 6, 8, 10],
        "reversed": [5, 4, 3, 2, 1],
        "textSlice": "llo W",
        "textReverse": "!dlroW olleH",
        "arrayDiff": [1, 3],
        "objectDiffKeys": {"b": 2, "c": 3},
        "objectDiffObject": {"a": 1, "c": 3},
        "stringDiff": "acac",
        "arrayPlusArray": [2, [2]],
        "arrayPlusScalar": [2, 2],
    }
    assert runtime._rust_runtime.last_execution_engine() == "rust-core"


def test_rust_bridge_executes_descendant_selectors_in_core() -> None:
    runtime = DataWeaveRuntime(backend="rust")
    script = """%dw 2.0
output application/python
---
{
  names: payload..name,
  quoted: payload.."value 2",
  children: payload.user..,
  pairs: payload..&name
}
"""

    result = runtime.execute(
        script,
        {
            "name": "root",
            "user": {"name": "Weave", "child": {"name": "BAT"}},
            "items": [
                {"value 2": "a"},
                {"nested": {"name": "Nested", "value 2": "b"}},
            ],
        },
    )

    assert result == {
        "names": ["root", "Weave", "BAT", "Nested"],
        "quoted": ["a", "b"],
        "children": ["Weave", {"name": "BAT"}, "BAT"],
        "pairs": [
            {"name": "root"},
            {"name": "Weave"},
            {"name": "BAT"},
            {"name": "Nested"},
        ],
    }
    assert runtime._rust_runtime.last_execution_engine() == "rust-core"


def test_rust_bridge_executes_map_filter_and_lambdas_in_core() -> None:
    runtime = DataWeaveRuntime()
    script = """%dw 2.0
output application/python
---
{
  names: payload.users map ((user, index) -> user.name ++ "-" ++ index),
  cheap: payload.users filter (($.price < 10) and ($$ > 0)),
  implicit: payload.users map {name: $.name, id: $$}
}
"""

    assert runtime.execute(
        script,
        {
            "users": [
                {"name": "A", "price": 12},
                {"name": "B", "price": 8},
                {"name": "C", "price": 5},
            ]
        },
    ) == {
        "names": ["A-0", "B-1", "C-2"],
        "cheap": [{"name": "B", "price": 8}, {"name": "C", "price": 5}],
        "implicit": [
            {"name": "A", "id": 0},
            {"name": "B", "id": 1},
            {"name": "C", "id": 2},
        ],
    }
    assert runtime._rust_runtime.last_execution_engine() == "rust-core"


def test_rust_bridge_executes_reduce_in_core() -> None:
    runtime = DataWeaveRuntime()
    script = """%dw 2.0
output application/python
---
{
  total: payload.items reduce ((item, acc = 0) -> acc + item.price * (item.quantity default 1)),
  joined: payload.names reduce ((item, acc = "") -> acc ++ upper(item)),
  product: [2, 3, 3] reduce ((item, acc) -> acc * item),
  empty: [] reduce ((item, acc = 7) -> acc + item)
}
"""

    assert runtime.execute(
        script,
        {
            "items": [{"price": 4, "quantity": 2}, {"price": 3}],
            "names": ["a", "b"],
        },
    ) == {"total": 11, "joined": "AB", "product": 18, "empty": 7}
    assert runtime._rust_runtime.last_execution_engine() == "rust-core"


def test_rust_bridge_executes_flat_map_and_group_by_in_core() -> None:
    runtime = DataWeaveRuntime()
    script = """%dw 2.0
output application/python
---
{
  flattened: payload.items flatMap ((item) -> item.tags map ((tag) -> {id: item.id, tag: tag})),
  grouped: payload.items groupBy $.kind
}
"""

    assert runtime.execute(
        script,
        {
            "items": [
                {"id": 1, "kind": "a", "tags": ["x", "y"]},
                {"id": 2, "kind": "b", "tags": ["z"]},
                {"id": 3, "kind": "a", "tags": []},
            ]
        },
    ) == {
        "flattened": [
            {"id": 1, "tag": "x"},
            {"id": 1, "tag": "y"},
            {"id": 2, "tag": "z"},
        ],
        "grouped": {
            "a": [
                {"id": 1, "kind": "a", "tags": ["x", "y"]},
                {"id": 3, "kind": "a", "tags": []},
            ],
            "b": [{"id": 2, "kind": "b", "tags": ["z"]}],
        },
    }
    assert runtime._rust_runtime.last_execution_engine() == "rust-core"


def test_rust_bridge_executes_distinct_by_and_order_by_in_core() -> None:
    runtime = DataWeaveRuntime()
    script = """%dw 2.0
output application/python
---
{
  distinct: payload.items distinctBy $.kind,
  ordered: payload.items orderBy $.score,
  orderedByIndex: payload.items orderBy $$
}
"""

    assert runtime.execute(
        script,
        {
            "items": [
                {"kind": "b", "score": 20},
                {"kind": "a", "score": 10},
                {"kind": "b", "score": 5},
            ]
        },
    ) == {
        "distinct": [{"kind": "b", "score": 20}, {"kind": "a", "score": 10}],
        "ordered": [
            {"kind": "b", "score": 5},
            {"kind": "a", "score": 10},
            {"kind": "b", "score": 20},
        ],
        "orderedByIndex": [
            {"kind": "b", "score": 20},
            {"kind": "a", "score": 10},
            {"kind": "b", "score": 5},
        ],
    }
    assert runtime._rust_runtime.last_execution_engine() == "rust-core"


def test_rust_bridge_executes_object_transforms_in_core() -> None:
    runtime = DataWeaveRuntime()
    script = """%dw 2.0
output application/python
---
{
  plucked: payload.object pluck ((value, key, index) -> key ++ ":" ++ value ++ ":" ++ index),
  mapped: payload.object mapObject ((value, key, index) -> {(upper(key)): value + index}),
  filtered: payload.object filterObject ((value, key) -> value > 1 and key != "c")
}
"""

    assert runtime.execute(script, {"object": {"a": 1, "b": 2, "c": 3}}) == {
        "plucked": ["a:1:0", "b:2:1", "c:3:2"],
        "mapped": {"A": 1, "B": 3, "C": 5},
        "filtered": {"b": 2},
    }
    assert runtime._rust_runtime.last_execution_engine() == "rust-core"


def test_rust_bridge_executes_object_merge_and_array_property_selectors_in_core() -> None:
    runtime = DataWeaveRuntime()
    script = """%dw 2.0
output application/python
---
{
  merged: payload.user ++ {password: "****", active: true},
  messages: payload.users.message,
  multiMessages: payload.users.*message,
  books: payload.catalog.*book map ((book) -> {title: book.title}),
  mappedMessages: payload.users map ((user) -> user.message)
}
"""

    assert runtime.execute(
        script,
        {
            "user": {"name": "A", "password": "1234"},
            "users": [
                {"message": "Hello"},
                {"other": 1},
                {"message": "World"},
            ],
            "catalog": {"book": [{"title": "A"}, {"title": "B"}]},
        },
    ) == {
        "merged": {"name": "A", "password": "****", "active": True},
        "messages": ["Hello", "World"],
        "multiMessages": ["Hello", "World"],
        "books": [{"title": "A"}, {"title": "B"}],
        "mappedMessages": ["Hello", None, "World"],
    }
    assert runtime._rust_runtime.last_execution_engine() == "rust-core"


def test_rust_bridge_executes_vars_scope_in_core() -> None:
    runtime = DataWeaveRuntime()
    script = """%dw 2.0
output application/python
---
{
  target: vars.target,
  matched: payload.users map ((user) -> user.name == vars.target),
  scaled: payload.values map ((value) -> value * vars.multiplier),
  fallback: vars.missing default "ok"
}
"""

    assert runtime.execute(
        script,
        {
            "users": [{"name": "Ana"}, {"name": "Bob"}],
            "values": [1, 2, 3],
        },
        vars={"target": "Bob", "multiplier": 10},
    ) == {
        "target": "Bob",
        "matched": [False, True],
        "scaled": [10, 20, 30],
        "fallback": "ok",
    }
    assert runtime._rust_runtime.last_execution_engine() == "rust-core"


def test_rust_bridge_executes_json_vars_scope_in_core() -> None:
    runtime = DataWeaveRuntime()
    script = """%dw 2.0
output application/json
---
{
  scaled: payload.values map ((value) -> value * vars.multiplier),
  filtered: payload.values filter ((value) -> value > vars.min)
}
"""

    result = runtime.execute(
        script,
        {"values": [1, 2, 3]},
        vars={"multiplier": 10, "min": 1},
    )

    assert json.loads(result) == {"scaled": [10, 20, 30], "filtered": [2, 3]}
    assert runtime._rust_runtime.last_execution_engine() == "rust-core"


def test_rust_bridge_executes_read_write_format_helpers_in_core() -> None:
    runtime = DataWeaveRuntime()
    script = """%dw 2.0
output application/json
import read, write from dw::Core
---
{
  jsonParsed: read("{\\"a\\": 1}", "application/json").a,
  jsonWritten: write({a: 1}, "application/json"),
  plain: write("hello", "text/plain"),
  yamlParsed: read("name: Ana\\nroles:\\n  - admin\\n", "application/yaml").roles[0],
  yamlWritten: write({name: "Ana", enabled: true}, "application/yaml")
}
"""

    result = runtime.execute(script, {})

    assert json.loads(result) == {
        "jsonParsed": 1,
        "jsonWritten": '{"a":1}',
        "plain": "hello",
        "yamlParsed": "admin",
        "yamlWritten": "name: Ana\nenabled: true\n",
    }
    assert runtime._rust_runtime.last_execution_engine() == "rust-core"


def test_rust_bridge_executes_markdown_output_and_write_in_core() -> None:
    runtime = DataWeaveRuntime(backend="rust")

    rendered = runtime.execute(
        """%dw 2.0
output text/markdown
---
[
  { name: "Jane", age: 30 },
  { name: "Bob", age: 25 }
]
""",
        {},
    )

    assert "| name   | age   |" in rendered
    assert "| Jane   | 30    |" in rendered
    assert "| Bob    | 25    |" in rendered
    assert runtime._rust_runtime.last_execution_engine() == "rust-core"

    written = runtime.execute(
        """%dw 2.0
output application/python
import write from dw::Core
---
write([{name: "Jane", age: 30}], "text/markdown")
""",
        {},
    )

    assert "| name   | age   |" in written
    assert "| Jane   | 30    |" in written
    assert runtime._rust_runtime.last_execution_engine() == "rust-core"


def test_rust_bridge_executes_core_math_and_mime_helpers_in_core() -> None:
    runtime = DataWeaveRuntime()
    script = """%dw 2.0
import * from dw::Core
import * from dw::util::Math
import * from dw::core::Numbers
import * from dw::module::Mime
output application/json
---
{
  uuidLength: sizeOf(uuid()),
  root: sqrt(25),
  powered: pow(2, 3),
  modded: mod(7, 4),
  sinZero: sin(0),
  cosZero: cos(0),
  tanZero: tan(0),
  fromHex: fromRadixNumber("ff", 16),
  toHex: toRadixNumber(255, 16),
  zipped: zip([1, 2], ["a", "b", "c"]),
  unzipped: unzip([[0, "a"], [1, "b"], [2, "c"]]),
  top: maxBy([{name: "A", score: 1}, {name: "B", score: 3}], (item) -> item.score).name,
  bottom: minBy([{name: "A", score: 1}, {name: "B", score: 3}], (item) -> item.score).name,
  chained: 1 then ((value) -> value + 1),
  fallback: null onNull (() -> "empty"),
  mime: fromString("application/json"),
  mimeText: toString({type: "application", subtype: "json", parameters: {}}),
  handled: isHandledBy({type: "application", subtype: "*", parameters: {}}, {type: "application", subtype: "json", parameters: {}})
}
"""

    result = json.loads(runtime.execute(script, {}))

    assert result["uuidLength"] == 36
    assert result["root"] == 5
    assert result["powered"] == 8
    assert result["modded"] == 3
    assert result["sinZero"] == 0
    assert result["cosZero"] == 1
    assert result["tanZero"] == 0
    assert result["fromHex"] == 255
    assert result["toHex"] == "ff"
    assert result["zipped"] == [[1, "a"], [2, "b"]]
    assert result["unzipped"] == [[0, 1, 2], ["a", "b", "c"]]
    assert result["top"] == "B"
    assert result["bottom"] == "A"
    assert result["chained"] == 2
    assert result["fallback"] == "empty"
    assert result["mime"] == {
        "result": {"parameters": {}, "subtype": "json", "type": "application"},
        "success": True,
    }
    assert result["mimeText"] == "application/json"
    assert result["handled"] is True
    assert runtime._rust_runtime.last_execution_engine() == "rust-core"


def test_rust_bridge_executes_header_vars_in_core() -> None:
    runtime = DataWeaveRuntime()
    script = """%dw 2.0
output application/json
var greeting = upper("hello")
var captured = vars.requestTime default "missing"
var summary = greeting ++ " " ++ payload.name
---
{
  message: summary,
  captured: captured
}
"""

    result = runtime.execute(
        script,
        {"name": "DW"},
        vars={"requestTime": "2024-05-05T12:00:00Z"},
    )

    assert json.loads(result) == {
        "message": "HELLO DW",
        "captured": "2024-05-05T12:00:00Z",
    }
    assert runtime._rust_runtime.last_execution_engine() == "rust-core"


def test_rust_bridge_renders_safe_json_output_in_core() -> None:
    runtime = DataWeaveRuntime()
    script = """%dw 2.0
output application/json
---
{
  id: payload.id,
  status: upper(payload.status default "pending"),
  tags: payload.tags,
  merged: payload.user ++ {password: "****"},
  messages: payload.users.message
}
"""

    result = runtime.execute(
        script,
        {
            "id": 7,
            "status": "ok",
            "tags": ["a", "b"],
            "user": {"name": "A", "password": "1234"},
            "users": [{"message": "Hi"}, {"other": 1}],
        },
    )

    assert json.loads(result) == {
        "id": 7,
        "status": "OK",
        "tags": ["a", "b"],
        "merged": {"name": "A", "password": "****"},
        "messages": ["Hi"],
    }
    assert runtime._rust_runtime.last_execution_engine() == "rust-core"


def test_rust_bridge_renders_duplicate_key_json_in_core() -> None:
    runtime = DataWeaveRuntime()
    script = """%dw 2.0
output application/json
---
{
  a: 2,
  a: 3
}
"""

    assert runtime.execute(script, {}) == '{"a":2,"a":3}'
    assert runtime._rust_runtime.last_execution_engine() == "rust-core"


def test_rust_bridge_executes_json_writer_indent_in_core() -> None:
    runtime = DataWeaveRuntime()
    script = """%dw 2.0
output application/json indent=2
---
{
  id: payload.id,
  tags: payload.tags
}
"""

    result = runtime.execute(script, {"id": 1, "tags": ["a", "b"]})

    assert json.loads(result) == {"id": 1, "tags": ["a", "b"]}
    assert result.count("\n") > 1
    assert runtime._rust_runtime.last_execution_engine() == "rust-core"


def test_rust_bridge_executes_csv_writer_in_core() -> None:
    runtime = DataWeaveRuntime()
    script = """%dw 2.0
output application/csv separator=";" header=true
---
payload
"""

    result = runtime.execute(script, [{"name": "Ann", "age": 20}])

    assert result == "name;age\nAnn;20\n"
    assert runtime._rust_runtime.last_execution_engine() == "rust-core"


def test_rust_bridge_parses_nested_yaml_payload_in_core() -> None:
    runtime = DataWeaveRuntime(backend="rust")
    script = """%dw 2.0
output application/python
---
{
  customer: upper(payload.order.customer.name),
  firstSku: payload.order.items[0].sku,
  totalItems: sizeOf(payload.order.items)
}
"""

    result = runtime.execute(
        script,
        """order:
  customer:
    name: mule
  items:
    - sku: A-1
      quantity: 2
    - sku: B-2
      quantity: 1
""",
        payload_format="application/yaml",
    )

    assert result == {"customer": "MULE", "firstSku": "A-1", "totalItems": 2}
    assert runtime._rust_runtime.last_execution_engine() == "rust-core"


def test_rust_bridge_parses_xml_payload_in_core() -> None:
    runtime = DataWeaveRuntime()
    script = """%dw 2.0
output application/json
---
{title: payload.catalog.book.title}
"""

    result = runtime.execute(
        script,
        "<catalog><book><title>DW</title></book></catalog>",
        payload_format="application/xml",
    )

    assert json.loads(result) == {"title": "DW"}
    assert runtime._rust_runtime.last_execution_engine() == "rust-core"


def test_rust_bridge_renders_xml_output_in_core() -> None:
    runtime = DataWeaveRuntime()
    script = """%dw 2.0
output application/xml
---
{user: {"@id": "123", "#text": "Max"}}
"""

    result = runtime.execute(script, {})

    assert result == '<user id="123">Max</user>'
    assert runtime._rust_runtime.last_execution_engine() == "rust-core"


def test_rust_bridge_reads_and_writes_xml_attributes_in_core() -> None:
    runtime = DataWeaveRuntime()
    script = """%dw 2.0
output application/xml
---
{customer: (payload.root.customer -- {"@secret": payload.root.customer["@secret"]})}
"""

    result = runtime.execute(
        script,
        '<root><customer id="7" secret="x">Ada</customer></root>',
        payload_format="application/xml",
    )

    assert result == '<customer id="7">Ada</customer>'
    assert runtime._rust_runtime.last_execution_engine() == "rust-core"


def test_rust_bridge_parses_text_payload_formats_in_core() -> None:
    runtime = DataWeaveRuntime()

    assert runtime.execute(
        "%dw 2.0\noutput application/python\n---\nupper(payload.name)",
        '{"name": "mule"}',
        payload_format="application/json",
    ) == "MULE"
    assert runtime._rust_runtime.last_execution_engine() == "rust-core"

    assert runtime.execute(
        "%dw 2.0\noutput application/python\n---\npayload.name",
        "name: Ana",
        payload_format="yaml",
    ) == "Ana"
    assert runtime._rust_runtime.last_execution_engine() == "rust-core"

    assert runtime.execute(
        "%dw 2.0\noutput application/python\n---\npayload map ((item) -> item.city)",
        "name,city\nAna,London\nBob,Berlin",
        payload_format="application/csv",
    ) == ["London", "Berlin"]
    assert runtime._rust_runtime.last_execution_engine() == "rust-core"

    assert runtime.execute(
        "%dw 2.0\noutput application/python\n---\npayload[0].city",
        "name;city\nAna;Lisbon",
        payload_format="csv",
        payload_format_options={"separator": ";"},
    ) == "Lisbon"
    assert runtime._rust_runtime.last_execution_engine() == "rust-core"

    assert runtime.execute(
        "%dw 2.0\noutput application/python\n---\npayload[1][1]",
        "name;city\nAna;Lisbon",
        payload_format="csv",
        payload_format_options={"separator": ";", "header": False},
    ) == "Lisbon"
    assert runtime._rust_runtime.last_execution_engine() == "rust-core"

    assert runtime.execute(
        "%dw 2.0\noutput application/python\n---\npayload[0].city",
        "name|city\nAna|'Lisbon'",
        payload_format="csv",
        payload_format_options={"separator": "|", "quote": "'"},
    ) == "Lisbon"
    assert runtime._rust_runtime.last_execution_engine() == "rust-core"

    assert runtime.execute(
        "%dw 2.0\noutput application/python\n---\npayload map ((item) -> item.city)",
        "| name | city |\n|:---|:---|\n| Ana | London |",
        payload_format="text/markdown",
    ) == ["London"]
    assert runtime._rust_runtime.last_execution_engine() == "rust-core"

    assert runtime.execute(
        "%dw 2.0\noutput application/python\n---\npayload[0][1]",
        "| name | city |\n|:---|:---|\n| Ana | London |",
        payload_format="markdown",
        payload_format_options={"header": False},
    ) == "London"
    assert runtime._rust_runtime.last_execution_engine() == "rust-core"


def test_rust_bridge_renders_yaml_writer_options_in_core() -> None:
    runtime = DataWeaveRuntime()
    script = """%dw 2.0
output application/yaml skipNullOn="everywhere" writeDeclaration=true
---
{
  keep: [1, null, 2],
  remove: null,
  nested: { remove: null, values: [null, "yes"] }
}
"""

    result = runtime.execute(script, {})

    assert result.startswith("---\n")
    assert "remove" not in result
    assert "null" not in result
    assert runtime._rust_runtime.last_execution_engine() == "rust-core"


def test_rust_bridge_executes_one_line_output_script_in_core() -> None:
    runtime = DataWeaveRuntime(backend="rust")

    result = runtime.execute("output application/yaml --- payload", {"name": "dw"})

    assert "name: dw" in result
    assert runtime._rust_runtime.last_execution_engine() == "rust-core"


def test_rust_bridge_dispatches_typed_overloads_in_core() -> None:
    runtime = DataWeaveRuntime()
    script = """%dw 2.0
output text/plain
fun hola(obj: String) = "hello!"
fun hola(obj: Number, level = 0) = "hi"
---
hola("") ++ hola(1)
"""

    assert runtime.execute(script, {}) == "hello!hi"
    assert runtime._rust_runtime.last_execution_engine() == "rust-core"


def test_rust_bridge_executes_csv_group_by_map_object_in_core() -> None:
    runtime = DataWeaveRuntime()
    script = """%dw 2.0
output application/json
---
(payload map {
  "business_unit" : $.NWMCU_BusinessUnit,
  "business_name" : $.NWMCU_BusinessUnit_BU_Desc,
  "unit_number" : trim($.NWUNIT_UnitNo),
  "sq_ft" : $.NWPMU1_Units_CALC,
} groupBy ((value, key) -> value.business_name))
mapObject ((value, key, index) -> {
    "$(key)" : value
})
"""

    result = runtime.execute(
        script,
        "NWMCU_BusinessUnit,NWMCU_BusinessUnit_BU_Desc,NWUNIT_UnitNo,NWPMU1_Units_CALC\n"
        "8751591,HGIT Liverpool Limited-US GAAP,1,102302\n"
        "8751591,HGIT Liverpool Limited-US GAAP,7",
        payload_format="application/csv",
    )

    assert json.loads(result) == {
        "HGIT Liverpool Limited-US GAAP": [
            {
                "business_unit": "8751591",
                "business_name": "HGIT Liverpool Limited-US GAAP",
                "unit_number": "1",
                "sq_ft": "102302",
            },
            {
                "business_unit": "8751591",
                "business_name": "HGIT Liverpool Limited-US GAAP",
                "unit_number": "7",
                "sq_ft": None,
            },
        ]
    }
    assert runtime._rust_runtime.last_execution_engine() == "rust-core"


def test_rust_bridge_exposes_type_descriptor_for_python_inference() -> None:
    runtime = DataWeaveRuntime()
    descriptor = runtime._rust_runtime.infer_type_descriptor(
        "{ name: payload.user.name, tags: payload.user.tags }",
        payload={"user": {"name": "Mule", "tags": ["dw"]}},
    )

    assert descriptor["kind"] == "Object"
    assert descriptor["fields"]["name"]["type"]["kind"] == "String"
    assert descriptor["fields"]["tags"]["type"]["kind"] == "Array"

    inferred = infer_script_type(
        "{ name: payload.user.name, tags: payload.user.tags }",
        payload_type={"user": {"name": "Mule", "tags": ["dw"]}},
    )
    assert isinstance(inferred, ObjectType)
    assert inferred.field_dict()["name"][0] == STRING
    assert isinstance(inferred.field_dict()["tags"][0], ArrayType)
    assert inferred.field_dict()["tags"][0].element == STRING


def test_rust_bridge_executes_string_collection_helpers_in_core() -> None:
    runtime = DataWeaveRuntime()
    script = """%dw 2.0
output application/python
---
{
  infixContains: payload.list contains 3,
  prefixContains: contains(payload.list, 3),
  objectContainsKey: payload.object contains "a",
  objectContainsValue: payload.object contains 2,
  infixJoin: payload.words joinBy "-",
  prefixJoin: joinBy(payload.words, "-"),
  infixSplit: payload.phrase splitBy "-",
  prefixSplit: splitBy(payload.phrase, "-"),
  starts: payload.text startsWith "ba",
  ends: endsWith(payload.text, "na"),
  foundText: payload.text find "na",
  prefixGroup: groupBy(payload.objects, (item) -> item.language)
}
"""

    assert runtime.execute(
        script,
        {
            "list": [1, 2, 3],
            "object": {"a": 1, "b": 2},
            "words": ["a", "b", "c"],
            "phrase": "a-b-c",
            "text": "banana",
            "objects": [
                {"name": "Foo", "language": "Java"},
                {"name": "Bar", "language": "Scala"},
                {"name": "FooBar", "language": "Java"},
            ],
        },
        render_output=False,
    ) == {
        "infixContains": True,
        "prefixContains": True,
        "objectContainsKey": True,
        "objectContainsValue": True,
        "infixJoin": "a-b-c",
        "prefixJoin": "a-b-c",
        "infixSplit": ["a", "b", "c"],
        "prefixSplit": ["a", "b", "c"],
        "starts": True,
        "ends": True,
        "foundText": [2, 4],
        "prefixGroup": {
            "Java": [
                {"name": "Foo", "language": "Java"},
                {"name": "FooBar", "language": "Java"},
            ],
            "Scala": [{"name": "Bar", "language": "Scala"}],
        },
    }
    assert runtime._rust_runtime.last_execution_engine() == "rust-core"


def test_rust_bridge_smoke_json_identity() -> None:
    runtime = DataWeaveRuntime()
    native = runtime._rust_runtime

    result = native.execute_smoke_json("%dw 2.0\n---\npayload", '{"id":7}')

    assert json.loads(result) == {"id": 7}
