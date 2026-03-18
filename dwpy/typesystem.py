from __future__ import annotations

from dataclasses import dataclass
from typing import Any, Dict, Iterable, List, Optional, Tuple


class DWType:
    def describe(self) -> str:
        raise NotImplementedError


@dataclass(frozen=True)
class AnyType(DWType):
    def describe(self) -> str:
        return "Any"


@dataclass(frozen=True)
class StringType(DWType):
    def describe(self) -> str:
        return "String"


@dataclass(frozen=True)
class NumberType(DWType):
    def describe(self) -> str:
        return "Number"


@dataclass(frozen=True)
class BooleanType(DWType):
    def describe(self) -> str:
        return "Boolean"


@dataclass(frozen=True)
class NullType(DWType):
    def describe(self) -> str:
        return "Null"


@dataclass(frozen=True)
class NothingType(DWType):
    def describe(self) -> str:
        return "Nothing"


@dataclass(frozen=True)
class BinaryType(DWType):
    def describe(self) -> str:
        return "Binary"


@dataclass(frozen=True)
class LiteralType(DWType):
    value: Any
    base_type: DWType

    def describe(self) -> str:
        if isinstance(self.value, str):
            return f'"{self.value}"'
        elif self.value is True:
            return "true"
        elif self.value is False:
            return "false"
        return str(self.value)


@dataclass(frozen=True)
class ArrayType(DWType):
    element: DWType

    def describe(self) -> str:
        return f"Array<{self.element.describe()}>"


@dataclass(frozen=True)
class ObjectType(DWType):
    fields: Tuple[Tuple[str, DWType, bool, bool], ...]
    is_open: bool = True

    def describe(self) -> str:
        body_parts: List[str] = []
        for name, type_, is_optional, is_repeatable in self.fields:
            modifier = "?" if is_optional else ("*" if is_repeatable else "")
            body_parts.append(f"{name}{modifier}: {type_.describe()}")

        body = ", ".join(body_parts)
        suffix = "..." if self.is_open else ""
        parts = [part for part in (body, suffix) if part]
        return "{ " + ", ".join(parts) + " }"

    def field_dict(self) -> Dict[str, Tuple[DWType, bool, bool]]:
        return {name: (type_, is_optional, is_repeatable) for name, type_, is_optional, is_repeatable in self.fields}

    def get(self, name: str) -> Optional[Tuple[DWType, bool, bool]]:
        for field_name, field_type, is_optional, is_repeatable in self.fields:
            if field_name == name:
                return (field_type, is_optional, is_repeatable)
        return None

    @property
    def open(self) -> bool:
        """Compatibility alias so existing callers using `.open` keep working."""
        return self.is_open


@dataclass(frozen=True)
class FunctionType(DWType):
    parameter_types: List[DWType]
    return_type: DWType

    def describe(self) -> str:
        params = ", ".join(p.describe() for p in self.parameter_types)
        return f"Function({params}) -> {self.return_type.describe()}"


@dataclass(frozen=True)
class UnionType(DWType):
    options: Tuple[DWType, ...]

    def describe(self) -> str:
        return " | ".join(sorted({opt.describe() for opt in self.options}))


@dataclass(frozen=True)
class IntersectionType(DWType):
    options: Tuple[DWType, ...]

    def describe(self) -> str:
        return " & ".join(sorted({opt.describe() for opt in self.options}))


# Common singletons
ANY = AnyType()
STRING = StringType()
NUMBER = NumberType()
BOOLEAN = BooleanType()
NULL = NullType()
NOTHING = NothingType()
BINARY = BinaryType()


def object_type(
    fields_map: Dict[str, Any],
    is_open_flag: bool = True,
    open: Optional[bool] = None
) -> ObjectType:
    """
    Build an ObjectType.

    The values in `fields_map` can be either a DWType (shorthand, treated as required/non-repeatable)
    or a tuple of (DWType, is_optional, is_repeatable).
    """
    normalised_items: List[Tuple[str, DWType, bool, bool]] = []
    for name, value in fields_map.items():
        if isinstance(value, tuple) and len(value) == 3:
            type_, is_optional, is_repeatable = value
        else:
            type_, is_optional, is_repeatable = value, False, False
        normalised_items.append((name, type_, is_optional, is_repeatable))

    items = tuple(sorted(normalised_items, key=lambda item: item[0]))
    effective_open = is_open_flag if open is None else open
    return ObjectType(fields=items, is_open=effective_open)


def array_type(element: DWType) -> ArrayType:
    return ArrayType(element=element)


def union_types(*types: DWType) -> DWType:
    flattened = list(_flatten_union(types))
    
    # Remove NothingType as it doesn't add to the union
    filtered = [t for t in flattened if not isinstance(t, NothingType)]

    if not filtered:
        return NOTHING # If all were NothingType or input was empty
    
    unique: list[DWType] = []
    for t in filtered:
        if t not in unique:
            unique.append(t)
    
    if not unique:
        return NOTHING
    if len(unique) == 1:
        return unique[0]
    
    # If AnyType is in the union, the whole union is AnyType
    if any(isinstance(t, AnyType) for t in unique):
        return ANY
    
    # Sort for consistent representation
    return UnionType(options=tuple(sorted(unique, key=lambda x: x.describe())))


def _flatten_union(types: Iterable[DWType]) -> Iterable[DWType]:
    for t in types:
        if isinstance(t, UnionType):
            yield from t.options
        elif isinstance(t, IntersectionType):
            # An intersection within a union: flatten its components
            yield from t.options
        else:
            yield t


def is_string(type_: DWType) -> bool:
    if isinstance(type_, StringType):
        return True
    if isinstance(type_, UnionType):
        return all(is_string(option) for option in type_.options)
    return False


def is_number(type_: DWType) -> bool:
    if isinstance(type_, NumberType):
        return True
    if isinstance(type_, UnionType):
        return all(is_number(option) for option in type_.options)
    return False


def is_boolean(type_: DWType) -> bool:
    if isinstance(type_, BooleanType):
        return True
    if isinstance(type_, UnionType):
        return all(is_boolean(option) for option in type_.options)
    return False


def is_array(type_: DWType) -> bool:
    if isinstance(type_, ArrayType):
        return True
    if isinstance(type_, UnionType):
        return all(is_array(option) for option in type_.options)
    return False


def merge_array_types(left: ArrayType, right: ArrayType) -> ArrayType:
    element = union_types(left.element, right.element)
    return ArrayType(element=element)


def merge_object_types(left: ObjectType, right: ObjectType) -> ObjectType:
    merged_fields: Dict[str, Tuple[DWType, bool, bool]] = {}

    # Collect fields from left
    for key, (l_type, l_optional, l_repeatable) in left.field_dict().items():
        if key in right.field_dict():
            r_type, r_optional, r_repeatable = right.field_dict()[key]
            # For intersection, if a field is defined in both, its type is the intersection of both types.
            # Optional/Repeatable flags: if either is optional, the result is optional. If either is repeatable, the result is repeatable.
            merged_fields[key] = (
                union_types(l_type, r_type), # Using union_types for now, should be intersection
                l_optional or r_optional,
                l_repeatable or r_repeatable,
            )
        else:
            merged_fields[key] = (l_type, l_optional, l_repeatable)
    
    # Add fields unique to right
    for key, (r_type, r_optional, r_repeatable) in right.field_dict().items():
        if key not in merged_fields:
            merged_fields[key] = (r_type, r_optional, r_repeatable)

    open_flag = left.open and right.open # For intersection, if either is closed, the result is closed
    return object_type(merged_fields, is_open_flag=open_flag)


def intersection_types(*types: DWType) -> DWType:
    flattened = list(_flatten_intersection(types))
    if not flattened:
        return ANY # Intersection of no types is Any (no restrictions)

    # If NOTHING is in the intersection, the result is NOTHING
    if any(isinstance(t, NothingType) for t in flattened):
        return NOTHING

    # If ANY is in the intersection, it can be removed as it doesn't restrict
    filtered = [t for t in flattened if not isinstance(t, AnyType)]

    if not filtered:
        return ANY
    if len(filtered) == 1:
        return filtered[0]

    # Basic intersection of simple types (e.g., String & Number = Nothing) needs to be handled
    # For now, we will only handle object intersections more deeply

    # Merge objects if present
    objects = [t for t in filtered if isinstance(t, ObjectType)]
    non_objects = [t for t in filtered if not isinstance(t, ObjectType)]

    if objects:
        base_object = objects[0]
        for obj in objects[1:]:
            base_object = merge_object_types(base_object, obj)
        
        if not non_objects: # Only objects were intersected
            return base_object
        else:
            # Intersect the merged object with non-objects. This is complex.
            # For now, return an IntersectionType containing the merged object and other types.
            return IntersectionType(options=tuple(sorted([base_object] + non_objects, key=lambda x: x.describe())))

    # If no objects, return a generic IntersectionType for now
    return IntersectionType(options=tuple(sorted(filtered, key=lambda x: x.describe())))


def _flatten_intersection(types: Iterable[DWType]) -> Iterable[DWType]:
    for t in types:
        if isinstance(t, IntersectionType):
            yield from t.options
        elif isinstance(t, UnionType):
            # An union within an intersection implies distributive law (A | B) & C = (A & C) | (B & C).
            # This is too complex to fully implement in flattening. For now, we will simplify
            # by treating it as if the union elements are independent components of the intersection.
            # This might be lossy for true intersection semantics.
            yield from t.options
        else:
            yield t
