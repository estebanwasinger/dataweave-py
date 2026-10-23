from __future__ import annotations

import json

import pytest

from dwpy.runtime import DataWeaveEvaluationError, DataWeaveRuntime


@pytest.fixture(params=["python", "rust", "wasm"])
def runtime(
    request: pytest.FixtureRequest, monkeypatch: pytest.MonkeyPatch
) -> DataWeaveRuntime:
    monkeypatch.delenv("DWPY_TEST_BACKEND", raising=False)
    monkeypatch.delenv("DWPY_BACKEND", raising=False)
    try:
        return DataWeaveRuntime(backend=request.param)
    except RuntimeError as error:
        if request.param == "wasm":
            pytest.skip(str(error))
        raise


@pytest.mark.parametrize(
    ("expression", "expected"),
    [
        ('"2026-10-26" as Date + |P3D|', "2026-10-29"),
        ('"2026-10-26" as Date - |P3D|', "2026-10-23"),
        ('("2026-10-26" as Date) + |P3D|', "2026-10-29"),
        ('"2026-10-26" as Date + |P3D| - |P1D|', "2026-10-28"),
        ('"2026-10-26" as Date + "P3D" as Period', "2026-10-29"),
        ('"2" as Number + 3', 5),
        ('10 - "2" as Number', 8),
        ('"12" as Number - 3 - 2', 7),
        ('"2" as Number + 3 * 4', 14),
        ('"2" as Number * 3 + 4', 10),
        ('"12" as Number / 3 / 2', 2),
        ('"12" as Number / "3" as Number', 4),
        ('"2" as Number + 3 > "4" as Number', True),
        ('"2" as Number + 3 == 5', True),
        ('"2" as Number to "4" as Number', [2, 3, 4]),
        ('"2" as Number as String as Number + 3', 5),
        ('("2" as Number + 3) as String', "5"),
        ('2 as String ++ 3 as String ++ "!"', "23!"),
        ('"2026-10-26" as Date {} + |P3D|', "2026-10-29"),
        ('|2026-10-26| as String {format: "uuuu-MM-dd"} ++ "!"', "2026-10-26!"),
    ],
)
def test_coercion_binds_before_arithmetic(runtime, expression, expected):
    script = f"%dw 2.0\noutput application/json\n---\n{expression}"
    assert json.loads(runtime.execute(script, {})) == expected


def test_coercion_precedence_in_nested_expressions_and_headers(runtime):
    script = '''%dw 2.0
output application/json
type Day = Date
var start = "2026-10-26" as Day + |P1D|
fun advance(value) = value as Date + |P3D|
---
{
  header: start,
  call: advance("2026-10-26"),
  mapped: ["2026-10-26", "2026-10-27"] map ($ as Date + |P3D|),
  multiline: "2026-10-26" as Date /* + |P99D| --- */
    + // as String
    |P3D|
}'''
    assert json.loads(runtime.execute(script, {})) == {
        "header": "2026-10-27",
        "call": "2026-10-29",
        "mapped": ["2026-10-29", "2026-10-30"],
        "multiline": "2026-10-29",
    }


def test_invalid_coercion_before_arithmetic_reports_error(runtime):
    with pytest.raises((TypeError, DataWeaveEvaluationError), match="(?i)number"):
        runtime.execute('"invalid" as Number + 3', {})
