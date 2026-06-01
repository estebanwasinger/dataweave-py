from __future__ import annotations

from datetime import date, datetime, time
from typing import Any


try:
    import pandas as pd
except Exception:  # pragma: no cover - optional dependency
    pd = None  # type: ignore[assignment]


def json_default(value: Any) -> Any:
    """Normalize Python boundary values before they enter the Rust core."""
    if pd is not None:
        if isinstance(value, pd.DataFrame):
            return value.to_dict(orient="records")
        if isinstance(value, pd.Series):
            return value.to_dict()
        try:
            missing = pd.isna(value)
        except Exception:
            missing = False
        if isinstance(missing, bool) and missing:
            return None
    if isinstance(value, datetime):
        return {"__dwpy_temporal": "datetime", "value": value.isoformat()}
    if isinstance(value, date):
        return {"__dwpy_temporal": "date", "value": value.isoformat()}
    if isinstance(value, time):
        return {"__dwpy_temporal": "time", "value": value.isoformat()}
    if hasattr(value, "isoformat"):
        return value.isoformat()
    raise TypeError(f"Object of type {type(value).__name__} is not JSON serializable")
