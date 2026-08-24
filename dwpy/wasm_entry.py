from __future__ import annotations

import base64
import math
from collections.abc import Mapping
from datetime import date, datetime, time, timedelta
from decimal import Decimal
from typing import Any

from .lsp.engine import DataWeaveLanguageEngine
from .runtime import DataWeaveRuntime


_LANGUAGE_ENGINE = DataWeaveLanguageEngine()


def run_dataweave(
    script: str,
    payload: Any,
    vars: dict[str, Any] | None = None,
    attributes: Any = None,
    properties: dict[str, str] | None = None,
    payload_format: str | None = None,
    payload_format_options: dict[str, Any] | None = None,
    render_output: bool = True,
) -> Any:
    """
    Execute DataWeave and return JSON-serializable output for JS/WASM interop.
    """
    runtime = DataWeaveRuntime()
    result = runtime.execute(
        script,
        payload,
        vars=vars,
        attributes=attributes,
        properties=properties,
        payload_format=payload_format,
        payload_format_options=payload_format_options,
        render_output=render_output,
    )
    return _to_json_compatible(result)


def complete_dataweave(
    *,
    script: str,
    line: int,
    column: int,
    payload: Any = None,
    vars: dict[str, Any] | None = None,
    attributes: Any = None,
    properties: dict[str, str] | None = None,
) -> dict[str, Any]:
    items = _LANGUAGE_ENGINE.complete(
        script=script,
        line=line,
        column=column,
        payload=payload,
        vars=vars if vars is not None else {},
        attributes=attributes if attributes is not None else {},
        properties=properties if properties is not None else {},
    )
    return {
        "items": [
            {
                "label": item.label,
                "kind": item.kind,
                "insertText": item.insert_text,
                "detail": item.detail,
                "documentation": item.documentation,
                "insertTextFormat": item.insert_text_format,
                "sortText": item.sort_text,
            }
            for item in items
        ]
    }


def hover_dataweave(
    *,
    script: str,
    line: int,
    column: int,
    payload: Any = None,
    vars: dict[str, Any] | None = None,
    attributes: Any = None,
    properties: dict[str, str] | None = None,
) -> dict[str, Any] | None:
    hover = _LANGUAGE_ENGINE.hover(
        script=script,
        line=line,
        column=column,
        payload=payload,
        vars=vars if vars is not None else {},
        attributes=attributes if attributes is not None else {},
        properties=properties if properties is not None else {},
    )
    if hover is None:
        return None
    return {"contents": hover.contents}


def signature_help_dataweave(
    *,
    script: str,
    line: int,
    column: int,
    payload: Any = None,
    vars: dict[str, Any] | None = None,
    attributes: Any = None,
    properties: dict[str, str] | None = None,
) -> dict[str, Any] | None:
    help_value = _LANGUAGE_ENGINE.signature_help(
        script=script,
        line=line,
        column=column,
        payload=payload,
        vars=vars if vars is not None else {},
        attributes=attributes if attributes is not None else {},
        properties=properties if properties is not None else {},
    )
    if help_value is None:
        return None
    return {
        "signatures": [
            {
                "label": signature.label,
                "documentation": signature.documentation,
                "parameters": list(signature.parameters),
            }
            for signature in help_value.signatures
        ],
        "activeSignature": help_value.active_signature,
        "activeParameter": help_value.active_parameter,
    }


def _to_json_compatible(value: Any) -> Any:
    if value is None or isinstance(value, (bool, int, str)):
        return value

    if isinstance(value, float):
        return value if math.isfinite(value) else str(value)

    if isinstance(value, Decimal):
        if value.is_nan() or value.is_infinite():
            return str(value)
        return str(value)

    if isinstance(value, (bytes, bytearray)):
        return {
            "__type__": "bytes",
            "base64": base64.b64encode(bytes(value)).decode("ascii"),
        }

    if hasattr(value, "to_iso8601") and callable(getattr(value, "to_iso8601")):
        return value.to_iso8601()

    if isinstance(value, datetime):
        return value.isoformat().replace("+00:00", "Z")
    if isinstance(value, date):
        return value.isoformat()
    if isinstance(value, time):
        return value.isoformat().replace("+00:00", "Z")
    if isinstance(value, timedelta):
        return value.total_seconds()

    if isinstance(value, Mapping):
        return {str(key): _to_json_compatible(item) for key, item in value.items()}

    if isinstance(value, (list, tuple, set)):
        return [_to_json_compatible(item) for item in value]

    return str(value)
