from __future__ import annotations

import pytest

from dwpy.runtime import DataWeaveRuntime


RANGE_REDUCE = "1 to 1000000 reduce $ + $$"


@pytest.mark.parametrize("backend", ["auto", "python", "rust", "wasm"])
def test_implicit_range_reduce_sums_million_values_across_backends(
    backend: str,
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    monkeypatch.delenv("DWPY_TEST_BACKEND", raising=False)
    monkeypatch.delenv("DWPY_BACKEND", raising=False)
    try:
        runtime = (
            DataWeaveRuntime()
            if backend == "auto"
            else DataWeaveRuntime(backend=backend)  # type: ignore[arg-type]
        )
    except RuntimeError as error:
        if backend == "wasm":
            pytest.skip(str(error))
        raise

    if backend != "auto":
        assert runtime.active_backend == backend
    else:
        assert runtime.active_backend in {"python", "rust"}
    assert runtime.execute(RANGE_REDUCE, {}, render_output=False) == 500_000_500_000
