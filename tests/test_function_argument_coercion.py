import json

import pytest

from dwpy.runtime import DataWeaveRuntime


@pytest.mark.parametrize("backend", ["python", "rust", "wasm"])
def test_number_argument_coercion_and_concat(backend, monkeypatch):
    monkeypatch.delenv("DWPY_TEST_BACKEND", raising=False)
    script = '%dw 2.0\noutput application/json\nfun hi(a: Number, b: Number) = a ++ b\n---\nhi(2,"3")'
    assert json.loads(DataWeaveRuntime(backend=backend).execute(script, {})) == "23"


@pytest.mark.parametrize("backend", ["python", "rust", "wasm"])
def test_exact_overload_wins_and_coercion_supplies_number(backend, monkeypatch):
    monkeypatch.delenv("DWPY_TEST_BACKEND", raising=False)
    script = '%dw 2.0\noutput application/json\nfun f(x: Number) = "number"\nfun f(x: String) = "string"\nfun add(a: Number, b: Number) = a + b\n---\n[f("3"),f(3),add(2,"3")]'
    assert json.loads(DataWeaveRuntime(backend=backend).execute(script, {})) == ["string", "number", 5]


@pytest.mark.parametrize("backend", ["python", "rust", "wasm"])
def test_non_numeric_string_does_not_execute_typed_body(backend, monkeypatch):
    monkeypatch.delenv("DWPY_TEST_BACKEND", raising=False)
    script = '%dw 2.0\nfun f(x: Number) = fail("body executed")\n---\nf("bad")'
    with pytest.raises(Exception) as error:
        DataWeaveRuntime(backend=backend).execute(script, {})
    assert "body executed" not in str(error.value)


@pytest.mark.parametrize("backend", ["python", "rust", "wasm"])
def test_union_parameter_coercion(backend, monkeypatch):
    monkeypatch.delenv("DWPY_TEST_BACKEND", raising=False)
    script = '%dw 2.0\noutput application/json\nfun f(x: Number | Null) = x default 0\n---\n[f("3"), f(null)]'
    assert json.loads(DataWeaveRuntime(backend=backend).execute(script, {})) == [3, 0]
