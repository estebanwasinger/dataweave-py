from __future__ import annotations

from dataclasses import dataclass
from typing import Dict, List, Optional, Tuple, Any, Union, get_origin, get_args

from . import parser
from .parser import (
    ReferenceTypeSpec, ObjectTypeSpec, UnionTypeSpec as ParserUnionTypeSpec,
    IntersectionTypeSpec as ParserIntersectionTypeSpec, FunctionTypeSpec as ParserFunctionTypeSpec,
    LiteralTypeSpec as ParserLiteralTypeSpec, TypeSpec as ParserTypeSpec
)
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
    FunctionType,
    UnionType,
    IntersectionType,
    LiteralType,
    NothingType,
    ANY,
    STRING,
    NUMBER,
    BOOLEAN,
    NULL,
    BINARY,
    NOTHING,
    array_type,
    object_type,
    union_types,
    intersection_types,
    merge_array_types,
    merge_object_types,
    is_string,
    is_number,
    is_boolean,
    is_array,
)


class TypeInferenceContext:
    def __init__(self, payload_type: DWType, vars_type: DWType) -> None:
        self.payload_type = payload_type
        self.vars_type = vars_type
        self.env: Dict[str, DWType] = {}
        # Keep type definitions case-insensitive to mirror DW resolution
        self._defined_types: Dict[str, DWType] = {}

    @staticmethod
    def _normalize_name(name: str) -> str:
        return name.lower()

    def lookup(self, name: str) -> DWType:
        if name == "payload":
            return self.payload_type
        if name == "vars":
            return self.vars_type
        
        # Check defined types first
        defined = self._defined_types.get(self._normalize_name(name))
        if defined is not None:
            return defined

        return self.env.get(name, ANY)

    def bind(self, name: str, type_: DWType) -> None:
        self.env[name] = type_

    def define_type(self, name: str, type_: DWType) -> None:
        self._defined_types[self._normalize_name(name)] = type_

    def resolve_type(self, name: str) -> Optional[DWType]:
        return self._defined_types.get(self._normalize_name(name))

    def clone(self) -> "TypeInferenceContext":
        new_ctx = TypeInferenceContext(self.payload_type, self.vars_type)
        new_ctx.env = dict(self.env)
        new_ctx._defined_types = dict(self._defined_types)
        return new_ctx


def infer_script_type(
    script_source: str,
    *,
    payload_type: DWType | Any = ANY,
    vars_type: DWType | Any = ANY,
) -> DWType:
    payload_type = _python_value_to_type(payload_type)
    vars_type = _python_value_to_type(vars_type)
    script = parser.parse_script(script_source)
    inferencer = TypeInferencer(payload_type=payload_type, vars_type=vars_type)
    return inferencer.infer_script(script)


def _python_value_to_type(value: Any) -> DWType:
    """
    Convenience: allow passing concrete Python payloads/vars instead of DWType.
    """
    if isinstance(value, DWType):
        return value
    if value is None:
        return NULL
    if isinstance(value, bool):
        return BOOLEAN
    if isinstance(value, (int, float)) and not isinstance(value, bool):
        return NUMBER
    if isinstance(value, str):
        return STRING
    if isinstance(value, (bytes, bytearray)):
        return BINARY
    if isinstance(value, list):
        if not value:
            return array_type(ANY)
        element_type = _python_value_to_type(value[0])
        for item in value[1:]:
            element_type = union_types(element_type, _python_value_to_type(item))
        return array_type(element_type)
    if isinstance(value, dict):
        fields: Dict[str, Tuple[DWType, bool, bool]] = {}
        for k, v in value.items():
            fields[str(k)] = (_python_value_to_type(v), False, False)
        return object_type(fields, is_open_flag=True)
    # Pydantic BaseModel (or similar) support: treat model fields as an object type
    try:
        from pydantic import BaseModel  # type: ignore
    except Exception:  # pragma: no cover
        BaseModel = None  # type: ignore
    if BaseModel is not None and isinstance(value, BaseModel):  # type: ignore
        data = value.model_dump()  # type: ignore[attr-defined]
        return _python_value_to_type(data)
    return ANY


def infer_script_pydantic_model(
    script_source: str,
    *,
    payload_type: DWType | Any = ANY,
    vars_type: DWType | Any = ANY,
    model_name: str = "DWOutputModel",
):
    """
    Infer the script type and return a dynamic Pydantic model that represents the output shape.
    """
    try:
        from pydantic import BaseModel, create_model  # type: ignore
    except Exception as exc:  # pragma: no cover - optional dependency
        raise RuntimeError("pydantic is required for infer_script_pydantic_model") from exc

    inferred = infer_script_type(script_source, payload_type=payload_type, vars_type=vars_type)

    def to_annotation(dw_type: DWType) -> Any:
        if isinstance(dw_type, AnyType) or isinstance(dw_type, NothingType):
            return Any
        if isinstance(dw_type, StringType):
            return str
        if isinstance(dw_type, NumberType):
            return Union[int, float]
        if isinstance(dw_type, BooleanType):
            return bool
        if isinstance(dw_type, NullType):
            return type(None)
        if isinstance(dw_type, BinaryType):
            return bytes
        if isinstance(dw_type, LiteralType):
            return type(dw_type.value)
        if isinstance(dw_type, ArrayType):
            return List[to_annotation(dw_type.element)]
        if isinstance(dw_type, UnionType):
            annos = [to_annotation(opt) for opt in dw_type.options]
            return Union[tuple(annos)]  # type: ignore
        if isinstance(dw_type, ObjectType):
            fields_def: Dict[str, Tuple[Any, Any]] = {}
            for name, f_type, is_optional, is_repeat in dw_type.fields:
                field_type = f_type
                # If nullable union, remove Null and mark optional
                nullable = False
                if isinstance(f_type, UnionType) and any(isinstance(o, NullType) for o in f_type.options):
                    non_null_opts = [o for o in f_type.options if not isinstance(o, NullType)]
                    field_type = union_types(*non_null_opts) if non_null_opts else NULL
                    nullable = True
                annotation = to_annotation(field_type)
                if is_repeat:
                    annotation = List[annotation]  # type: ignore
                if is_optional or nullable or isinstance(field_type, NullType):
                    annotation = Optional[annotation]
                    default = None
                else:
                    default = ...
                fields_def[name] = (annotation, default)
            # Config: allow extra fields if object is open
            extra_mode = "allow" if dw_type.is_open else "forbid"
            return create_model(
                f"{model_name}_{id(dw_type)}",
                __base__=BaseModel,
                __module__="dwpy.dynamic_models",
                __config__={"extra": extra_mode},
                **fields_def,
            )
        return Any

    root_annotation = to_annotation(inferred)
    if isinstance(inferred, ObjectType) and isinstance(root_annotation, type):
        return root_annotation
    # Wrap non-object roots into a model with a single field `value`
    from pydantic import create_model  # type: ignore

    return create_model(model_name, value=(root_annotation, ...))


class TypeInferencer:
    def __init__(self, payload_type: DWType, vars_type: DWType) -> None:
        self.payload_type = payload_type
        self.vars_type = vars_type
        self.context: Optional[TypeInferenceContext] = None
        self._functions: Dict[str, parser.FunctionDeclaration] = {}

    def infer_script(self, script: parser.Script) -> DWType:
        ctx = TypeInferenceContext(self.payload_type, self.vars_type)
        self.context = ctx

        # Process type definitions first
        for type_def in script.header.types:
            inferred_type = self._type_from_spec(type_def.type)
            ctx.define_type(type_def.name, inferred_type)

        for function_decl in script.header.functions:
            param_types: List[DWType] = []
            for param in function_decl.parameters:
                if param.type_annotation is not None:
                    param_types.append(self._type_from_spec(param.type_annotation))
                else:
                    param_types.append(ANY)

            inferred_return: DWType = ANY
            if function_decl.return_type is not None:
                inferred_return = self._type_from_spec(function_decl.return_type)
            else:
                # Infer from body with parameters bound
                fn_ctx = ctx.clone()
                for param_name, ptype in zip(function_decl.parameters, param_types):
                    fn_ctx.bind(param_name.name, ptype)
                inferred_return = self._infer_expression(function_decl.body, fn_ctx)

            ctx.bind(
                function_decl.name,
                FunctionType(
                    parameter_types=param_types,
                    return_type=inferred_return,
                ),
            )
            self._functions[function_decl.name] = function_decl
        for var_decl in script.header.variables:
            var_type = self._infer_expression(var_decl.expression, ctx)
            ctx.bind(var_decl.name, var_type)
        return self._infer_expression(script.body, ctx)

    def _infer_expression(self, expr: parser.Expression, ctx: TypeInferenceContext) -> DWType:
        if isinstance(expr, parser.StringLiteral):
            return STRING
        if isinstance(expr, parser.InterpolatedString):
            return STRING
        if isinstance(expr, parser.NumberLiteral):
            return NUMBER
        if isinstance(expr, parser.BooleanLiteral):
            return BOOLEAN
        if isinstance(expr, parser.NullLiteral):
            return NULL
        if isinstance(expr, parser.Placeholder):
            return ANY
        if isinstance(expr, parser.Identifier):
            return ctx.lookup(expr.name)
        if isinstance(expr, parser.ObjectLiteral):
            return self._infer_object(expr, ctx)
        if isinstance(expr, parser.ListLiteral):
            return self._infer_list(expr, ctx)
        if isinstance(expr, parser.PropertyAccess):
            base_type = self._infer_expression(expr.value, ctx)
            return self._infer_property(base_type, expr.attribute)
        if isinstance(expr, parser.IndexAccess):
            base_type = self._infer_expression(expr.value, ctx)
            if self._is_range_selector(expr.index):
                return self._infer_range_index(base_type)
            index_type = self._infer_expression(expr.index, ctx)
            return self._infer_index(base_type, index_type)
        if isinstance(expr, parser.FunctionCall):
            return self._infer_function_call(expr, ctx)
        if isinstance(expr, parser.DefaultOp):
            left_type = self._infer_expression(expr.left, ctx)
            right_type = self._infer_expression(expr.right, ctx)
            return union_types(left_type, right_type)
        if isinstance(expr, parser.IfExpression):
            true_type = self._infer_expression(expr.when_true, ctx)
            false_type = self._infer_expression(expr.when_false, ctx)
            return union_types(true_type, false_type)
        if isinstance(expr, parser.MatchExpression):
            return self._infer_match_expression(expr, ctx)
        if isinstance(expr, parser.LambdaExpression):
            return FunctionType(
                parameter_types=[
                    self._type_from_spec(p.type_annotation)
                    if p.type_annotation is not None else ANY
                    for p in expr.parameters
                ],
                return_type=ANY,
            )
        if isinstance(expr, parser.TypeCoercion):
            return self._type_from_spec(expr.target)
        return ANY

    def _infer_object(self, expr: parser.ObjectLiteral, ctx: TypeInferenceContext) -> DWType:
        fields: Dict[str, Tuple[DWType, bool, bool]] = {}
        open_object = False
        for key_expr, value_expr in expr.fields:
            key_constant = self._extract_constant_string(key_expr, ctx)
            value_type = self._infer_expression(value_expr, ctx)
            if key_constant is None:
                open_object = True
            else:
                if key_constant in fields:
                    existing_type, existing_opt, existing_rep = fields[key_constant]
                    fields[key_constant] = (
                        union_types(existing_type, value_type),
                        existing_opt,
                        existing_rep
                    )
                else:
                    fields[key_constant] = (value_type, False, False) # Inferred objects are simple for now
        return object_type(fields, is_open_flag=open_object or not fields)

    def _infer_list(self, expr: parser.ListLiteral, ctx: TypeInferenceContext) -> DWType:
        if not expr.elements:
            return array_type(ANY)
        element_type = self._infer_expression(expr.elements[0], ctx)
        for element in expr.elements[1:]:
            element_type = union_types(element_type, self._infer_expression(element, ctx))
        return array_type(element_type)

    def _infer_property(self, base_type: DWType, attribute: str) -> DWType:
        if isinstance(base_type, ArrayType):
            element_property_type = self._infer_property(base_type.element, attribute)
            return array_type(element_property_type)
        if isinstance(base_type, ObjectType):
            field_info = base_type.get(attribute)
            if field_info is not None:
                field_type, is_optional, is_repeatable = field_info
                result_type = field_type
                if is_optional:
                    result_type = union_types(result_type, NULL)
                if is_repeatable:
                    result_type = array_type(result_type)
                return result_type
            return ANY if base_type.open else NULL
        if isinstance(base_type, UnionType):
            inferred = [self._infer_property(option, attribute) for option in base_type.options]
            return union_types(*inferred)
        if isinstance(base_type, IntersectionType):
            inferred = [self._infer_property(option, attribute) for option in base_type.options]
            return intersection_types(*inferred)
        return ANY

    def _infer_index(self, base_type: DWType, index_type: DWType) -> DWType:
        if isinstance(base_type, ArrayType):
            return base_type.element
        if isinstance(base_type, UnionType):
            inferred = [self._infer_index(option, index_type) for option in base_type.options]
            return union_types(*inferred)
        if isinstance(base_type, IntersectionType):
            inferred = [self._infer_index(option, index_type) for option in base_type.options]
            return intersection_types(*inferred)
        if isinstance(base_type, ObjectType) and is_string(index_type):
            if not base_type.open and base_type.fields:
                all_field_types = [t for _, t, _, _ in base_type.fields]
                return union_types(*all_field_types)
            return ANY if base_type.open else NULL
        return ANY

    def _infer_range_index(self, base_type: DWType) -> DWType:
        if isinstance(base_type, ArrayType):
            return base_type
        if base_type == STRING:
            return STRING
        if isinstance(base_type, UnionType):
            inferred = [self._infer_range_index(option) for option in base_type.options]
            return union_types(*inferred)
        if isinstance(base_type, IntersectionType):
            inferred = [self._infer_range_index(option) for option in base_type.options]
            return intersection_types(*inferred)
        return ANY

    @staticmethod
    def _is_range_selector(index_expr: parser.Expression) -> bool:
        return (
            isinstance(index_expr, parser.FunctionCall)
            and isinstance(index_expr.function, parser.Identifier)
            and index_expr.function.name == "_infix_to"
            and len(index_expr.arguments) == 2
        )

    def _infer_function_call(self, expr: parser.FunctionCall, ctx: TypeInferenceContext) -> DWType:
        if isinstance(expr.function, parser.Identifier):
            name = expr.function.name
            if name == "_binary_plus" and len(expr.arguments) == 2:
                left = self._infer_expression(expr.arguments[0], ctx)
                right = self._infer_expression(expr.arguments[1], ctx)
                return self._infer_binary_plus(left, right)
            if name == "_binary_concat" and len(expr.arguments) == 2:
                left = self._infer_expression(expr.arguments[0], ctx)
                right = self._infer_expression(expr.arguments[1], ctx)
                return self._infer_binary_concat(left, right)
            if name == "_binary_minus" and len(expr.arguments) == 2:
                left = self._infer_expression(expr.arguments[0], ctx)
                right = self._infer_expression(expr.arguments[1], ctx)
                return self._infer_binary_minus(left, right, expr.arguments[1])
            if name == "_infix_to" and len(expr.arguments) == 2:
                start = self._infer_expression(expr.arguments[0], ctx)
                end = self._infer_expression(expr.arguments[1], ctx)
                if is_number(start) and is_number(end):
                    return array_type(NUMBER)
                return array_type(union_types(start, end))
            if name == "_infix_map" and len(expr.arguments) == 2:
                seq_type = self._infer_expression(expr.arguments[0], ctx)
                mapper = expr.arguments[1]
                elem_type = self._element_type(seq_type)
                mapped_type = self._infer_map_result(elem_type, mapper, ctx)
                return array_type(mapped_type)
            if name == "_infix_filter" and len(expr.arguments) == 2:
                seq_type = self._infer_expression(expr.arguments[0], ctx)
                # Filter keeps same element type
                elem_type = self._element_type(seq_type)
                return array_type(elem_type)
            if name in {"_infix_groupBy", "groupBy"} and len(expr.arguments) == 2:
                seq_type = self._infer_expression(expr.arguments[0], ctx)
                key_mapper = expr.arguments[1]
                elem_type = self._element_type(seq_type)
                self._infer_group_key(elem_type, key_mapper, ctx)
                grouped_array = array_type(elem_type)
                # groupBy returns Object<keyType, Array<elem>>
                # We model as open object with dynamic keys all having the same array type
                return object_type({"__grouped__": grouped_array}, is_open_flag=True)
            if name == "_binary_times" or name == "_binary_divide":
                return NUMBER
            if name in {"_binary_lt", "_binary_lte", "_binary_gt", "_binary_gte"} and len(expr.arguments) == 2:
                recovered = self._recover_filter_comparison(expr.arguments[0], expr.arguments[1], ctx)
                if recovered is not None:
                    return recovered
                return BOOLEAN
            if name in {"_binary_eq", "_binary_neq"}:
                return BOOLEAN
            bound = ctx.lookup(name)
            # Prefer re-inferencing user-defined functions with actual argument types
            if name in self._functions:
                fn_decl = self._functions[name]
                arg_types = [self._infer_expression(arg, ctx) for arg in expr.arguments]
                fn_ctx = ctx.clone()
                for param, arg_type in zip(fn_decl.parameters, arg_types):
                    fn_ctx.bind(param.name, arg_type)
                return self._infer_expression(fn_decl.body, fn_ctx)
            if isinstance(bound, FunctionType):
                return bound.return_type
        return ANY

    def _infer_binary_plus(self, left: DWType, right: DWType) -> DWType:
        if is_number(left) and is_number(right):
            return NUMBER
        if is_string(left) and is_string(right):
            return STRING
        if is_array(left) and is_array(right):
            if isinstance(left, ArrayType) and isinstance(right, ArrayType):
                return merge_array_types(left, right)
        return ANY

    def _infer_binary_concat(self, left: DWType, right: DWType) -> DWType:
        if is_string(left) and is_string(right):
            return STRING
        if is_array(left) and is_array(right):
            if isinstance(left, ArrayType) and isinstance(right, ArrayType):
                return merge_array_types(left, right)
        if isinstance(left, ObjectType) and isinstance(right, ObjectType):
            return merge_object_types(left, right)
        if isinstance(left, UnionType):
            merged = [self._infer_binary_concat(option, right) for option in left.options]
            return union_types(*merged)
        if isinstance(right, UnionType):
            merged = [self._infer_binary_concat(left, option) for option in right.options]
            return union_types(*merged)
        return ANY

    def _infer_binary_minus(self, left: DWType, right: DWType, right_expr: Optional[parser.Expression]) -> DWType:
        # Numeric subtraction
        if is_number(left) and is_number(right):
            return NUMBER
        if isinstance(left, UnionType):
            merged = [self._infer_binary_minus(option, right, right_expr) for option in left.options]
            return union_types(*merged)
        if isinstance(right, UnionType):
            merged = [self._infer_binary_minus(left, option, right_expr) for option in right.options]
            return union_types(*merged)
        if isinstance(left, ObjectType):
            # Attempt to remove a concrete key if provided as string literal
            key_name: Optional[str] = None
            if isinstance(right_expr, parser.StringLiteral):
                key_name = right_expr.value
            elif isinstance(right_expr, parser.InterpolatedString):
                key_name = self._extract_constant_string(right_expr, None)

            if key_name is not None:
                new_fields = left.field_dict().copy()
                new_fields.pop(key_name, None)
                return object_type(new_fields, is_open_flag=left.is_open)
            return object_type(left.field_dict(), is_open_flag=True)
        return ANY

    def _recover_filter_comparison(
        self,
        left_expr: parser.Expression,
        right_expr: parser.Expression,
        ctx: TypeInferenceContext,
    ) -> Optional[DWType]:
        """
        Recover type for parser shape: `_binary_lt(_infix_filter(seq, $), rhs)`.

        When this shape appears, it corresponds to `seq filter $ < rhs`, where the
        comparison should be part of the filter predicate and the result type should
        remain `Array<elem>`.
        """
        if not isinstance(left_expr, parser.FunctionCall):
            return None
        if not isinstance(left_expr.function, parser.Identifier):
            return None
        if left_expr.function.name != "_infix_filter":
            return None
        if len(left_expr.arguments) != 2:
            return None
        predicate = left_expr.arguments[1]
        if not isinstance(predicate, parser.Placeholder):
            return None
        seq_type = self._infer_expression(left_expr.arguments[0], ctx)
        return array_type(self._element_type(seq_type))

    def _element_type(self, seq_type: DWType) -> DWType:
        if isinstance(seq_type, ArrayType):
            return seq_type.element
        if isinstance(seq_type, UnionType):
            elements = [self._element_type(opt) for opt in seq_type.options]
            return union_types(*elements)
        return ANY

    def _infer_map_result(self, element_type: DWType, mapper: parser.Expression, ctx: TypeInferenceContext) -> DWType:
        # Placeholder returns the element as-is
        if isinstance(mapper, parser.Placeholder):
            return element_type
        if isinstance(mapper, parser.LambdaExpression):
            fn_ctx = ctx.clone()
            if mapper.parameters:
                fn_ctx.bind(mapper.parameters[0].name, element_type)
                if len(mapper.parameters) > 1:
                    fn_ctx.bind(mapper.parameters[1].name, NUMBER)
            return self._infer_expression(mapper.body, fn_ctx)
        if isinstance(mapper, parser.TypeCoercion):
            # Direct coercion of the element
            coerced = self._type_from_spec(mapper.target)
            return coerced
        return ANY

    def _infer_group_key(self, element_type: DWType, mapper: parser.Expression, ctx: TypeInferenceContext) -> DWType:
        if isinstance(mapper, parser.Placeholder):
            return element_type
        if isinstance(mapper, parser.LambdaExpression):
            fn_ctx = ctx.clone()
            if mapper.parameters:
                fn_ctx.bind(mapper.parameters[0].name, element_type)
                if len(mapper.parameters) > 1:
                    fn_ctx.bind(mapper.parameters[1].name, NUMBER)
            return self._infer_expression(mapper.body, fn_ctx)
        if isinstance(mapper, parser.TypeCoercion):
            return self._type_from_spec(mapper.target)
        return ANY

    def _infer_match_expression(self, expr: parser.MatchExpression, ctx: TypeInferenceContext) -> DWType:
        result_type = NULL
        for case in expr.cases:
            result_type = union_types(result_type, self._infer_expression(case.expression, ctx))
        return result_type

    def _extract_constant_string(self, expr: parser.Expression, ctx: TypeInferenceContext) -> Optional[str]:
        if isinstance(expr, parser.StringLiteral):
            return expr.value
        if isinstance(expr, parser.Identifier):
            return expr.name
        if isinstance(expr, parser.InterpolatedString):
            parts = []
            for part in expr.parts:
                if isinstance(part, parser.StringLiteral):
                    parts.append(part.value)
                else:
                    return None
            return "".join(parts)
        return None

    def _type_from_spec(self, spec: ParserTypeSpec) -> DWType:
        if isinstance(spec, ParserLiteralTypeSpec):
            base_type = self._type_from_spec(spec.base_type) # Recursively infer base type
            return LiteralType(value=spec.value, base_type=base_type)
        
        if isinstance(spec, ParserUnionTypeSpec):
            options = [self._type_from_spec(opt) for opt in spec.options]
            return union_types(*options)
        
        if isinstance(spec, ParserIntersectionTypeSpec):
            options = [self._type_from_spec(opt) for opt in spec.options]
            return intersection_types(*options)

        if isinstance(spec, ParserFunctionTypeSpec):
            param_types = [self._type_from_spec(p) for p in spec.parameters]
            return_type = self._type_from_spec(spec.return_type)
            return FunctionType(parameter_types=param_types, return_type=return_type)

        if isinstance(spec, ObjectTypeSpec):
            fields: List[Tuple[str, DWType, bool, bool]] = []
            for name, field_type_spec, is_optional, is_repeatable in spec.fields:
                field_dw_type = self._type_from_spec(field_type_spec)
                # Optional fields can be absent, effectively allowing Null
                if is_optional:
                    field_dw_type = union_types(field_dw_type, NULL)
                fields.append((name, field_dw_type, is_optional, is_repeatable))
            return object_type(
                {name: (t, opt, rep) for name, t, opt, rep in fields},
                is_open_flag=spec.is_open
            )

        if isinstance(spec, ReferenceTypeSpec):
            raw_name = spec.name
            name = raw_name.lower()

            # Check for defined types in the context (case-insensitive)
            if self.context:
                resolved = self.context.resolve_type(raw_name)
                if resolved is not None:
                    return resolved

            if name == "string":
                return STRING
            if name in {"number", "decimal", "integer"}:
                return NUMBER
            if name in {"boolean", "bool"}:
                return BOOLEAN
            if name == "binary":
                return BINARY
            if name == "null":
                return NULL
            if name == "any":
                return ANY
            if name == "nothing":
                return NOTHING
            if name == "array":
                if spec.generics:
                    element = self._type_from_spec(spec.generics[0])
                    return array_type(element)
                return array_type(ANY)
            if name == "object":
                # If it's a generic Object reference without explicit fields
                return object_type({}, is_open_flag=True)
            
            # Handle Temporal types as String for now, if not specifically supported
            if name in {"date", "datetime", "localdatetime", "localtime", "time", "period"}:
                return STRING
            
            # For unknown reference types, default to ANY
            return ANY
        
        # Fallback for any unhandled TypeSpec type
        return ANY
