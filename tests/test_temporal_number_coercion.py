from __future__ import annotations

import pytest

from dwpy.runtime import DataWeaveRuntime


DATETIME_NUMBER_SCRIPT = """%dw 2.0
output application/python
---
{
  direct: (|2026-09-24T13:39:49Z| as DateTime) as Number,
  afterPeriod: ((|2026-09-24T13:39:49Z| as DateTime) + |P1D|) as Number
}
"""

EPOCH_ROUND_TRIP = """%dw 2.0
output application/json
---
{
  EpochTime: ((payload.Database as DateTime as Number)) ++ payload.Database[-3 to -1],
  NormalDateTime: (((payload.EpochTime / 1000) as Number) as DateTime) as String {format: "YYYY-MM-dd'T'HH:mm:ss."} ++ payload.EpochTime[-3 to -1]
}
"""


@pytest.mark.parametrize("backend", ["python", "rust"])
def test_epoch_datetime_example(backend: str) -> None:
    runtime = DataWeaveRuntime(backend=backend)
    assert runtime.execute(EPOCH_ROUND_TRIP, {
        "Database": "2023-10-30T00:00:00.000",
        "EpochTime": "1698423899751",
    }, render_output=False) == {
        "EpochTime": "1698624000000",
        "NormalDateTime": "2023-10-27T16:24:59.751",
    }


@pytest.mark.parametrize("backend", ["python", "rust"])
@pytest.mark.parametrize(("seconds", "expected"), [
    (0, "1970-01-01T00:00:00"),
    (-1.5, "1969-12-31T23:59:58"),
    (1698423899.751, "2023-10-27T16:24:59"),
])
def test_number_to_datetime_uses_utc(backend: str, seconds: float, expected: str) -> None:
    runtime = DataWeaveRuntime(backend=backend)
    assert runtime.execute(
        "payload as DateTime as String {format: \"yyyy-MM-dd'T'HH:mm:ss\"}",
        seconds, render_output=False,
    ) == expected


@pytest.mark.parametrize("backend", ["python", "rust", "wasm"])
def test_datetime_number_coercion_matches_across_backends(
    backend: str,
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    monkeypatch.delenv("DWPY_TEST_BACKEND", raising=False)
    monkeypatch.delenv("DWPY_BACKEND", raising=False)
    try:
        runtime = DataWeaveRuntime(backend=backend)  # type: ignore[arg-type]
    except RuntimeError as error:
        if backend == "wasm":
            pytest.skip(str(error))
        raise

    assert runtime.active_backend == backend
    assert runtime.execute(
        DATETIME_NUMBER_SCRIPT,
        {},
        render_output=False,
    ) == {
        "direct": 1_790_257_189,
        "afterPeriod": 1_790_343_589,
    }
