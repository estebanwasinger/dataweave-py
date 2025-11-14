from __future__ import annotations

from dataclasses import dataclass
from typing import Dict, Optional

from . import parser
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
    ANY,
    STRING,
    NUMBER,
    BOOLEAN,
    NULL,
    BINARY,
    array_type,
    object_type,
    union_types,
    merge_array_types,
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

    def lookup(self, name: str) -> DWType:
        if name == "payload":
            return self.payload_type
        if name == "vars":
            return self.vars_type
        return self.env.get(name, ANY)

    def bind(self, name: str, type_: DWType) -> None:
        self.env[name] = type_


def infer_script_type(
    script_source: str,
    *,
    payload_type: DWType = ANY,
    vars_type: DWType = ANY,
) -> DWType:
    script = parser.parse_script(script_source)
    inferencer = TypeInferencer(payload_type=payload_type, vars_type=vars_type)
    return inferencer.infer_script(script)


class TypeInferencer:
    def __init__(self, payload_type: DWType, vars_type: DWType) -> None:
        self.payload_type = payload_type
        self.vars_type = vars_type

    def infer_script(self, script: parser.Script) -> DWType:
        ctx = TypeInferenceContext(self.payload_type, self.vars_type)
        for function_decl in script.header.functions:
            if function_decl.return_type is not None:
                return_type = self._type_from_spec(function_decl.return_type)
                ctx.bind(
                    function_decl.name,
                    FunctionType(
                        parameter_count=len(function_decl.parameters),
                        return_type=return_type,
                    ),
                )
            else:
                ctx.bind(
                    function_decl.name,
                    FunctionType(parameter_count=len(function_decl.parameters), return_type=ANY),
                )
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
                parameter_count=len(expr.parameters),
                return_type=ANY,
            )
        if isinstance(expr, parser.TypeCoercion):
            return self._type_from_spec(expr.target)
        return ANY

    def _infer_object(self, expr: parser.ObjectLiteral, ctx: TypeInferenceContext) -> DWType:
        fields: Dict[str, DWType] = {}
        open_object = False
        for key_expr, value_expr in expr.fields:
            key_constant = self._extract_constant_string(key_expr, ctx)
            value_type = self._infer_expression(value_expr, ctx)
            if key_constant is None:
                open_object = True
            else:
                if key_constant in fields:
                    fields[key_constant] = union_types(fields[key_constant], value_type)
                else:
                    fields[key_constant] = value_type
        return object_type(fields, open=open_object or not fields)

    def _infer_list(self, expr: parser.ListLiteral, ctx: TypeInferenceContext) -> DWType:
        if not expr.elements:
            return array_type(ANY)
        element_type = self._infer_expression(expr.elements[0], ctx)
        for element in expr.elements[1:]:
            element_type = union_types(element_type, self._infer_expression(element, ctx))
        return array_type(element_type)

    def _infer_property(self, base_type: DWType, attribute: str) -> DWType:
        if isinstance(base_type, ObjectType):
            field_type = base_type.get(attribute)
            if field_type is not None:
                return field_type
            return ANY if base_type.open else NULL
        if isinstance(base_type, UnionType):
            inferred = [self._infer_property(option, attribute) for option in base_type.options]
            return union_types(*inferred)
        return ANY

    def _infer_index(self, base_type: DWType, index_type: DWType) -> DWType:
        if isinstance(base_type, ArrayType):
            return base_type.element
        if isinstance(base_type, UnionType):
            inferred = [self._infer_index(option, index_type) for option in base_type.options]
            return union_types(*inferred)
        if isinstance(base_type, ObjectType) and is_string(index_type):
            return ANY if base_type.open else NULL
        return ANY

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
            if name == "_binary_times" or name == "_binary_divide":
                return NUMBER
            if name == "_binary_eq":
                return BOOLEAN
            bound = ctx.lookup(name)
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

    def _type_from_spec(self, spec: parser.TypeSpec) -> DWType:
        name = (spec.name or "").lower()
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
        if name == "array" and spec.generics:
            element = self._type_from_spec(spec.generics[0])
            return array_type(element)
        if name == "object":
            return object_type({}, open=True)
        return ANY
