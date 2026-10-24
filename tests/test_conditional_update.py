from __future__ import annotations

import copy
import json
from pathlib import Path

import pytest

from dwpy import parser
from dwpy.parser import ParseError
from dwpy.runtime import DataWeaveRuntime
from dwpy.type_inference import TypeInferencer
from dwpy.typesystem import ANY, BOOLEAN

SCRIPT = """%dw 2.0
output application/json
---
payload map (item) ->
    if (item.role == "tool")
        item update {
            case content at .content ->
                if (content is String)
                    content[0 to 50]
                else
                    content
        }
    else
        item
"""


@pytest.fixture(params=["python", "rust", "wasm"])
def runtime(request: pytest.FixtureRequest, monkeypatch: pytest.MonkeyPatch):
    monkeypatch.delenv("DWPY_TEST_BACKEND", raising=False)
    monkeypatch.delenv("DWPY_BACKEND", raising=False)
    return DataWeaveRuntime(backend=request.param)


def test_conditional_update_truncates_only_tool_content(runtime):
    payload = json.loads(
        (Path(__file__).parent / "fixtures" / "conditional_update_input.json").read_text()
    )
    original = copy.deepcopy(payload)
    expected = copy.deepcopy(payload)
    expected[1]["content"] = "Asset          |     Address      |    City     | S"

    assert json.loads(runtime.execute(SCRIPT, payload)) == expected
    assert runtime.execute(SCRIPT, payload, render_output=False) == expected
    assert payload == original


@pytest.mark.parametrize("content", [None, 42, True, ["long" * 30], {"text": "long" * 30}, "short", ""])
def test_conditional_update_preserves_non_string_and_short_content(runtime, content):
    payload = [{"role": "tool", "content": content, "other": "preserved"}]
    assert json.loads(runtime.execute(SCRIPT, payload)) == payload


def test_conditional_update_in_both_branches_and_nested_lambdas(runtime):
    script = """%dw 2.0
output application/json
---
payload map (group) -> group map (item) ->
    if (item.role == "tool")
        item update { case value at .content -> value[0 to 2] }
    else
        item update { case value at .content -> upper(value) }
"""
    payload = [[{"role": "tool", "content": "abcdef"}, {"role": "user", "content": "hello"}]]
    assert json.loads(runtime.execute(script, payload)) == [
        [{"role": "tool", "content": "abc"}, {"role": "user", "content": "HELLO"}]
    ]


def test_conditional_update_with_comments_delimiters_and_following_filter(runtime):
    script = """%dw 2.0
output application/json
// --- update else
---
(payload map (item) ->
    if (item.role == "tool")
        item update /* else case --- */ {
            case content at .content ->
                if (content is /* String check */ String)
                    content[0 to 50] // else update
                else
                    content
            case tag at .tag -> "else update case --- }"
        }
    else
        item)
filter ($.role == "tool")
"""
    payload = [
        {"role": "user", "content": "unchanged", "tag": "old"},
        {"role": "tool", "content": "abc", "tag": "old"},
    ]
    assert json.loads(runtime.execute(script, payload)) == [
        {"role": "tool", "content": "abc", "tag": "else update case --- }"}
    ]


def test_selector_update_without_with_reports_parse_error(runtime):
    with pytest.raises(ParseError, match="(?:[Ee]xpected.*with|update expression missing with)"):
        runtime.execute('payload update "content"', {"content": "hello"})


def test_selector_update_in_unparenthesized_lambda(runtime):
    script = 'payload map (item) -> item update "content" with upper(item.content)'
    assert runtime.execute(script, [{"content": "hello", "other": 1}]) == [
        {"content": "HELLO", "other": 1}
    ]


def test_type_check_composes_with_implicit_lambdas_and_logical_operators(runtime):
    script = 'payload map ($ is String or $ is Number)'
    assert runtime.execute(script, ["hello", 42, True, None, {}, []]) == [
        True, True, False, False, False, False
    ]


def test_type_check_infers_boolean():
    script = parser.parse_script("payload.content is String")
    assert TypeInferencer(ANY, ANY).infer_script(script) == BOOLEAN


def test_type_check_requires_a_type_name():
    with pytest.raises(ParseError, match="Expected type name"):
        parser.parse_script("payload is")
