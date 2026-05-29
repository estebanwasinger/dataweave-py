import json
from pathlib import Path
import textwrap
from datetime import date, datetime, timedelta, timezone

import pandas as pd
import pytest
import yaml

import dwpy.parser as parser
from dwpy.runtime import DataWeaveRuntime, DataWeaveEvaluationError


class PythonResultRuntime(DataWeaveRuntime):
    def execute(self, *args, **kwargs):
        kwargs.setdefault("render_output", False)
        return super().execute(*args, **kwargs)


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
    result = runtime.execute(script, payload, render_output=False)

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
    result = runtime.execute(script, payload={}, render_output=False)

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
    result = runtime.execute(script, payload={}, render_output=False)

    assert result == {
        "message": "HELLO WORLD",
    }


def test_header_var_and_function_bodies_can_start_on_following_lines():
    script = """%dw 2.0
output application/json
var values =
  if (isEmpty(payload.values default [])) []
  else payload.values default []

fun average(items) =
  if (isEmpty(items)) null
  else round((sum(items) / sizeOf(items)) * 100) / 100
---
{
  values: values,
  average: average(values)
}
"""

    runtime = DataWeaveRuntime()
    result = runtime.execute(script, payload={"values": [4, 6]}, render_output=False)

    assert result == {
        "values": [4, 6],
        "average": 5,
    }


def test_header_var_preserves_left_associative_infix_chains():
    script = """%dw 2.0
output application/json
var values =
  (payload.values default [])
    map ((value) -> value * 2)
    filter ((value) -> value >= 4)
---
values
"""

    runtime = DataWeaveRuntime()
    result = runtime.execute(script, payload={"values": [1, 2, 3]}, render_output=False)

    assert result == [4, 6]


def test_logical_operators_have_lower_precedence_than_comparisons():
    script = """%dw 2.0
output application/json
---
{
  boxed: if (payload.bbox == null or sizeOf(payload.bbox) < 4) null else payload.bbox[1],
  eligible: payload.count > 2 and payload.enabled == true
}
"""

    runtime = DataWeaveRuntime()
    result = runtime.execute(
        script,
        payload={"bbox": [32.7, 32.8, 791.8, 609.2], "count": 3, "enabled": True},
        render_output=False,
    )

    assert result == {"boxed": 32.8, "eligible": True}


def test_pluck_map_transform_can_pass_array_selector_to_function_with_logical_guard():
    script = """%dw 2.0
output application/json

fun toBox(bbox) =
  if (bbox == null or sizeOf(bbox) < 4)
    null
  else
    {
      top: round(bbox[1] as Number),
      left: round(bbox[0] as Number),
      bottom: round(bbox[3] as Number),
      right: round(bbox[2] as Number)
    }

---
flatten(
  payload pluck ((blocks, pageKey) ->
    blocks map ((block, index) -> {
      pageIdx: (block.page_idx default pageKey) as Number,
      blockIdx: ((block.blockIdx default index) as Number) + 1,
      text: block.text default "",
      box: toBox(block.bbox)
    })
  )
)
"""
    payload = {
        "0": [
            {
                "bbox": [
                    32.74958388910061,
                    32.85486091427686,
                    791.8984155868903,
                    609.2496515339177,
                ],
                "page_idx": 0,
                "blockIdx": "0",
                "text": "Photograph of the Exchange at Crestview apartment complex with the Cushman & Wakefield logo in the top right corner.",
            },
            {
                "bbox": [44.64, 40.514, 342.632, 90.51400000000001],
                "page_idx": 0,
                "blockIdx": "1",
                "text": "# EXCHANGE AT",
            },
        ],
    }

    runtime = DataWeaveRuntime()
    result = runtime.execute(script, payload=payload, render_output=False)

    assert result == [
        {
            "pageIdx": 0,
            "blockIdx": 1,
            "text": "Photograph of the Exchange at Crestview apartment complex with the Cushman & Wakefield logo in the top right corner.",
            "box": {"top": 33, "left": 33, "bottom": 609, "right": 792},
        },
        {
            "pageIdx": 0,
            "blockIdx": 2,
            "text": "# EXCHANGE AT",
            "box": {"top": 41, "left": 45, "bottom": 91, "right": 343},
        },
    ]


def test_group_by_accepts_implicit_placeholder_lambda():
    script = """%dw 2.0
output application/json
---
payload.items groupBy $.kind
"""

    runtime = DataWeaveRuntime()
    result = runtime.execute(
        script,
        payload={
            "items": [
                {"kind": "A", "value": 1},
                {"kind": "B", "value": 2},
                {"kind": "A", "value": 3},
            ]
        },
        render_output=False,
    )

    assert result == {
        "A": [
            {"kind": "A", "value": 1},
            {"kind": "A", "value": 3},
        ],
        "B": [
            {"kind": "B", "value": 2},
        ],
    }


def test_object_literal_duplicate_keys_are_preserved_in_json_output():
    script = """%dw 2.0
output application/json
---
{
  a: 2,
  a: 3
}
"""
    runtime = DataWeaveRuntime()
    result = runtime.execute(script, payload={})

    assert result == '{"a":2,"a":3}'


def test_nested_object_literal_duplicate_keys_are_preserved_in_json_output():
    script = """%dw 2.0
output application/json
---
{
  outer: {
    a: 2,
    a: 3
  }
}
"""
    runtime = DataWeaveRuntime()
    result = runtime.execute(script, payload={})

    assert result == '{"outer":{"a":2,"a":3}}'


def test_do_block_inside_function_returns_local_value():
    script = """%dw 2.0
output application/json
fun myfun() = do {
    var name = "DataWeave"
    ---
    name
}
---
{ result: myfun() }
"""
    runtime = DataWeaveRuntime()
    result = runtime.execute(script, payload={}, render_output=False)

    assert result == {"result": "DataWeave"}


def test_do_block_inside_header_variable_returns_local_value():
    script = """%dw 2.0
output application/json
var myVar = do {
    var name = "DataWeave"
    ---
    name
}
---
{ result: myVar }
"""
    runtime = DataWeaveRuntime()
    result = runtime.execute(script, payload={}, render_output=False)

    assert result == {"result": "DataWeave"}


def test_do_block_can_be_nested_inside_object_fields():
    script = """%dw 2.0
output application/json
---
{
  result: do {
    var name = "DataWeave"
    ---
    name
  }
}
"""
    runtime = DataWeaveRuntime()
    result = runtime.execute(script, payload={}, render_output=False)

    assert result == {"result": "DataWeave"}


def test_script_delimiter_ignores_block_comments():
    script = """%dw 2.0
output application/json
/*
---
*/
var greeting = "DataWeave"
---
{
  result: greeting
}
"""
    runtime = DataWeaveRuntime()
    result = runtime.execute(script, payload={}, render_output=False)

    assert result == {"result": "DataWeave"}


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
    result = runtime.execute(script, payload={}, render_output=False)

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
        render_output=False,
    )

    assert result["id"] == "A123"
    assert result["status"] == "CONFIRMED"
    assert result["generatedAt"] == "2024-05-05T12:00:00Z"

    fallback = runtime.execute(script, payload=payload, vars={}, render_output=False)
    assert fallback["generatedAt"].endswith("Z")


def test_now_coerced_as_date_renders_date_only():
    script = """%dw 2.0
output application/json
---
now() as Date
"""
    runtime = DataWeaveRuntime()
    before = datetime.now(timezone.utc).date().isoformat()
    result = json.loads(runtime.execute(script, payload={}))
    after = datetime.now(timezone.utc).date().isoformat()
    assert result in {before, after}


def test_temporal_string_coercion_honors_format_option():
    script = """%dw 2.0
output application/json
---
{
  formattedDate: |2020-10-01T23:57:59| as String {format: "uuuu-MM-dd"},
  formattedTime: |2020-10-01T23:57:59| as String {format: "KK:mm:ss a"},
  formattedDateTime: |2020-10-01T23:57:59| as String {format: "KK:mm:ss a, MMMM dd, uuuu"}
}
"""
    runtime = DataWeaveRuntime()
    result = json.loads(runtime.execute(script, payload={}))

    assert result == {
        "formattedDate": "2020-10-01",
        "formattedTime": "11:57:59 PM",
        "formattedDateTime": "11:57:59 PM, October 01, 2020",
    }


def test_date_arithmetic_supports_period_pipe_literals():
    script = """%dw 2.0
output application/json
var base = now() as Date
---
{
  today: base,
  tomorrow: base + |P1D|,
  nextWeek: base + |P7D|
}
"""
    runtime = DataWeaveRuntime()
    result = json.loads(runtime.execute(script, payload={}))

    today = date.fromisoformat(result["today"])
    tomorrow = date.fromisoformat(result["tomorrow"])
    next_week = date.fromisoformat(result["nextWeek"])

    assert tomorrow == (today + timedelta(days=1))
    assert next_week == (today + timedelta(days=7))


def test_payload_accepts_json_string_when_format_specified():
    script = """%dw 2.0
output application/python
---
{
  upper: upper(payload.name)
}
"""
    payload = '{"name": "mule"}'
    runtime = DataWeaveRuntime()

    result = runtime.execute(
        script,
        payload=payload,
        payload_format="application/json",
    )

    assert result == {"upper": "MULE"}


def test_payload_accepts_dataframe_input():
    script = """%dw 2.0
output application/json
---
payload map ((item) -> {
  identifier: item.id,
  city: item.city
})
"""
    payload = pd.DataFrame(
        [
            {"id": 1, "city": "London"},
            {"id": 2, "city": "Berlin"},
        ]
    )

    runtime = DataWeaveRuntime()
    result = runtime.execute(script, payload=payload, render_output=False)

    assert result == [
        {"identifier": 1, "city": "London"},
        {"identifier": 2, "city": "Berlin"},
    ]


def test_vars_accept_dataframe_inputs():
    script = """%dw 2.0
output application/json
---
{
  names: vars.source map ((item) -> upper(item.name))
}
"""
    vars_df = pd.DataFrame(
        [
            {"name": "alice"},
            {"name": "Bob"},
        ]
    )

    runtime = DataWeaveRuntime()
    result = runtime.execute(script, payload={}, vars={"source": vars_df}, render_output=False)

    assert result["names"] == ["ALICE", "BOB"]


def test_output_directive_serialises_to_json():
    script = """%dw 2.0
output application/json indent=2
---
{
  id: payload.id,
  tags: payload.tags
}
"""
    runtime = DataWeaveRuntime()
    payload = {"id": 1, "tags": ["a", "b"]}

    raw = runtime.execute(script, payload=payload)
    parsed = json.loads(raw)

    assert parsed == payload
    assert raw.count("\n") > 1  # pretty printed


def test_output_directive_returns_python_when_requested():
    script = """%dw 2.0
output application/python
---
payload
"""
    runtime = PythonResultRuntime()
    payload = {"a": 1}

    result = runtime.execute(script, payload=payload)

    assert result == payload


def test_output_csv_with_custom_separator():
    script = """%dw 2.0
output application/csv separator=";" header=true
---
[
  { name: "Jane", age: 30 },
  { name: "Bob", age: 25 }
]
"""
    runtime = DataWeaveRuntime()
    payload = {}

    csv_data = runtime.execute(script, payload=payload)

    assert "name;age" in csv_data
    assert "Jane;30" in csv_data


def test_payload_accepts_yaml_string_when_format_specified():
    script = """%dw 2.0
output application/python
---
{
  customer: upper(payload.order.customer.name),
  firstSku: payload.order.items[0].sku,
  totalItems: sizeOf(payload.order.items)
}
"""
    payload = textwrap.dedent(
        """\
        order:
          customer:
            name: mule
          items:
            - sku: A-1
              quantity: 2
            - sku: B-2
              quantity: 1
        """
    )
    runtime = PythonResultRuntime()

    result = runtime.execute(
        script,
        payload=payload,
        payload_format="application/yaml",
    )

    assert result == {"customer": "MULE", "firstSku": "A-1", "totalItems": 2}


def test_payload_yaml_aliases_are_supported():
    script = """%dw 2.0
output application/python
---
payload.name
"""
    runtime = PythonResultRuntime()

    assert runtime.execute(script, payload="name: Ana", payload_format="yaml") == "Ana"
    assert runtime.execute(script, payload="name: Bob", payload_format="text/yaml") == "Bob"


def test_output_directive_serialises_to_yaml():
    script = """%dw 2.0
output application/yaml
---
{
  name: "Jane",
  active: true,
  tags: ["a", "b"]
}
"""
    runtime = DataWeaveRuntime()

    raw = runtime.execute(script, payload={})

    assert yaml.safe_load(raw) == {"name": "Jane", "active": True, "tags": ["a", "b"]}
    assert "name: Jane" in raw


def test_output_yaml_short_format_directive():
    script = """%dw 2.0
output yaml
---
{ name: "Jane" }
"""
    runtime = DataWeaveRuntime()

    raw = runtime.execute(script, payload={})

    assert yaml.safe_load(raw) == {"name": "Jane"}


def test_read_and_write_support_yaml_format():
    script = """%dw 2.0
output application/python
import read, write from dw::Core
---
{
  parsed: read("name: Ana\\nroles:\\n  - admin\\n", "application/yaml").roles[0],
  written: write({name: "Ana", enabled: true}, "application/yaml")
}
"""
    runtime = PythonResultRuntime()

    result = runtime.execute(script, payload={})

    assert result["parsed"] == "admin"
    assert yaml.safe_load(result["written"]) == {"name": "Ana", "enabled": True}


def test_output_yaml_skip_null_on_objects():
    script = """%dw 2.0
output application/yaml skipNullOn="objects"
---
{
  keep: [1, null, 2],
  remove: null,
  nested: { remove: null, keep: "yes" }
}
"""
    runtime = DataWeaveRuntime()

    raw = runtime.execute(script, payload={})

    assert yaml.safe_load(raw) == {"keep": [1, None, 2], "nested": {"keep": "yes"}}


def test_output_yaml_skip_null_on_arrays():
    script = """%dw 2.0
output application/yaml skipNullOn="arrays"
---
{
  keep: [1, null, 2],
  retained: null,
  nested: { retained: null, values: [null, "yes"] }
}
"""
    runtime = DataWeaveRuntime()

    raw = runtime.execute(script, payload={})

    assert yaml.safe_load(raw) == {
        "keep": [1, 2],
        "retained": None,
        "nested": {"retained": None, "values": ["yes"]},
    }


def test_output_yaml_skip_null_on_everywhere_and_declaration():
    script = """%dw 2.0
output application/yaml skipNullOn="everywhere" writeDeclaration=true
---
{
  keep: [1, null, 2],
  remove: null,
  nested: { remove: null, values: [null, "yes"] }
}
"""
    runtime = DataWeaveRuntime()

    raw = runtime.execute(script, payload={})

    assert raw.startswith("---\n")
    assert yaml.safe_load(raw) == {"keep": [1, 2], "nested": {"values": ["yes"]}}


def test_payload_yaml_invalid_input_raises_evaluation_error():
    script = """%dw 2.0
output application/python
---
payload
"""
    runtime = PythonResultRuntime()

    with pytest.raises(DataWeaveEvaluationError) as exc:
        runtime.execute(script, payload="name: [unterminated", payload_format="application/yaml")

    assert "Failed to parse input as yaml" in str(exc.value)


def test_output_plain_text_returns_string_verbatim():
    script = """%dw 2.0
output text/plain
---
"hello"
"""
    runtime = DataWeaveRuntime()

    assert runtime.execute(script, payload={}) == "hello"


def test_output_plain_text_rejects_non_string_values():
    script = """%dw 2.0
output text/plain
---
{ greeting: "hello" }
"""
    runtime = DataWeaveRuntime()

    with pytest.raises(DataWeaveEvaluationError) as exc:
        runtime.execute(script, payload={})

    assert "Plain text writer expects a string value" in str(exc.value)


def test_output_markdown_from_array_of_objects():
    script = """%dw 2.0
output text/markdown
---
[
  { name: "Jane", age: 30 },
  { name: "Bob", age: 25 }
]
"""
    runtime = DataWeaveRuntime()
    result = runtime.execute(script, payload={})

    assert "| name   | age   |" in result
    assert "| Jane   | 30    |" in result
    assert "| Bob    | 25    |" in result


def test_output_markdown_from_single_object():
    script = """%dw 2.0
output text/markdown
---
{ name: "Jane", age: 30 }
"""
    runtime = DataWeaveRuntime()
    result = runtime.execute(script, payload={})

    assert "| name   | age   |" in result
    assert "| Jane   | 30    |" in result


def test_output_markdown_rejects_header_false():
    script = """%dw 2.0
output text/markdown header=false
---
[
  { name: "Jane", age: 30 }
]
"""
    runtime = DataWeaveRuntime()

    with pytest.raises(DataWeaveEvaluationError) as exc:
        runtime.execute(script, payload={})

    assert "Markdown writer requires header=true" in str(exc.value)


def test_output_markdown_rejects_scalar_values():
    script = """%dw 2.0
output text/markdown
---
"hello"
"""
    runtime = DataWeaveRuntime()

    with pytest.raises(DataWeaveEvaluationError) as exc:
        runtime.execute(script, payload={})

    assert "Markdown writer expects a list or dict value" in str(exc.value)


def test_write_supports_plain_and_markdown_formats():
    script = """%dw 2.0
output application/python
import * from dw::Core
---
{
  plain: write("hello", "text/plain"),
  markdown: write([{name: "Jane", age: 30}], "text/markdown")
}
"""
    runtime = PythonResultRuntime()
    result = runtime.execute(script, payload={})

    assert result["plain"] == "hello"
    assert "| name   | age   |" in result["markdown"]
    assert "| Jane   | 30    |" in result["markdown"]


def test_write_plain_rejects_non_string_values():
    script = """%dw 2.0
output application/python
import * from dw::Core
---
write(1, "text/plain")
"""
    runtime = PythonResultRuntime()

    with pytest.raises(DataWeaveEvaluationError) as exc:
        runtime.execute(script, payload={})

    assert "Plain text writer expects a string value" in str(exc.value)


def test_payload_csv_parsing_with_header():
    script = """%dw 2.0
output application/python
---
payload map ((item) -> item.city)
"""
    csv_input = textwrap.dedent(
        """\
        name,city
        Ana,London
        Bob,Berlin
        """
    ).strip()
    runtime = PythonResultRuntime()

    result = runtime.execute(
        script,
        payload=csv_input,
        payload_format="application/csv",
    )

    assert result == ["London", "Berlin"]


def test_payload_csv_respects_separator_option():
    script = """%dw 2.0
output application/python
---
payload[0].city
"""
    csv_input = textwrap.dedent(
        """\
        name;city
        Ana;Lisbon
        """
    ).strip()
    runtime = PythonResultRuntime()

    result = runtime.execute(
        script,
        payload=csv_input,
        payload_format="csv",
        payload_format_options={"separator": ";"},
    )

    assert result == "Lisbon"


def test_payload_markdown_parsing_with_header():
    script = """%dw 2.0
output application/python
---
payload map ((item) -> item.city)
"""
    markdown_input = textwrap.dedent(
        """\
        | name | city   |
        |:-----|:-------|
        | Ana  | London |
        | Bob  | Berlin |
        """
    ).strip()
    runtime = PythonResultRuntime()

    result = runtime.execute(
        script,
        payload=markdown_input,
        payload_format="text/markdown",
    )

    assert result == ["London", "Berlin"]


def test_payload_markdown_parsing_without_header_returns_rows():
    script = """%dw 2.0
output application/python
---
payload[0][1]
"""
    markdown_input = textwrap.dedent(
        """\
        | name | city   |
        |:-----|:-------|
        | Ana  | London |
        | Bob  | Berlin |
        """
    ).strip()
    runtime = PythonResultRuntime()

    result = runtime.execute(
        script,
        payload=markdown_input,
        payload_format="markdown",
        payload_format_options={"header": False},
    )

    assert result == "London"


def test_payload_xml_parsing_with_wildcard_selection():
    runtime = DataWeaveRuntime()
    script = """%dw 2.0
output application/json
type Currency = String { format: "\\$#,###.00" }
---
{
  books: (payload.items.*item) map ((item) -> {
    book: {
      price: item.price as Currency
    }
  })
}
"""
    xml_payload = """<items>
    <item>
        <price>22.30</price>
    </item>
    <item>
        <price>20.31</price>
    </item>
</items>"""

    result = runtime.execute(
        script,
        payload=xml_payload,
        payload_format="application/xml",
    )
    parsed = json.loads(result)
    prices = [entry["book"]["price"] for entry in parsed["books"]]
    assert prices == ["22.30", "20.31"]


XML_TREE_PAYLOAD = """<root>
    <name>John</name>
    <children>
        <name a="b">Jane</name>
        <name>John</name>
    </children>
</root>"""


def test_xml_attribute_access_returns_first_attribute_value():
    runtime = PythonResultRuntime()
    script = """%dw 2.0
---
payload.root.children.name.@a
"""
    result = runtime.execute(
        script,
        payload=XML_TREE_PAYLOAD,
        payload_format="application/xml",
    )
    assert result == "b"


def test_xml_property_without_wildcard_returns_first_match():
    runtime = PythonResultRuntime()
    script = """%dw 2.0
---
payload.root.children.name
"""
    result = runtime.execute(
        script,
        payload=XML_TREE_PAYLOAD,
        payload_format="application/xml",
    )
    assert result == "Jane"


def test_xml_children_structure_preserves_entries_for_json_output():
    runtime = DataWeaveRuntime()
    script = """%dw 2.0
output application/json
---
payload.root.children
"""
    raw = runtime.execute(
        script,
        payload=XML_TREE_PAYLOAD,
        payload_format="application/xml",
    )
    assert raw == '{"name":"Jane","name":"John"}'


def test_xml_like_python_mapping_collapses_to_text_value():
    runtime = PythonResultRuntime()
    script = """%dw 2.0
---
payload.root.children.name
"""
    payload = {
        "root": {
            "children": {
                "name": {
                    "@b": "c",
                    "#text": "Jane",
                }
            }
        }
    }
    result = runtime.execute(script, payload=payload)
    assert result == "Jane"


def test_value_set_with_xml_children_returns_flat_strings():
    runtime = DataWeaveRuntime()
    script = """%dw 2.0
output json
import * from dw::core::Objects
---
valueSet(payload.root.children)
"""
    raw = runtime.execute(
        script,
        payload="""<root>
    <name>John</name>
    <children>
        <name b="c">Jane</name>
        <name>John</name>
    </children>
</root>""",
        payload_format="xml",
    )
    assert json.loads(raw) == ["Jane", "John"]


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

    runtime = PythonResultRuntime()
    result = runtime.execute(script, payload=payload)

    assert result["total"] == pytest.approx(40.99, rel=1e-9)


def test_reduce_uses_default_accumulator_for_recursive_array_concat():
    script = """%dw 2.0
output application/json
fun traverse(obj) = [{"name" : obj.name}] ++ (obj.children reduce ((value, accumulator = [] ) -> accumulator ++ traverse(value)))
---
traverse(payload)
"""
    payload = {
        "name": "Some Name",
        "children": [
            {
                "name": "Inner",
                "children": [],
            }
        ],
    }

    result = DataWeaveRuntime().execute(script, payload=payload)

    assert json.loads(result) == [
        {"name": "Some Name"},
        {"name": "Inner"},
    ]


def test_reduce_without_default_accumulator_starts_from_first_item():
    script = """%dw 2.0
output application/json
---
[2, 3, 3] reduce ((item, acc) -> acc * item)
"""

    result = DataWeaveRuntime().execute(script, payload={})

    assert json.loads(result) == 18


def test_defaulted_function_parameter_with_joined_recursive_reduce():
    script = """%dw 2.0
output application/json
fun hola(obj, level = 0) = [{"account" : (0 to level map "-") joinBy "" ++ " " ++ obj.name, }] ++ (obj.children reduce ((value, accumulator = [] ) -> accumulator ++ hola(value, level + 1)))
---
(hola(payload, 0) map [$.account,$$]) reduce ((item, accumulator = "") -> accumulator ++ item[0] ++ " - ID:$(item[1])" ++"\n")
"""
    payload = {
        "name": "Some Name",
        "children": [
            {
                "name": "Inner",
                "children": [],
            }
        ],
    }

    result = DataWeaveRuntime().execute(script, payload=payload)

    assert json.loads(result) == "- Some Name - ID:0\n-- Inner - ID:1\n"


def test_overload_dispatch_uses_arity_before_defaulted_function_fallback():
    script = """%dw 2.0
output text/plain
fun hola(obj) = "hello!"
fun hola(obj, level = 0) = "hi"

---
hola("",0)
"""

    result = DataWeaveRuntime().execute(script, payload={})

    assert result == "hi"


def test_overload_dispatch_uses_type_annotations_with_defaulted_parameters():
    script = """%dw 2.0
output text/plain
fun hola(obj: String) = "hello!"
fun hola(obj: Number, level = 0) = "hi"

---
hola("") ++ hola(1)
"""

    result = DataWeaveRuntime().execute(script, payload={})

    assert result == "hello!hi"


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

    runtime = PythonResultRuntime()
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

    runtime = PythonResultRuntime()
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

    runtime = PythonResultRuntime()
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

    runtime = PythonResultRuntime()
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
    runtime = PythonResultRuntime()

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

    runtime = PythonResultRuntime()
    result = runtime.execute(script, payload=payload)

    assert result["id"] == "IDX-1"
    assert result["status"] == "CONFIRMED"
    assert result["values"] == [4, 6]


def test_quoted_dot_selector_matches_bracket_selector():
    script = """%dw 2.0
output application/json
---
{
  dot: payload."value 2",
  bracket: payload["value 2"]
}
"""
    runtime = PythonResultRuntime()
    result = runtime.execute(script, payload={"value 2": "s"})

    assert result["dot"] == "s"
    assert result["dot"] == result["bracket"]


def test_array_property_selector_keeps_single_match_as_array():
    script = """%dw 2.0
output application/json
---
payload.message
"""
    payload = [{"message": "Hello, World!"}]

    python_runtime = PythonResultRuntime()
    python_result = python_runtime.execute(script, payload=payload)
    assert python_result == ["Hello, World!"]

    json_runtime = DataWeaveRuntime()
    rendered = json_runtime.execute(script, payload=payload)
    assert json.loads(rendered) == ["Hello, World!"]


def test_descendant_selector_collects_array_object_fields():
    script = """%dw 2.0
output application/json
---
payload..name
"""
    payload = [
        {"name": "Esteban"},
        {"name": "Esteban"},
        {"name": "Esteban"},
        {"name": "Esteban"},
        {"name": "Esteban"},
    ]

    runtime = PythonResultRuntime()
    result = runtime.execute(script, payload=payload)

    assert result == ["Esteban", "Esteban", "Esteban", "Esteban", "Esteban"]


def test_descendant_selector_recurses_through_nested_objects_and_arrays():
    script = """%dw 2.0
output application/json
---
payload..name
"""
    payload = {
        "name": "root",
        "users": [
            {"profile": {"name": "alpha"}},
            {"profile": {"name": "beta"}},
        ],
        "meta": {
            "owner": {"name": "gamma"}
        },
    }

    runtime = PythonResultRuntime()
    result = runtime.execute(script, payload=payload)

    assert result == ["root", "alpha", "beta", "gamma"]


def test_descendant_selector_without_attribute_returns_all_child_values():
    script = """%dw 2.0
output application/json
---
payload..
"""
    payload = {
        "user": {
            "name": "Weave",
            "child": {
                "name": "BAT"
            },
        }
    }

    runtime = PythonResultRuntime()
    result = runtime.execute(script, payload=payload)

    assert result == [
        {"name": "Weave", "child": {"name": "BAT"}},
        "Weave",
        {"name": "BAT"},
        "BAT",
    ]


def test_descendant_selector_supports_quoted_attributes():
    script = """%dw 2.0
output application/json
---
payload.."value 2"
"""
    payload = {
        "items": [
            {"value 2": "a"},
            {"nested": {"value 2": "b"}},
        ]
    }

    runtime = PythonResultRuntime()
    result = runtime.execute(script, payload=payload)

    assert result == ["a", "b"]


def test_descendant_multivalue_selector_includes_repeated_xml_siblings():
    runtime = PythonResultRuntime()
    script = """%dw 2.0
---
payload..*name
"""
    payload = """<users><user><name>Weave</name><user><name>BAT</name><name>Munit</name><user><name>BDD</name></user></user></user></users>"""
    result = runtime.execute(script, payload=payload, payload_format="application/xml")
    assert result == ["Weave", "BAT", "Munit", "BDD"]


def test_key_value_selector_returns_object_for_matching_key_pairs():
    runtime = PythonResultRuntime()
    script = """%dw 2.0
output application/json
---
payload.people.&name
"""
    payload = {
        "people": [
            {"name": "Ana"},
            {"name": "Bob"},
            {"age": 30},
        ]
    }
    result = runtime.execute(script, payload=payload)
    assert list(result.items()) == [("name", "Ana"), ("name", "Bob")]


def test_descendant_key_value_selector_returns_array_of_matching_objects():
    runtime = PythonResultRuntime()
    script = """%dw 2.0
output application/json
---
payload..&name
"""
    payload = {
        "people": {
            "person": {
                "name": "Nial",
                "address": {
                    "street": {"name": "Italia"},
                    "area": {"name": "Martinez"},
                },
            }
        }
    }
    result = runtime.execute(script, payload=payload)
    assert [list(item.items()) for item in result] == [
        [("name", "Nial")],
        [("name", "Italia")],
        [("name", "Martinez")],
    ]


def test_key_present_selector_supports_xml_attributes():
    runtime = PythonResultRuntime()
    script = """%dw 2.0
output application/json
---
{
  item: {
    typePresent : payload.product.@."type"?
  }
}
"""
    payload = '<product type="book"/>'
    result = runtime.execute(script, payload=payload, payload_format="application/xml")
    assert result == {"item": {"typePresent": True}}


def test_assert_present_selector_raises_when_missing():
    runtime = DataWeaveRuntime()
    script = """%dw 2.0
output application/json
---
payload.lastName!
"""
    with pytest.raises(DataWeaveEvaluationError) as exc:
        runtime.execute(script, {"name": "Annie"})

    assert "There is no key named 'lastName'" in str(exc.value)


def test_filter_selector_filters_selected_values_and_returns_null_on_empty():
    runtime = PythonResultRuntime()
    script = """%dw 2.0
output application/json
---
{
  selected: payload.users.*name[?($ == "Mariano")],
  missing: payload.users.*name[?($ == "Nobody")]
}
"""
    payload = {"users": [{"name": "Mariano"}, {"name": "Ana"}]}
    result = runtime.execute(script, payload=payload)
    assert result == {"selected": ["Mariano"], "missing": None}


def test_filter_selector_can_filter_scalar_values():
    runtime = PythonResultRuntime()
    script = """%dw 2.0
output application/json
---
{
  first: payload.first[?($=="Mariano")],
  second: payload.second[?($=="Mariano")]
}
"""
    result = runtime.execute(script, payload={"first": "Mariano", "second": "Ana"})
    assert result == {"first": "Mariano", "second": None}


def test_object_literal_can_merge_parenthesized_if_expression():
    runtime = PythonResultRuntime()
    script = """%dw 2.0
output application/json
---
{
  (if (payload.include == true) {name: payload.name} else {})
}
"""
    assert runtime.execute(script, payload={"include": True, "name": "Esteban"}) == {"name": "Esteban"}
    assert runtime.execute(script, payload={"include": False, "name": "Esteban"}) == {}


def test_map_object_literal_can_merge_conditional_object_expression_before_filter():
    runtime = PythonResultRuntime()
    script = """%dw 2.0
output application/json
---
(payload.items map (item, index) -> {
    // Only include the record if successful is false
    (if (item.successful == false) {
        index: index,
        errorCode: log(item).payload.errors[0].statusCode,
        errorMessage: item.payload.errors[0].message
    } else {})
}) filter (!isEmpty($))
"""
    payload = {
        "items": [
            {
                "successful": False,
                "payload": {
                    "errors": [
                        {
                            "statusCode": 404,
                            "message": "Error",
                        }
                    ]
                },
            }
        ]
    }

    result = runtime.execute(script, payload=payload)

    assert result == [
        {
            "index": 0,
            "errorCode": 404,
            "errorMessage": "Error",
        }
    ]


def test_dynamic_multivalue_selector_returns_values():
    runtime = PythonResultRuntime()
    script = """%dw 2.0
output application/json
var key = "name"
---
payload.users[*(key)]
"""
    payload = {"users": [{"name": "Mariano"}, {"name": "Ana"}]}
    result = runtime.execute(script, payload=payload)
    assert result == ["Mariano", "Ana"]


def test_dynamic_key_value_selector_returns_object():
    runtime = PythonResultRuntime()
    script = """%dw 2.0
output application/json
var key = "name"
---
payload.users[&(key)]
"""
    payload = {"users": [{"name": "Mariano"}, {"name": "Ana"}]}
    result = runtime.execute(script, payload=payload)
    assert list(result.items()) == [("name", "Mariano"), ("name", "Ana")]


def test_local_name_selector_matches_namespaced_xml_keys():
    runtime = PythonResultRuntime()
    script = """%dw 2.0
---
payload.root.table
"""
    payload = '<root><h:table xmlns:h="http://www.w3.org/TR/html4/"><h:tr>Apples</h:tr></h:table></root>'
    result = runtime.execute(script, payload=payload, payload_format="application/xml")
    assert result == {"{http://www.w3.org/TR/html4/}tr": "Apples"}


def test_null_safe_quoted_dot_selector_with_default():
    script = """%dw 2.0
---
payload.user?."value 2" default "UNKNOWN"
"""
    runtime = PythonResultRuntime()

    assert runtime.execute(script, payload={"user": {"value 2": "s"}}) == "s"
    assert runtime.execute(script, payload={}) == "UNKNOWN"


def test_null_safe_selectors_fall_back_to_default():
    script = """%dw 2.0
output application/json
var city = payload.user?.address?.city default "UNKNOWN"
---
{
  city: city
}
"""

    runtime = PythonResultRuntime()
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
    runtime = PythonResultRuntime()
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

    runtime = PythonResultRuntime()
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

    runtime = PythonResultRuntime()
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

    runtime = PythonResultRuntime()
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
    runtime = PythonResultRuntime()
    result = runtime.execute(
        "%dw 2.0\noutput application/json\n---\n{ price: randomInt(1000), ratio: random() }",
        {}
    )
    assert 0 <= result["price"] < 1000
    assert 0 <= result["ratio"] < 1


def test_numeric_range_to_operator():
    runtime = PythonResultRuntime()
    result = runtime.execute(
        "%dw 2.0\noutput application/json\n---\n{ up: 1 to 5, down: 5 to 1 }",
        {}
    )
    assert result["up"] == [1, 2, 3, 4, 5]
    assert result["down"] == [5, 4, 3, 2, 1]


def test_numeric_range_with_legacy_lambda_syntax():
    runtime = PythonResultRuntime()
    result = runtime.execute(
        "%dw 2.0\noutput application/json\n---\n1 to 5 map ((value) -> value * 2)",
        {}
    )
    assert result == [2, 4, 6, 8, 10]


def test_range_selector_supports_reverse_from_end_indexes():
    runtime = PythonResultRuntime()
    result = runtime.execute(
        "%dw 2.0\noutput application/json\n---\n(1 to 10)[-1 to 0]",
        {}
    )
    assert result == [10, 9, 8, 7, 6, 5, 4, 3, 2, 1]


def test_range_selector_on_string_supports_reverse_and_negative_end():
    runtime = PythonResultRuntime()
    result = runtime.execute(
        """%dw 2.0
output application/json
var myVar = "Hello World!"
---
{
  indices2to6: myVar[2 to 6],
  indicesFromEnd: myVar[6 to -1],
  reversal: myVar[11 to -0]
}
""",
        {}
    )
    assert result == {
        "indices2to6": "llo W",
        "indicesFromEnd": "World!",
        "reversal": "!dlroW olleH",
    }


def test_import_star_from_strings_module():
    runtime = PythonResultRuntime()
    script = """%dw 2.0
output application/json
import * from dw::core::Strings
---
upper(payload.name)
"""
    result = runtime.execute(script, {"name": "dw"})
    assert result == "DW"


def test_import_named_function_with_alias():
    runtime = PythonResultRuntime()
    script = """%dw 2.0
output application/json
import trim as tidy from dw::core::Strings
---
tidy(payload.value)
"""
    result = runtime.execute(script, {"value": "  hello  "})
    assert result == "hello"


def test_import_from_module_file():
    runtime = PythonResultRuntime()
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
    runtime = PythonResultRuntime()
    result = runtime.execute("payload.name", {"name": "hello"})
    assert result == "hello"


def test_header_defined_function_invocation():
    runtime = PythonResultRuntime()
    script = """%dw 2.0
output application/json
fun toUpper(aString) = upper(aString)
---
toUpper(\"h\" ++ \"el\" ++ lower(\"LO\"))
"""
    result = runtime.execute(script, {})
    assert result == "HELLO"


def test_map_over_range_generates_objects():
    runtime = PythonResultRuntime()
    script = """%dw 2.0
output application/json
---
1 to 10 map {
  "hi": "Esteban"
}
"""
    result = runtime.execute(script, {})
    assert result == [{"hi": "Esteban"}] * 10


def test_map_with_implicit_placeholders():
    runtime = PythonResultRuntime()
    script = """%dw 2.0
output application/json
---
["jose", "pedro", "mateo"] map { ($$): $ }
"""
    result = runtime.execute(script, {})
    assert result == [{"0": "jose"}, {"1": "pedro"}, {"2": "mateo"}]


def test_map_using_only_index_placeholder():
    runtime = PythonResultRuntime()
    script = """%dw 2.0
output application/json
---
["a", "b", "c"] map $$
"""
    result = runtime.execute(script, {})
    assert result == [0, 1, 2]


def test_filter_with_placeholder_condition():
    runtime = PythonResultRuntime()
    script = """%dw 2.0
output application/json
---
[9, 2, 3, 4, 5] filter (($$ > 1) and ($ < 5))
"""
    result = runtime.execute(script, {})
    assert result == [3, 4]


def test_filter_supports_unary_not_with_placeholder_selector():
    runtime = PythonResultRuntime()
    script = """%dw 2.0
output application/json
---
((payload.items map ((item, index) -> { "idx" : index} ++ item )) filter !$.success)
"""
    payload = {
        "items": [
            {
                "success": False,
                "rec": {
                    "idx": "ddd",
                    "errors": [],
                },
            }
        ]
    }

    result = runtime.execute(script, payload)

    assert result == [
        {
            "idx": 0,
            "success": False,
            "rec": {
                "idx": "ddd",
                "errors": [],
            },
        }
    ]


def test_string_literal_coerced_to_number():
    runtime = PythonResultRuntime()
    script = """%dw 2.0
output application/json
---
"3" as Number
"""
    result = runtime.execute(script, {})
    assert result == 3


def test_function_with_return_type_annotation():
    runtime = PythonResultRuntime()
    script = """%dw 2.0
output text/plain
fun toNumber(aString): Number = aString as Number
---
toNumber("3") + 5
"""
    result = runtime.execute(script, {})
    assert result == 8


def test_negative_number_literals_inside_nested_arrays():
    runtime = PythonResultRuntime()
    script = """%dw 2.0
output application/json
---
[
  [
    "Jan-25",
    -51894
  ],
  [
    "Feb-25",
    -143932
  ],
  [
    "Mar-25",
    329745
  ],
  [
    "Apr-25",
    126467
  ]
]
"""
    result = runtime.execute(script, {})
    assert result == [
        ["Jan-25", -51894],
        ["Feb-25", -143932],
        ["Mar-25", 329745],
        ["Apr-25", 126467],
    ]


def test_unary_minus_applies_to_selector_expression():
    runtime = PythonResultRuntime()
    script = """%dw 2.0
output application/json
---
-payload.amount
"""
    result = runtime.execute(script, {"amount": 7})
    assert result == -7


def test_unary_not_applies_to_selector_expression():
    runtime = PythonResultRuntime()
    script = """%dw 2.0
output application/json
---
!payload.success
"""
    result = runtime.execute(script, {"success": False})
    assert result is True


def test_plus_operator_type_error_message():
    runtime = DataWeaveRuntime()
    script = """%dw 2.0
output text/plain
---
"a" + 5
"""
    with pytest.raises(DataWeaveEvaluationError) as exc:
        runtime.execute(script, {})

    message = str(exc.value)
    assert "You called the function '+'" in message
    assert "(Number, Number)" in message
    assert "Location:" in message


def test_unresolved_infix_reference_reports_operator_location():
    runtime = DataWeaveRuntime()
    script = """%dw 2.0
output application/json
---
{
  message: payload.message,
  uppercased: upper(payload.message)
} then {}
"""
    with pytest.raises(DataWeaveEvaluationError) as exc:
        runtime.execute(script, {"message": "hello"})

    message = str(exc.value)
    assert "Unable to resolve reference of `then`." in message
    assert "7| } then {}" in message
    assert "main (line: 7, column: 3)" in message


def test_plus_operator_appends_arrays_as_values():
    runtime = DataWeaveRuntime()
    script = """%dw 2.0
output application/json
---
[2] + [2]
"""
    result = runtime.execute(script, {})
    assert json.loads(result) == [2, [2]]


def test_plus_operator_appends_scalar_values_to_arrays():
    runtime = DataWeaveRuntime()
    script = """%dw 2.0
output application/json
---
[2] + 2
"""
    result = runtime.execute(script, {})
    assert json.loads(result) == [2, 2]


def test_recursive_node_processing_logs_and_flattens_hierarchy():
    runtime = PythonResultRuntime()
    script = """%dw 2.0
fun processNode(node: Object) = [log(node.name)] ++ flatten((node.children map ((item, index) -> processNode(item))))
fun processNode(node: Null) = []
---
processNode(payload)
"""
    payload = {
        "name": "Net Operating Income",
        "children": [
            {
                "name": "Revenues",
                "children": [
                    {
                        "name": "Rental Income",
                        "children": [
                            {"name": "Base Rent", "children": []},
                            {"name": "Base Rent Concession", "children": []},
                        ],
                    },
                    {
                        "name": "Tenant CAM Reimbursements",
                        "children": [
                            {"name": "Electric", "children": []},
                            {"name": "Miscellaneous", "children": []},
                        ],
                    },
                    {
                        "name": "Monthly Maintenance",
                        "children": [
                            {"name": "Monthly Maintenance OPEX Concession", "children": []},
                        ],
                    },
                    {
                        "name": "Other Charges",
                        "children": [
                            {"name": "Storage", "children": []},
                        ],
                    },
                ],
            }
        ],
    }

    result = runtime.execute(script, payload)
    assert result == [
        "Net Operating Income",
        "Revenues",
        "Rental Income",
        "Base Rent",
        "Base Rent Concession",
        "Tenant CAM Reimbursements",
        "Electric",
        "Miscellaneous",
        "Monthly Maintenance",
        "Monthly Maintenance OPEX Concession",
        "Other Charges",
        "Storage",
    ]
