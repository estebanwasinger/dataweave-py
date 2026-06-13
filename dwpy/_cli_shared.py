from __future__ import annotations

import json
from datetime import date, datetime, time, timedelta
from pathlib import Path
from typing import Any, Optional

from . import parser
from .formats import DWObject, FormatRegistry, XMLNodeDict, XMLNodeList, _JSONEncoder


def infer_payload_format_from_path(path: str) -> Optional[str]:
    suffix = Path(path).suffix.lower()
    if suffix == ".json":
        return "application/json"
    if suffix == ".xml":
        return "application/xml"
    if suffix == ".csv":
        return "application/csv"
    if suffix in {".yaml", ".yml"}:
        return "application/yaml"
    return None


def parse_inline_payload(value: str) -> Any:
    try:
        return json.loads(value)
    except json.JSONDecodeError:
        return value


def script_renders_text(script_source: str) -> bool:
    try:
        script = parser.parse_script(script_source)
    except Exception:
        return False

    if not script.header.output:
        return False

    format_token = script.header.output.split()[0]
    format_definition = FormatRegistry.get(format_token)
    if format_definition is None:
        return format_token not in {"application/python", "python", "text/x-python"}
    return format_definition.writer is not None


def render_cli_value(value: Any) -> str:
    normalized = _normalize_cli_value(value)
    encoder = _JSONEncoder(indent=2, ensure_ascii=False, sort_keys=False)
    return encoder.encode(normalized)


def _normalize_cli_value(value: Any) -> Any:
    if isinstance(value, bytes):
        return list(value)
    if isinstance(value, bytearray):
        return list(value)
    if isinstance(value, datetime):
        return value.isoformat().replace("+00:00", "Z")
    if isinstance(value, date):
        return value.isoformat()
    if isinstance(value, time):
        return value.isoformat().replace("+00:00", "Z")
    if isinstance(value, timedelta):
        total_seconds = int(value.total_seconds())
        return f"PT{total_seconds}S"
    if isinstance(value, DWObject):
        normalized = DWObject()
        for key, item in value.items():
            normalized.add(key, _normalize_cli_value(item))
        return normalized
    if isinstance(value, XMLNodeList):
        normalized = XMLNodeList()
        normalized.extend(_normalize_cli_value(item) for item in value)
        return normalized
    if isinstance(value, XMLNodeDict):
        normalized = XMLNodeDict()
        for key, item in value.items():
            normalized[key] = _normalize_cli_value(item)
        return normalized
    if isinstance(value, list):
        return [_normalize_cli_value(item) for item in value]
    if isinstance(value, tuple):
        return [_normalize_cli_value(item) for item in value]
    if isinstance(value, dict):
        return {key: _normalize_cli_value(item) for key, item in value.items()}
    return value
