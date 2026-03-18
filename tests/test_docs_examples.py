import pytest

from dwpy.runtime import DataWeaveRuntime


def test_default_keyword_example_from_docs():
    script = """%dw 2.0
output application/json
---
{
  "userId": payload.id default "0000",
  "userName": payload.name default "Undefined"
}
"""
    runtime = DataWeaveRuntime()

    present = runtime.execute(
        script,
        payload={"id": "123", "name": "Max the Mule"},
        render_output=False,
    )
    missing = runtime.execute(script, payload={"id": None}, render_output=False)

    assert present == {"userId": "123", "userName": "Max the Mule"}
    assert missing == {"userId": "0000", "userName": "Undefined"}


def test_if_else_default_example_from_docs():
    script = """%dw 2.0
output application/json
---
if (payload.location != null) {
  "userLocation" : payload.location
} else {
  "userLocation" : "United States"
}
"""
    runtime = DataWeaveRuntime()

    present = runtime.execute(script, payload={"location": "Argentina"}, render_output=False)
    missing = runtime.execute(script, payload={}, render_output=False)

    assert present == {"userLocation": "Argentina"}
    assert missing == {"userLocation": "United States"}


def test_filter_anonymous_parameters_example_from_docs():
    script = """%dw 2.0
output application/json
---
[9, 2, 3, 4, 5] filter (($$ > 1) and ($ < 5))
"""
    runtime = DataWeaveRuntime()
    result = runtime.execute(script, payload={}, render_output=False)

    assert result == [3, 4]


def test_filter_object_example_from_docs():
    script = """%dw 2.0
output application/json
---
{"a" : "apple", "b" : "banana"} filterObject ((value) -> value == "apple")
"""
    runtime = DataWeaveRuntime()
    result = runtime.execute(script, payload={}, render_output=False)

    assert result == {"a": "apple"}


def test_map_with_default_variables_example_from_docs():
    script = """%dw 2.0
output application/json
---
{
  items: (payload.books map {
    category: "book",
    price: $.price as Number,
    id: $$,
    properties: {
      title: $.title,
      author: $.author,
      year: $.year as Number
    }
  })
}
"""
    payload = {
        "books": [
            {
                "title": {"-lang": "en", "#text": "Everyday Italian"},
                "author": "Giada De Laurentiis",
                "year": "2005",
                "price": "30.00",
            },
            {
                "title": {"-lang": "en", "#text": "XQuery Kick Start"},
                "author": ["James McGovern", "Per Bothner"],
                "year": "2003",
                "price": "49.99",
            },
        ]
    }

    runtime = DataWeaveRuntime()
    result = runtime.execute(script, payload=payload, render_output=False)

    assert result["items"][0]["category"] == "book"
    assert result["items"][0]["id"] == 0
    assert result["items"][0]["price"] == 30
    assert result["items"][0]["properties"]["year"] == 2005
    assert result["items"][1]["id"] == 1
    assert result["items"][1]["price"] == pytest.approx(49.99)
    assert result["items"][1]["properties"]["author"] == ["James McGovern", "Per Bothner"]
