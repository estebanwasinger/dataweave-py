from __future__ import annotations

import json
import logging
import math
import random
import re
from datetime import datetime, timedelta, timezone
from typing import Any, Callable, Dict, Iterable, List, Mapping, Optional, Sequence


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


def _log_with_level(level: int, prefix: str, value: Any) -> Any:
    message = f"{prefix} - {value}" if prefix else str(value)
    logging.log(level, message)
    return value


def builtin_log(prefix: Optional[str], value: Any) -> Any:
    return _log_with_level(logging.WARNING, prefix or "", value)


def builtin_log_debug(prefix: Optional[str], value: Any) -> Any:
    return _log_with_level(logging.DEBUG, prefix or "", value)


def builtin_log_info(prefix: Optional[str], value: Any) -> Any:
    return _log_with_level(logging.INFO, prefix or "", value)


def builtin_log_warn(prefix: Optional[str], value: Any) -> Any:
    return _log_with_level(logging.WARNING, prefix or "", value)


def builtin_log_error(prefix: Optional[str], value: Any) -> Any:
    return _log_with_level(logging.ERROR, prefix or "", value)


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
        "contains",
        "endsWith",
        "startsWith",
        "joinBy",
        "splitBy",
        "lower",
        "upper",
        "trim",
        "sizeOf",
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
    return list(obj.values())


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
    "avg": builtin_avg,
    "ceil": builtin_ceil,
    "contains": builtin_contains,
    "endsWith": builtin_endswith,
    "entriesOf": builtin_entries_of,
    "entrySet": builtin_entry_set,
    "isBlank": builtin_is_blank,
    "isDecimal": builtin_is_decimal,
    "filterObject": builtin_filter_object,
    "find": builtin_find,
    "divideBy": builtin_divide_by,
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
    "lower": builtin_lower,
    "lastIndexOf": builtin_last_index_of,
    "max": builtin_max,
    "min": builtin_min,
    "maxBy": builtin_max_by,
    "minBy": builtin_min_by,
    "now": builtin_now,
    "distinctBy": builtin_distinct_by,
    "groupBy": builtin_group_by,
    "orderBy": builtin_order_by,
    "match": builtin_match,
    "matches": builtin_matches,
    "round": builtin_round,
    "splitBy": builtin_split_by,
    "to": builtin_to,
    "sizeOf": builtin_size_of,
    "startsWith": builtin_startswith,
    "sum": builtin_sum,
    "trim": builtin_trim,
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
