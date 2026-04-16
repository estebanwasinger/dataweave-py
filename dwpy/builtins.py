from __future__ import annotations

import calendar
import json
import logging
import math
import random
import re
from dataclasses import dataclass
from datetime import date, datetime, time, timedelta, timezone
from typing import Any, Callable, Dict, Iterable, List, Mapping, Optional, Sequence

from .formats import XMLNodeList, XMLNodeDict


def _format_decimal(value: float) -> str:
    if math.isclose(value, round(value)):
        return str(int(round(value)))
    rendered = f"{value:.12f}".rstrip("0").rstrip(".")
    return rendered if rendered != "-0" else "0"


def _add_months_to_date(value: date, months: int) -> date:
    month_index = (value.month - 1) + months
    year_delta, month_zero = divmod(month_index, 12)
    year = value.year + year_delta
    month = month_zero + 1
    day = min(value.day, calendar.monthrange(year, month)[1])
    return date(year, month, day)


@dataclass(frozen=True)
class DWPeriod:
    years: int = 0
    months: int = 0
    days: float = 0.0
    hours: float = 0.0
    minutes: float = 0.0
    seconds: float = 0.0
    date_based: bool = False

    def total_months(self) -> int:
        return self.years * 12 + self.months

    def total_seconds(self) -> float:
        return (self.days * 86400.0) + (self.hours * 3600.0) + (self.minutes * 60.0) + self.seconds

    def as_timedelta(self) -> timedelta:
        return timedelta(seconds=self.total_seconds())

    def negate(self) -> "DWPeriod":
        return DWPeriod(
            years=-self.years,
            months=-self.months,
            days=-self.days,
            hours=-self.hours,
            minutes=-self.minutes,
            seconds=-self.seconds,
            date_based=self.date_based,
        )

    def to_iso8601(self) -> str:
        if self.date_based:
            parts: List[str] = []
            if self.years != 0:
                parts.append(f"{self.years}Y")
            if self.months != 0:
                parts.append(f"{self.months}M")
            if not math.isclose(self.days, 0.0):
                parts.append(f"{_format_decimal(self.days)}D")
            if not parts:
                return "P0D"
            return "P" + "".join(parts)

        total_seconds = self.total_seconds()
        if math.isclose(total_seconds, 0.0):
            return "PT0S"
        sign = -1 if total_seconds < 0 else 1
        remaining = abs(total_seconds)
        hours = int(remaining // 3600)
        remaining -= hours * 3600
        minutes = int(remaining // 60)
        remaining -= minutes * 60
        seconds = remaining
        if sign < 0:
            if hours != 0:
                hours = -hours
            if minutes != 0:
                minutes = -minutes
            if not math.isclose(seconds, 0.0):
                seconds = -seconds
            elif hours == 0 and minutes == 0:
                seconds = -0.0
        parts: List[str] = []
        if hours != 0:
            parts.append(f"{hours}H")
        if minutes != 0:
            parts.append(f"{minutes}M")
        if not math.isclose(seconds, 0.0) or not parts:
            parts.append(f"{_format_decimal(seconds)}S")
        return "PT" + "".join(parts)

    def __str__(self) -> str:
        return self.to_iso8601()


def _coerce_iterable(value: Any) -> Iterable[Any]:
    if value is None:
        return []
    if isinstance(value, (list, tuple)):
        return value
    if isinstance(value, Mapping):
        return value.values()
    return list(value)


def parameter_count(function: Callable[..., Any]) -> Optional[int]:
    params = getattr(function, "parameters", None)
    if params is None:
        return None
    return len(params)


def invoke_lambda(function: Callable[..., Any], *candidates: Any) -> Any:
    param_count = parameter_count(function)
    if param_count is None:
        return function(*candidates)
    return function(*candidates[:param_count])


def _hashable_key(value: Any) -> Any:
    if isinstance(value, (str, int, float, bool)) or value is None:
        return value
    try:
        return json.dumps(value, sort_keys=True, default=str)
    except TypeError:
        return repr(value)


def binary_concat(left: Any, right: Any) -> Any:
    if isinstance(left, list) and isinstance(right, list):
        return left + right
    if isinstance(left, str) and isinstance(right, str):
        return left + right
    if isinstance(left, Mapping) and isinstance(right, Mapping):
        merged = dict(left)
        merged.update(right)
        return merged
    if isinstance(left, (bytes, bytearray)) and isinstance(right, (bytes, bytearray)):
        return bytes(left) + bytes(right)
    return f"{left}{right}"


def binary_diff(left: Any, right: Any) -> Any:
    if isinstance(left, list):
        right_values = set(right if isinstance(right, list) else [right])
        return [item for item in left if item not in right_values]
    if isinstance(left, Mapping):
        result = dict(left)
        if isinstance(right, Mapping):
            for key in right.keys():
                result.pop(key, None)
        elif isinstance(right, list):
            for key in right:
                result.pop(key, None)
        else:
            result.pop(str(right), None)
        return result
    if isinstance(left, str):
        remove = right if isinstance(right, str) else str(right)
        return left.replace(remove, "")
    return left


def builtin_abs(value: Any) -> Any:
    return abs(value)


def builtin_avg(values: Sequence[Any]) -> float:
    numbers = [float(v) for v in values]
    if not numbers:
        raise ValueError("avg expects a non-empty array")
    return sum(numbers) / len(numbers)


def builtin_ceil(value: Any) -> int:
    return math.ceil(float(value))


def builtin_floor(value: Any) -> int:
    return math.floor(float(value))


def builtin_round(value: Any) -> int:
    return round(float(value))


def builtin_contains(items: Any, element: Any) -> bool:
    if isinstance(items, str):
        if element is None:
            return False
        return str(element) in items
    if isinstance(items, Mapping):
        return element in items.values() or element in items.keys()
    iterable = _coerce_iterable(items)
    return any(item == element for item in iterable)


def builtin_endswith(text: Any, suffix: Any) -> bool:
    if text is None:
        return False
    return str(text).endswith("" if suffix is None else str(suffix))


def builtin_startswith(text: Any, prefix: Any) -> bool:
    if text is None:
        return False
    return str(text).startswith("" if prefix is None else str(prefix))


def builtin_joinby(elements: Any, separator: Any) -> Any:
    if elements is None:
        return None
    if not isinstance(elements, (list, tuple)):
        raise TypeError("joinBy expects an array")
    sep = "" if separator is None else str(separator)
    return sep.join("" if el is None else str(el) for el in elements)


def builtin_keys_of(obj: Any) -> List[Any]:
    if obj is None:
        return None
    if not isinstance(obj, Mapping):
        raise TypeError("keysOf expects an object")
    return list(obj.keys())


def builtin_values_of(obj: Any) -> List[Any]:
    if obj is None:
        return None
    if not isinstance(obj, Mapping):
        raise TypeError("valuesOf expects an object")
    return list(obj.values())


def builtin_lower(value: Any) -> Any:
    if value is None:
        return None
    return str(value).lower()


def builtin_trim(value: Any) -> Any:
    if value is None:
        return None
    return str(value).strip()


def _to_floor_int(value: Any) -> int:
    return math.floor(_coerce_number(value))


def _to_ceil_int(value: Any) -> int:
    return math.ceil(_coerce_number(value))


def _build_padding(pad_text: Any, length: int) -> str:
    if length <= 0:
        return ""
    token = " " if pad_text is None else str(pad_text)
    if token == "":
        return ""
    repeats = (length // len(token)) + 1
    return (token * repeats)[:length]


def builtin_append_if_missing(text: Any, suffix: Any) -> Any:
    if text is None:
        return None
    source = str(text)
    suffix_text = "" if suffix is None else str(suffix)
    if source.endswith(suffix_text):
        return source
    return source + suffix_text


def builtin_prepend_if_missing(text: Any, prefix: Any) -> Any:
    if text is None:
        return None
    source = str(text)
    prefix_text = "" if prefix is None else str(prefix)
    if source.startswith(prefix_text):
        return source
    return prefix_text + source


def builtin_camelize(text: Any) -> Any:
    if text is None:
        return None
    parts = [part for part in str(text).split("_") if part]
    if not parts:
        return ""
    first = parts[0].lower()
    rest = [part[:1].upper() + part[1:].lower() for part in parts[1:]]
    return first + "".join(rest)


def builtin_capitalize(text: Any) -> Any:
    if text is None:
        return None
    with_camel_split = re.sub(r"([a-z0-9])([A-Z])", r"\1 \2", str(text))
    with_spaces = with_camel_split.replace("_", " ")
    words = [word for word in re.split(r"\s+", with_spaces.strip()) if word]
    if not words:
        return ""
    return " ".join(word.title() for word in words)


def builtin_char_code(text: Any) -> Any:
    if text is None:
        return None
    source = str(text)
    if source == "":
        raise ValueError("charCode expects a non-empty string")
    return ord(source[0])


def builtin_char_code_at(content: Any, position: Any) -> Any:
    if content is None:
        return None
    source = str(content)
    index = int(_coerce_number(position))
    if index < 0 or index >= len(source):
        raise ValueError("charCodeAt index out of range")
    return ord(source[index])


def builtin_collapse(text: Any) -> Any:
    if text is None:
        return None
    source = str(text)
    if source == "":
        return []
    groups: List[str] = []
    current = source[0]
    run = [current]
    for char in source[1:]:
        if char == current:
            run.append(char)
            continue
        groups.append("".join(run))
        current = char
        run = [char]
    groups.append("".join(run))
    return groups


def builtin_count_characters_by(text: Any, predicate: Callable[..., Any]) -> Any:
    if text is None:
        return None
    count = 0
    for index, char in enumerate(str(text)):
        if invoke_lambda(predicate, char, index):
            count += 1
    return count


def builtin_count_matches(text: Any, pattern: Any) -> Any:
    if text is None:
        return None
    source = str(text)
    if pattern is None:
        return 0
    if isinstance(pattern, str) and pattern.startswith("/") and pattern.endswith("/") and len(pattern) >= 2:
        regex = re.compile(pattern[1:-1])
        return sum(1 for _ in regex.finditer(source))
    needle = str(pattern)
    if needle == "":
        return 0
    matches = 0
    cursor = 0
    while True:
        idx = source.find(needle, cursor)
        if idx < 0:
            break
        matches += 1
        cursor = idx + len(needle)
    return matches


def builtin_dasherize(text: Any) -> Any:
    if text is None:
        return None
    source = re.sub(r"([a-z0-9])([A-Z])", r"\1-\2", str(text))
    source = re.sub(r"[_\s]+", "-", source)
    source = re.sub(r"-+", "-", source).strip("-")
    return source.lower()


def builtin_every_character(text: Any, condition: Callable[..., Any]) -> bool:
    if text is None:
        return True
    for index, char in enumerate(str(text)):
        if not invoke_lambda(condition, char, index):
            return False
    return True


def builtin_some_character(text: Any, condition: Callable[..., Any]) -> bool:
    if text is None:
        return False
    for index, char in enumerate(str(text)):
        if invoke_lambda(condition, char, index):
            return True
    return False


def builtin_first(text: Any, amount: Any) -> Any:
    if text is None:
        return None
    source = str(text)
    size = _to_floor_int(amount)
    if size <= 0:
        return ""
    if size >= len(source):
        return source
    return source[:size]


def builtin_last(text: Any, amount: Any) -> Any:
    if text is None:
        return None
    source = str(text)
    size = _to_ceil_int(amount)
    if size <= 0:
        return ""
    if size >= len(source):
        return source
    return source[-size:]


def builtin_from_char_code(char_code: Any) -> Any:
    if char_code is None:
        return None
    return chr(int(_coerce_number(char_code)))


def builtin_hamming_distance(a: Any, b: Any) -> Any:
    if a is None or b is None:
        return None
    left = str(a)
    right = str(b)
    if len(left) != len(right):
        return None
    return sum(1 for left_char, right_char in zip(left, right) if left_char != right_char)


def builtin_is_alpha(text: Any) -> bool:
    if text is None:
        return False
    source = str(text)
    return bool(source) and all(char.isalpha() for char in source)


def builtin_is_alphanumeric(text: Any) -> bool:
    if text is None:
        return False
    source = str(text)
    return bool(source) and all(char.isalnum() for char in source)


def builtin_is_lower_case(text: Any) -> bool:
    if text is None:
        return False
    source = str(text)
    return bool(source) and all(char.isalpha() and char.islower() for char in source)


def builtin_is_upper_case(text: Any) -> bool:
    if text is None:
        return False
    source = str(text)
    return bool(source) and all(char.isalpha() and char.isupper() for char in source)


def builtin_is_numeric(text: Any) -> bool:
    if text is None:
        return False
    source = str(text)
    return bool(source) and all(char.isdigit() for char in source)


def builtin_is_whitespace(text: Any) -> bool:
    if text is None:
        return False
    source = str(text)
    return source == "" or all(char.isspace() for char in source)


def builtin_left_pad(text: Any, size: Any, pad_text: Any = " ") -> Any:
    if text is None:
        return None
    source = str(text)
    target_size = int(_coerce_number(size))
    if target_size <= len(source):
        return source
    padding = _build_padding(pad_text, target_size - len(source))
    if padding == "":
        return source
    return f"{padding}{source}"


def builtin_right_pad(text: Any, size: Any, pad_text: Any = " ") -> Any:
    if text is None:
        return None
    source = str(text)
    target_size = int(_coerce_number(size))
    if target_size <= len(source):
        return source
    padding = _build_padding(pad_text, target_size - len(source))
    if padding == "":
        return source
    return f"{source}{padding}"


def builtin_levenshtein_distance(a: Any, b: Any) -> Any:
    if a is None or b is None:
        return None
    left = str(a)
    right = str(b)
    if left == right:
        return 0
    if not left:
        return len(right)
    if not right:
        return len(left)
    previous = list(range(len(right) + 1))
    for left_index, left_char in enumerate(left, start=1):
        current = [left_index]
        for right_index, right_char in enumerate(right, start=1):
            insertion = current[right_index - 1] + 1
            deletion = previous[right_index] + 1
            replacement = previous[right_index - 1] + (left_char != right_char)
            current.append(min(insertion, deletion, replacement))
        previous = current
    return previous[-1]


def builtin_lines(text: Any) -> Any:
    if text is None:
        return None
    return str(text).splitlines()


def builtin_map_string(text: Any, mapper: Callable[..., Any]) -> Any:
    if text is None:
        return None
    output: List[str] = []
    for index, char in enumerate(str(text)):
        mapped = invoke_lambda(mapper, char, index)
        output.append("" if mapped is None else str(mapped))
    return "".join(output)


def builtin_ordinalize(num: Any) -> Any:
    if num is None:
        return None
    value = int(_coerce_number(num))
    absolute = abs(value)
    if 10 <= (absolute % 100) <= 20:
        suffix = "th"
    else:
        suffix = {1: "st", 2: "nd", 3: "rd"}.get(absolute % 10, "th")
    return f"{value}{suffix}"


def builtin_singularize(text: Any) -> Any:
    if text is None:
        return None
    source = str(text)
    lower = source.lower()
    if lower.endswith("ies") and len(source) > 3:
        return source[:-3] + "y"
    if re.search(r"(ses|xes|zes|ches|shes)$", lower):
        return source[:-2]
    if lower.endswith("s") and not lower.endswith("ss"):
        return source[:-1]
    return source


def builtin_pluralize(text: Any) -> Any:
    if text is None:
        return None
    source = str(text)
    if source == "":
        return source
    if builtin_singularize(source) != source:
        return source
    lower = source.lower()
    if re.search(r"(s|x|z|ch|sh)$", lower):
        return f"{source}es"
    if re.search(r"[^aeiou]y$", lower):
        return source[:-1] + "ies"
    return f"{source}s"


def builtin_remove(text: Any, to_remove: Any) -> Any:
    if text is None:
        return None
    source = str(text)
    needle = "" if to_remove is None else str(to_remove)
    if needle == "":
        return source
    return source.replace(needle, "")


def builtin_repeat(text: Any, times: Any) -> Any:
    if text is None:
        return None
    count = int(_coerce_number(times))
    if count <= 0:
        return ""
    return str(text) * count


def builtin_replace_all(text: Any, target: Any, replacement: Any) -> Any:
    if text is None:
        return None
    source = str(text)
    needle = "" if target is None else str(target)
    if needle == "":
        return source
    replacement_text = "" if replacement is None else str(replacement)
    return source.replace(needle, replacement_text)


def builtin_reverse(text: Any) -> Any:
    if text is None:
        return None
    return str(text)[::-1]


def builtin_substring(text: Any, from_index: Any, until_index: Any) -> Any:
    if text is None:
        return None
    source = str(text)
    start = int(_coerce_number(from_index))
    end = int(_coerce_number(until_index))
    if start < 0:
        start = 0
    if end < 0:
        end = 0
    if start >= end or start >= len(source):
        return ""
    return source[start:end]


def builtin_substring_after(text: Any, separator: Any) -> Any:
    if text is None:
        return None
    source = str(text)
    marker = "" if separator is None else str(separator)
    if marker == "":
        return source
    idx = source.find(marker)
    if idx < 0:
        return ""
    return source[idx + len(marker) :]


def builtin_substring_after_last(text: Any, separator: Any) -> Any:
    if text is None:
        return None
    source = str(text)
    marker = "" if separator is None else str(separator)
    if marker == "":
        return None
    idx = source.rfind(marker)
    if idx < 0:
        return ""
    return source[idx + len(marker) :]


def builtin_substring_before(text: Any, separator: Any) -> Any:
    if text is None:
        return None
    source = str(text)
    marker = "" if separator is None else str(separator)
    if marker == "":
        return ""
    idx = source.find(marker)
    if idx < 0:
        return ""
    return source[:idx]


def builtin_substring_before_last(text: Any, separator: Any) -> Any:
    if text is None:
        return None
    source = str(text)
    marker = "" if separator is None else str(separator)
    if marker == "":
        return source[:-1] if source else ""
    idx = source.rfind(marker)
    if idx < 0:
        return ""
    return source[:idx]


def builtin_substring_by(text: Any, predicate: Callable[..., Any]) -> Any:
    if text is None:
        return None
    source = str(text)
    result: List[str] = []
    current: List[str] = []
    for index, char in enumerate(source):
        if invoke_lambda(predicate, char, index):
            result.append("".join(current))
            current = []
        else:
            current.append(char)
    result.append("".join(current))
    return result


def builtin_substring_every(text: Any, amount: Any) -> Any:
    if text is None:
        return None
    source = str(text)
    size = _to_floor_int(amount)
    if size <= 0:
        return []
    return [source[idx : idx + size] for idx in range(0, len(source), size)]


def builtin_underscore(text: Any) -> Any:
    if text is None:
        return None
    source = re.sub(r"([a-z0-9])([A-Z])", r"\1_\2", str(text))
    source = re.sub(r"[-\s]+", "_", source)
    source = re.sub(r"_+", "_", source).strip("_")
    return source.lower()


def builtin_unwrap(text: Any, wrapper: Any) -> Any:
    if text is None:
        return None
    source = str(text)
    token = "" if wrapper is None else str(wrapper)
    if token == "":
        return source
    if len(source) < len(token) * 2:
        return source
    if source.startswith(token) and source.endswith(token):
        return source[len(token) : len(source) - len(token)]
    return source


def builtin_with_max_size(text: Any, max_length: Any) -> Any:
    if text is None:
        return None
    source = str(text)
    limit = _to_floor_int(max_length)
    if limit <= 0:
        return source
    if len(source) <= limit:
        return source
    return source[:limit]


def builtin_words(text: Any) -> Any:
    if text is None:
        return None
    source = str(text).strip()
    if source == "":
        return []
    return [token for token in re.split(r"\s+", source) if token]


def builtin_wrap_with(text: Any, wrapper: Any) -> Any:
    if text is None:
        return None
    token = "" if wrapper is None else str(wrapper)
    source = str(text)
    return f"{token}{source}{token}"


def builtin_wrap_if_missing(text: Any, wrapper: Any) -> Any:
    if text is None:
        return None
    token = "" if wrapper is None else str(wrapper)
    source = str(text)
    if token == "":
        return source
    if source == "":
        return token
    if not source.startswith(token):
        source = token + source
    if not source.endswith(token):
        source = source + token
    return source


def builtin_is_blank(value: Any) -> bool:
    if value is None:
        return True
    return str(value).strip() == ""


def builtin_is_empty(value: Any) -> bool:
    if value is None:
        return True
    if isinstance(value, (list, tuple, Mapping)):
        return len(value) == 0
    if isinstance(value, str):
        return len(value) == 0
    return False


def builtin_size_of(value: Any) -> int:
    if value is None:
        return 0
    if isinstance(value, (list, tuple, Mapping)):
        return len(value)
    if isinstance(value, (bytes, bytearray)):
        return len(value)
    return len(str(value))


def builtin_sum(values: Sequence[Any]) -> Any:
    numbers = [float(v) for v in values]
    if not numbers:
        return 0
    result = sum(numbers)
    if all(float(v).is_integer() for v in numbers):
        return int(result)
    return result


def builtin_now() -> str:
    return datetime.now(timezone.utc).isoformat().replace("+00:00", "Z")


def _coerce_number(value: Any) -> float:
    if isinstance(value, (int, float)):
        return float(value)
    return float(str(value))


def _parse_datetime(value: Any) -> datetime:
    if isinstance(value, datetime):
        return value
    text = str(value).strip()
    text = text.strip("|")
    if text.endswith("Z"):
        text = text[:-1] + "+00:00"
    return datetime.fromisoformat(text)


def builtin_is_decimal(value: Any) -> bool:
    if value is None:
        return False
    number = _coerce_number(value)
    return not math.isclose(number, round(number))


def builtin_is_integer(value: Any) -> bool:
    if value is None:
        return False
    number = _coerce_number(value)
    return math.isclose(number, round(number))


def builtin_is_even(value: Any) -> bool:
    number = int(_coerce_number(value))
    return number % 2 == 0


def builtin_is_odd(value: Any) -> bool:
    number = int(_coerce_number(value))
    return number % 2 != 0


def builtin_is_leap_year(value: Any) -> bool:
    try:
        dt = _parse_datetime(value)
    except ValueError:
        return False
    year = dt.year
    return (year % 4 == 0 and year % 100 != 0) or (year % 400 == 0)


def builtin_is_even(value: Any) -> bool:
    number = int(_coerce_number(value))
    return number % 2 == 0


def builtin_is_odd(value: Any) -> bool:
    number = int(_coerce_number(value))
    return number % 2 != 0


def builtin_is_leap_year(value: Any) -> bool:
    if value is None:
        return False
    if isinstance(value, str):
        value = value.strip("|")
    try:
        if isinstance(value, datetime):
            year = value.year
        else:
            year = int(str(value)[:4])
    except ValueError:
        return False
    return (year % 4 == 0 and year % 100 != 0) or (year % 400 == 0)


def builtin_distinct_by(items: Any, criteria: Callable[..., Any]) -> List[Any]:
    if items is None:
        return None
    iterable = list(_coerce_iterable(items))
    if criteria is None:
        return iterable
    seen = []
    result: List[Any] = []
    for index, item in enumerate(iterable):
        key = invoke_lambda(criteria, item, index)
        marker = _hashable_key(key)
        if marker not in seen:
            seen.append(marker)
            result.append(item)
    return result


def builtin_flatten(items: Any) -> List[Any]:
    if items is None:
        return None
    result: List[Any] = []
    for element in _coerce_iterable(items):
        if isinstance(element, (list, tuple)):
            result.extend(element)
        else:
            result.append(element)
    return result


def builtin_flat_map(items: Any, mapper: Callable[..., Any]) -> List[Any]:
    if items is None:
        return None
    result: List[Any] = []
    for index, item in enumerate(_coerce_iterable(items)):
        mapped = invoke_lambda(mapper, item, index)
        result.extend(_coerce_iterable(mapped))
    return result


def builtin_index_of(value: Any, target: Any) -> int:
    if value is None:
        return -1
    if isinstance(value, str):
        if target is None:
            return -1
        return value.find(str(target))
    items = list(_coerce_iterable(value))
    for idx, item in enumerate(items):
        if item == target:
            return idx
    return -1


def builtin_max(values: Sequence[Any]) -> Any:
    if values is None:
        return None
    iterable = list(values)
    if not iterable:
        return None
    return max(iterable)


def builtin_min(values: Sequence[Any]) -> Any:
    if values is None:
        return None
    iterable = list(values)
    if not iterable:
        return None
    return min(iterable)


def builtin_max_by(items: Any, criteria: Callable[..., Any]) -> Any:
    if items is None:
        return None
    iterable = list(_coerce_iterable(items))
    if not iterable:
        return None
    best_item = iterable[0]
    best_key = invoke_lambda(criteria, best_item, 0)
    for index, item in enumerate(iterable[1:], start=1):
        key = invoke_lambda(criteria, item, index)
        if key > best_key:
            best_key = key
            best_item = item
    return best_item


def builtin_min_by(items: Any, criteria: Callable[..., Any]) -> Any:
    if items is None:
        return None
    iterable = list(_coerce_iterable(items))
    if not iterable:
        return None
    best_item = iterable[0]
    best_key = invoke_lambda(criteria, best_item, 0)
    for index, item in enumerate(iterable[1:], start=1):
        key = invoke_lambda(criteria, item, index)
        if key < best_key:
            best_key = key
            best_item = item
    return best_item


def builtin_pluck(obj: Any, mapper: Callable[..., Any]) -> List[Any]:
    if obj is None:
        return None
    if not isinstance(obj, Mapping):
        raise TypeError("pluck expects an object")
    return [invoke_lambda(mapper, value, key, index) for index, (key, value) in enumerate(obj.items())]


def builtin_last_index_of(value: Any, target: Any) -> int:
    if value is None:
        return -1
    if isinstance(value, str):
        if target is None:
            return -1
        return value.rfind(str(target))
    iterable = list(_coerce_iterable(value))
    for index in range(len(iterable) - 1, -1, -1):
        if iterable[index] == target:
            return index
    return -1


def builtin_entries_of(obj: Any) -> Any:
    if obj is None:
        return None
    if isinstance(obj, Mapping):
        return [
            {
                "key": key,
                "value": value,
                "attributes": getattr(value, "attributes", {}),
            }
            for key, value in obj.items()
        ]
    raise TypeError("entriesOf expects an object")


_LOG_SENTINEL = object()


def _log_with_level(level: int, prefix: Optional[str], value: Any = _LOG_SENTINEL) -> Any:
    actual_prefix = prefix or ""
    actual_value = value
    if value is _LOG_SENTINEL:
        actual_value = prefix
        actual_prefix = ""
    message = f"{actual_prefix} - {actual_value}" if actual_prefix else str(actual_value)
    logging.log(level, message)
    return actual_value


def builtin_log(prefix: Optional[str], value: Any = _LOG_SENTINEL) -> Any:
    return _log_with_level(logging.WARNING, prefix, value)


def builtin_log_debug(prefix: Optional[str], value: Any = _LOG_SENTINEL) -> Any:
    return _log_with_level(logging.DEBUG, prefix, value)


def builtin_log_info(prefix: Optional[str], value: Any = _LOG_SENTINEL) -> Any:
    return _log_with_level(logging.INFO, prefix, value)


def builtin_log_warn(prefix: Optional[str], value: Any = _LOG_SENTINEL) -> Any:
    return _log_with_level(logging.WARNING, prefix, value)


def builtin_log_error(prefix: Optional[str], value: Any = _LOG_SENTINEL) -> Any:
    return _log_with_level(logging.ERROR, prefix, value)


def builtin_pow(base: Any, exponent: Any) -> Any:
    return math.pow(_coerce_number(base), _coerce_number(exponent))


def builtin_mod(dividend: Any, divisor: Any) -> Any:
    return _coerce_number(dividend) % _coerce_number(divisor)


def builtin_random() -> float:
    return random.random()


def builtin_random_int(upper_bound: Any) -> int:
    return int(random.random() * _coerce_number(upper_bound))


def builtin_days_between(start_date: str, end_date: str) -> int:
    start = _parse_datetime(start_date)
    end = _parse_datetime(end_date)
    delta = end - start
    return int(delta / timedelta(days=1))


def _coerce_date_value(value: Any) -> date:
    if isinstance(value, datetime):
        return value.date()
    if isinstance(value, date):
        return value
    text = str(value).strip().strip("|")
    if "T" in text:
        parsed = _parse_datetime(text)
        return parsed.date()
    return date.fromisoformat(text)


def _coerce_whole_number(value: Any, *, name: str) -> int:
    number = _coerce_number(value)
    if not float(number).is_integer():
        raise ValueError(f"{name} expects a whole number")
    return int(number)


def _coerce_period_value(value: Any, key: str, *, whole_only: bool) -> float:
    if value is None:
        return 0.0
    number = _coerce_number(value)
    if whole_only and not float(number).is_integer():
        raise ValueError(f"{key} only accepts whole numbers")
    return float(number)


def _period_between_dates(end_date: date, start_date: date) -> DWPeriod:
    total_months = (end_date.year * 12 + (end_date.month - 1)) - (
        start_date.year * 12 + (start_date.month - 1)
    )
    days = end_date.day - start_date.day
    if total_months > 0 and days < 0:
        total_months -= 1
        adjusted = _add_months_to_date(start_date, total_months)
        days = (end_date - adjusted).days
    elif total_months < 0 and days > 0:
        total_months += 1
        days -= calendar.monthrange(end_date.year, end_date.month)[1]
    years = int(total_months / 12)  # Truncate toward zero to match DW semantics.
    months = total_months - (years * 12)
    return DWPeriod(years=years, months=months, days=float(days), date_based=True)


def builtin_period(period_value: Any) -> DWPeriod:
    if period_value is None:
        period_value = {}
    if not isinstance(period_value, Mapping):
        raise TypeError("period expects an object")
    years = _coerce_period_value(period_value.get("years", 0), "years", whole_only=True)
    months = _coerce_period_value(period_value.get("months", 0), "months", whole_only=True)
    days = _coerce_period_value(period_value.get("days", 0), "days", whole_only=True)
    return DWPeriod(
        years=int(years),
        months=int(months),
        days=float(days),
        date_based=True,
    )


def builtin_duration(period_value: Any) -> DWPeriod:
    if period_value is None:
        period_value = {}
    if not isinstance(period_value, Mapping):
        raise TypeError("duration expects an object")
    days = _coerce_period_value(period_value.get("days", 0), "days", whole_only=False)
    hours = _coerce_period_value(period_value.get("hours", 0), "hours", whole_only=False)
    minutes = _coerce_period_value(period_value.get("minutes", 0), "minutes", whole_only=False)
    seconds = _coerce_period_value(period_value.get("seconds", 0), "seconds", whole_only=False)
    return DWPeriod(
        days=days,
        hours=hours,
        minutes=minutes,
        seconds=seconds,
        date_based=False,
    )


def builtin_years(n_years: Any) -> DWPeriod:
    return DWPeriod(years=_coerce_whole_number(n_years, name="years"), date_based=True)


def builtin_months(n_months: Any) -> DWPeriod:
    return DWPeriod(months=_coerce_whole_number(n_months, name="months"), date_based=True)


def builtin_days(n_days: Any) -> DWPeriod:
    number = _coerce_number(n_days)
    if float(number).is_integer():
        return DWPeriod(days=float(int(number)), date_based=True)
    return builtin_duration({"days": number})


def builtin_hours(n_hours: Any) -> DWPeriod:
    return builtin_duration({"hours": _coerce_number(n_hours)})


def builtin_minutes(n_minutes: Any) -> DWPeriod:
    return builtin_duration({"minutes": _coerce_number(n_minutes)})


def builtin_seconds(n_seconds: Any) -> DWPeriod:
    return builtin_duration({"seconds": _coerce_number(n_seconds)})


def builtin_between(end_date_exclusive: Any, start_date_inclusive: Any) -> DWPeriod:
    end_date = _coerce_date_value(end_date_exclusive)
    start_date = _coerce_date_value(start_date_inclusive)
    return _period_between_dates(end_date, start_date)


def builtin_match(text: Any, pattern: Any) -> List[str]:
    if text is None:
        return []
    pattern_text = str(pattern)
    if pattern_text.startswith("/") and pattern_text.endswith("/"):
        pattern_text = pattern_text[1:-1]
    regex = re.compile(pattern_text)
    match = regex.match(str(text))
    if match is None:
        return []
    return [match.group(0)] + list(match.groups())


def builtin_matches(text: Any, pattern: Any) -> bool:
    if text is None:
        return False
    pattern_text = str(pattern)
    if pattern_text.startswith("/") and pattern_text.endswith("/"):
        pattern_text = pattern_text[1:-1]
    regex = re.compile(pattern_text)
    return bool(regex.fullmatch(str(text)))


MODULE_EXPORTS: Dict[str, List[str]] = {
    "dw::core::Strings": [
        "appendIfMissing",
        "camelize",
        "capitalize",
        "charCode",
        "charCodeAt",
        "collapse",
        "contains",
        "countCharactersBy",
        "countMatches",
        "dasherize",
        "endsWith",
        "everyCharacter",
        "first",
        "fromCharCode",
        "hammingDistance",
        "isAlpha",
        "isAlphanumeric",
        "isLowerCase",
        "isNumeric",
        "isUpperCase",
        "isWhitespace",
        "startsWith",
        "joinBy",
        "last",
        "leftPad",
        "levenshteinDistance",
        "lines",
        "splitBy",
        "lower",
        "mapString",
        "ordinalize",
        "pluralize",
        "prependIfMissing",
        "remove",
        "repeat",
        "replaceAll",
        "reverse",
        "rightPad",
        "singularize",
        "someCharacter",
        "substring",
        "substringAfter",
        "substringAfterLast",
        "substringBefore",
        "substringBeforeLast",
        "substringBy",
        "substringEvery",
        "underscore",
        "unwrap",
        "upper",
        "withMaxSize",
        "words",
        "wrapIfMissing",
        "wrapWith",
        "trim",
        "sizeOf",
    ],
    "dw::core::Periods": [
        "between",
        "days",
        "duration",
        "hours",
        "minutes",
        "months",
        "period",
        "seconds",
        "years",
    ],
    "dw::core::Objects": [
        "entrySet",
        "nameSet",
        "keySet",
        "valueSet",
        "mergeWith",
        "divideBy",
        "takeWhile",
        "everyEntry",
        "someEntry",
    ],
}


def resolve_module_exports(module: str) -> Dict[str, Callable[..., Any]]:
    exports: Dict[str, Callable[..., Any]] = {}
    names = MODULE_EXPORTS.get(module)
    if not names:
        return exports
    for name in names:
        func = CORE_FUNCTIONS.get(name)
        if func is not None:
            exports[name] = func
    return exports


def builtin_filter_object(obj: Any, criteria: Callable[..., Any]) -> Any:
    if obj is None:
        return None
    if not isinstance(obj, Mapping):
        raise TypeError("filterObject expects an object")
    if criteria is None:
        return dict(obj)
    result: Dict[Any, Any] = {}
    for index, (key, value) in enumerate(obj.items()):
        if invoke_lambda(criteria, value, key, index):
            result[key] = value
    return result


def _normalise_group_key(raw_key: Any) -> str:
    return str(_hashable_key(raw_key))


def builtin_divide_by(items: Any, amount: Any) -> List[Any]:
    try:
        size = int(amount)
    except (TypeError, ValueError):
        raise TypeError("divideBy expects a numeric amount")
    if size <= 0:
        return []
    if items is None:
        return []
    if isinstance(items, Mapping):
        groups: List[Dict[Any, Any]] = []
        current: Dict[Any, Any] = {}
        for key, value in items.items():
            current[key] = value
            if len(current) == size:
                groups.append(current)
                current = {}
        if current:
            groups.append(current)
        return groups
    iterable: Iterable[Any]
    if isinstance(items, (list, tuple)):
        iterable = items
    else:
        iterable = _coerce_iterable(items)
    groups: List[List[Any]] = []
    current: List[Any] = []
    for value in iterable:
        current.append(value)
        if len(current) == size:
            groups.append(current)
            current = []
    if current:
        groups.append(current)
    return groups


def builtin_filter(items: Any, condition: Callable[..., Any]) -> Any:
    if items is None:
        return [] if not isinstance(items, Mapping) else {}
    if condition is None:
        return dict(items) if isinstance(items, Mapping) else list(_coerce_iterable(items))
    if isinstance(items, Mapping):
        result: Dict[Any, Any] = {}
        for index, (key, value) in enumerate(items.items()):
            if invoke_lambda(condition, value, key, index):
                result[key] = value
        return result
    result = []
    for index, value in enumerate(_coerce_iterable(items)):
        if invoke_lambda(condition, value, index):
            result.append(value)
    return result


def builtin_entry_set(obj: Any) -> Any:
    return builtin_entries_of(obj)


def builtin_name_set(obj: Any) -> Optional[List[str]]:
    if obj is None:
        return None
    if not isinstance(obj, Mapping):
        raise TypeError("nameSet expects an object")
    return [str(key) for key in obj.keys()]


def builtin_key_set(obj: Any) -> Optional[List[Any]]:
    if obj is None:
        return None
    if not isinstance(obj, Mapping):
        raise TypeError("keySet expects an object")
    return list(obj.keys())


def builtin_value_set(obj: Any) -> Optional[List[Any]]:
    if obj is None:
        return None
    if not isinstance(obj, Mapping):
        raise TypeError("valueSet expects an object")
    values: List[Any] = []
    for value in obj.values():
        if isinstance(value, XMLNodeList):
            for entry in value:
                values.append(_collapse_xml_node(entry))
        else:
            values.append(_collapse_xml_node(value))
    return values


def builtin_merge_with(source: Any, target: Any) -> Any:
    if source is None:
        return dict(target) if isinstance(target, Mapping) else target
    if target is None:
        return dict(source) if isinstance(source, Mapping) else source
    if not isinstance(source, Mapping) or not isinstance(target, Mapping):
        raise TypeError("mergeWith expects objects")
    result = dict(source)
    for key in target.keys():
        result.pop(key, None)
    result.update(target)
    return result


def builtin_take_while(obj: Any, condition: Callable[..., Any]) -> Any:
    if obj is None:
        return {}
    if not isinstance(obj, Mapping):
        raise TypeError("takeWhile expects an object")
    if condition is None:
        raise TypeError("takeWhile expects a condition function")
    result: Dict[Any, Any] = {}
    for index, (key, value) in enumerate(obj.items()):
        if invoke_lambda(condition, value, key, index):
            result[key] = value
        else:
            break
    return result


def _collapse_xml_node(value: Any) -> Any:
    if isinstance(value, XMLNodeList):
        return [_collapse_xml_node(item) for item in value]
    if isinstance(value, XMLNodeDict):
        text_value = value.get("#text")
        if text_value is not None:
            return text_value
        collapsed: Dict[str, Any] = {}
        for key, child in value.items():
            if key.startswith("@"):
                continue
            collapsed[key] = _collapse_xml_node(child)
        return collapsed or text_value
    return value


def builtin_every_entry(obj: Any, condition: Callable[..., Any]) -> bool:
    if obj is None:
        return True
    if not isinstance(obj, Mapping):
        raise TypeError("everyEntry expects an object")
    if condition is None:
        raise TypeError("everyEntry expects a condition function")
    for index, (key, value) in enumerate(obj.items()):
        if not invoke_lambda(condition, value, key, index):
            return False
    return True


def builtin_some_entry(obj: Any, condition: Callable[..., Any]) -> bool:
    if obj is None:
        return False
    if not isinstance(obj, Mapping):
        raise TypeError("someEntry expects an object")
    if condition is None:
        raise TypeError("someEntry expects a condition function")
    for index, (key, value) in enumerate(obj.items()):
        if invoke_lambda(condition, value, key, index):
            return True
    return False


def builtin_group_by(items: Any, criteria: Callable[..., Any]) -> Any:
    if items is None:
        return None
    if isinstance(items, Mapping):
        result: Dict[str, Dict[Any, Any]] = {}
        for index, (key, value) in enumerate(items.items()):
            group_key = _normalise_group_key(invoke_lambda(criteria, value, key, index)) if criteria else _normalise_group_key(key)
            bucket = result.setdefault(group_key, {})
            bucket[key] = value
        return result
    iterable = list(_coerce_iterable(items))
    if criteria is None:
        return {str(index): [item] for index, item in enumerate(iterable)}
    grouped: Dict[str, List[Any]] = {}
    for index, item in enumerate(iterable):
        group_key = _normalise_group_key(invoke_lambda(criteria, item, index))
        grouped.setdefault(group_key, []).append(item)
    return grouped


def builtin_order_by(items: Any, criteria: Optional[Callable[..., Any]]) -> Any:
    if items is None:
        return None
    if isinstance(items, Mapping):
        entries = list(items.items())
        if criteria is None:
            ordered = sorted(entries, key=lambda pair: pair[0])
        else:
            ordered = sorted(
                entries,
                key=lambda pair: invoke_lambda(criteria, pair[1], pair[0]),
            )
        return {key: value for key, value in ordered}
    iterable = list(_coerce_iterable(items))
    if criteria is None:
        return sorted(iterable)
    decorated = [
        (invoke_lambda(criteria, item, index), index, item)
        for index, item in enumerate(iterable)
    ]
    decorated.sort(key=lambda entry: (entry[0], entry[1]))
    return [item for _, _, item in decorated]


def builtin_find(value: Any, matcher: Any) -> Any:
    if value is None:
        return []
    if isinstance(value, str):
        if matcher is None:
            return []
        if isinstance(matcher, str) and matcher.startswith("/") and matcher.endswith("/"):
            pattern = re.compile(matcher[1:-1])
            return [match.start() for match in pattern.finditer(value)]
        needle = str(matcher)
        indices: List[int] = []
        start = 0
        step = max(len(needle), 1)
        while True:
            idx = value.find(needle, start)
            if idx == -1:
                break
            indices.append(idx)
            start = idx + step
        return indices
    iterable = list(_coerce_iterable(value))
    return [index for index, item in enumerate(iterable) if item == matcher]


def builtin_split_by(text: Any, separator: Any) -> Any:
    if text is None:
        return None
    string = str(text)
    if separator is None:
        return [string]
    if isinstance(separator, str) and separator.startswith("/") and separator.endswith("/"):
        pattern = re.compile(separator[1:-1])
        return [segment for segment in pattern.split(string)]
    sep = str(separator)
    if sep == "":
        return list(string)
    return string.split(sep)


def builtin_to(start: Any, end: Any) -> List[Any]:
    start_num = int(_coerce_number(start))
    end_num = int(_coerce_number(end))
    step = 1 if end_num >= start_num else -1
    return list(range(start_num, end_num + step, step))


CORE_FUNCTIONS: Dict[str, Callable[..., Any]] = {
    "_binary_concat": binary_concat,
    "_binary_diff": binary_diff,
    "abs": builtin_abs,
    "appendIfMissing": builtin_append_if_missing,
    "avg": builtin_avg,
    "between": builtin_between,
    "camelize": builtin_camelize,
    "capitalize": builtin_capitalize,
    "ceil": builtin_ceil,
    "charCode": builtin_char_code,
    "charCodeAt": builtin_char_code_at,
    "collapse": builtin_collapse,
    "contains": builtin_contains,
    "countCharactersBy": builtin_count_characters_by,
    "countMatches": builtin_count_matches,
    "dasherize": builtin_dasherize,
    "days": builtin_days,
    "endsWith": builtin_endswith,
    "entriesOf": builtin_entries_of,
    "entrySet": builtin_entry_set,
    "everyCharacter": builtin_every_character,
    "first": builtin_first,
    "fromCharCode": builtin_from_char_code,
    "isBlank": builtin_is_blank,
    "isAlpha": builtin_is_alpha,
    "isAlphanumeric": builtin_is_alphanumeric,
    "isLowerCase": builtin_is_lower_case,
    "isNumeric": builtin_is_numeric,
    "isUpperCase": builtin_is_upper_case,
    "isWhitespace": builtin_is_whitespace,
    "isDecimal": builtin_is_decimal,
    "filterObject": builtin_filter_object,
    "find": builtin_find,
    "divideBy": builtin_divide_by,
    "duration": builtin_duration,
    "hammingDistance": builtin_hamming_distance,
    "hours": builtin_hours,
    "mergeWith": builtin_merge_with,
    "filter": builtin_filter,
    "nameSet": builtin_name_set,
    "keySet": builtin_key_set,
    "valueSet": builtin_value_set,
    "takeWhile": builtin_take_while,
    "everyEntry": builtin_every_entry,
    "someEntry": builtin_some_entry,
    "floor": builtin_floor,
    "flatMap": builtin_flat_map,
    "flatten": builtin_flatten,
    "isEmpty": builtin_is_empty,
    "isInteger": builtin_is_integer,
    "isEven": builtin_is_even,
    "isOdd": builtin_is_odd,
    "indexOf": builtin_index_of,
    "joinBy": builtin_joinby,
    "keysOf": builtin_keys_of,
    "last": builtin_last,
    "leftPad": builtin_left_pad,
    "lower": builtin_lower,
    "levenshteinDistance": builtin_levenshtein_distance,
    "lines": builtin_lines,
    "lastIndexOf": builtin_last_index_of,
    "mapString": builtin_map_string,
    "max": builtin_max,
    "min": builtin_min,
    "maxBy": builtin_max_by,
    "minBy": builtin_min_by,
    "minutes": builtin_minutes,
    "months": builtin_months,
    "now": builtin_now,
    "ordinalize": builtin_ordinalize,
    "pluralize": builtin_pluralize,
    "prependIfMissing": builtin_prepend_if_missing,
    "distinctBy": builtin_distinct_by,
    "groupBy": builtin_group_by,
    "orderBy": builtin_order_by,
    "period": builtin_period,
    "match": builtin_match,
    "matches": builtin_matches,
    "remove": builtin_remove,
    "repeat": builtin_repeat,
    "replaceAll": builtin_replace_all,
    "reverse": builtin_reverse,
    "rightPad": builtin_right_pad,
    "round": builtin_round,
    "seconds": builtin_seconds,
    "singularize": builtin_singularize,
    "someCharacter": builtin_some_character,
    "splitBy": builtin_split_by,
    "substring": builtin_substring,
    "substringAfter": builtin_substring_after,
    "substringAfterLast": builtin_substring_after_last,
    "substringBefore": builtin_substring_before,
    "substringBeforeLast": builtin_substring_before_last,
    "substringBy": builtin_substring_by,
    "substringEvery": builtin_substring_every,
    "to": builtin_to,
    "underscore": builtin_underscore,
    "unwrap": builtin_unwrap,
    "sizeOf": builtin_size_of,
    "startsWith": builtin_startswith,
    "sum": builtin_sum,
    "trim": builtin_trim,
    "years": builtin_years,
    "withMaxSize": builtin_with_max_size,
    "words": builtin_words,
    "wrapIfMissing": builtin_wrap_if_missing,
    "wrapWith": builtin_wrap_with,
    "pluck": builtin_pluck,
    "upper": lambda value: None if value is None else str(value).upper(),
    "valuesOf": builtin_values_of,
    "log": builtin_log,
    "logDebug": builtin_log_debug,
    "logInfo": builtin_log_info,
    "logWarn": builtin_log_warn,
    "logError": builtin_log_error,
    "random": builtin_random,
    "randomInt": builtin_random_int,
}
INFIX_ALIASES: Dict[str, str] = {
    "map": "map",
    "reduce": "reduce",
    "filter": "filter",
    "flatMap": "flatMap",
    "distinctBy": "distinctBy",
    "contains": "contains",
    "startsWith": "startsWith",
    "endsWith": "endsWith",
    "joinBy": "joinBy",
    "splitBy": "splitBy",
    "indexOf": "indexOf",
    "find": "find",
    "orderBy": "orderBy",
    "groupBy": "groupBy",
}
