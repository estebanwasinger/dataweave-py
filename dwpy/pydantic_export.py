from __future__ import annotations

from collections.abc import Callable
from functools import reduce
from operator import or_
from typing import Any, cast

from pydantic import BaseModel, ConfigDict, create_model

from .type_inference import infer_script_type
from .typesystem import (
    AnyType,
    ArrayType,
    BinaryType,
    BooleanType,
    DWType,
    FunctionType,
    NullType,
    NumberType,
    ObjectType,
    StringType,
    UnionType,
)


def python_type_from_dw_type(
    dw_type: DWType,
    *,
    model_name: str = "DataWeaveModel",
    registry: dict[DWType, type[BaseModel]] | None = None,
) -> Any:
    registry = registry or {}
    return _build_python_type(dw_type, model_name, registry)


def pydantic_model_from_script(
    script_source: str,
    *,
    model_name: str = "DataWeaveOutput",
) -> Any:
    inferred = infer_script_type(script_source)
    return python_type_from_dw_type(inferred, model_name=model_name)


def _build_python_type(
    dw_type: DWType,
    model_name: str,
    registry: dict[DWType, type[BaseModel]],
) -> Any:
    if isinstance(dw_type, AnyType):
        return Any
    if isinstance(dw_type, StringType):
        return str
    if isinstance(dw_type, NumberType):
        return float
    if isinstance(dw_type, BooleanType):
        return bool
    if isinstance(dw_type, NullType):
        return type(None)
    if isinstance(dw_type, BinaryType):
        return bytes
    if isinstance(dw_type, ArrayType):
        item_type = _build_python_type(dw_type.element, f"{model_name}Item", registry)
        return list.__class_getitem__(item_type)
    if isinstance(dw_type, ObjectType):
        if dw_type in registry:
            return registry[dw_type]
        fields: dict[str, tuple[Any, Any]] = {}
        for key, (value_type, is_optional, is_repeat) in dw_type.field_dict().items():
            field_annotation = _build_python_type(
                value_type, f"{model_name}_{key.capitalize()}", registry
            )
            if is_repeat:
                field_annotation = list.__class_getitem__(field_annotation)
            allow_none = is_optional or _allows_none(value_type)
            if allow_none:
                field_annotation = field_annotation | None
            default = None if allow_none else ...
            fields[key] = (field_annotation, default)

        config = ConfigDict(extra="allow" if dw_type.open else "forbid")
        dynamic_create_model = cast(Callable[..., type[BaseModel]], create_model)
        model = dynamic_create_model(
            model_name,
            __module__="dwpy.pydantic_export",
            __config__=config,
            **fields,
        )
        registry[dw_type] = model
        return model
    if isinstance(dw_type, UnionType):
        annotations = tuple(
            _build_python_type(option, model_name, registry) for option in dw_type.options
        )
        if any(ann is type(None) for ann in annotations):
            non_none = tuple(ann for ann in annotations if ann is not type(None))
            if not non_none:
                return type(None)
            return reduce(or_, non_none) | None
        if len(annotations) == 1:
            return annotations[0]
        return reduce(or_, annotations)
    if isinstance(dw_type, FunctionType):
        return Any
    return Any


def _allows_none(dw_type: DWType) -> bool:
    if isinstance(dw_type, NullType):
        return True
    if isinstance(dw_type, UnionType):
        return any(_allows_none(option) for option in dw_type.options)
    return False
