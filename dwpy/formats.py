from __future__ import annotations

import csv
import io
import json
import re
from collections.abc import Mapping
from dataclasses import dataclass
from datetime import date, datetime, time, timedelta
from typing import Any, Callable, Dict, Optional
import xml.etree.ElementTree as ET

from tabulate import tabulate
import yaml


class FormatError(ValueError):
    pass


Reader = Callable[[Any, Dict[str, Any]], Any]
Writer = Callable[[Any, Dict[str, Any]], Any]


class XMLNodeList(list):
    """List wrapper used to mark XML repeated elements."""


class XMLNodeDict(dict):
    """Dictionary wrapper used to mark XML element nodes."""


class FormattedNumber(float):
    """Numeric value that preserves DataWeave formatting metadata for string writers."""

    def __new__(cls, value: Any, formatted_text: str) -> "FormattedNumber":
        instance = float.__new__(cls, float(value))
        instance.formatted_text = str(formatted_text)
        return instance

    def raw_value(self) -> int | float:
        numeric_value = float(self)
        return int(numeric_value) if numeric_value.is_integer() else numeric_value

    def __str__(self) -> str:
        return self.formatted_text


class DWObject(dict):
    """Dictionary-like object that preserves duplicate key entries."""

    def __init__(self, entries: Optional[list[tuple[str, Any]]] = None) -> None:
        super().__init__()
        self._entries: list[tuple[str, Any]] = []
        if entries:
            for key, value in entries:
                self.add(key, value)

    def add(self, key: str, value: Any) -> None:
        normalized_key = str(key)
        self._entries.append((normalized_key, value))
        super().__setitem__(normalized_key, value)

    def items(self):  # type: ignore[override]
        return list(self._entries)



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
            id="ndjson",
            mime_type="application/x-ndjson",
            reader=_ndjson_reader,
            writer=_ndjson_writer,
        ),
        aliases=["ndjson", "application/x-ldjson"],
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
    FormatRegistry.register(
        FormatDefinition(
            id="plain",
            mime_type="text/plain",
            reader=None,
            writer=_plain_writer,
        ),
        aliases=["plain"],
    )
    FormatRegistry.register(
        FormatDefinition(
            id="markdown",
            mime_type="text/markdown",
            reader=_markdown_reader,
            writer=_markdown_writer,
        ),
        aliases=["markdown", "md"],
    )
    FormatRegistry.register(
        FormatDefinition(
            id="xml",
            mime_type="application/xml",
            reader=_xml_reader,
            writer=_xml_writer,
        ),
        aliases=["xml", "text/xml"],
    )
    FormatRegistry.register(
        FormatDefinition(
            id="yaml",
            mime_type="application/yaml",
            reader=_yaml_reader,
            writer=_yaml_writer,
        ),
        aliases=["yaml", "yml", "text/yaml", "application/x-yaml", "text/x-yaml"],
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
    encoder = _JSONEncoder(indent=indent, ensure_ascii=ensure_ascii, sort_keys=sort_keys)
    return encoder.encode(value)


def _ndjson_reader(value: Any, options: Dict[str, Any]) -> Any:
    if isinstance(value, (dict, list)):
        return value
    text = _ensure_text(value, options)
    ignore_empty_line = _to_bool(options.get("ignoreEmptyLine", True))
    skip_invalid = _to_bool(options.get("skipInvalid", False))
    records: list[Any] = []
    for line_number, line in enumerate(text.splitlines(), start=1):
        if not line.strip() and ignore_empty_line:
            continue
        try:
            records.append(json.loads(line))
        except json.JSONDecodeError as err:
            if skip_invalid:
                continue
            raise FormatError(f"Invalid NDJSON record on line {line_number}: {err}") from err
    return records


def _ndjson_writer(value: Any, options: Dict[str, Any]) -> str:
    rows = value if isinstance(value, list) else [value]
    skip_null_on = options.get("skipNullOn")
    normalized_mode = str(skip_null_on).lower() if skip_null_on is not None else ""
    if normalized_mode and normalized_mode not in {"arrays", "objects", "everywhere"}:
        raise FormatError("NDJSON skipNullOn must be 'arrays', 'objects', or 'everywhere'")
    write_attributes = _to_bool(options.get("writeAttributes", False))
    ensure_ascii = _to_bool(options.get("ensure_ascii", True))
    encoder = _JSONEncoder(indent=None, ensure_ascii=ensure_ascii, sort_keys=False)
    rendered: list[str] = []
    for row in rows:
        normalized = _normalize_ndjson_value(row, write_attributes=write_attributes)
        if normalized_mode:
            normalized = _skip_ndjson_nulls(normalized, normalized_mode)
        rendered.append(encoder.encode(normalized))
    return "\n".join(rendered) + ("\n" if rendered else "")


def _normalize_ndjson_value(value: Any, *, write_attributes: bool) -> Any:
    if isinstance(value, FormattedNumber):
        return value.raw_value()
    if hasattr(value, "to_iso8601") and callable(getattr(value, "to_iso8601")):
        return value.to_iso8601()
    if isinstance(value, datetime):
        return value.isoformat().replace("+00:00", "Z")
    if isinstance(value, date):
        return value.isoformat()
    if isinstance(value, time):
        return value.isoformat().replace("+00:00", "Z")
    if isinstance(value, timedelta):
        return _format_timedelta_iso(value)
    if isinstance(value, XMLNodeDict):
        normalized_children: Dict[str, Any] = {}
        text_value = None
        for key, child in value.items():
            if key.startswith("@") and not write_attributes:
                continue
            if key == "#text":
                text_value = _normalize_ndjson_value(child, write_attributes=write_attributes)
                if write_attributes:
                    normalized_children["__text"] = text_value
                continue
            normalized_children[key] = _normalize_ndjson_value(
                child, write_attributes=write_attributes
            )
        if normalized_children:
            return normalized_children
        if text_value is not None:
            return text_value
        return ""
    if isinstance(value, XMLNodeList):
        return [
            _normalize_ndjson_value(item, write_attributes=write_attributes) for item in value
        ]
    if isinstance(value, (list, tuple)):
        return [
            _normalize_ndjson_value(item, write_attributes=write_attributes) for item in value
        ]
    if isinstance(value, Mapping):
        normalized: Dict[str, Any] = {}
        for key, item in value.items():
            normalized_key = "__text" if write_attributes and str(key) == "#text" else str(key)
            normalized[normalized_key] = _normalize_ndjson_value(
                item, write_attributes=write_attributes
            )
        return normalized
    return value


def _skip_ndjson_nulls(value: Any, mode: str) -> Any:
    if isinstance(value, list):
        items = [_skip_ndjson_nulls(item, mode) for item in value]
        if mode in {"arrays", "everywhere"}:
            return [item for item in items if item is not None]
        return items
    if isinstance(value, Mapping):
        result = {}
        for key, item in value.items():
            if item is None and mode in {"objects", "everywhere"}:
                continue
            result[key] = _skip_ndjson_nulls(item, mode)
        return result
    return value


def _yaml_reader(value: Any, options: Dict[str, Any]) -> Any:
    if isinstance(value, (dict, list)):
        return value
    text = _ensure_text(value, options)
    return yaml.safe_load(text)


def _yaml_writer(value: Any, options: Dict[str, Any]) -> str:
    skip_null_on = options.get("skipNullOn")
    normalized = _normalize_yaml_value(value)
    if skip_null_on is not None:
        normalized = _skip_yaml_nulls(normalized, str(skip_null_on))
    return yaml.safe_dump(
        normalized,
        allow_unicode=True,
        default_flow_style=False,
        explicit_start=_to_bool(options.get("writeDeclaration", False)),
        sort_keys=False,
    )


def _normalize_yaml_value(value: Any) -> Any:
    if isinstance(value, FormattedNumber):
        return value.raw_value()
    if hasattr(value, "to_iso8601") and callable(getattr(value, "to_iso8601")):
        return value.to_iso8601()
    if isinstance(value, datetime):
        return value.isoformat().replace("+00:00", "Z")
    if isinstance(value, date):
        return value.isoformat()
    if isinstance(value, time):
        return value.isoformat().replace("+00:00", "Z")
    if isinstance(value, timedelta):
        return _format_timedelta_iso(value)
    if isinstance(value, XMLNodeDict):
        text_value = None
        normalized_children: Dict[str, Any] = {}
        for key, child in value.items():
            if key == "#text":
                text_value = _normalize_yaml_value(child)
                continue
            if key.startswith("@"):
                continue
            normalized_children[key] = _normalize_yaml_value(child)
        if normalized_children:
            return normalized_children
        if text_value is not None:
            return text_value
        return ""
    if isinstance(value, XMLNodeList):
        return [_normalize_yaml_value(item) for item in value]
    if isinstance(value, list):
        return [_normalize_yaml_value(item) for item in value]
    if isinstance(value, tuple):
        return [_normalize_yaml_value(item) for item in value]
    if isinstance(value, Mapping):
        return {str(key): _normalize_yaml_value(val) for key, val in value.items()}
    return value


def _skip_yaml_nulls(value: Any, mode: str) -> Any:
    normalized_mode = mode.lower()
    if normalized_mode not in {"arrays", "objects", "everywhere"}:
        raise FormatError("YAML skipNullOn must be 'arrays', 'objects', or 'everywhere'")
    if isinstance(value, list):
        items = [_skip_yaml_nulls(item, normalized_mode) for item in value]
        if normalized_mode in {"arrays", "everywhere"}:
            return [item for item in items if item is not None]
        return items
    if isinstance(value, Mapping):
        result = {}
        for key, item in value.items():
            if item is None and normalized_mode in {"objects", "everywhere"}:
                continue
            result[key] = _skip_yaml_nulls(item, normalized_mode)
        return result
    return value


def _format_timedelta_iso(value: timedelta) -> str:
    total_seconds = value.total_seconds()
    if total_seconds == 0:
        return "PT0S"
    sign = -1 if total_seconds < 0 else 1
    remaining = abs(total_seconds)
    hours = int(remaining // 3600)
    remaining -= hours * 3600
    minutes = int(remaining // 60)
    remaining -= minutes * 60
    seconds = remaining
    if sign < 0:
        if hours:
            hours = -hours
        if minutes:
            minutes = -minutes
        if seconds:
            seconds = -seconds
    parts: list[str] = []
    if hours:
        parts.append(f"{hours}H")
    if minutes:
        parts.append(f"{minutes}M")
    if seconds or not parts:
        if float(seconds).is_integer():
            parts.append(f"{int(seconds)}S")
        else:
            parts.append(f"{seconds}S")
    return "PT" + "".join(parts)


class _JSONEncoder:
    def __init__(self, indent: Optional[int], ensure_ascii: bool, sort_keys: bool) -> None:
        self.indent = indent if indent is not None and indent >= 0 else None
        self.ensure_ascii = ensure_ascii
        self.sort_keys = sort_keys

    def encode(self, value: Any, level: int = 0) -> str:
        if isinstance(value, FormattedNumber):
            return json.dumps(value.raw_value(), ensure_ascii=self.ensure_ascii)
        if isinstance(value, XMLNodeDict):
            return self._encode_object(value, level)
        if isinstance(value, Mapping):
            return self._encode_object(value, level)
        if isinstance(value, XMLNodeList):
            return self._encode_array(list(value), level)
        if isinstance(value, list):
            return self._encode_array(value, level)
        if isinstance(value, (str, int, float, bool)) or value is None:
            return json.dumps(value, ensure_ascii=self.ensure_ascii)
        normalised = self._normalize_value(value)
        return json.dumps(normalised, ensure_ascii=self.ensure_ascii)

    def _encode_object(self, obj: Mapping[str, Any], level: int) -> str:
        if not obj:
            return "{}"
        items = obj.items()
        if self.sort_keys:
            items = sorted(items, key=lambda kv: kv[0])
        key_values: list[tuple[str, Any]] = []
        for key, value in items:
            if isinstance(value, XMLNodeList):
                for entry in value:
                    key_values.append((key, entry))
            else:
                key_values.append((key, value))
        parts = []
        for key, value in key_values:
            normalized = self._normalize_value(value)
            encoded_key = json.dumps(key, ensure_ascii=self.ensure_ascii)
            encoded_value = self._encode_normalized(normalized, level + 1)
            if self.indent is None:
                parts.append(f"{encoded_key}:{encoded_value}")
            else:
                pad = " " * self.indent * (level + 1)
                parts.append(f"{pad}{encoded_key}: {encoded_value}")
        if self.indent is None:
            return "{" + ",".join(parts) + "}"
        else:
            newline = "\n"
            closing_pad = " " * self.indent * level
            return "{" + newline + (",\n".join(parts)) + newline + closing_pad + "}"

    def _encode_array(self, items: list[Any], level: int) -> str:
        if not items:
            return "[]"
        parts = []
        for item in items:
            normalized = self._normalize_value(item)
            encoded = self._encode_normalized(normalized, level + 1)
            if self.indent is None:
                parts.append(encoded)
            else:
                pad = " " * self.indent * (level + 1)
                parts.append(f"{pad}{encoded}")
        if self.indent is None:
            return "[" + ",".join(parts) + "]"
        else:
            newline = "\n"
            closing_pad = " " * self.indent * level
            return "[" + newline + (",\n".join(parts)) + newline + closing_pad + "]"

    def _encode_normalized(self, value: Any, level: int) -> str:
        if isinstance(value, dict):
            return self._encode_standard_object(value, level)
        if isinstance(value, list):
            return self._encode_standard_array(value, level)
        return json.dumps(value, ensure_ascii=self.ensure_ascii)

    def _encode_standard_object(self, obj: Mapping[str, Any], level: int) -> str:
        if not obj:
            return "{}"
        items = obj.items()
        if self.sort_keys:
            items = sorted(items, key=lambda kv: kv[0])
        parts = []
        for key, value in items:
            encoded_key = json.dumps(key, ensure_ascii=self.ensure_ascii)
            encoded_value = self._encode_normalized(value, level + 1)
            if self.indent is None:
                parts.append(f"{encoded_key}:{encoded_value}")
            else:
                pad = " " * self.indent * (level + 1)
                parts.append(f"{pad}{encoded_key}: {encoded_value}")
        if self.indent is None:
            return "{" + ",".join(parts) + "}"
        newline = "\n"
        closing_pad = " " * self.indent * level
        return "{" + newline + (",\n".join(parts)) + newline + closing_pad + "}"

    def _encode_standard_array(self, items: list[Any], level: int) -> str:
        if not items:
            return "[]"
        parts = []
        for item in items:
            encoded = self._encode_normalized(item, level + 1)
            if self.indent is None:
                parts.append(encoded)
            else:
                pad = " " * self.indent * (level + 1)
                parts.append(f"{pad}{encoded}")
        if self.indent is None:
            return "[" + ",".join(parts) + "]"
        newline = "\n"
        closing_pad = " " * self.indent * level
        return "[" + newline + (",\n".join(parts)) + newline + closing_pad + "]"

    def _normalize_value(self, value: Any) -> Any:
        if isinstance(value, FormattedNumber):
            return value.raw_value()
        if hasattr(value, "to_iso8601") and callable(getattr(value, "to_iso8601")):
            return value.to_iso8601()
        if isinstance(value, datetime):
            return value.isoformat().replace("+00:00", "Z")
        if isinstance(value, date):
            return value.isoformat()
        if isinstance(value, time):
            return value.isoformat().replace("+00:00", "Z")
        if isinstance(value, timedelta):
            return _format_timedelta_iso(value)
        if isinstance(value, XMLNodeDict):
            text_value = None
            normalized_children: Dict[str, Any] = {}
            for key, child in value.items():
                if key == "#text":
                    text_value = self._normalize_value(child)
                    continue
                if key.startswith("@"):
                    continue
                normalized_children[key] = self._normalize_value(child)
            if normalized_children:
                return normalized_children
            if text_value is not None:
                return text_value
            return ""
        if isinstance(value, XMLNodeList):
            return [self._normalize_value(item) for item in value]
        if isinstance(value, list):
            return [self._normalize_value(item) for item in value]
        if isinstance(value, DWObject):
            normalized = DWObject()
            for key, val in value.items():
                normalized.add(key, self._normalize_value(val))
            return normalized
        if isinstance(value, Mapping):
            return {key: self._normalize_value(val) for key, val in value.items()}
        return value


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


def _markdown_reader(value: Any, options: Dict[str, Any]) -> Any:
    if isinstance(value, list):
        return value
    if isinstance(value, Mapping):
        return value
    text = _ensure_text(value, options).strip()
    if not text:
        return []
    row_lines = [line for line in text.splitlines() if line.strip()]
    parsed_rows = [_parse_markdown_row(line) for line in row_lines if "|" in line]
    if len(parsed_rows) < 2:
        raise FormatError("Markdown reader expects a header row and a separator row")
    headers = parsed_rows[0]
    separator = parsed_rows[1]
    if not headers:
        raise FormatError("Markdown reader requires at least one column")
    if len(separator) != len(headers) or not all(_is_markdown_separator_cell(cell) for cell in separator):
        raise FormatError("Markdown reader expects a valid separator row after the header")
    data_rows = parsed_rows[2:]
    if _to_bool(options.get("header", True)):
        return [_markdown_cells_to_dict(headers, row) for row in data_rows]
    return [_markdown_cells_to_row(headers, row) for row in data_rows]


def _parse_markdown_row(line: str) -> list[str]:
    row = line.strip()
    if row.startswith("|"):
        row = row[1:]
    if row.endswith("|"):
        row = row[:-1]
    cells: list[str] = []
    current: list[str] = []
    idx = 0
    while idx < len(row):
        char = row[idx]
        next_char = row[idx + 1] if idx + 1 < len(row) else ""
        if char == "\\" and next_char == "|":
            current.append("|")
            idx += 2
            continue
        if char == "|":
            cells.append(_decode_markdown_cell("".join(current)))
            current = []
            idx += 1
            continue
        current.append(char)
        idx += 1
    cells.append(_decode_markdown_cell("".join(current)))
    return cells


def _decode_markdown_cell(cell: str) -> str:
    return cell.strip().replace("<br>", "\n")


def _is_markdown_separator_cell(cell: str) -> bool:
    candidate = cell.strip().replace(" ", "")
    return bool(re.fullmatch(r":?-{3,}:?", candidate))


def _markdown_cells_to_dict(headers: list[str], row: list[str]) -> dict[str, str]:
    normalized = _markdown_cells_to_row(headers, row)
    return {headers[index]: normalized[index] for index in range(len(headers))}


def _markdown_cells_to_row(headers: list[str], row: list[str]) -> list[str]:
    if len(row) >= len(headers):
        return row[: len(headers)]
    return row + [""] * (len(headers) - len(row))


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


def _plain_writer(value: Any, options: Dict[str, Any]) -> str:
    del options
    if not isinstance(value, str):
        raise FormatError("Plain text writer expects a string value")
    return value


def _markdown_writer(value: Any, options: Dict[str, Any]) -> str:
    if not _to_bool(options.get("header", True)):
        raise FormatError("Markdown writer requires header=true")
    rows = value
    if isinstance(value, dict):
        rows = [value]
    if not isinstance(rows, list):
        raise FormatError("Markdown writer expects a list or dict value")
    if not rows:
        return ""
    if isinstance(rows[0], Mapping):
        return _markdown_table_from_mapping_rows(rows, options)
    return _markdown_table_from_sequence_rows(rows)


def _markdown_table_from_mapping_rows(rows: list[Any], options: Dict[str, Any]) -> str:
    if not all(isinstance(row, Mapping) for row in rows):
        raise FormatError(
            "Markdown writer expects all rows to be dictionaries when the first row is a dictionary"
        )
    fieldnames = options.get("columns")
    if fieldnames is None:
        first_row = rows[0]
        fieldnames = list(first_row.keys())
    elif isinstance(fieldnames, str):
        fieldnames = [segment.strip() for segment in fieldnames.split(",") if segment.strip()]
    elif isinstance(fieldnames, (list, tuple)):
        fieldnames = [str(segment).strip() for segment in fieldnames if str(segment).strip()]
    else:
        raise FormatError("Markdown columns option must be a string, list, or tuple")
    if not fieldnames:
        raise FormatError("Markdown writer requires at least one column when writing dictionaries")
    table_rows = [
        [_normalize_markdown_cell(row.get(column, "")) for column in fieldnames]
        for row in rows
    ]
    headers = [_normalize_markdown_cell(column) for column in fieldnames]
    return tabulate(
        table_rows,
        headers=headers,
        tablefmt="pipe",
        showindex=False,
        disable_numparse=True,
    )


def _markdown_table_from_sequence_rows(rows: list[Any]) -> str:
    materialized_rows: list[list[Any]] = []
    for row in rows:
        if isinstance(row, (list, tuple)):
            materialized_rows.append(list(row))
        else:
            materialized_rows.append([row])
    max_columns = max((len(row) for row in materialized_rows), default=0)
    if max_columns == 0:
        return ""
    headers = [f"column{index}" for index in range(1, max_columns + 1)]
    table_rows = [
        [
            _normalize_markdown_cell(row[index] if index < len(row) else "")
            for index in range(max_columns)
        ]
        for row in materialized_rows
    ]
    return tabulate(
        table_rows,
        headers=headers,
        tablefmt="pipe",
        showindex=False,
        disable_numparse=True,
    )


def _normalize_markdown_cell(value: Any) -> str:
    if value is None:
        return ""
    text = str(value)
    text = text.replace("\r\n", "\n").replace("\r", "\n")
    text = text.replace("|", r"\|")
    return text.replace("\n", "<br>")


def _xml_reader(value: Any, options: Dict[str, Any]) -> Any:
    text = _ensure_text(value, options)
    try:
        root = ET.fromstring(text)
    except ET.ParseError as err:
        raise FormatError(f"Invalid XML input: {err}") from err
    return {root.tag: _element_to_value(root)}


def _element_to_value(element: ET.Element) -> Any:
    children = list(element)
    text = (element.text or "").strip()
    if not children and not element.attrib:
        return text
    result: XMLNodeDict = XMLNodeDict()
    for attr_name, attr_value in element.attrib.items():
        result[f"@{attr_name}"] = attr_value
    for child in children:
        child_value = _element_to_value(child)
        existing = result.get(child.tag)
        if existing is None:
            result[child.tag] = child_value
        else:
            if not isinstance(existing, XMLNodeList):
                node_list = XMLNodeList()
                node_list.append(existing)
                result[child.tag] = node_list
            result[child.tag].append(child_value)
    if text:
        if children or element.attrib:
            result["#text"] = text
        else:
            return text
    return result


def _xml_writer(value: Any, options: Dict[str, Any]) -> str:
    if isinstance(value, Mapping) and len(value) == 1 and "root" not in options:
        root_name, root_value = next(iter(value.items()))
    else:
        root_name = options.get("root", "root")
        root_value = value
    element = ET.Element(str(root_name))
    _populate_xml_element(element, root_value)
    return ET.tostring(element, encoding="unicode")


def _populate_xml_element(element: ET.Element, value: Any) -> None:
    if isinstance(value, Mapping):
        for key, child_value in value.items():
            if key.startswith("@"):
                element.set(key[1:], str(child_value))
                continue
            if key == "#text":
                element.text = str(child_value)
                continue
            values = child_value if isinstance(child_value, list) else [child_value]
            for item in values:
                child_el = ET.SubElement(element, key)
                _populate_xml_element(child_el, item)
        if not element.text:
            element.text = ""
        return
    if isinstance(value, list):
        for item in value:
            child_el = ET.SubElement(element, "item")
            _populate_xml_element(child_el, item)
        if not element.text:
            element.text = ""
        return
    element.text = "" if value is None else str(value)


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
