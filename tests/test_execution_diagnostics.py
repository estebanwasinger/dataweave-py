import pytest

from dwpy.runtime import DataWeaveRuntime


SCRIPT = '''%dw 2.0
import * from dw::core::Arrays
output application/json
var grp7 = payload groupBy ((item) -> ((item.val default "") as String) as String) ++ "_" ++ ((item.val default "") as String)
---
grp7'''


@pytest.mark.parametrize("backend", ["python", "rust", "wasm"])
def test_group_by_fails_at_invalid_concat(monkeypatch, backend):
    monkeypatch.delenv("DWPY_TEST_BACKEND", raising=False)
    runtime = DataWeaveRuntime(backend=backend)
    with pytest.raises(Exception) as error:
        runtime.execute(SCRIPT, [{"val": "a"}])
    message = str(error.value)
    assert "++" in message
    assert "Object" in message
    assert "String" in message
    assert "Unable to resolve reference" not in message


def test_explicit_lambda_body_concat_remains_inside_group_by():
    runtime = DataWeaveRuntime(backend="python")
    assert runtime.execute('payload groupBy ((item) -> item.val ++ "_")', [{"val": "a"}], render_output=False) == {"a_": [{"val": "a"}]}


@pytest.mark.parametrize("backend", ["rust", "wasm"])
def test_optimized_callback_errors_include_location_and_trace(monkeypatch, backend):
    monkeypatch.delenv("DWPY_TEST_BACKEND", raising=False)
    script = "%dw 2.0\noutput application/json\n---\n[1, 2] map ($ / 0)"
    with pytest.raises(Exception) as error:
        DataWeaveRuntime(backend=backend).execute(script, {})
    message = str(error.value)
    assert "Trace:" in message
    assert "line: 4, column: 15" in message
