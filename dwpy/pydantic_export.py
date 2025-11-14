from __future__ import annotations

from typing import Any, Dict, Optional, Tuple, Type, Union, List, get_args

from pydantic import BaseModel, create_model

from .type_inference import infer_script_type
from .typesystem import (
    DWType,
    AnyType,
    StringType,
    NumberType,
    BooleanType,
    NullType,
    BinaryType,
    ArrayType,
    ObjectType,
    UnionType,
    FunctionType,
    ANY,
)


def python_type_from_dw_type(
    dw_type: DWType,
    *,
    model_name: str = "DataWeaveModel",
    registry: Optional[Dict[DWType, Type[BaseModel]]] = None,
) -> Type[Any]:
    registry = registry or {}
    return _build_python_type(dw_type, model_name, registry)


def pydantic_model_from_script(
    script_source: str,
    *,
    model_name: str = "DataWeaveOutput",
) -> Type[Any]:
    inferred = infer_script_type(script_source)
    return python_type_from_dw_type(inferred, model_name=model_name)


def _build_python_type(
    dw_type: DWType,
    model_name: str,
    registry: Dict[DWType, Type[BaseModel]],
) -> Type[Any]:
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
        return List[item_type]  # type: ignore[index]
    if isinstance(dw_type, ObjectType):
        if dw_type in registry:
            return registry[dw_type]
        fields: Dict[str, Tuple[Type[Any], Any]] = {}
        for key, value_type in dw_type.field_dict().items():
            field_annotation = _build_python_type(
                value_type, f"{model_name}_{key.capitalize()}", registry
            )
            allow_none = _allows_none(value_type)
            if allow_none:
                field_annotation = Optional[field_annotation]  # type: ignore[index]
            default = None if allow_none else ...
            fields[key] = (field_annotation, default)

        extra = "allow" if dw_type.open else "forbid"
        config = {"extra": extra}
        model = create_model(
            model_name,
            __base__=BaseModel,
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
            if len(non_none) == 1:
                return Optional[non_none[0]]  # type: ignore[index]
            return Optional[Union[non_none]]  # type: ignore[index]
        if len(annotations) == 1:
            return annotations[0]
        return Union[annotations]  # type: ignore[index]
    if isinstance(dw_type, FunctionType):
        return Any
    return Any


def _allows_none(dw_type: DWType) -> bool:
    if isinstance(dw_type, NullType):
        return True
    if isinstance(dw_type, UnionType):
        return any(_allows_none(option) for option in dw_type.options)
    return False
