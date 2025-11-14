from __future__ import annotations

from dataclasses import dataclass
from typing import Iterable, Tuple, Dict, Optional


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
class BinaryType(DWType):
    def describe(self) -> str:
        return "Binary"


@dataclass(frozen=True)
class ArrayType(DWType):
    element: DWType

    def describe(self) -> str:
        return f"Array<{self.element.describe()}>"


@dataclass(frozen=True)
class ObjectType(DWType):
    fields: Tuple[Tuple[str, DWType], ...]
    open: bool = True

    def describe(self) -> str:
        body = ", ".join(f"{name}: {type_.describe()}" for name, type_ in self.fields)
        suffix = ", ..." if self.open and self.fields else ("..." if self.open else "")
        parts = [part for part in (body, suffix) if part]
        return "{ " + ", ".join(parts) + " }"

    def field_dict(self) -> Dict[str, DWType]:
        return dict(self.fields)

    def get(self, name: str) -> Optional[DWType]:
        return self.field_dict().get(name)


@dataclass(frozen=True)
class FunctionType(DWType):
    parameter_count: int
    return_type: DWType

    def describe(self) -> str:
        return f"Function({self.parameter_count}) -> {self.return_type.describe()}"


@dataclass(frozen=True)
class UnionType(DWType):
    options: Tuple[DWType, ...]

    def describe(self) -> str:
        return " | ".join(sorted({opt.describe() for opt in self.options}))


# Common singletons
ANY = AnyType()
STRING = StringType()
NUMBER = NumberType()
BOOLEAN = BooleanType()
NULL = NullType()
BINARY = BinaryType()


def object_type(fields: Dict[str, DWType], open: bool = True) -> ObjectType:
    items = tuple(sorted(fields.items(), key=lambda item: item[0]))
    return ObjectType(fields=items, open=open)


def array_type(element: DWType) -> ArrayType:
    return ArrayType(element=element)


def union_types(*types: DWType) -> DWType:
    flattened = list(_flatten_union(types))
    if not flattened:
        return ANY
    unique: list[DWType] = []
    for t in flattened:
        if t not in unique:
            unique.append(t)
    if not unique:
        return ANY
    if len(unique) == 1:
        return unique[0]
    if any(isinstance(t, AnyType) for t in unique):
        return ANY
    return UnionType(options=tuple(unique))


def _flatten_union(types: Iterable[DWType]) -> Iterable[DWType]:
    for t in types:
        if isinstance(t, UnionType):
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
    merged_fields: Dict[str, DWType] = {}
    for key, type_ in left.field_dict().items():
        if key in right.field_dict():
            merged_fields[key] = union_types(type_, right.field_dict()[key])
        else:
            merged_fields[key] = type_
    for key, type_ in right.field_dict().items():
        if key not in merged_fields:
            merged_fields[key] = type_
    open_flag = left.open or right.open
    return object_type(merged_fields, open=open_flag)
