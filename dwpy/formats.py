from __future__ import annotations

import csv
import io
import json
from dataclasses import dataclass
from typing import Any, Callable, Dict, Optional


class FormatError(ValueError):
    pass


Reader = Callable[[Any, Dict[str, Any]], Any]
Writer = Callable[[Any, Dict[str, Any]], Any]


@dataclass(frozen=True)
class FormatDefinition:
    id: str
    mime_type: str
    reader: Optional[Reader]
    writer: Optional[Writer]


class FormatRegistry:
    _FORMATS: Dict[str, FormatDefinition] = {}
    _ALIASES: Dict[str, str] = {}

    @classmethod
    def register(
        cls,
        definition: FormatDefinition,
        *,
        aliases: Optional[Any] = None,
    ) -> None:
        cls._FORMATS[definition.id] = definition
        cls._ALIASES[definition.id.lower()] = definition.id
        cls._ALIASES[definition.mime_type.lower()] = definition.id
        if aliases:
            for alias in aliases:
                cls._ALIASES[alias.lower()] = definition.id

    @classmethod
    def get(cls, name: Optional[str]) -> Optional[FormatDefinition]:
        if not name:
            return None
        key = cls._ALIASES.get(name.lower())
        if key is None:
            return None
        return cls._FORMATS.get(key)

    @classmethod
    def read(cls, value: Any, format_name: str, options: Dict[str, Any]) -> Any:
        definition = cls.get(format_name)
        if definition is None:
            raise FormatError(f"Unsupported input format '{format_name}'")
        if definition.reader is None:
            return value
        try:
            return definition.reader(value, options)
        except Exception as err:
            raise FormatError(f"Failed to parse input as {definition.id}: {err}") from err

    @classmethod
    def write(cls, value: Any, format_name: str, options: Dict[str, Any]) -> Any:
        definition = cls.get(format_name)
        if definition is None:
            raise FormatError(f"Unsupported output format '{format_name}'")
        if definition.writer is None:
            return value
        try:
            return definition.writer(value, options)
        except Exception as err:
            raise FormatError(f"Failed to render output as {definition.id}: {err}") from err


def _register_builtin_formats() -> None:
    FormatRegistry.register(
        FormatDefinition(
            id="python",
            mime_type="application/python",
            reader=None,
            writer=None,
        ),
        aliases=["text/x-python"],
    )
    FormatRegistry.register(
        FormatDefinition(
            id="json",
            mime_type="application/json",
            reader=_json_reader,
            writer=_json_writer,
        ),
        aliases=["json", "text/json"],
    )
    FormatRegistry.register(
        FormatDefinition(
            id="csv",
            mime_type="application/csv",
            reader=_csv_reader,
            writer=_csv_writer,
        ),
        aliases=["csv", "text/csv"],
    )


def _ensure_text(value: Any, options: Dict[str, Any]) -> str:
    if isinstance(value, str):
        return value
    encoding = options.get("encoding", "utf-8")
    if isinstance(value, bytes):
        return value.decode(encoding)
    if isinstance(value, bytearray):
        return bytes(value).decode(encoding)
    raise FormatError("Expected textual input for this format")


def _json_reader(value: Any, options: Dict[str, Any]) -> Any:
    if isinstance(value, (dict, list)):
        return value
    text = _ensure_text(value, options)
    return json.loads(text)


def _json_writer(value: Any, options: Dict[str, Any]) -> str:
    indent_opt = options.get("indent")
    indent = None
    if indent_opt is not None:
        try:
            indent = int(indent_opt)
        except (TypeError, ValueError) as err:
            raise FormatError("JSON indent must be an integer") from err
    ensure_ascii = True
    if "ensure_ascii" in options:
        ensure_ascii = _to_bool(options.get("ensure_ascii"))
    sort_keys = _to_bool(options.get("sort_keys", False))
    return json.dumps(
        value,
        indent=indent,
        ensure_ascii=ensure_ascii,
        sort_keys=sort_keys,
    )


def _csv_reader(value: Any, options: Dict[str, Any]) -> Any:
    if isinstance(value, list):
        return value
    text = _ensure_text(value, options)
    delimiter = str(options.get("separator", ",")) or ","
    quote = str(options.get("quote", '"')) or '"'
    header = _to_bool(options.get("header", True))
    stream = io.StringIO(text)
    if header:
        reader = csv.DictReader(stream, delimiter=delimiter, quotechar=quote)
        return [dict(row) for row in reader]
    reader = csv.reader(stream, delimiter=delimiter, quotechar=quote)
    return [row for row in reader]


def _csv_writer(value: Any, options: Dict[str, Any]) -> str:
    delimiter = str(options.get("separator", ",")) or ","
    quote = options.get("quote", '"')
    header = _to_bool(options.get("header", True))
    newline = options.get("newline")
    rows = value
    if isinstance(value, dict):
        rows = [value]
    if not isinstance(rows, list):
        raise FormatError("CSV writer expects a list or dict value")
    output = io.StringIO()
    if rows and isinstance(rows[0], dict):
        fieldnames = options.get("columns")
        if fieldnames is None:
            fieldnames = list(rows[0].keys())
        elif isinstance(fieldnames, str):
            fieldnames = [segment.strip() for segment in fieldnames.split(",") if segment.strip()]
        if not fieldnames:
            raise FormatError("CSV writer requires at least one column when writing dictionaries")
        writer = csv.DictWriter(
            output,
            fieldnames=fieldnames,
            delimiter=delimiter,
            quotechar=quote,
            extrasaction="ignore",
            lineterminator=newline if newline is not None else "\n",
        )
        if header:
            writer.writeheader()
        writer.writerows(rows)
    else:
        writer = csv.writer(
            output,
            delimiter=delimiter,
            quotechar=quote,
            lineterminator=newline if newline is not None else "\n",
        )
        for row in rows:
            if isinstance(row, (list, tuple)):
                writer.writerow(list(row))
            else:
                writer.writerow([row])
    return output.getvalue()


def _to_bool(value: Any) -> bool:
    if isinstance(value, bool):
        return value
    if value is None:
        return False
    if isinstance(value, str):
        lowered = value.lower()
        if lowered in {"true", "yes", "1"}:
            return True
        if lowered in {"false", "no", "0"}:
            return False
    return bool(value)


_register_builtin_formats()
