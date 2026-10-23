from __future__ import annotations

from datetime import date, datetime, time, timedelta
from typing import Any

from .formats import DWObject


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
        return {"__dwpy_temporal": "datetime", "value": value.isoformat().replace("+00:00", "Z")}
    if isinstance(value, date):
        return {"__dwpy_temporal": "date", "value": value.isoformat().replace("+00:00", "Z")}
    if isinstance(value, time):
        return {"__dwpy_temporal": "time", "value": value.isoformat().replace("+00:00", "Z")}
    from .builtins import DWPeriod

    if isinstance(value, timedelta):
        value = DWPeriod(days=value.days, seconds=value.seconds + value.microseconds / 1_000_000)
    if isinstance(value, DWPeriod):
        return {
            "__dwpy_period": True,
            "years": value.years, "months": value.months, "days": value.days,
            "hours": value.hours, "minutes": value.minutes, "seconds": value.seconds,
            "dateBased": value.date_based, "text": value.to_iso8601(),
        }
    if hasattr(value, "isoformat"):
        return value.isoformat()
    raise TypeError(f"Object of type {type(value).__name__} is not JSON serializable")


def encode_bridge_value(value: Any) -> Any:
    """Encode typed host values and escape user dictionaries with wire keys."""
    if isinstance(value, DWObject):
        return {
            "__dwpy_object_pairs": [
                {"key": key, "value": encode_bridge_value(item)}
                for key, item in value.items()
            ]
        }
    if isinstance(value, dict):
        encoded = {key: encode_bridge_value(item) for key, item in value.items()}
        if any(key in value for key in ("__dwpy_temporal", "__dwpy_period", "__dwpy_object")):
            return {"__dwpy_object": encoded}
        return encoded
    if isinstance(value, (list, tuple)):
        return [encode_bridge_value(item) for item in value]
    if isinstance(value, (datetime, date, time)):
        return json_default(value)
    if pd is not None and isinstance(value, (pd.DataFrame, pd.Series)):
        return encode_bridge_value(json_default(value))
    return value
