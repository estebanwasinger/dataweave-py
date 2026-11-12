import json

import pytest

from dwpy.runtime import DataWeaveRuntime


@pytest.mark.parametrize("backend", ["rust", "wasm"])
def test_negative_literals_and_unary_minus_have_distinct_precedence(backend, monkeypatch):
    monkeypatch.delenv("DWPY_TEST_BACKEND", raising=False)
    monkeypatch.delenv("DWPY_BACKEND", raising=False)
    try:
        runtime = DataWeaveRuntime(backend=backend)
    except RuntimeError:
        if backend == "wasm":
            pytest.skip("WASM bridge is unavailable")
        raise

    script = """%dw 2.0
output application/json
var x = 2
var offset = -3 + 1
---
{
  unary: -x + 3,
  parenthesized: (-x) + 3,
  product: -x * 2 + 1,
  literal: -3 + 1,
  decimal: -3.5 + 1,
  multiplication: -3 * 2 + 1,
  nested: [offset, -3 /* comment + --- */ + 1, 2 * -3 + 1]
}"""
    assert json.loads(runtime.execute(script, None)) == {
        "unary": -5,
        "parenthesized": 1,
        "product": -5,
        "literal": -2,
        "decimal": -2.5,
        "multiplication": -5,
        "nested": [-2, -2, -5],
    }
