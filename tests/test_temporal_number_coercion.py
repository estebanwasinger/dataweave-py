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
