from __future__ import annotations

import logging
import re
import inspect
import copy
import os
import json
import math
import uuid as uuid_lib
import hashlib
import hmac
from datetime import date, datetime, time, timedelta, timezone
from zoneinfo import ZoneInfo
from decimal import Decimal, InvalidOperation, localcontext
from dataclasses import dataclass
from importlib import metadata as importlib_metadata
from pathlib import Path
from urllib.parse import urlparse
from urllib.request import urlopen
from typing import Any, Callable, Dict, List, Optional, Mapping, Set, Tuple

from . import builtins, parser
from .formats import DWObject, FormatRegistry, FormatError, XMLNodeList, XMLNodeDict

try:  # pragma: no cover - optional dependency guard
    import pandas as pd  # type: ignore
except Exception:  # pragma: no cover
    pd = None
PANDAS_AVAILABLE = pd is not None

Missing = object()
MODULE_BASE_PATH = Path(__file__).resolve().parent / "modules"
LOGGER = logging.getLogger(__name__)


class DataWeaveEvaluationError(RuntimeError):
    def __init__(
        self,
        message: str,
        line: Optional[int] = None,
        column: Optional[int] = None,
        length: int = 1,
        original: Optional[BaseException] = None,
    ) -> None:
        super().__init__(message)
        self.line = line
        self.column = column
        self.length = max(length, 1)
        self.original = original


@dataclass
class EvaluationContext:
    payload: Any
    variables: Dict[str, Any]
    header: Optional[parser.Header] = None
    line_offset: int = 0


@dataclass
class OutputDirective:
    mime_type: str
    format_id: str
    properties: Dict[str, Any]


@dataclass
class LambdaCallable:
    runtime: "DataWeaveRuntime"
    parameters: List[parser.Parameter]
    body: parser.Expression
    closure_variables: Dict[str, Any]
    payload: Any
    header: Optional[parser.Header]

    def __call__(self, *args: Any) -> Any:
        local_vars: Dict[str, Any] = dict(self.closure_variables)
        provided_args = list(args)
        if len(provided_args) > len(self.parameters):
            raise TypeError("Too many arguments supplied to lambda expression")
        for index, parameter in enumerate(self.parameters):
            if index < len(provided_args):
                local_vars[parameter.name] = provided_args[index]
            else:
                if parameter.default is not None:
                    default_ctx = EvaluationContext(
                        payload=self.payload,
                        variables=dict(local_vars),
                        header=self.header,
                    )
                    local_vars[parameter.name] = self.runtime._evaluate(
                        parameter.default, default_ctx
                    )
                else:
                    raise TypeError(f"Missing argument '{parameter.name}' for lambda")
        if self.parameters:
            first_param = self.parameters[0].name
            if first_param in local_vars:
                local_vars.setdefault("$", local_vars[first_param])
        if len(self.parameters) > 1:
            second_param = self.parameters[1].name
            if second_param in local_vars:
                local_vars.setdefault("$$", local_vars[second_param])
        body_ctx = EvaluationContext(
            payload=self.payload,
            variables=local_vars,
            header=self.header,
        )
        return self.runtime._evaluate(self.body, body_ctx)


@dataclass
class DefinedFunction:
    runtime: "DataWeaveRuntime"
    parameters: List[parser.Parameter]
    body: parser.Expression
    context: EvaluationContext
    return_type: Optional[parser.TypeSpec]

    def __call__(self, *args: Any) -> Any:
        local_vars: Dict[str, Any] = dict(self.context.variables)
        provided_args = list(args)
        if len(provided_args) > len(self.parameters):
            raise TypeError("Too many arguments supplied to function")
        for index, parameter in enumerate(self.parameters):
            if index < len(provided_args):
                local_vars[parameter.name] = provided_args[index]
            else:
                if parameter.default is not None:
                    default_ctx = EvaluationContext(
                        payload=self.context.payload,
                        variables=dict(local_vars),
                        header=self.context.header,
                    )
                    local_vars[parameter.name] = self.runtime._evaluate(
                        parameter.default, default_ctx
                    )
                else:
                    raise TypeError(f"Missing argument '{parameter.name}' for function")
        body_ctx = EvaluationContext(
            payload=self.context.payload,
            variables=local_vars,
            header=self.context.header,
        )
        result = self.runtime._evaluate(self.body, body_ctx)
        if self.return_type is not None:
            result = self.runtime._coerce_value(result, self.return_type, None, body_ctx)
        return result


@dataclass
class OverloadedFunction:
    runtime: "DataWeaveRuntime"
    functions: List[DefinedFunction]

    def add(self, function: DefinedFunction) -> None:
        self.functions.append(function)

    def __call__(self, *args: Any) -> Any:
        for function in self.functions:
            if self._matches(function, args):
                return function(*args)
        return self.functions[-1](*args)

    def _matches(self, function: DefinedFunction, args: Tuple[Any, ...]) -> bool:
        expected_params = function.parameters
        required_count = sum(1 for param in expected_params if param.default is None)
        if len(args) < required_count or len(args) > len(expected_params):
            return False
        for index, param in enumerate(expected_params):
            if index >= len(args):
                break
            if not self.runtime._matches_type_annotation(args[index], param.type_annotation):
                return False
        return True


@dataclass
class ImplicitLambdaCallable:
    runtime: "DataWeaveRuntime"
    body: parser.Expression
    closure_variables: Dict[str, Any]
    payload: Any
    header: Optional[parser.Header]
    placeholders: Set[int]

    def __post_init__(self) -> None:
        if 2 in self.placeholders:
            self.parameters = [
                parser.Parameter(name="$"),
                parser.Parameter(name="$$"),
            ]
        else:
            self.parameters = [parser.Parameter(name="$")]

    def __call__(self, *args: Any) -> Any:
        local_vars: Dict[str, Any] = dict(self.closure_variables)
        value = args[0] if args else None
        index = args[1] if len(args) > 1 else None
        local_vars["$"] = value
        local_vars["$$"] = index
        body_ctx = EvaluationContext(
            payload=self.payload,
            variables=local_vars,
            header=self.header,
        )
        return self.runtime._evaluate(self.body, body_ctx)


class DataWeaveRuntime:
    _MD2_S_TABLE: Tuple[int, ...] = (
        41, 46, 67, 201, 162, 216, 124, 1, 61, 54, 84, 161, 236, 240, 6, 19,
        98, 167, 5, 243, 192, 199, 115, 140, 152, 147, 43, 217, 188, 76, 130, 202,
        30, 155, 87, 60, 253, 212, 224, 22, 103, 66, 111, 24, 138, 23, 229, 18,
        190, 78, 196, 214, 218, 158, 222, 73, 160, 251, 245, 142, 187, 47, 238, 122,
        169, 104, 121, 145, 21, 178, 7, 63, 148, 194, 16, 137, 11, 34, 95, 33,
        128, 127, 93, 154, 90, 144, 50, 39, 53, 62, 204, 231, 191, 247, 151, 3,
        255, 25, 48, 179, 72, 165, 181, 209, 215, 94, 146, 42, 172, 86, 170, 198,
        79, 184, 56, 210, 150, 164, 125, 182, 118, 252, 107, 226, 156, 116, 4, 241,
        69, 157, 112, 89, 100, 113, 135, 32, 134, 91, 207, 101, 230, 45, 168, 2,
        27, 96, 37, 173, 174, 176, 185, 246, 28, 70, 97, 105, 52, 64, 126, 15,
        85, 71, 163, 35, 221, 81, 175, 58, 195, 92, 249, 206, 186, 197, 234, 38,
        44, 83, 13, 110, 133, 40, 132, 9, 211, 223, 205, 244, 65, 129, 77, 82,
        106, 220, 55, 200, 108, 193, 171, 250, 36, 225, 123, 8, 12, 189, 177, 74,
        120, 136, 149, 139, 227, 99, 232, 109, 233, 203, 213, 254, 59, 0, 29, 57,
        242, 239, 183, 14, 102, 88, 208, 228, 166, 119, 114, 248, 235, 117, 75, 10,
        49, 68, 80, 180, 143, 237, 31, 26, 219, 153, 141, 51, 159, 17, 131, 20,
    )

    _IMPLICIT_LAMBDA_ARGUMENTS: Dict[str, Tuple[int, ...]] = {
        "_infix_map": (1,),
        "_infix_filter": (1,),
        "_infix_flatMap": (1,),
        "_infix_reduce": (1,),
        "_infix_distinctBy": (1,),
        "groupBy": (1,),
        "orderBy": (1,),
        "pluck": (1,),
        "maxBy": (1,),
        "minBy": (1,),
        "map": (1,),
        "filter": (1,),
        "flatMap": (1,),
        "reduce": (1,),
        "distinctBy": (1,),
    }

    def __init__(self, *, enable_module_imports: bool = True) -> None:
        self._enable_module_imports = enable_module_imports
        self._builtins: Dict[str, Callable[..., Any]] = dict(builtins.CORE_FUNCTIONS)
        self._builtins.update(
            {
                "_binary_plus": self._func_binary_plus,
                "_binary_minus": self._func_binary_minus,
                "_binary_times": self._func_binary_times,
                "_binary_divide": self._func_binary_divide,
                "_infix_map": self._func_infix_map,
                "_infix_reduce": self._func_infix_reduce,
                "_infix_filter": self._func_infix_filter,
                "_infix_flatMap": self._func_infix_flat_map,
                "_infix_distinctBy": self._func_infix_distinct_by,
                "_infix_to": self._func_infix_to,
                "_binary_eq": self._func_binary_eq,
                "_binary_neq": self._func_binary_neq,
                "_binary_gt": self._func_binary_gt,
                "_binary_lt": self._func_binary_lt,
                "_binary_gte": self._func_binary_gte,
                "_binary_lte": self._func_binary_lte,
                "_binary_and": self._func_binary_and,
                "_binary_or": self._func_binary_or,
                "_unary_not": self._func_unary_not,
                "native": self._func_native,
            }
        )
        self._native_functions = self._build_native_function_registry()

    def execute(
        self,
        script_source: str,
        payload: Any,
        vars: Optional[Dict[str, Any]] = None,
        *,
        payload_format: Optional[str] = None,
        payload_format_options: Optional[Dict[str, Any]] = None,
        render_output: bool = True,
    ) -> Any:
        payload = self._convert_input_format(payload, payload_format, payload_format_options)
        payload = self._normalise_input_value(payload)
        provided_vars = vars or {}
        variables = {
            name: self._normalise_input_value(value) for name, value in provided_vars.items()
        }
        try:
            script = parser.parse_script(script_source)
        except parser.ParseError as err:
            formatted = self._format_error_message(
                script_source,
                str(err),
                err.line,
                err.column,
            )
            raise parser.ParseError(formatted, err.line, err.column) from err
        header_context = EvaluationContext(
            payload=payload,
            variables=variables,
            header=script.header,
            line_offset=0,
        )
        self._populate_context_from_header(script.header, header_context)
        body_line_offset = self._compute_body_line_offset(script_source)
        body_context = EvaluationContext(
            payload=payload,
            variables=header_context.variables,
            header=script.header,
            line_offset=body_line_offset,
        )
        try:
            result = self._evaluate(script.body, body_context)
        except DataWeaveEvaluationError as err:
            formatted = self._format_error_message(
                script_source,
                str(err),
                err.line,
                err.column,
                err.length,
            )
            raise DataWeaveEvaluationError(
                formatted,
                err.line,
                err.column,
                err.length,
                err.original or err,
            ) from (err.original or err)
        if not render_output:
            return self._collapse_xml_nodes(result)
        directive = self._parse_output_directive(script.header.output)
        if directive is None or directive.format_id == "python":
            return self._collapse_xml_nodes(result)
        return self._render_output(result, directive)

    def _normalise_input_value(self, value: Any) -> Any:
        if PANDAS_AVAILABLE:
            if isinstance(value, pd.DataFrame):
                records = value.to_dict(orient="records")
                return [self._normalise_input_value(record) for record in records]
            if isinstance(value, pd.Series):
                series_data = value.to_dict()
                return self._normalise_input_value(series_data)
        if isinstance(value, DWObject):
            node = DWObject()
            for key, val in value.items():
                node.add(key, self._normalise_input_value(val))
            return node
        if isinstance(value, XMLNodeList):
            normalised_list = XMLNodeList()
            for item in value:
                normalised_list.append(self._normalise_input_value(item))
            return normalised_list
        if isinstance(value, XMLNodeDict):
            node = XMLNodeDict()
            for key, val in value.items():
                node[key] = self._normalise_input_value(val)
            return node
        if isinstance(value, Mapping):
            return {key: self._normalise_input_value(val) for key, val in value.items()}
        if isinstance(value, list):
            return [self._normalise_input_value(item) for item in value]
        if isinstance(value, tuple):
            return [self._normalise_input_value(item) for item in value]
        return value

    def _convert_input_format(
        self,
        value: Any,
        format_name: Optional[str],
        options: Optional[Dict[str, Any]],
    ) -> Any:
        if format_name is None:
            return value
        try:
            return FormatRegistry.read(value, format_name, options or {})
        except FormatError as err:
            raise DataWeaveEvaluationError(str(err)) from err

    def _render_output(
        self,
        value: Any,
        directive: Optional[OutputDirective],
    ) -> Any:
        if directive is None or directive.format_id == "python":
            return value
        try:
            return FormatRegistry.write(value, directive.format_id, directive.properties)
        except FormatError as err:
            raise DataWeaveEvaluationError(str(err)) from err

    def _collapse_xml_nodes(self, value: Any) -> Any:
        if isinstance(value, XMLNodeList):
            return [self._collapse_xml_nodes(item) for item in value]
        if isinstance(value, DWObject):
            collapsed_object = DWObject()
            for key, val in value.items():
                collapsed_object.add(key, self._collapse_xml_nodes(val))
            return collapsed_object
        if isinstance(value, XMLNodeDict):
            collapsed: Dict[str, Any] = {}
            for key, val in value.items():
                collapsed[key] = self._collapse_xml_nodes(val)
            text_value = collapsed.get("#text")
            if text_value is not None:
                return text_value
            return collapsed
        if isinstance(value, list):
            return [self._collapse_xml_nodes(item) for item in value]
        if isinstance(value, Mapping):
            collapsed = {key: self._collapse_xml_nodes(val) for key, val in value.items()}
            text_value = collapsed.get("#text")
            if text_value is not None:
                non_meta_keys = [
                    key for key in collapsed.keys() if key != "#text" and not key.startswith("@")
                ]
                if not non_meta_keys:
                    return text_value
            return collapsed
        return value

    def _parse_output_directive(self, directive: Optional[str]) -> Optional[OutputDirective]:
        if not directive:
            return None
        import shlex

        try:
            tokens = shlex.split(directive)
        except ValueError as err:
            raise DataWeaveEvaluationError(f"Invalid output directive: {directive}") from err
        if not tokens:
            return None
        idx = 0
        first = tokens[idx]
        idx += 1
        mime_value: Optional[str] = None
        format_token: Optional[str] = None
        if idx < len(tokens) and tokens[idx].lower() == "with":
            mime_value = first
            idx += 1
            if idx >= len(tokens):
                raise DataWeaveEvaluationError("Missing writer format after 'with' in output directive")
            format_token = tokens[idx]
            idx += 1
        else:
            if "/" in first:
                mime_value = first
            else:
                format_token = first
        if format_token is None:
            format_token = mime_value
        if format_token is None:
            raise DataWeaveEvaluationError("Unable to determine writer format for output directive")
        format_def = FormatRegistry.get(format_token)
        if format_def is None:
            raise DataWeaveEvaluationError(f"Unsupported output format '{format_token}'")
        if mime_value is None:
            mime_value = format_def.mime_type
        properties = self._parse_directive_properties(tokens[idx:])
        return OutputDirective(mime_type=mime_value, format_id=format_def.id, properties=properties)

    def _parse_directive_properties(self, tokens: List[str]) -> Dict[str, Any]:
        properties: Dict[str, Any] = {}
        for token in tokens:
            if "=" not in token:
                properties[token] = True
                continue
            key, raw_value = token.split("=", 1)
            properties[key] = self._coerce_property_value(raw_value)
        return properties

    @staticmethod
    def _coerce_property_value(value: str) -> Any:
        if not value:
            return ""
        unescaped = bytes(value, "utf-8").decode("unicode_escape")
        lowered = unescaped.lower()
        if lowered == "true":
            return True
        if lowered == "false":
            return False
        try:
            if "." in unescaped:
                return float(unescaped)
            return int(unescaped)
        except ValueError:
            return unescaped

    def _matches_type_annotation(self, value: Any, type_spec: Optional[parser.TypeSpec]) -> bool:
        if type_spec is None:
            return True
        type_name = (type_spec.name or "").lower()
        if type_name in ("any", "anytype"):
            return True
        if type_name == "null":
            return value is None
        if type_name == "object":
            return isinstance(value, Mapping)
        if type_name == "array":
            return isinstance(value, list)
        if type_name == "string":
            return isinstance(value, str)
        if type_name == "number":
            return isinstance(value, (int, float))
        if type_name == "boolean":
            return isinstance(value, bool)
        return True

    def _evaluate(self, expr: parser.Expression, ctx: EvaluationContext) -> Any:
        if isinstance(expr, parser.ObjectLiteral):
            result_obj = DWObject()
            for key_expr, value_expr in expr.fields:
                if key_expr is None:
                    merged_value = self._evaluate(value_expr, ctx)
                    if merged_value is None:
                        continue
                    if not isinstance(merged_value, Mapping):
                        raise TypeError("Object expression entries must evaluate to an object")
                    for merged_key, merged_item in merged_value.items():
                        result_obj.add(str(merged_key), merged_item)
                    continue
                key_value = self._evaluate(key_expr, ctx)
                if isinstance(key_value, str):
                    key_str = key_value
                else:
                    key_str = self._to_string(key_value)
                result_obj.add(key_str, self._evaluate(value_expr, ctx))
            return result_obj
        if isinstance(expr, parser.ListLiteral):
            return [self._evaluate(item, ctx) for item in expr.elements]
        if isinstance(expr, parser.StringLiteral):
            return self._evaluate_string_literal(expr.value, ctx)
        if isinstance(expr, parser.TemporalLiteral):
            return self._parse_temporal_literal(expr.value)
        if isinstance(expr, parser.Placeholder):
            placeholder_name = "$" if expr.level == 1 else "$$"
            if placeholder_name in ctx.variables:
                return ctx.variables[placeholder_name]
            raise DataWeaveEvaluationError(
                f"Placeholder '{placeholder_name}' is not defined in this context",
                line=expr.line or None,
                column=expr.column or None,
            )
        if isinstance(expr, parser.InterpolatedString):
            result_parts = []
            for part in expr.parts:
                value = self._evaluate(part, ctx)
                result_parts.append(self._to_string(value))
            return "".join(result_parts)
        if isinstance(expr, parser.NumberLiteral):
            # Prefer int when possible for friendlier outputs.
            return int(expr.value) if expr.value.is_integer() else expr.value
        if isinstance(expr, parser.BooleanLiteral):
            return expr.value
        if isinstance(expr, parser.NullLiteral):
            return None
        if isinstance(expr, parser.Identifier):
            return self._resolve_identifier(
                expr.name,
                ctx,
                line=expr.line,
                column=expr.column,
                length=len(expr.name or ""),
            )
        if isinstance(expr, parser.PropertyAccess):
            base = self._evaluate(expr.value, ctx)
            try:
                if expr.recursive:
                    if expr.key_value:
                        return self._resolve_descendant_key_value_pairs(base, expr.attribute or "")
                    return self._resolve_descendant_property(
                        base,
                        expr.attribute,
                        multi_value=expr.multi_value,
                    )
                if expr.key_value:
                    return self._resolve_key_value_pairs(base, expr.attribute or "")
                return self._resolve_property(
                    base,
                    expr.attribute or "",
                    multi_value=expr.multi_value,
                )
            except TypeError:
                if expr.null_safe:
                    return None
                raise
        if isinstance(expr, parser.IndexAccess):
            base = self._evaluate(expr.value, ctx)
            if (
                isinstance(expr.index, parser.FunctionCall)
                and isinstance(expr.index.function, parser.Identifier)
                and expr.index.function.name == "_infix_to"
                and len(expr.index.arguments) == 2
            ):
                start_index = self._evaluate(expr.index.arguments[0], ctx)
                end_index = self._evaluate(expr.index.arguments[1], ctx)
                return self._resolve_range_index(base, start_index, end_index)
            index = self._evaluate(expr.index, ctx)
            return self._resolve_index(base, index)
        if isinstance(expr, parser.DynamicSelector):
            base = self._evaluate(expr.value, ctx)
            selector = self._evaluate(expr.selector, ctx)
            return self._resolve_dynamic_selector(base, selector, expr.mode)
        if isinstance(expr, parser.FilterSelector):
            base = self._evaluate(expr.value, ctx)
            return self._apply_selector_filter(base, expr.predicate, ctx)
        if isinstance(expr, parser.SelectorModifier):
            if expr.mode == "present":
                return self._selector_present(expr.value, ctx)
            if expr.mode == "assert":
                return self._assert_selector_present(expr.value, ctx)
            raise TypeError(f"Unsupported selector modifier: {expr.mode}")
        if isinstance(expr, parser.FunctionCall):
            function = self._evaluate(expr.function, ctx)
            placeholder_positions = self._resolve_placeholder_argument_indexes(expr.function)
            args: List[Any] = []
            for idx, argument in enumerate(expr.arguments):
                if idx in placeholder_positions and not isinstance(argument, parser.LambdaExpression):
                    placeholders = self._collect_placeholders(argument)
                    if placeholders:
                        args.append(
                            ImplicitLambdaCallable(
                                runtime=self,
                                body=argument,
                                closure_variables=dict(ctx.variables),
                                payload=ctx.payload,
                                header=ctx.header,
                                placeholders=placeholders,
                            )
                        )
                        continue
                args.append(self._evaluate(argument, ctx))
            if not callable(function):
                raise TypeError(f"Expression {expr.function!r} is not callable")
            return function(*args)
        if isinstance(expr, parser.DefaultOp):
            left_value = self._evaluate(expr.left, ctx)
            if self._is_missing(left_value):
                return self._evaluate(expr.right, ctx)
            return left_value
        if isinstance(expr, parser.LambdaExpression):
            return LambdaCallable(
                runtime=self,
                parameters=expr.parameters,
                body=expr.body,
                closure_variables=dict(ctx.variables),
                payload=ctx.payload,
                header=ctx.header,
            )
        if isinstance(expr, parser.IfExpression):
            condition_value = self._evaluate(expr.condition, ctx)
            branch = expr.when_true if self._is_truthy(condition_value) else expr.when_false
            return self._evaluate(branch, ctx)
        if isinstance(expr, parser.DoExpression):
            scoped_variables = dict(ctx.variables)
            do_context = EvaluationContext(
                payload=ctx.payload,
                variables=scoped_variables,
                header=expr.header,
                line_offset=ctx.line_offset,
            )
            self._populate_context_from_header(expr.header, do_context)
            return self._evaluate(expr.body, do_context)
        if isinstance(expr, parser.MatchExpression):
            value = self._evaluate(expr.value, ctx)
            for case in expr.cases:
                if case.pattern is None:
                    return self._evaluate(case.expression, ctx)
                pattern = case.pattern
                match_context = ctx
                if pattern.binding:
                    bound_variables = dict(ctx.variables)
                    bound_variables[pattern.binding] = value
                    match_context = EvaluationContext(
                        payload=ctx.payload,
                        variables=bound_variables,
                        header=ctx.header,
                    )
                matches = True
                if pattern.matcher is not None:
                    expected = self._evaluate(pattern.matcher, ctx)
                    matches = self._match_values(value, expected)
                if matches and pattern.guard is not None:
                    guard_value = self._evaluate(pattern.guard, match_context)
                    matches = self._is_truthy(guard_value)
                if matches:
                    return self._evaluate(case.expression, match_context)
            return None
        if isinstance(expr, parser.TypeCoercion):
            value = self._evaluate(expr.expression, ctx)
            options = self._evaluate(expr.options, ctx) if expr.options else None
            return self._coerce_value(value, expr.target, options, ctx)
        raise TypeError(f"Unsupported expression: {expr!r}")

    def _resolve_identifier(
        self,
        name: str,
        ctx: EvaluationContext,
        *,
        line: Optional[int] = None,
        column: Optional[int] = None,
        length: int = 1,
    ) -> Any:
        if name == "payload":
            return ctx.payload
        if name == "vars":
            return ctx.variables
        if name in self._builtins:
            builtin = self._builtins[name]
            if name == "_binary_plus" and line is not None:
                offset = ctx.line_offset if ctx else 0

                def plus_wrapper(left: Any, right: Any, _builtin=builtin) -> Any:
                    actual_line = line + offset if line is not None else None
                    return _builtin(
                        left,
                        right,
                        line=actual_line,
                        column=column,
                    )

                return plus_wrapper
            return builtin
        if name in ctx.variables:
            return ctx.variables[name]
        actual_line = line + ctx.line_offset if line is not None else None
        raise DataWeaveEvaluationError(
            f"Unable to resolve reference of `{name}`.",
            line=actual_line,
            column=column,
            length=max(length, 1),
        )

    def _resolve_property(self, base: Any, attribute: str, *, multi_value: bool = False) -> Any:
        if base is None:
            return None
        if isinstance(base, list):
            collected: List[Any] = []
            for item in base:
                value = self._resolve_property(item, attribute, multi_value=multi_value)
                if value is None:
                    continue
                if multi_value and isinstance(value, list):
                    collected.extend(value)
                else:
                    collected.append(value)
            if not collected:
                return None
            return collected
        if isinstance(base, Mapping):
            matches = self._matching_mapping_items(base, attribute)
            if not matches:
                return None
            if multi_value:
                results: List[Any] = []
                for _, value in matches:
                    self._append_selector_value(results, value, expand_xml_list=True, expand_list=True)
                return results or None
            value = matches[0][1]
            if isinstance(value, XMLNodeList):
                return value[0] if value else None
            return value
        if hasattr(base, attribute):
            return getattr(base, attribute)
        if attribute.startswith("@"):
            return None
        raise TypeError(f"Cannot access attribute '{attribute}' on {type(base).__name__}")

    @staticmethod
    def _xml_local_name(key: str) -> str:
        if key.startswith("@"):
            return key[1:]
        if key.startswith("{") and "}" in key:
            return key.split("}", 1)[1]
        return key

    def _matching_mapping_items(self, node: Mapping[str, Any], attribute: str) -> List[Tuple[str, Any]]:
        if isinstance(node, XMLNodeDict):
            matches: List[Tuple[str, Any]] = []
            for key, value in node.items():
                if attribute.startswith("@"):
                    if key == attribute:
                        matches.append((key, value))
                    continue
                if key == attribute or self._xml_local_name(key) == attribute:
                    matches.append((key, value))
            return matches
        if attribute in node:
            return [(attribute, node.get(attribute))]
        return []

    @staticmethod
    def _append_selector_value(
        results: List[Any],
        value: Any,
        *,
        expand_xml_list: bool = False,
        expand_list: bool = False,
    ) -> None:
        if expand_xml_list and isinstance(value, XMLNodeList):
            results.extend(value)
            return
        if expand_list and isinstance(value, list):
            results.extend(value)
            return
        results.append(value)

    def _resolve_key_value_pairs(self, base: Any, attribute: str) -> Optional[DWObject]:
        if base is None:
            return None
        if isinstance(base, (list, tuple)):
            result = DWObject()
            for item in base:
                partial = self._resolve_key_value_pairs(item, attribute)
                if partial is None:
                    continue
                for key, value in partial.items():
                    result.add(key, value)
            return result if result.items() else None
        if not isinstance(base, Mapping):
            return None
        matches = self._matching_mapping_items(base, attribute)
        if not matches:
            return None
        result = DWObject()
        for key, value in matches:
            if isinstance(value, XMLNodeList):
                for item in value:
                    result.add(key, item)
            else:
                result.add(key, value)
        return result

    def _resolve_descendant_property(
        self,
        base: Any,
        attribute: Optional[str],
        *,
        multi_value: bool = False,
    ) -> List[Any]:
        results: List[Any] = []

        def visit(node: Any) -> None:
            if node is None:
                return
            if isinstance(node, list):
                for item in node:
                    if attribute is None:
                        self._append_selector_value(results, item, expand_xml_list=True)
                    visit(item)
                return
            if isinstance(node, tuple):
                for item in node:
                    if attribute is None:
                        self._append_selector_value(results, item, expand_xml_list=True)
                    visit(item)
                return
            if isinstance(node, Mapping):
                if attribute is None:
                    for value in node.values():
                        self._append_selector_value(results, value, expand_xml_list=True)
                        visit(value)
                    return
                matches = self._matching_mapping_items(node, attribute)
                if matches:
                    if multi_value:
                        for _, value in matches:
                            self._append_selector_value(results, value, expand_xml_list=True, expand_list=True)
                    else:
                        value = matches[0][1]
                        if isinstance(value, XMLNodeList):
                            if value:
                                results.append(value[0])
                        else:
                            results.append(value)
                for value in node.values():
                    visit(value)
                return

        visit(base)
        return results

    def _resolve_descendant_key_value_pairs(self, base: Any, attribute: str) -> List[DWObject]:
        results: List[DWObject] = []

        def visit(node: Any) -> None:
            if node is None:
                return
            if isinstance(node, (list, tuple)):
                for item in node:
                    visit(item)
                return
            if isinstance(node, Mapping):
                current = self._resolve_key_value_pairs(node, attribute)
                if current is not None:
                    results.append(current)
                for value in node.values():
                    visit(value)

        visit(base)
        return results

    def _resolve_dynamic_selector(self, base: Any, selector: Any, mode: str) -> Any:
        name = str(selector)
        if mode == "multi":
            return self._resolve_property(base, name, multi_value=True)
        if mode == "attribute":
            return self._resolve_property(base, f"@{name}")
        if mode == "key_value":
            return self._resolve_key_value_pairs(base, name)
        raise TypeError(f"Unsupported dynamic selector mode: {mode}")

    def _apply_selector_filter(
        self,
        base: Any,
        predicate: parser.Expression,
        ctx: EvaluationContext,
    ) -> Any:
        if base is None:
            return None
        if isinstance(base, (list, tuple, XMLNodeList)):
            results = [item for index, item in enumerate(base) if self._selector_predicate_matches(item, index, predicate, ctx)]
            return results or None
        if isinstance(base, XMLNodeDict):
            result = XMLNodeDict()
            matched = False
            for key, value in base.items():
                if self._selector_predicate_matches(value, key, predicate, ctx):
                    result[key] = value
                    matched = True
            return result if matched else None
        if isinstance(base, DWObject):
            result = DWObject()
            for key, value in base.items():
                if self._selector_predicate_matches(value, key, predicate, ctx):
                    result.add(key, value)
            return result if result.items() else None
        if isinstance(base, Mapping):
            result: Dict[str, Any] = {}
            for key, value in base.items():
                if self._selector_predicate_matches(value, key, predicate, ctx):
                    result[str(key)] = value
            return result or None
        return base if self._selector_predicate_matches(base, 0, predicate, ctx) else None

    def _selector_predicate_matches(
        self,
        value: Any,
        key_or_index: Any,
        predicate: parser.Expression,
        ctx: EvaluationContext,
    ) -> bool:
        scoped = EvaluationContext(
            payload=ctx.payload,
            variables=dict(ctx.variables),
            header=ctx.header,
            line_offset=ctx.line_offset,
        )
        scoped.variables["$"] = value
        scoped.variables["$$"] = key_or_index
        return self._is_truthy(self._evaluate(predicate, scoped))

    def _selector_present(self, expr: parser.Expression, ctx: EvaluationContext) -> bool:
        if isinstance(expr, parser.PropertyAccess):
            base = self._evaluate(expr.value, ctx)
            return self._property_access_present(base, expr)
        if isinstance(expr, parser.IndexAccess):
            base = self._evaluate(expr.value, ctx)
            if (
                isinstance(expr.index, parser.FunctionCall)
                and isinstance(expr.index.function, parser.Identifier)
                and expr.index.function.name == "_infix_to"
                and len(expr.index.arguments) == 2
            ):
                start = self._evaluate(expr.index.arguments[0], ctx)
                end = self._evaluate(expr.index.arguments[1], ctx)
                result = self._resolve_range_index(base, start, end)
                return result is not None and result != []
            index = self._evaluate(expr.index, ctx)
            return self._index_present(base, index)
        if isinstance(expr, parser.DynamicSelector):
            base = self._evaluate(expr.value, ctx)
            selector = self._evaluate(expr.selector, ctx)
            resolved = self._resolve_dynamic_selector(base, selector, expr.mode)
            return resolved is not None and resolved != []
        value = self._evaluate(expr, ctx)
        return not self._is_missing(value)

    def _assert_selector_present(self, expr: parser.Expression, ctx: EvaluationContext) -> Any:
        if self._selector_present(expr, ctx):
            return self._evaluate(expr, ctx)
        raise DataWeaveEvaluationError(
            f"There is no key named '{self._selector_display_name(expr)}'"
        )

    def _property_access_present(self, base: Any, expr: parser.PropertyAccess) -> bool:
        if expr.recursive:
            if expr.key_value:
                return bool(self._resolve_descendant_key_value_pairs(base, expr.attribute or ""))
            return bool(
                self._resolve_descendant_property(
                    base,
                    expr.attribute,
                    multi_value=expr.multi_value,
                )
            )
        if expr.key_value:
            return self._resolve_key_value_pairs(base, expr.attribute or "") is not None
        if expr.multi_value:
            return self._resolve_property(base, expr.attribute or "", multi_value=True) is not None
        return self._property_present(base, expr.attribute or "")

    def _property_present(self, base: Any, attribute: str) -> bool:
        if base is None:
            return False
        if isinstance(base, (list, tuple)):
            return any(self._property_present(item, attribute) for item in base)
        if isinstance(base, Mapping):
            return bool(self._matching_mapping_items(base, attribute))
        return hasattr(base, attribute)

    def _index_present(self, base: Any, index: Any) -> bool:
        if base is None:
            return False
        if isinstance(base, (list, tuple, str)):
            idx = self._coerce_index(index)
            if idx is None:
                return False
            return self._normalise_sequence_index(idx, len(base)) is not None
        if isinstance(base, Mapping):
            return str(index) in base
        try:
            base[index]
            return True
        except (TypeError, KeyError, IndexError):
            return False

    @staticmethod
    def _selector_display_name(expr: parser.Expression) -> str:
        if isinstance(expr, parser.PropertyAccess) and expr.attribute is not None:
            return expr.attribute[1:] if expr.attribute.startswith("@") else expr.attribute
        if isinstance(expr, parser.DynamicSelector):
            return "<dynamic selector>"
        if isinstance(expr, parser.IndexAccess):
            return "<index>"
        return "<selector>"

    def _resolve_index(self, base: Any, index: Any) -> Any:
        if base is None:
            return None
        if isinstance(base, (list, tuple, str)):
            idx = self._coerce_index(index)
            if idx is None:
                return None
            resolved_index = self._normalise_sequence_index(idx, len(base))
            if resolved_index is None:
                return None
            return base[resolved_index]
        if isinstance(base, dict):
            key = str(index)
            return base.get(key, None)
        try:
            return base[index]
        except (TypeError, KeyError, IndexError):
            return None

    def _resolve_range_index(self, base: Any, start: Any, end: Any) -> Any:
        if base is None:
            return None
        if not isinstance(base, (list, tuple, str)):
            return None

        start_index = self._coerce_index(start)
        end_index = self._coerce_index(end)
        if start_index is None or end_index is None:
            return None

        size = len(base)
        resolved_start = self._normalise_sequence_index(start_index, size)
        resolved_end = self._normalise_sequence_index(end_index, size)
        if resolved_start is None or resolved_end is None:
            return None

        step = 1 if resolved_end >= resolved_start else -1
        selected = [base[idx] for idx in range(resolved_start, resolved_end + step, step)]
        if isinstance(base, str):
            return "".join(selected)
        return selected

    @staticmethod
    def _coerce_index(value: Any) -> Optional[int]:
        try:
            return int(value)
        except (TypeError, ValueError):
            return None

    @staticmethod
    def _normalise_sequence_index(index: int, size: int) -> Optional[int]:
        if size <= 0:
            return None
        resolved = index
        if resolved < 0:
            resolved = size + resolved
        if resolved < 0 or resolved >= size:
            return None
        return resolved

    def _populate_context_from_header(
        self,
        header: parser.Header,
        context: EvaluationContext,
    ) -> None:
        if self._enable_module_imports:
            imported = self._resolve_imports(header.imports)
            context.variables.update(imported)

        for function_decl in header.functions:
            defined_function = DefinedFunction(
                runtime=self,
                parameters=function_decl.parameters,
                body=function_decl.body,
                context=context,
                return_type=function_decl.return_type,
            )
            existing_function = context.variables.get(function_decl.name)
            if isinstance(existing_function, OverloadedFunction):
                existing_function.add(defined_function)
            elif isinstance(existing_function, DefinedFunction):
                context.variables[function_decl.name] = OverloadedFunction(
                    runtime=self,
                    functions=[existing_function, defined_function],
                )
            else:
                context.variables[function_decl.name] = defined_function

        for declaration in header.variables:
            value = self._evaluate(declaration.expression, context)
            context.variables[declaration.name] = value

    def _func_native(self, identifier: Any) -> Callable[..., Any]:
        key = str(identifier)
        function = self._native_functions.get(key)
        if function is None:
            raise DataWeaveEvaluationError(f"Unknown native function identifier '{key}'")
        return function

    def _build_native_function_registry(self) -> Dict[str, Callable[..., Any]]:
        return {
            "system::read": self._func_read,
            "system::readUrl": self._func_read_url,
            "system::write": self._func_write,
            "system::random": builtins.builtin_random,
            "system::uuid": self._func_uuid,
            "system::now": builtins.builtin_now,
            "system::log": builtins.builtin_log,
            "system::EvaluateCompatibilityFlagFunctionValue": self._func_evaluate_compatibility_flag,
            "system::ArrayAppendArrayFunctionValue": builtins.binary_concat,
            "system::StringAppendStringFunctionValue": builtins.binary_concat,
            "system::ObjectAppendObjectFunctionValue": builtins.binary_concat,
            "system::ArrayMapFunctionValue": self._func_infix_map,
            "system::ArrayFilterFunctionValue": self._func_infix_filter,
            "system::ArrayReduceFunctionValue": self._func_infix_reduce,
            "system::MapObjectObjectFunctionValue": self._func_map_object,
            "system::PluckObjectFunctionValue": builtins.builtin_pluck,
            "system::ObjectFilterFunctionValue": builtins.builtin_filter_object,
            "system::ArrayGroupByFunctionValue": builtins.builtin_group_by,
            "system::ObjectGroupByFunctionValue": builtins.builtin_group_by,
            "system::ArrayFindFunctionValue": builtins.builtin_find,
            "system::StringFindRegexFunctionValue": builtins.builtin_find,
            "system::StringFindStringFunctionValue": builtins.builtin_find,
            "system::ArrayDistinctFunctionValue": self._func_infix_distinct_by,
            "system::ObjectDistinctFunctionValue": self._func_object_distinct_by,
            "system::ArrayContainsFunctionValue": builtins.builtin_contains,
            "system::StringStringContainsFunctionValue": builtins.builtin_contains,
            "system::StringRegexContainsFunctionValue": builtins.builtin_contains,
            "system::ArrayOrderByFunctionValue": builtins.builtin_order_by,
            "system::ObjectOrderByFunctionValue": builtins.builtin_order_by,
            "system::ArraySizeOfFunctionValue": builtins.builtin_size_of,
            "system::ObjectSizeOfFunctionValue": builtins.builtin_size_of,
            "system::StringSizeOfFunctionValue": builtins.builtin_size_of,
            "system::BinarySizeOfFunctionValue": builtins.builtin_size_of,
            "system::ArrayFlattenFunctionValue": builtins.builtin_flatten,
            "system::StringEndsWithFunctionValue": builtins.builtin_endswith,
            "system::StringSplitStringFunctionValue": builtins.builtin_split_by,
            "system::StringSplitRegexFunctionValue": builtins.builtin_split_by,
            "system::StringStartsWithFunctionValue": builtins.builtin_startswith,
            "system::StringMatchesFunctionValue": builtins.builtin_matches,
            "system::StringRegexMatchFunctionValue": builtins.builtin_match,
            "system::StringLowerFunctionValue": builtins.builtin_lower,
            "system::StringTrimFunctionValue": builtins.builtin_trim,
            "system::StringUpperFunctionValue": self._func_upper,
            "system::PowNumberFunctionValue": builtins.builtin_pow,
            "system::ModuleNumberFunctionValue": builtins.builtin_mod,
            "system::SqrtNumberFunctionValue": self._func_sqrt,
            "system::AbsNumberFunctionValue": builtins.builtin_abs,
            "system::CeilNumberFunctionValue": builtins.builtin_ceil,
            "system::FloorNumberFunctionValue": builtins.builtin_floor,
            "system::TypeOfAnyFunctionValue": self._func_type_of_any,
            "system::RoundNumberFunctionValue": builtins.builtin_round,
            "system::EmptyArrayFunctionValue": builtins.builtin_is_empty,
            "system::EmptyStringFunctionValue": builtins.builtin_is_empty,
            "system::EmptyObjectFunctionValue": builtins.builtin_is_empty,
            "system::LeapDateTimeFunctionValue": builtins.builtin_is_leap_year,
            "system::LeapLocalDateFunctionValue": builtins.builtin_is_leap_year,
            "system::LeapLocalDateTimeFunctionValue": builtins.builtin_is_leap_year,
            "system::DecimalNumberFunctionValue": builtins.builtin_is_decimal,
            "system::IntegerNumberFunctionValue": builtins.builtin_is_integer,
            "system::ToRangeFunctionValue": builtins.builtin_to,
            "system::daysBetween": self._func_days_between,
            "system::ArrayRemoveFunctionValue": builtins.binary_diff,
            "system::ObjectRemoveFunctionValue": builtins.binary_diff,
            "system::StringScanFunctionValue": self._func_scan,
            "system::ReplaceStringRegexFunctionValue": self._func_replace_regex,
            "system::ReplaceStringStringFunctionValue": self._func_replace_string,
            "system::StringReduceFunctionValue": self._func_string_reduce,
            "system::SinFunctionValue": lambda angle: math.sin(self._coerce_number(angle)),
            "system::CosFunctionValue": lambda angle: math.cos(self._coerce_number(angle)),
            "system::TanFunctionValue": lambda angle: math.tan(self._coerce_number(angle)),
            "system::ASinFunctionValue": self._func_asin,
            "system::ACosFunctionValue": self._func_acos,
            "system::ATanFunctionValue": lambda angle: math.atan(self._coerce_number(angle)),
            "system::LognFunctionValue": self._func_logn,
            "system::Log10FunctionValue": self._func_log10,
            "system::BigDecimalAdditionFunctionValue": self._func_decimal_add,
            "system::BigDecimalSubtractionFunctionValue": self._func_decimal_subtract,
            "system::BigDecimalDivisionFunctionValue": self._func_decimal_divide,
            "system::BigDecimalMultiplicationFunctionValue": self._func_decimal_multiply,
            "system::BigDecimalPowerFunctionValue": self._func_decimal_pow,
            "system::BigDecimalSqrtFunctionValue": self._func_decimal_sqrt,
            "system::BigDecimalRoundFunctionValue": self._func_decimal_round,
            "system::StringWithRadixToNumber": self._func_from_radix_number,
            "system::NumberToRadixFunction": self._func_to_radix_number,
            "system::BinaryAppendBinaryFunctionValue": builtins.binary_concat,
            "system::ReadLinesFunctionValue": self._func_read_lines_with,
            "system::WriteLinesFunctionValue": self._func_write_lines_with,
            "system::FromMimeTypeString": self._func_mime_from_string,
            "system::ToMimeTypeString": self._func_mime_to_string,
            "system::IsHandledBy": self._func_mime_is_handled_by,
            "system::env": self._func_env,
            "system::fail": self._func_fail,
            "system::RunScriptFunctionValue": self._func_run_script,
            "system::EvalScriptFunctionValue": self._func_eval_script,
            "system::DataFormatDescriptorsFunctionValue": self._func_data_format_descriptors,
            "system::wait": self._func_wait,
            "system::try": self._func_try,
            "system::location": self._func_location,
            "system::props": self._func_props,
            "system::version": self._func_version,
            "system::FindDataFormatDescriptorByMimeFunctionValue": self._func_find_data_format_descriptor_by_mime,
            "system::HashFunctionValue": self._func_hash_with,
            "system::HMACFunctionValue": self._func_hmac_binary,
            "system::LocalDateAppendLocalTimeFunctionValue": self._func_date_append_time,
            "system::LocalTimeAppendLocalDateFunctionValue": self._func_time_append_date,
            "system::LocalDateAppendTimeFunctionValue": self._func_date_append_time,
            "system::TimeAppendLocalDateFunctionValue": self._func_time_append_date,
            "system::LocalDateAppendTimeZoneFunctionValue": self._func_date_append_timezone,
            "system::TimeZoneAppendLocalDateFunctionValue": self._func_timezone_append_date,
            "system::LocalDateTimeAppendTimeZoneFunctionValue": self._func_datetime_append_timezone,
            "system::TimeZoneAppendLocalDateTimeFunctionValue": self._func_timezone_append_datetime,
            "system::LocalTimeAppendTimeZoneFunctionValue": self._func_time_append_timezone,
            "system::TimeZoneValueAppendLocalTimeFunctionValue": self._func_timezone_append_time,
        }

    @staticmethod
    def _coerce_number(value: Any) -> float:
        if isinstance(value, bool):
            return 1.0 if value else 0.0
        if isinstance(value, (int, float)):
            return float(value)
        return float(str(value))

    @staticmethod
    def _func_upper(value: Any) -> Any:
        if value is None:
            return None
        return str(value).upper()

    def _func_sqrt(self, value: Any) -> Any:
        number = self._coerce_number(value)
        if number < 0:
            return float("nan")
        result = math.sqrt(number)
        return int(result) if float(result).is_integer() else result

    def _func_asin(self, value: Any) -> Any:
        number = self._coerce_number(value)
        if number < -1 or number > 1:
            return float("nan")
        return math.asin(number)

    def _func_acos(self, value: Any) -> Any:
        number = self._coerce_number(value)
        if number < -1 or number > 1:
            return float("nan")
        return math.acos(number)

    def _func_logn(self, value: Any) -> Any:
        number = self._coerce_number(value)
        if number <= 0:
            return float("nan")
        return math.log(number)

    def _func_log10(self, value: Any) -> Any:
        number = self._coerce_number(value)
        if number <= 0:
            return float("nan")
        return math.log10(number)

    def _func_date_append_time(self, left: Any, right: Any) -> datetime:
        if isinstance(left, date) and isinstance(right, time):
            return datetime.combine(left, right)
        if isinstance(left, time) and isinstance(right, date):
            return datetime.combine(right, left)
        raise TypeError("Expected (Date, Time) or (Time, Date)")

    def _func_time_append_date(self, left: Any, right: Any) -> datetime:
        return self._func_date_append_time(left, right)

    def _func_date_append_timezone(self, value: Any, tz: Any) -> datetime:
        if not isinstance(value, date):
            raise TypeError("Expected Date as first argument")
        return datetime.combine(value, time.min).replace(tzinfo=self._coerce_timezone(tz))

    def _func_timezone_append_date(self, tz: Any, value: Any) -> datetime:
        return self._func_date_append_timezone(value, tz)

    def _func_datetime_append_timezone(self, value: Any, tz: Any) -> datetime:
        if not isinstance(value, datetime):
            raise TypeError("Expected DateTime as first argument")
        return value.replace(tzinfo=self._coerce_timezone(tz))

    def _func_timezone_append_datetime(self, tz: Any, value: Any) -> datetime:
        return self._func_datetime_append_timezone(value, tz)

    def _func_time_append_timezone(self, value: Any, tz: Any) -> time:
        if not isinstance(value, time):
            raise TypeError("Expected Time as first argument")
        return value.replace(tzinfo=self._coerce_timezone(tz))

    def _func_timezone_append_time(self, tz: Any, value: Any) -> time:
        return self._func_time_append_timezone(value, tz)

    @staticmethod
    def _coerce_timezone(value: Any) -> Any:
        if isinstance(value, datetime):
            return value.tzinfo
        if isinstance(value, time):
            return value.tzinfo
        if isinstance(value, str):
            token = value.strip().strip("|")
            if token == "Z":
                return timezone.utc
            if re.fullmatch(r"[+-]\d{2}:\d{2}", token):
                sign = 1 if token[0] == "+" else -1
                hours, minutes = token[1:].split(":")
                delta = timedelta(hours=int(hours), minutes=int(minutes))
                return timezone(sign * delta)
            try:
                return ZoneInfo(token)
            except Exception:
                return timezone.utc
        return timezone.utc

    def _func_map_object(self, value: Any, mapper: Callable[..., Any]) -> Any:
        if value is None:
            return None
        if not isinstance(value, Mapping):
            raise TypeError("mapObject expects an object")
        result: Dict[Any, Any] = {}
        for index, (key, item) in enumerate(value.items()):
            mapped = builtins.invoke_lambda(mapper, item, key, index)
            if mapped is None:
                continue
            if not isinstance(mapped, Mapping):
                raise TypeError("mapObject mapper must return an object")
            result.update(mapped)
        return result

    def _func_object_distinct_by(self, value: Any, criteria: Callable[..., Any]) -> Any:
        if value is None:
            return None
        if not isinstance(value, Mapping):
            raise TypeError("distinctBy expects an object")
        seen: List[Any] = []
        result: Dict[Any, Any] = {}
        for key, item in value.items():
            marker = builtins._hashable_key(builtins.invoke_lambda(criteria, item, key))
            if marker in seen:
                continue
            seen.append(marker)
            result[key] = item
        return result

    def _func_string_reduce(self, value: Any, callback: Callable[..., Any]) -> Any:
        if value is None:
            return None
        text = str(value)
        accumulator: Any = Missing
        for char in text:
            if accumulator is Missing:
                accumulator = builtins.invoke_lambda(callback, char)
            else:
                accumulator = builtins.invoke_lambda(callback, char, accumulator)
        return None if accumulator is Missing else accumulator

    def _func_scan(self, text: Any, matcher: Any) -> List[List[str]]:
        if text is None:
            return []
        pattern = self._coerce_regex_pattern(matcher)
        compiled = re.compile(pattern)
        result: List[List[str]] = []
        for match in compiled.finditer(str(text)):
            result.append([match.group(0)] + list(match.groups()))
        return result

    def _func_replace_regex(
        self, text: Any, matcher: Any
    ) -> Callable[[Callable[..., Any]], str]:
        pattern = self._coerce_regex_pattern(matcher)
        compiled = re.compile(pattern)
        source = "" if text is None else str(text)

        def replace_with(callback: Callable[..., Any]) -> str:
            index = -1

            def repl(match: re.Match[str]) -> str:
                nonlocal index
                index += 1
                groups = [match.group(0)] + list(match.groups())
                return str(builtins.invoke_lambda(callback, groups, index))

            return compiled.sub(repl, source)

        return replace_with

    def _func_replace_string(
        self, text: Any, matcher: Any
    ) -> Callable[[Callable[..., Any]], str]:
        source = "" if text is None else str(text)
        needle = "" if matcher is None else str(matcher)

        def replace_with(callback: Callable[..., Any]) -> str:
            if needle == "":
                return source
            parts: List[str] = []
            cursor = 0
            index = 0
            while True:
                pos = source.find(needle, cursor)
                if pos < 0:
                    parts.append(source[cursor:])
                    break
                parts.append(source[cursor:pos])
                replacement = builtins.invoke_lambda(callback, [needle], index)
                parts.append("" if replacement is None else str(replacement))
                cursor = pos + len(needle)
                index += 1
            return "".join(parts)

        return replace_with

    @staticmethod
    def _coerce_regex_pattern(matcher: Any) -> str:
        pattern = "" if matcher is None else str(matcher)
        if pattern.startswith("/") and pattern.endswith("/") and len(pattern) >= 2:
            return pattern[1:-1]
        return pattern

    def _func_type_of_any(self, value: Any) -> str:
        return self._dw_type_name(value)

    def _func_read_lines_with(self, content: Any, charset: Any) -> List[str]:
        if isinstance(content, (bytes, bytearray)):
            encoding = "utf-8" if charset is None else str(charset)
            text = bytes(content).decode(encoding)
        else:
            text = "" if content is None else str(content)
        return text.splitlines()

    def _func_write_lines_with(self, content: Any, charset: Any) -> bytes:
        if content is None:
            return b""
        if not isinstance(content, list):
            raise TypeError("writeLinesWith expects an array of strings")
        encoding = "utf-8" if charset is None else str(charset)
        text = "\n".join("" if line is None else str(line) for line in content)
        return text.encode(encoding)

    def _func_from_radix_number(self, number_str: Any, radix: Any) -> Any:
        base = int(self._coerce_number(radix))
        value = int(str(number_str).strip(), base)
        return value

    def _func_to_radix_number(self, number: Any, radix: Any) -> str:
        base = int(self._coerce_number(radix))
        if base < 2 or base > 36:
            raise ValueError("Radix must be between 2 and 36")
        value = int(self._coerce_number(number))
        digits = "0123456789abcdefghijklmnopqrstuvwxyz"
        if value == 0:
            return "0"
        sign = "-" if value < 0 else ""
        value = abs(value)
        parts: List[str] = []
        while value:
            value, rem = divmod(value, base)
            parts.append(digits[rem])
        return sign + "".join(reversed(parts))

    def _func_mime_from_string(self, mime_type: Any) -> Dict[str, Any]:
        text = "" if mime_type is None else str(mime_type).strip()
        try:
            type_part, *param_parts = [segment.strip() for segment in text.split(";")]
            if "/" not in type_part:
                raise ValueError(f"Unable to find a sub type in `{text}`.")
            major, sub = [segment.strip() for segment in type_part.split("/", 1)]
            if not major or not sub:
                raise ValueError(f"Invalid MIME type `{text}`.")
            parameters: Dict[str, str] = {}
            for token in param_parts:
                if "=" not in token:
                    continue
                key, value = token.split("=", 1)
                parameters[key.strip()] = value.strip()
            return {
                "success": True,
                "result": {
                    "type": major,
                    "subtype": sub,
                    "parameters": parameters,
                },
            }
        except Exception as err:
            return {"success": False, "error": {"message": str(err)}}

    def _func_mime_to_string(self, mime_type: Any) -> str:
        if not isinstance(mime_type, Mapping):
            raise TypeError("toString expects a MIME object")
        major = str(mime_type.get("type", "")).strip()
        sub = str(mime_type.get("subtype", "")).strip()
        if not major or not sub:
            raise ValueError("MIME object requires 'type' and 'subtype'")
        base = f"{major}/{sub}"
        parameters = mime_type.get("parameters", {})
        if isinstance(parameters, Mapping) and parameters:
            suffix = ";".join(f"{key}={value}" for key, value in parameters.items())
            return f"{base};{suffix}"
        return base

    def _func_mime_is_handled_by(self, base: Any, other: Any) -> bool:
        if not isinstance(base, Mapping) or not isinstance(other, Mapping):
            return False
        base_type = str(base.get("type", "")).strip()
        base_sub = str(base.get("subtype", "")).strip()
        other_type = str(other.get("type", "")).strip()
        other_sub = str(other.get("subtype", "")).strip()
        if base_type not in {"*", other_type}:
            return False
        if base_sub == "*":
            return True
        if base_sub.endswith("*+xml"):
            return other_sub.endswith("+xml")
        return base_sub in {"*", other_sub}

    def _func_env(self) -> Dict[str, str]:
        return dict(os.environ)

    def _func_props(self) -> Dict[str, str]:
        return dict(os.environ)

    def _func_fail(self, message: Any = "Error") -> Any:
        raise DataWeaveEvaluationError(str(message))

    @staticmethod
    def _func_wait(value: Any, timeout: Any) -> Any:  # noqa: ARG004
        return value

    def _func_try(self, delegate: Callable[..., Any]) -> Dict[str, Any]:
        try:
            result = builtins.invoke_lambda(delegate)
            return {"success": True, "result": result}
        except Exception as err:
            return {
                "success": False,
                "error": {
                    "kind": type(err).__name__,
                    "message": str(err),
                },
            }

    @staticmethod
    def _func_location(value: Any) -> Dict[str, Any]:  # noqa: ARG004
        return {"locationString": "Unknown location", "text": ""}

    def _func_version(self) -> str:
        try:
            return importlib_metadata.version("dataweave-py")
        except Exception:
            return "0.0.0"

    def _func_data_format_descriptors(self) -> List[Dict[str, Any]]:
        descriptors: List[Dict[str, Any]] = []
        for definition in FormatRegistry._FORMATS.values():  # type: ignore[attr-defined]
            descriptors.append(
                {
                    "name": definition.id,
                    "binary": definition.id == "python",
                    "extensions": [],
                    "defaultMimeType": definition.mime_type,
                    "acceptedMimeTypes": [definition.mime_type],
                    "readerProperties": [],
                    "writerProperties": [],
                }
            )
        return descriptors

    def _func_find_data_format_descriptor_by_mime(self, mime: Any) -> Any:
        mime_text = self._mime_to_text(mime)
        for descriptor in self._func_data_format_descriptors():
            if mime_text in descriptor.get("acceptedMimeTypes", []):
                return descriptor
        return None

    @staticmethod
    def _mime_to_text(value: Any) -> str:
        if isinstance(value, str):
            return value
        if isinstance(value, Mapping):
            major = value.get("type")
            sub = value.get("subtype")
            if major is not None and sub is not None:
                return f"{major}/{sub}"
        return str(value)

    def _func_read(self, string_to_parse: Any, content_type: Any = "application/dw", reader_properties: Any = None) -> Any:
        content_type_text = "application/dw" if content_type is None else str(content_type)
        options = reader_properties if isinstance(reader_properties, Mapping) else {}
        if content_type_text.startswith("application/dw"):
            script_text = self._decode_text_content(string_to_parse, options)
            source = script_text.strip()
            if not source:
                return None
            if source.startswith("%dw"):
                return self.execute(source, payload={}, render_output=False)
            expr = parser.parse_expression_from_source(source)
            ctx = EvaluationContext(payload={}, variables={}, header=None)
            return self._evaluate(expr, ctx)
        format_name = content_type_text.split(";", 1)[0].strip()
        return self._convert_input_format(string_to_parse, format_name, dict(options))

    def _func_read_url(self, url: Any, content_type: Any = "application/dw", reader_properties: Any = None) -> Any:
        resource = "" if url is None else str(url)
        content = self._load_resource_bytes(resource)
        return self._func_read(content, content_type, reader_properties)

    def _func_write(self, value: Any, content_type: Any = "application/dw", writer_properties: Any = None) -> Any:
        content_type_text = "application/dw" if content_type is None else str(content_type)
        options = writer_properties if isinstance(writer_properties, Mapping) else {}
        if content_type_text.startswith("application/dw"):
            if isinstance(value, (Mapping, list)):
                return json.dumps(value, ensure_ascii=False)
            if isinstance(value, (bytes, bytearray)):
                return bytes(value)
            return "" if value is None else str(value)
        format_name = content_type_text.split(";", 1)[0].strip()
        return self._render_output(value, OutputDirective(content_type_text, format_name, dict(options)))

    def _parse_temporal_literal(self, token: str) -> Any:
        value = token.strip()
        if not value:
            return ""
        if value.startswith("P"):
            try:
                return self._coerce_period(value)
            except TypeError:
                pass
        datetime_patterns = ("%Y-%m-%dT%H:%M:%S.%f", "%Y-%m-%dT%H:%M:%S")
        if "T" in value:
            normalised = value
            if normalised.endswith("Z"):
                normalised = normalised[:-1] + "+00:00"
            try:
                return datetime.fromisoformat(normalised)
            except ValueError:
                for pattern in datetime_patterns:
                    try:
                        return datetime.strptime(value, pattern)
                    except ValueError:
                        continue
        if re.fullmatch(r"\d{4}-\d{2}-\d{2}", value):
            try:
                return date.fromisoformat(value)
            except ValueError:
                return value
        if re.fullmatch(r"\d{2}:\d{2}(?::\d{2}(?:\.\d{1,6})?)?(?:Z|[+-]\d{2}:\d{2})?", value):
            time_value = value
            if time_value.endswith("Z"):
                time_value = time_value[:-1] + "+00:00"
            try:
                return time.fromisoformat(time_value)
            except ValueError:
                return value
        if value == "Z":
            return timezone.utc
        if re.fullmatch(r"[+-]\d{2}:\d{2}", value):
            sign = 1 if value[0] == "+" else -1
            hours, minutes = value[1:].split(":")
            offset = timedelta(hours=int(hours), minutes=int(minutes))
            return timezone(sign * offset)
        return value

    @staticmethod
    def _func_uuid() -> str:
        return str(uuid_lib.uuid4())

    @staticmethod
    def _func_evaluate_compatibility_flag(flag: Any) -> bool:  # noqa: ARG004
        return False

    def _func_hash_with(self, content: Any, algorithm: Any = "SHA-1") -> bytes:
        text = self._resolve_hashlib_algorithm(algorithm)
        payload = self._coerce_binary(content)
        if text == "md2":
            return self._md2_digest(payload)
        digest = hashlib.new(text)
        digest.update(payload)
        return digest.digest()

    def _func_hmac_binary(self, secret: Any, content: Any, algorithm: Any = "HmacSHA1") -> bytes:
        algo = "sha1" if algorithm is None else str(algorithm).lower().replace("hmac", "").replace("-", "")
        key = self._coerce_binary(secret)
        payload = self._coerce_binary(content)
        return hmac.new(key, payload, algo).digest()

    @staticmethod
    def _resolve_hashlib_algorithm(algorithm: Any) -> str:
        if algorithm is None:
            return "sha1"
        token = str(algorithm).strip().upper().replace("_", "-")
        mapping = {
            "MD2": "md2",
            "MD5": "md5",
            "SHA1": "sha1",
            "SHA-1": "sha1",
            "SHA256": "sha256",
            "SHA-256": "sha256",
            "SHA384": "sha384",
            "SHA-384": "sha384",
            "SHA512": "sha512",
            "SHA-512": "sha512",
        }
        resolved = mapping.get(token)
        if resolved is None:
            raise ValueError(
                f"Unsupported hash algorithm '{algorithm}'. Supported values: "
                "MD2, MD5, SHA-1, SHA-256, SHA-384, SHA-512"
            )
        return resolved

    def _md2_digest(self, payload: bytes) -> bytes:
        s = self._MD2_S_TABLE
        pad_len = 16 - (len(payload) % 16)
        padded = payload + bytes([pad_len] * pad_len)

        checksum = [0] * 16
        l_value = 0
        for start in range(0, len(padded), 16):
            block = padded[start : start + 16]
            for idx in range(16):
                c = block[idx]
                checksum[idx] ^= s[c ^ l_value]
                l_value = checksum[idx]

        message = padded + bytes(checksum)
        state = [0] * 48

        for start in range(0, len(message), 16):
            block = message[start : start + 16]
            for idx in range(16):
                state[16 + idx] = block[idx]
                state[32 + idx] = state[16 + idx] ^ state[idx]
            t = 0
            for round_index in range(18):
                for idx in range(48):
                    state[idx] ^= s[t]
                    t = state[idx]
                t = (t + round_index) % 256

        return bytes(state[:16])

    @staticmethod
    def _decimal_number(value: Any) -> Decimal:
        if isinstance(value, Decimal):
            return value
        if isinstance(value, bool):
            return Decimal(1 if value else 0)
        if isinstance(value, (int, float)):
            return Decimal(str(value))
        return Decimal(str(value))

    @staticmethod
    def _decimal_precision(context: Any) -> Optional[int]:
        if isinstance(context, Mapping):
            precision = context.get("precision") or context.get("prec")
            if precision is not None:
                try:
                    return int(precision)
                except (TypeError, ValueError):
                    return None
        return None

    def _decimal_result(self, value: Decimal) -> Any:
        as_float = float(value)
        return int(as_float) if as_float.is_integer() else as_float

    def _func_decimal_add(self, lhs: Any, rhs: Any, ctx: Any = None) -> Any:
        with localcontext() as decimal_ctx:
            precision = self._decimal_precision(ctx)
            if precision:
                decimal_ctx.prec = precision
            return self._decimal_result(self._decimal_number(lhs) + self._decimal_number(rhs))

    def _func_decimal_subtract(self, lhs: Any, rhs: Any, ctx: Any = None) -> Any:
        with localcontext() as decimal_ctx:
            precision = self._decimal_precision(ctx)
            if precision:
                decimal_ctx.prec = precision
            return self._decimal_result(self._decimal_number(lhs) - self._decimal_number(rhs))

    def _func_decimal_divide(self, dividend: Any, divisor: Any, ctx: Any = None) -> Any:
        with localcontext() as decimal_ctx:
            precision = self._decimal_precision(ctx)
            if precision:
                decimal_ctx.prec = precision
            try:
                return self._decimal_result(self._decimal_number(dividend) / self._decimal_number(divisor))
            except InvalidOperation:
                return float("nan")

    def _func_decimal_multiply(self, left_factor: Any, right_factor: Any, ctx: Any = None) -> Any:
        with localcontext() as decimal_ctx:
            precision = self._decimal_precision(ctx)
            if precision:
                decimal_ctx.prec = precision
            return self._decimal_result(self._decimal_number(left_factor) * self._decimal_number(right_factor))

    def _func_decimal_pow(self, base: Any, exponent: Any, ctx: Any = None) -> Any:
        with localcontext() as decimal_ctx:
            precision = self._decimal_precision(ctx)
            if precision:
                decimal_ctx.prec = precision
            exp = int(self._coerce_number(exponent))
            return self._decimal_result(self._decimal_number(base) ** exp)

    def _func_decimal_sqrt(self, number: Any, ctx: Any = None) -> Any:
        with localcontext() as decimal_ctx:
            precision = self._decimal_precision(ctx)
            if precision:
                decimal_ctx.prec = precision
            value = self._decimal_number(number)
            if value < 0:
                return float("nan")
            return self._decimal_result(value.sqrt())

    def _func_decimal_round(self, number: Any, ctx: Any = None) -> Any:
        with localcontext() as decimal_ctx:
            precision = self._decimal_precision(ctx)
            if precision:
                decimal_ctx.prec = precision
            value = self._decimal_number(number)
            return self._decimal_result(value.to_integral_value())

    def _func_run_script(
        self,
        file_to_execute: Any,
        fs: Any,
        reader_inputs: Any = None,
        input_values: Any = None,
        configuration: Any = None,  # noqa: ARG002
    ) -> Dict[str, Any]:
        result = self._execute_embedded_script(file_to_execute, fs, reader_inputs, input_values)
        if result.get("success") is True:
            return {
                "success": True,
                "value": result.get("result"),
                "logs": [],
            }
        return {
            "success": False,
            "error": result.get("error"),
        }

    def _func_eval_script(
        self,
        file_to_execute: Any,
        fs: Any,
        reader_inputs: Any = None,
        input_values: Any = None,
        configuration: Any = None,  # noqa: ARG002
    ) -> Dict[str, Any]:
        result = self._execute_embedded_script(file_to_execute, fs, reader_inputs, input_values)
        if result.get("success") is True:
            return {
                "success": True,
                "result": {
                    "value": result.get("result"),
                    "logs": [],
                },
            }
        return {
            "success": False,
            "error": result.get("error"),
        }

    def _execute_embedded_script(
        self,
        file_to_execute: Any,
        fs: Any,
        reader_inputs: Any,
        input_values: Any,
    ) -> Dict[str, Any]:
        try:
            script_name = str(file_to_execute)
            if not isinstance(fs, Mapping) or script_name not in fs:
                raise FileNotFoundError(f"Unable to resolve script '{script_name}'")
            script_source = fs[script_name]
            payload = {}
            vars_context: Dict[str, Any] = {}
            if isinstance(reader_inputs, Mapping):
                payload = self._extract_reader_input(reader_inputs.get("payload"))
                for key, value in reader_inputs.items():
                    if key == "payload":
                        continue
                    vars_context[key] = self._extract_reader_input(value)
            if isinstance(input_values, Mapping):
                vars_context.update(input_values)
            result = self.execute(str(script_source), payload=payload, vars=vars_context, render_output=False)
            return {"success": True, "result": result}
        except Exception as err:
            return {
                "success": False,
                "error": {
                    "kind": type(err).__name__,
                    "message": str(err),
                    "location": {"locationString": "Unknown location"},
                    "logs": [],
                },
            }

    def _extract_reader_input(self, value: Any) -> Any:
        if not isinstance(value, Mapping):
            return value
        raw = value.get("value", value)
        mime = value.get("mimeType")
        properties = value.get("properties", {})
        if mime is None:
            return raw
        return self._func_read(raw, mime, properties)

    def _decode_text_content(self, value: Any, options: Mapping[str, Any]) -> str:
        if isinstance(value, str):
            return value
        encoding = str(options.get("encoding", "utf-8"))
        if isinstance(value, (bytes, bytearray)):
            return bytes(value).decode(encoding)
        return str(value)

    def _load_resource_bytes(self, resource: str) -> bytes:
        parsed = urlparse(resource)
        if resource.startswith("classpath://"):
            relative_path = resource[len("classpath://") :].lstrip("/")
            candidates = [
                Path.cwd() / relative_path,
                MODULE_BASE_PATH / relative_path,
                Path(__file__).resolve().parent / relative_path,
            ]
            for candidate in candidates:
                if candidate.exists():
                    return candidate.read_bytes()
            raise FileNotFoundError(f"Classpath resource not found: {resource}")
        if parsed.scheme in {"http", "https"}:
            with urlopen(resource) as response:  # noqa: S310
                return response.read()
        if parsed.scheme == "file":
            return Path(parsed.path).read_bytes()
        path = Path(resource)
        if path.exists():
            return path.read_bytes()
        raise FileNotFoundError(f"Resource not found: {resource}")

    def _func_days_between(self, start_value: Any, end_value: Any) -> int:
        if isinstance(start_value, (date, datetime)) and isinstance(end_value, (date, datetime)):
            start_date = start_value.date() if isinstance(start_value, datetime) else start_value
            end_date = end_value.date() if isinstance(end_value, datetime) else end_value
            return (end_date - start_date).days
        return builtins.builtin_days_between(str(start_value), str(end_value))

    @staticmethod
    def _is_missing(value: Any) -> bool:
        return value is None

    @staticmethod
    def _is_truthy(value: Any) -> bool:
        if value is None:
            return False
        if isinstance(value, bool):
            return value
        return bool(value)

    @staticmethod
    def _match_values(value: Any, pattern: Any) -> bool:
        if isinstance(value, XMLNodeList):
            return value == pattern or (len(value) == 1 and value[0] == pattern)
        return value == pattern

    @staticmethod
    def _to_string(value: Any) -> str:
        """Convert a value to string for interpolation."""
        if value is None:
            return ""
        if isinstance(value, str):
            return value
        if isinstance(value, bool):
            return "true" if value else "false"
        if isinstance(value, (int, float)):
            return str(value)
        if isinstance(value, (list, dict)):
            import json
            return json.dumps(value)
        return str(value)

    def _func_binary_plus(
        self,
        left: Any,
        right: Any,
        *,
        line: Optional[int] = None,
        column: Optional[int] = None,
    ) -> Any:
        if isinstance(left, (int, float, bool)) and isinstance(right, (int, float, bool)):
            left_num = float(left)
            right_num = float(right)
            result = left_num + right_num
            return int(result) if result.is_integer() else result

        if isinstance(left, list):
            result_list = list(left)
            result_list.append(right)
            return result_list

        if isinstance(left, (datetime, date, time)) and self._is_period_value(right):
            return self._add_temporal_and_period(left, right)

        if self._is_period_value(left) and isinstance(right, (datetime, date, time)):
            return self._add_temporal_and_period(right, left)

        if self._is_period_value(left) and self._is_period_value(right):
            return self._combine_period_values(left, right, operation="+")

        message = self._format_plus_error(left, right)
        raise DataWeaveEvaluationError(
            message,
            line=line,
            column=column,
            length=1,
        )

    def _func_binary_minus(
        self,
        left: Any,
        right: Any,
        *,
        line: Optional[int] = None,
        column: Optional[int] = None,
    ) -> Any:
        if isinstance(left, (int, float, bool)) and isinstance(right, (int, float, bool)):
            left_num = float(left)
            right_num = float(right)
            result = left_num - right_num
            return int(result) if result.is_integer() else result

        if isinstance(left, (datetime, date, time)) and self._is_period_value(right):
            return self._add_temporal_and_period(left, self._negate_period_value(right))

        if self._is_period_value(left) and self._is_period_value(right):
            return self._combine_period_values(left, right, operation="-")

        message = self._format_minus_error(left, right)
        raise DataWeaveEvaluationError(
            message,
            line=line,
            column=column,
            length=1,
        )

    @staticmethod
    def _is_period_value(value: Any) -> bool:
        return isinstance(value, (timedelta, builtins.DWPeriod))

    @staticmethod
    def _period_to_timedelta(value: Any) -> timedelta:
        if isinstance(value, timedelta):
            return value
        if isinstance(value, builtins.DWPeriod):
            return value.as_timedelta()
        raise TypeError("Expected a Period value")

    @staticmethod
    def _negate_period_value(value: Any) -> Any:
        if isinstance(value, timedelta):
            return -value
        if isinstance(value, builtins.DWPeriod):
            return value.negate()
        raise TypeError("Expected a Period value")

    def _add_temporal_and_period(self, temporal: Any, period_value: Any) -> Any:
        if isinstance(period_value, builtins.DWPeriod):
            if isinstance(temporal, datetime):
                return self._apply_dw_period_to_datetime(temporal, period_value)
            if isinstance(temporal, date):
                return self._apply_dw_period_to_date(temporal, period_value)
            if isinstance(temporal, time):
                return self._apply_dw_period_to_time(temporal, period_value)
            raise TypeError("Unsupported temporal value")
        delta = self._period_to_timedelta(period_value)
        if isinstance(temporal, datetime):
            return temporal + delta
        if isinstance(temporal, date):
            return (datetime.combine(temporal, time()) + delta).date()
        if isinstance(temporal, time):
            base = datetime.combine(date(1970, 1, 1), temporal)
            return (base + delta).time()
        raise TypeError("Unsupported temporal value")

    def _apply_dw_period_to_datetime(self, value: datetime, period_value: builtins.DWPeriod) -> datetime:
        result = value
        total_months = period_value.total_months()
        if total_months != 0:
            shifted = builtins._add_months_to_date(result.date(), total_months)
            result = result.replace(year=shifted.year, month=shifted.month, day=shifted.day)
        delta = timedelta(
            days=period_value.days if period_value.date_based else 0,
            hours=period_value.hours if period_value.date_based else 0,
            minutes=period_value.minutes if period_value.date_based else 0,
            seconds=period_value.seconds if period_value.date_based else 0,
        )
        if period_value.date_based:
            return result + delta
        return result + period_value.as_timedelta()

    def _apply_dw_period_to_date(self, value: date, period_value: builtins.DWPeriod) -> date:
        result = datetime.combine(value, time())
        adjusted = self._apply_dw_period_to_datetime(result, period_value)
        return adjusted.date()

    def _apply_dw_period_to_time(self, value: time, period_value: builtins.DWPeriod) -> time:
        if period_value.total_months() != 0:
            raise TypeError("Cannot add a year/month period to a Time value")
        base = datetime.combine(date(1970, 1, 1), value)
        adjusted = self._apply_dw_period_to_datetime(base, period_value)
        return adjusted.time()

    @staticmethod
    def _combine_period_values(left: Any, right: Any, *, operation: str) -> Any:
        if isinstance(left, builtins.DWPeriod) and isinstance(right, builtins.DWPeriod):
            if left.date_based and right.date_based:
                left_months = left.total_months()
                right_months = right.total_months()
                if math.isclose(left.days, 0.0) and math.isclose(right.days, 0.0):
                    result = left_months + right_months if operation == "+" else left_months - right_months
                    return int(result)
            left_seconds = left.total_seconds()
            right_seconds = right.total_seconds()
            result_seconds = left_seconds + right_seconds if operation == "+" else left_seconds - right_seconds
            return int(result_seconds) if float(result_seconds).is_integer() else result_seconds

        left_delta = left if isinstance(left, timedelta) else left.as_timedelta()
        right_delta = right if isinstance(right, timedelta) else right.as_timedelta()
        result_delta = left_delta + right_delta if operation == "+" else left_delta - right_delta
        total_seconds = result_delta.total_seconds()
        return int(total_seconds) if total_seconds.is_integer() else total_seconds

    @staticmethod
    def _func_binary_times(left: Any, right: Any) -> Any:
        return (left or 0) * (right or 0)

    @staticmethod
    def _func_binary_divide(left: Any, right: Any) -> Any:
        return (left or 0) / (right or 1)

    @staticmethod
    def _to_iterable(value: Any) -> List[Any]:
        if value is None:
            return []
        if isinstance(value, list):
            return value
        if isinstance(value, tuple):
            return list(value)
        if isinstance(value, Mapping):
            return list(value.values())
        return list(value)

    def _prepare_sequence_callable(self, function: Any) -> Callable[..., Any]:
        if callable(function):
            return function
        constant_value = copy.deepcopy(function)

        def constant_callable(*_args: Any, **_kwargs: Any) -> Any:
            return copy.deepcopy(constant_value)

        return constant_callable

    def _func_infix_map(self, sequence: Any, function: Callable[..., Any]) -> List[Any]:
        callable_function = self._prepare_sequence_callable(function)
        result: List[Any] = []
        for index, item in enumerate(self._to_iterable(sequence)):
            result.append(builtins.invoke_lambda(callable_function, item, index))
        return result

    def _func_infix_reduce(self, sequence: Any, function: Callable[..., Any]) -> Any:
        iterable = self._to_iterable(sequence)
        has_default, accumulator = self._reduce_default_accumulator(function)
        if not iterable:
            return accumulator if has_default else None
        start_index = 0
        if not has_default:
            accumulator = iterable[0]
            start_index = 1
        for item in iterable[start_index:]:
            accumulator = builtins.invoke_lambda(function, item, accumulator)
        return accumulator

    def _reduce_default_accumulator(self, function: Callable[..., Any]) -> Tuple[bool, Any]:
        parameters = getattr(function, "parameters", None)
        if not parameters or len(parameters) < 2:
            return False, Missing
        default_expr = parameters[1].default
        if default_expr is None:
            return False, Missing
        if isinstance(function, LambdaCallable):
            default_ctx = EvaluationContext(
                payload=function.payload,
                variables=dict(function.closure_variables),
                header=function.header,
            )
            return True, self._evaluate(default_expr, default_ctx)
        if isinstance(function, DefinedFunction):
            default_ctx = EvaluationContext(
                payload=function.context.payload,
                variables=dict(function.context.variables),
                header=function.context.header,
            )
            return True, self._evaluate(default_expr, default_ctx)
        return False, Missing

    def _func_infix_filter(self, sequence: Any, function: Callable[..., Any]) -> List[Any]:
        callable_function = self._prepare_sequence_callable(function)
        result: List[Any] = []
        for index, item in enumerate(self._to_iterable(sequence)):
            if self._is_truthy(builtins.invoke_lambda(callable_function, item, index)):
                result.append(item)
        return result

    def _func_infix_flat_map(self, sequence: Any, function: Callable[..., Any]) -> List[Any]:
        callable_function = self._prepare_sequence_callable(function)
        result: List[Any] = []
        for index, item in enumerate(self._to_iterable(sequence)):
            mapped = builtins.invoke_lambda(callable_function, item, index)
            result.extend(self._to_iterable(mapped))
        return result

    def _func_infix_distinct_by(self, sequence: Any, function: Callable[..., Any]) -> List[Any]:
        callable_function = self._prepare_sequence_callable(function) if function is not None else None
        items = list(self._to_iterable(sequence))
        if callable_function is None:
            return items
        seen = []
        result: List[Any] = []
        for index, item in enumerate(items):
            key = builtins.invoke_lambda(callable_function, item, index)
            marker = builtins._hashable_key(key)
            if marker not in seen:
                seen.append(marker)
                result.append(item)
        return result

    def _func_infix_to(self, start: Any, end: Any) -> List[Any]:
        return builtins.builtin_to(start, end)

    @staticmethod
    def _func_binary_eq(left: Any, right: Any) -> bool:
        return left == right

    @staticmethod
    def _func_binary_neq(left: Any, right: Any) -> bool:
        return left != right

    @staticmethod
    def _func_binary_gt(left: Any, right: Any) -> bool:
        return left > right

    @staticmethod
    def _func_binary_lt(left: Any, right: Any) -> bool:
        return left < right

    @staticmethod
    def _func_binary_gte(left: Any, right: Any) -> bool:
        return left >= right

    @staticmethod
    def _func_binary_lte(left: Any, right: Any) -> bool:
        return left <= right

    def _func_binary_and(self, left: Any, right: Any) -> bool:
        return self._is_truthy(left) and self._is_truthy(right)

    def _func_binary_or(self, left: Any, right: Any) -> bool:
        return self._is_truthy(left) or self._is_truthy(right)

    def _func_unary_not(self, value: Any) -> bool:
        return not self._is_truthy(value)

    def _call_sequence_lambda(self, function: Callable[..., Any], item: Any, index: int) -> Any:
        return builtins.invoke_lambda(function, item, index)

    def _collect_placeholders(self, expr: parser.Expression) -> Set[int]:
        placeholders: Set[int] = set()

        def visit(node: parser.Expression) -> None:
            if isinstance(node, parser.Placeholder):
                placeholders.add(node.level)
                return
            if isinstance(node, parser.LambdaExpression):
                return
            if isinstance(node, parser.ObjectLiteral):
                for key_expr, value_expr in node.fields:
                    if key_expr is not None:
                        visit(key_expr)
                    visit(value_expr)
                return
            if isinstance(node, parser.ListLiteral):
                for element in node.elements:
                    visit(element)
                return
            if isinstance(node, parser.InterpolatedString):
                for part in node.parts:
                    visit(part)
                return
            if isinstance(node, parser.PropertyAccess):
                visit(node.value)
                return
            if isinstance(node, parser.IndexAccess):
                visit(node.value)
                visit(node.index)
                return
            if isinstance(node, parser.DynamicSelector):
                visit(node.value)
                visit(node.selector)
                return
            if isinstance(node, parser.FilterSelector):
                visit(node.value)
                visit(node.predicate)
                return
            if isinstance(node, parser.SelectorModifier):
                visit(node.value)
                return
            if isinstance(node, parser.FunctionCall):
                visit(node.function)
                for argument in node.arguments:
                    visit(argument)
                return
            if isinstance(node, parser.DefaultOp):
                visit(node.left)
                visit(node.right)
                return
            if isinstance(node, parser.IfExpression):
                visit(node.condition)
                visit(node.when_true)
                visit(node.when_false)
                return
            if isinstance(node, parser.MatchExpression):
                visit(node.value)
                for case in node.cases:
                    if case.pattern is not None:
                        pattern = case.pattern
                        if pattern.matcher is not None:
                            visit(pattern.matcher)
                        if pattern.guard is not None:
                            visit(pattern.guard)
                    visit(case.expression)
                return
            if isinstance(node, parser.TypeCoercion):
                visit(node.expression)
                if node.options is not None:
                    visit(node.options)
                return

        visit(expr)
        return placeholders

    def _resolve_placeholder_argument_indexes(self, function_expr: parser.Expression) -> Tuple[int, ...]:
        if isinstance(function_expr, parser.Identifier):
            return self._IMPLICIT_LAMBDA_ARGUMENTS.get(function_expr.name, ())
        return ()

    def _resolve_imports(self, imports: List[parser.ImportDirective]) -> Dict[str, Callable[..., Any]]:
        resolved: Dict[str, Callable[..., Any]] = {}
        for directive in imports:
            try:
                names_part, module_part = directive.raw.split(" from ", 1)
            except ValueError:
                continue
            module = module_part.strip()
            exports = self._load_module_exports(module)
            runtime_exports = self._runtime_module_exports(module)
            for name, func in runtime_exports.items():
                exports.setdefault(name, func)
            builtin_exports = builtins.resolve_module_exports(module)
            for name, func in builtin_exports.items():
                exports.setdefault(name, func)
            if not exports:
                continue
            names_part = names_part.strip()
            if names_part == "*":
                resolved.update(exports)
                continue
            for entry in names_part.split(","):
                entry = entry.strip()
                if not entry:
                    continue
                if " as " in entry:
                    original, alias = [segment.strip() for segment in entry.split(" as ", 1)]
                else:
                    original = alias = entry
                if original in exports:
                    resolved[alias] = exports[original]
        return resolved

    def _runtime_module_exports(self, module: str) -> Dict[str, Callable[..., Any]]:
        if module != "dw::Runtime":
            return {}
        return {
            "fail": self._func_fail,
            "try": self._func_try,
            "run": self._func_run_script,
            "wait": self._func_wait,
            "location": self._func_location,
            "version": self._func_version,
        }

    def _load_module_exports(self, module: str) -> Dict[str, Callable[..., Any]]:
        module_path = MODULE_BASE_PATH / (module.replace("::", "/") + ".dwl")
        if not module_path.exists():
            return {}
        module_runtime = DataWeaveRuntime(enable_module_imports=False)
        module_source = module_path.read_text()
        transformed = self._transform_module_source(module_source)
        source_to_execute = transformed or module_source
        try:
            result = module_runtime.execute(
                source_to_execute,
                payload={},
                vars=dict(builtins.CORE_FUNCTIONS),
            )
        except parser.ParseError:
            LOGGER.debug("Unable to parse module %s", module)
            return {}
        except Exception:
            LOGGER.warning("Failed to load module %s", module, exc_info=True)
            return {}
        if isinstance(result, dict):
            exports: Dict[str, Callable[..., Any]] = {}
            for key, value in result.items():
                resolved_callable = self._normalise_module_export(value)
                if resolved_callable is not None:
                    exports[key] = resolved_callable
            return exports
        return {}

    @staticmethod
    def _transform_module_source(source: str) -> Optional[str]:
        cleaned = re.sub(r"/\*.*?\*/", "", source, flags=re.S)
        cleaned = re.sub(r"//.*", "", cleaned)
        cleaned = re.sub(r"(?m)^\s*@.*$", "", cleaned)
        pattern = re.compile(
            r"^fun\s+([A-Za-z0-9_]+)(?:\s*<[^>]*>)?\s*\((.*?)\)\s*(?::[^=]+)?=\s*((?:.|\n)*?)(?=^fun\s+|\Z)",
            re.MULTILINE,
        )
        functions_map: Dict[str, List[Tuple[List[str], List[Optional[str]], str]]] = {}
        for match in pattern.finditer(cleaned):
            name = match.group(1)
            params_chunk = match.group(2) or ""
            body = (match.group(3) or "").strip()
            simplified_body = DataWeaveRuntime._simplify_module_body(body)
            if not simplified_body:
                continue
            native_id = DataWeaveRuntime._extract_native_identifier(simplified_body)
            if native_id is not None:
                simplified_body = f"native({DataWeaveRuntime._dw_string_literal(native_id)})"
            param_names, param_types = DataWeaveRuntime._parse_parameters(params_chunk)
            try:
                parser.parse_expression_from_source(simplified_body)
            except parser.ParseError:
                continue
            overloads = functions_map.setdefault(name, [])
            overloads.append((param_names, param_types, simplified_body))
        if not functions_map:
            return None
        header_lines: List[str] = ["%dw 2.0"]
        export_entries: List[str] = []
        for name, overloads in functions_map.items():
            overload_entries: List[str] = []
            for index, (param_names, param_types, body) in enumerate(overloads):
                native_id = DataWeaveRuntime._extract_native_identifier(body)
                if native_id is not None:
                    encoded_native_id = DataWeaveRuntime._dw_string_literal(native_id)
                    header_lines.append(
                        f"var {name}__overload_{index} = native({encoded_native_id})"
                    )
                else:
                    params_expr = ", ".join(param_names)
                    if params_expr:
                        header_lines.append(
                            f"var {name}__overload_{index} = ({params_expr}) -> {body}"
                        )
                    else:
                        header_lines.append(f"var {name}__overload_{index} = () -> {body}")
                types_expr_parts: List[str] = []
                for type_spec in param_types:
                    if not type_spec:
                        types_expr_parts.append("null")
                    else:
                        types_expr_parts.append(DataWeaveRuntime._dw_string_literal(type_spec))
                types_expr = ", ".join(types_expr_parts)
                overload_entries.append(
                    f"{{ function: {name}__overload_{index}, paramTypes: [{types_expr}] }}"
                )
            header_lines.append(f"var {name}__overloads = [{', '.join(overload_entries)}]")
            export_entries.append(f"{name}: {name}__overloads")
        script = "\n".join(header_lines) + "\n---\n" + "{ " + ", ".join(export_entries) + " }"
        return script

    @staticmethod
    def _extract_native_identifier(body: str) -> Optional[str]:
        match = re.fullmatch(r"native\(\s*(['\"])([^'\"]+)\1\s*\)", body.strip())
        if match is None:
            return None
        return match.group(2)

    @staticmethod
    def _simplify_module_body(body: str) -> str:
        if not body:
            return ""
        body = body.strip()
        if body.startswith("do"):
            inner = body[2:].strip()
            if inner.startswith("{") and inner.endswith("}"):
                inner = inner[1:-1].strip()
            else:
                return ""
            if "---" in inner or "\nfun" in inner:
                return ""
            body = inner
        if body.endswith(";"):
            body = body[:-1].strip()
        collapsed = " ".join(segment.strip() for segment in body.splitlines() if segment.strip())
        return collapsed

    @staticmethod
    def _dw_string_literal(value: str) -> str:
        escaped = value.replace("\\", "\\\\").replace('"', '\\"')
        return f'"{escaped}"'

    @staticmethod
    def _parse_parameters(params_chunk: str) -> Tuple[List[str], List[Optional[str]]]:
        if not params_chunk.strip():
            return [], []
        parts: List[str] = []
        current: List[str] = []
        depth = 0
        for char in params_chunk:
            if char == "(":
                depth += 1
            elif char == ")":
                if depth > 0:
                    depth -= 1
            elif char == "," and depth == 0:
                part = "".join(current).strip()
                if part:
                    parts.append(part)
                current = []
                continue
            current.append(char)
        if current:
            part = "".join(current).strip()
            if part:
                parts.append(part)
        names: List[str] = []
        types: List[Optional[str]] = []
        for part in parts:
            cleaned = re.sub(r"@[\w:<>]+(?:\([^)]*\))?", "", part).strip()
            if not cleaned:
                continue
            if ":" in cleaned:
                name_part, type_part = cleaned.split(":", 1)
                name = name_part.strip()
                type_spec = type_part.strip() or None
            else:
                name = cleaned
                type_spec = None
            names.append(name)
            types.append(type_spec)
        return names, types

    def _normalise_module_export(self, value: Any) -> Optional[Callable[..., Any]]:
        if callable(value):
            return value
        if isinstance(value, list):
            overloads: List[Tuple[Optional[List[Optional[str]]], Callable[..., Any]]] = []
            for entry in value:
                function: Optional[Callable[..., Any]]
                param_types: Optional[List[Optional[str]]]
                if isinstance(entry, Mapping):
                    function = entry.get("function")
                    if not callable(function):
                        continue
                    raw_types = entry.get("paramTypes")
                    if isinstance(raw_types, list):
                        param_types = [
                            item if isinstance(item, str) and item else None for item in raw_types
                        ]
                    else:
                        param_types = None
                elif callable(entry):
                    function = entry
                    param_types = None
                else:
                    continue
                overloads.append((param_types, function))
            if not overloads:
                return None
            if len(overloads) == 1 and overloads[0][0] is None:
                return overloads[0][1]
            return self._build_overload_dispatcher(overloads)
        return None

    def _build_overload_dispatcher(
        self, overloads: List[Tuple[Optional[List[Optional[str]]], Callable[..., Any]]]
    ) -> Callable[..., Any]:
        def dispatcher(*args: Any) -> Any:
            for param_types, function in overloads:
                if self._arguments_match(function, param_types, args):
                    return function(*args)
            # Fallback to the first overload when no match is found
            return overloads[0][1](*args)

        return dispatcher

    def _arguments_match(
        self,
        function: Callable[..., Any],
        param_types: Optional[List[Optional[str]]],
        args: Tuple[Any, ...],
    ) -> bool:
        expected_count = self._function_parameter_count(function)
        if expected_count is not None and expected_count != len(args):
            return False
        if not param_types:
            return True
        if len(param_types) != len(args):
            return False
        for spec, value in zip(param_types, args):
            if spec is None:
                continue
            if not self._type_matches(value, spec):
                return False
        return True

    @staticmethod
    def _function_parameter_count(function: Callable[..., Any]) -> Optional[int]:
        count = builtins.parameter_count(function)
        if count is not None:
            return count
        try:
            signature = inspect.signature(function)
        except (TypeError, ValueError):
            return None
        total = 0
        for parameter in signature.parameters.values():
            if parameter.kind in (
                inspect.Parameter.POSITIONAL_ONLY,
                inspect.Parameter.POSITIONAL_OR_KEYWORD,
            ):
                if parameter.default is inspect._empty:
                    total += 1
                else:
                    total += 1
            else:
                return None
        return total

    @staticmethod
    def _type_matches(value: Any, spec: str) -> bool:
        spec = spec.strip()
        if not spec:
            return True
        lower = spec.lower()
        if lower in {"any", "nothing"}:
            return True
        parts = [part.strip() for part in spec.split("|") if part.strip()]
        if not parts:
            parts = [spec.strip()]
        for part in parts:
            if DataWeaveRuntime._single_type_match(value, part):
                return True
        return False

    @staticmethod
    def _single_type_match(value: Any, spec: str) -> bool:
        lower = spec.lower()
        if lower == "null":
            return value is None
        if "->" in spec or lower in {"function"}:
            return callable(value)
        if lower in {"boolean", "bool"}:
            return isinstance(value, bool)
        if lower in {"number", "integer", "double", "long", "byte"}:
            return isinstance(value, (int, float)) and not isinstance(value, bool)
        if lower in {"string", "key"}:
            return isinstance(value, str)
        if lower.startswith("array"):
            return isinstance(value, (list, tuple))
        if lower == "object" or "object" in lower:
            return isinstance(value, Mapping)
        if lower == "binary":
            return isinstance(value, (bytes, bytearray))
        if lower == "period":
            return isinstance(value, (timedelta, builtins.DWPeriod))
        # Fallback for generic type variables (for example T, V, etc.)
        if len(spec) == 1 and spec.isupper():
            return True
        return True

    def _coerce_value(
        self,
        value: Any,
        type_spec: parser.TypeSpec,
        options: Any,
        ctx: EvaluationContext,
    ) -> Any:
        target_name = (type_spec.name or "Any").strip()
        normalised = target_name.lower()
        if normalised == "null":
            return None
        if value is None:
            if normalised == "array":
                return []
            if normalised == "object":
                return {}
            return None
        if normalised == "any":
            return value
        if normalised == "number":
            return self._coerce_number(value)
        if normalised == "string":
            return self._coerce_string(value, options)
        if normalised in {"boolean", "bool"}:
            return self._coerce_boolean(value)
        if normalised == "binary":
            return self._coerce_binary(value)
        if normalised == "period":
            return self._coerce_period(value)
        if normalised == "array":
            return self._coerce_array(value, type_spec.generics, options, ctx)
        if normalised == "object":
            return self._coerce_object(value, type_spec.generics, options, ctx)
        if normalised == "date":
            return self._coerce_date(value)
        if normalised == "datetime":
            return self._coerce_datetime(value)
        if normalised == "time":
            return self._coerce_time(value)
        return value

    @staticmethod
    def _coerce_number(value: Any) -> Any:
        if value is None:
            return None
        if isinstance(value, bool):
            return 1 if value else 0
        if isinstance(value, (int, float)) and not isinstance(value, bool):
            return int(value) if float(value).is_integer() else float(value)
        if isinstance(value, str):
            text = value.strip()
            if not text:
                return None
            try:
                number = float(text)
            except ValueError as exc:
                raise TypeError(f"Cannot coerce string '{value}' to Number") from exc
            return int(number) if number.is_integer() else number
        raise TypeError(f"Cannot coerce {type(value).__name__} to Number")

    def _coerce_string(self, value: Any, options: Any = None) -> Optional[str]:
        if value is None:
            return None
        format_pattern = self._extract_format_pattern(options)
        if format_pattern and isinstance(value, (datetime, date, time)):
            return self._format_temporal_as_string(value, format_pattern)
        if isinstance(value, str):
            return value
        if isinstance(value, bool):
            return "true" if value else "false"
        return str(value)

    def _coerce_date(self, value: Any) -> date:
        if isinstance(value, datetime):
            return value.date()
        if isinstance(value, date):
            return value
        if isinstance(value, str):
            parsed = self._parse_temporal_literal(value.strip())
            if isinstance(parsed, datetime):
                return parsed.date()
            if isinstance(parsed, date):
                return parsed
        raise TypeError(f"Cannot coerce {type(value).__name__} to Date")

    def _coerce_datetime(self, value: Any) -> datetime:
        if isinstance(value, datetime):
            return value
        if isinstance(value, date):
            return datetime.combine(value, time.min)
        if isinstance(value, str):
            parsed = self._parse_temporal_literal(value.strip())
            if isinstance(parsed, datetime):
                return parsed
            if isinstance(parsed, date):
                return datetime.combine(parsed, time.min)
        raise TypeError(f"Cannot coerce {type(value).__name__} to DateTime")

    def _coerce_time(self, value: Any) -> time:
        if isinstance(value, time):
            return value
        if isinstance(value, datetime):
            return value.timetz() if value.tzinfo is not None else value.time()
        if isinstance(value, str):
            parsed = self._parse_temporal_literal(value.strip())
            if isinstance(parsed, time):
                return parsed
            if isinstance(parsed, datetime):
                return parsed.timetz() if parsed.tzinfo is not None else parsed.time()
        raise TypeError(f"Cannot coerce {type(value).__name__} to Time")

    @staticmethod
    def _extract_format_pattern(options: Any) -> Optional[str]:
        if not isinstance(options, Mapping):
            return None
        raw_pattern = options.get("format")
        if raw_pattern is None:
            return None
        return str(raw_pattern)

    def _format_temporal_as_string(self, value: Any, pattern: str) -> str:
        temporal_value = self._to_datetime_for_format(value)
        result: List[str] = []
        index = 0
        in_literal = False
        while index < len(pattern):
            char = pattern[index]
            if char == "'":
                if index + 1 < len(pattern) and pattern[index + 1] == "'":
                    result.append("'")
                    index += 2
                    continue
                in_literal = not in_literal
                index += 1
                continue
            if in_literal:
                result.append(char)
                index += 1
                continue
            end = index + 1
            while end < len(pattern) and pattern[end] == char:
                end += 1
            token_length = end - index
            rendered = self._render_temporal_token(temporal_value, char, token_length)
            result.append(rendered)
            index = end
        return "".join(result)

    @staticmethod
    def _to_datetime_for_format(value: Any) -> datetime:
        if isinstance(value, datetime):
            return value
        if isinstance(value, date):
            return datetime.combine(value, time.min)
        if isinstance(value, time):
            return datetime.combine(date(1900, 1, 1), value)
        raise TypeError(f"Cannot format {type(value).__name__} as temporal String")

    @staticmethod
    def _render_temporal_token(value: datetime, token_char: str, token_length: int) -> str:
        if token_char in {"u", "y"}:
            year = value.year
            if token_length == 2:
                return f"{year % 100:02d}"
            return str(year).zfill(max(4, token_length))
        if token_char == "M":
            if token_length >= 4:
                return value.strftime("%B")
            if token_length == 3:
                return value.strftime("%b")
            if token_length == 2:
                return f"{value.month:02d}"
            return str(value.month)
        if token_char == "d":
            if token_length >= 2:
                return f"{value.day:02d}"
            return str(value.day)
        if token_char == "H":
            if token_length >= 2:
                return f"{value.hour:02d}"
            return str(value.hour)
        if token_char == "h":
            hour = value.hour % 12
            if hour == 0:
                hour = 12
            if token_length >= 2:
                return f"{hour:02d}"
            return str(hour)
        if token_char == "K":
            hour = value.hour % 12
            if token_length >= 2:
                return f"{hour:02d}"
            return str(hour)
        if token_char == "k":
            hour = value.hour if value.hour != 0 else 24
            if token_length >= 2:
                return f"{hour:02d}"
            return str(hour)
        if token_char == "m":
            if token_length >= 2:
                return f"{value.minute:02d}"
            return str(value.minute)
        if token_char == "s":
            if token_length >= 2:
                return f"{value.second:02d}"
            return str(value.second)
        if token_char == "S":
            micros = f"{value.microsecond:06d}"
            if token_length <= 6:
                return micros[:token_length]
            return micros + ("0" * (token_length - 6))
        if token_char == "a":
            return value.strftime("%p")
        return token_char * token_length

    @staticmethod
    def _coerce_boolean(value: Any) -> Optional[bool]:
        if value is None:
            return None
        if isinstance(value, bool):
            return value
        if isinstance(value, (int, float)):
            return bool(value)
        if isinstance(value, str):
            lowered = value.strip().lower()
            if lowered in {"true", "yes", "1"}:
                return True
            if lowered in {"false", "no", "0", ""}:
                return False
            raise TypeError(f"Cannot coerce string '{value}' to Boolean")
        return bool(value)

    @staticmethod
    def _coerce_binary(value: Any) -> bytes:
        if value is None:
            return b""
        if isinstance(value, (bytes, bytearray)):
            return bytes(value)
        if isinstance(value, str):
            return value.encode("utf-8")
        raise TypeError(f"Cannot coerce {type(value).__name__} to Binary")

    @staticmethod
    def _coerce_period(value: Any) -> Any:
        if value is None:
            return builtins.builtin_period({})
        if isinstance(value, (timedelta, builtins.DWPeriod)):
            return value
        if isinstance(value, Mapping):
            if "years" in value or "months" in value:
                return builtins.builtin_period(value)
            return builtins.builtin_duration(value)
        if isinstance(value, str):
            token = value.strip().strip("|")
            period_regex = re.fullmatch(
                r"P(?:(?P<years>[-+]?\d+)Y)?(?:(?P<months>[-+]?\d+)M)?(?:(?P<days>[-+]?\d+)D)?",
                token,
            )
            if period_regex:
                return builtins.builtin_period(
                    {
                        "years": int(period_regex.group("years") or 0),
                        "months": int(period_regex.group("months") or 0),
                        "days": int(period_regex.group("days") or 0),
                    }
                )
            duration_regex = re.fullmatch(
                r"PT(?:(?P<hours>[-+]?\d+(?:\.\d+)?)H)?(?:(?P<minutes>[-+]?\d+(?:\.\d+)?)M)?(?:(?P<seconds>[-+]?\d+(?:\.\d+)?)S)?",
                token,
            )
            if duration_regex:
                return builtins.builtin_duration(
                    {
                        "hours": float(duration_regex.group("hours") or 0),
                        "minutes": float(duration_regex.group("minutes") or 0),
                        "seconds": float(duration_regex.group("seconds") or 0),
                    }
                )
        raise TypeError(f"Cannot coerce {type(value).__name__} to Period")

    def _coerce_array(
        self,
        value: Any,
        generics: List[parser.TypeSpec],
        options: Any,
        ctx: EvaluationContext,
    ) -> List[Any]:
        iterable = self._to_iterable(value)
        if not generics:
            return list(iterable)
        coerced: List[Any] = []
        inner_type = generics[0]
        for item in iterable:
            coerced.append(self._coerce_value(item, inner_type, options, ctx))
        return coerced

    def _coerce_object(
        self,
        value: Any,
        generics: List[parser.TypeSpec],
        options: Any,
        ctx: EvaluationContext,
    ) -> Dict[str, Any]:
        if not isinstance(value, Mapping):
            raise TypeError(f"Cannot coerce {type(value).__name__} to Object")
        result: Dict[str, Any] = {}
        if generics:
            inner_type = generics[0]
            for key, item in value.items():
                result[str(key)] = self._coerce_value(item, inner_type, options, ctx)
            return result
        for key, item in value.items():
            result[str(key)] = item
        return result

    @staticmethod
    def _dw_type_name(value: Any) -> str:
        if value is None:
            return "Null"
        if isinstance(value, bool):
            return "Boolean"
        if isinstance(value, (int, float)) and not isinstance(value, bool):
            return "Number"
        if isinstance(value, str):
            return "String"
        if isinstance(value, (list, tuple)):
            return "Array"
        if isinstance(value, Mapping):
            return "Object"
        if isinstance(value, datetime):
            return "DateTime"
        if isinstance(value, date):
            return "Date"
        if isinstance(value, time):
            return "Time"
        if isinstance(value, (timedelta, builtins.DWPeriod)):
            return "Period"
        return type(value).__name__

    @staticmethod
    def _preview_value(value: Any) -> str:
        if isinstance(value, str):
            return f'"{value}"'
        if isinstance(value, bool):
            return "true" if value else "false"
        if value is None:
            return "null"
        return str(value)

    @staticmethod
    def _compute_body_line_offset(source: str) -> int:
        for index, line_text in enumerate(source.splitlines(), start=1):
            if line_text.strip() == "---":
                return index
        return 0

    def _evaluate_string_literal(self, template: str, ctx: EvaluationContext) -> str:
        result: List[str] = []
        i = 0
        length = len(template)
        while i < length:
            if template[i : i + 2] == "$(":
                start = i + 2
                depth = 1
                j = start
                while j < length and depth > 0:
                    char = template[j]
                    if char == "(":
                        depth += 1
                    elif char == ")":
                        depth -= 1
                    j += 1
                expression_text = template[start : j - 1]
                expr = parser.parse_expression_from_source(expression_text)
                value = self._evaluate(expr, ctx)
                if value is None:
                    interpolated = ""
                elif isinstance(value, bool):
                    interpolated = "true" if value else "false"
                else:
                    interpolated = str(value)
                result.append(interpolated)
                i = j
            else:
                result.append(template[i])
                i += 1
        return "".join(result)

    @staticmethod
    def _format_error_message(
        source: str,
        message: str,
        line: Optional[int],
        column: Optional[int],
        length: int = 1,
        location: str = "main",
    ) -> str:
        if line is None or column is None:
            if line is None and column is None:
                return message
            location_line = f"Location:\n{location} (line: {line}, column: {column})"
            return f"{message}\n\n{location_line}"
        lines = source.splitlines()
        if line < 1 or line > len(lines):
            location_line = f"Location:\n{location} (line: {line}, column: {column})"
            return f"{message}\n\n{location_line}"
        snippet_line = lines[line - 1]
        line_label = f"{line}"
        gutter = f"{line_label}| "
        pointer_offset = len(gutter) + max(column - 1, 0)
        caret_span = "^" * max(length, 1)
        pointer_line = " " * pointer_offset + caret_span
        location_line = f"Location:\n{location} (line: {line}, column: {column})"
        return (
            f"{message}\n\n"
            f"{gutter}{snippet_line}\n"
            f"{pointer_line}\n\n"
            f"{location_line}"
        )

    def _format_plus_error(self, left: Any, right: Any) -> str:
        allowed = [
            "(Array, Any)",
            "(Date, Period)",
            "(DateTime, Period)",
            "(LocalDateTime, Period)",
            "(LocalTime, Period)",
            "(Number, Number)",
            "(Period, Period)",
            "(Period, DateTime)",
            "(Period, LocalDateTime)",
            "(Period, Time)",
            "(Period, Date)",
            "(Period, LocalTime)",
            "(Time, Period)",
        ]
        lines = [
            "You called the function '+' with these arguments:",
            f"  1: {self._dw_type_name(left)} ({self._preview_value(left)})",
            f"  2: {self._dw_type_name(right)} ({self._preview_value(right)})",
            "",
            "But it expects one of these combinations:",
        ]
        lines.extend(f"  {combo}" for combo in allowed)
        return "\n".join(lines)

    def _format_minus_error(self, left: Any, right: Any) -> str:
        allowed = [
            "(Date, Period)",
            "(DateTime, Period)",
            "(LocalDateTime, Period)",
            "(LocalTime, Period)",
            "(Number, Number)",
            "(Period, Period)",
            "(Time, Period)",
        ]
        lines = [
            "You called the function '-' with these arguments:",
            f"  1: {self._dw_type_name(left)} ({self._preview_value(left)})",
            f"  2: {self._dw_type_name(right)} ({self._preview_value(right)})",
            "",
            "But it expects one of these combinations:",
        ]
        lines.extend(f"  {combo}" for combo in allowed)
        return "\n".join(lines)
