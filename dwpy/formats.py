from __future__ import annotations

import csv
import io
import json
from collections.abc import Mapping
from dataclasses import dataclass
from typing import Any, Callable, Dict, Optional
import xml.etree.ElementTree as ET


class FormatError(ValueError):
    pass


Reader = Callable[[Any, Dict[str, Any]], Any]
Writer = Callable[[Any, Dict[str, Any]], Any]


class XMLNodeList(list):
    """List wrapper used to mark XML repeated elements."""


class XMLNodeDict(dict):
    """Dictionary wrapper used to mark XML element nodes."""



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
    FormatRegistry.register(
        FormatDefinition(
            id="xml",
            mime_type="application/xml",
            reader=_xml_reader,
            writer=_xml_writer,
        ),
        aliases=["xml", "text/xml"],
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


class _JSONEncoder:
    def __init__(self, indent: Optional[int], ensure_ascii: bool, sort_keys: bool) -> None:
        self.indent = indent if indent is not None and indent >= 0 else None
        self.ensure_ascii = ensure_ascii
        self.sort_keys = sort_keys

    def encode(self, value: Any, level: int = 0) -> str:
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
        return json.dumps(value, ensure_ascii=self.ensure_ascii)

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
