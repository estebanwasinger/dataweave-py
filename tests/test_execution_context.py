from __future__ import annotations

import pytest

from dwpy.runtime import DataWeaveRuntime


SAMPLE_SCRIPT = r'''%dw 2.0
output application/python
---
{
  variableData: vars.myVariable,
  requestHeaders: attributes.headers,
  queryParams: attributes.queryParams,
  payloadData: payload,
  environment: p("env"),
  canonical: Mule::p("env"),
  compatibility: prop("env"),
  dotted: p("app.config.value"),
  missing: p("missing"),
  lambda: payload.items map ((item) -> {
    value: vars.myVariable,
    header: attributes.headers.x,
    property: p("env")
  }),
  doBlock: do {
    var nested = attributes.queryParams.page
    ---
    {nested: nested, property: p("env")}
  },
  matched: payload match {
    case var item when attributes.headers.x == "yes" -> {item: item, property: p("env")}
    else -> {}
  }
}
'''


@pytest.mark.parametrize("backend", ["python", "rust", "wasm"])
def test_full_execution_context_is_available_across_backends(backend: str) -> None:
    try:
        runtime = DataWeaveRuntime(backend=backend)  # type: ignore[arg-type]
        result = runtime.execute(
            SAMPLE_SCRIPT,
            {"items": [{"id": 1}]},
            vars={"myVariable": 7},
            attributes={
                "headers": {"x": "yes"},
                "queryParams": {"page": "2"},
            },
            properties={"env": "test", "app.config.value": "flat"},
            render_output=False,
        )
    except RuntimeError as error:
        if backend == "wasm":
            pytest.skip(str(error))
        raise

    assert result["variableData"] == 7
    assert result["requestHeaders"] == {"x": "yes"}
    assert result["queryParams"] == {"page": "2"}
    assert result["payloadData"] == {"items": [{"id": 1}]}
    assert result["environment"] == "test"
    assert result["canonical"] == "test"
    assert result["compatibility"] == "test"
    assert result["dotted"] == "flat"
    assert result["missing"] is None
    assert result["lambda"][0] == {
        "value": 7,
        "header": "yes",
        "property": "test",
    }
    assert result["doBlock"] == {"nested": "2", "property": "test"}
    assert result["matched"]["property"] == "test"


@pytest.mark.parametrize("backend", ["python", "rust", "wasm"])
def test_properties_require_string_values(backend: str) -> None:
    try:
        runtime = DataWeaveRuntime(backend=backend)  # type: ignore[arg-type]
        with pytest.raises(Exception, match="string"):
            runtime.execute(
                'p("env")',
                {},
                properties={"env": 1},  # type: ignore[dict-item]
                render_output=False,
            )
    except RuntimeError as error:
        if backend == "wasm":
            pytest.skip(str(error))
        raise


def test_supplied_properties_override_host_environment(monkeypatch: pytest.MonkeyPatch) -> None:
    monkeypatch.setenv("DWPY_CONTEXT_TEST_PROPERTY", "host")
    for backend in ("python", "rust", "wasm"):
        try:
            runtime = DataWeaveRuntime(backend=backend)  # type: ignore[arg-type]
            assert runtime.execute(
                'p("DWPY_CONTEXT_TEST_PROPERTY")',
                {},
                properties={"DWPY_CONTEXT_TEST_PROPERTY": "supplied"},
                render_output=False,
            ) == "supplied"
        except RuntimeError as error:
            if backend == "wasm":
                pytest.skip(str(error))
            raise
