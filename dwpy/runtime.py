from __future__ import annotations

import logging
import re
import inspect
import copy
from datetime import date, datetime, time, timedelta
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Callable, Dict, List, Optional, Mapping, Set, Tuple

from . import builtins, parser

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
    _IMPLICIT_LAMBDA_ARGUMENTS: Dict[str, Tuple[int, ...]] = {
        "_infix_map": (1,),
        "_infix_filter": (1,),
        "_infix_flatMap": (1,),
        "_infix_reduce": (1,),
        "_infix_distinctBy": (1,),
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
            }
        )

    def execute(
        self, script_source: str, payload: Any, vars: Optional[Dict[str, Any]] = None
    ) -> Any:
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
        if self._enable_module_imports:
            imported = self._resolve_imports(script.header.imports)
            header_context.variables.update(imported)
        for function_decl in script.header.functions:
            header_context.variables[function_decl.name] = DefinedFunction(
                runtime=self,
                parameters=function_decl.parameters,
                body=function_decl.body,
                context=header_context,
                return_type=function_decl.return_type,
            )
        for declaration in script.header.variables:
            value = self._evaluate(declaration.expression, header_context)
            header_context.variables[declaration.name] = value
        body_line_offset = self._compute_body_line_offset(script_source)
        body_context = EvaluationContext(
            payload=payload,
            variables=header_context.variables,
            header=script.header,
            line_offset=body_line_offset,
        )
        try:
            return self._evaluate(script.body, body_context)
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

    def _normalise_input_value(self, value: Any) -> Any:
        if PANDAS_AVAILABLE:
            if isinstance(value, pd.DataFrame):
                records = value.to_dict(orient="records")
                return [self._normalise_input_value(record) for record in records]
            if isinstance(value, pd.Series):
                series_data = value.to_dict()
                return self._normalise_input_value(series_data)
        if isinstance(value, Mapping):
            return {key: self._normalise_input_value(val) for key, val in value.items()}
        if isinstance(value, list):
            return [self._normalise_input_value(item) for item in value]
        if isinstance(value, tuple):
            return [self._normalise_input_value(item) for item in value]
        return value

    def _evaluate(self, expr: parser.Expression, ctx: EvaluationContext) -> Any:
        if isinstance(expr, parser.ObjectLiteral):
            result_obj: Dict[str, Any] = {}
            for key_expr, value_expr in expr.fields:
                key_value = self._evaluate(key_expr, ctx)
                if isinstance(key_value, str):
                    key_str = key_value
                else:
                    key_str = self._to_string(key_value)
                result_obj[key_str] = self._evaluate(value_expr, ctx)
            return result_obj
        if isinstance(expr, parser.ListLiteral):
            return [self._evaluate(item, ctx) for item in expr.elements]
        if isinstance(expr, parser.StringLiteral):
            return self._evaluate_string_literal(expr.value, ctx)
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
                return self._resolve_property(base, expr.attribute)
            except TypeError:
                if expr.null_safe:
                    return None
                raise
        if isinstance(expr, parser.IndexAccess):
            base = self._evaluate(expr.value, ctx)
            index = self._evaluate(expr.index, ctx)
            return self._resolve_index(base, index)
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

    def _resolve_property(self, base: Any, attribute: str) -> Any:
        if base is None:
            return None
        if isinstance(base, dict):
            return base.get(attribute, None)
        if hasattr(base, attribute):
            return getattr(base, attribute)
        raise TypeError(f"Cannot access attribute '{attribute}' on {type(base).__name__}")

    def _resolve_index(self, base: Any, index: Any) -> Any:
        if base is None:
            return None
        if isinstance(base, (list, tuple)):
            try:
                idx = int(index)
            except (TypeError, ValueError):
                return None
            if idx < 0 or idx >= len(base):
                return None
            return base[idx]
        if isinstance(base, dict):
            key = str(index)
            return base.get(key, None)
        try:
            return base[index]
        except (TypeError, KeyError, IndexError):
            return None

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
        def is_period(value: Any) -> bool:
            return isinstance(value, timedelta)

        def ensure_datetime(value: datetime, delta: timedelta) -> datetime:
            return value + delta

        if isinstance(left, (int, float, bool)) and isinstance(right, (int, float, bool)):
            left_num = float(left)
            right_num = float(right)
            result = left_num + right_num
            return int(result) if result.is_integer() else result

        if isinstance(left, list):
            right_list = (
                list(right)
                if isinstance(right, (list, tuple))
                else [right]
            )
            return list(left) + right_list

        if isinstance(left, (datetime, date)) and is_period(right):
            if isinstance(left, datetime):
                return ensure_datetime(left, right)
            return (datetime.combine(left, time()) + right).date()

        if isinstance(left, time) and is_period(right):
            base = datetime.combine(date(1970, 1, 1), left)
            result = (base + right).time()
            return result

        if is_period(left) and isinstance(right, datetime):
            return ensure_datetime(right, left)

        if is_period(left) and isinstance(right, date):
            return (datetime.combine(right, time()) + left).date()

        if is_period(left) and isinstance(right, time):
            base = datetime.combine(date(1970, 1, 1), right)
            return (left + base).time()

        if isinstance(left, timedelta) and isinstance(right, timedelta):
            return left + right

        message = self._format_plus_error(left, right)
        raise DataWeaveEvaluationError(
            message,
            line=line,
            column=column,
            length=1,
        )

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
        accumulator = Missing
        param_count = builtins.parameter_count(function)
        for item in iterable:
            if accumulator is Missing:
                accumulator = builtins.invoke_lambda(function, item)
            else:
                if param_count and param_count > 1:
                    accumulator = function(item, accumulator)
                else:
                    accumulator = function(item)
        if accumulator is Missing:
            return None
        return accumulator

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
            r"^fun\s+([A-Za-z0-9_]+)(?:<[^>]*>)?\s*\((.*?)\)\s*(?::[^=]+)?=\s*((?:.|\n)*?)(?=^fun\s+|\Z)",
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
            if "@" in params_chunk:
                continue
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
                params_expr = ", ".join(param_names)
                if params_expr:
                    header_lines.append(f"var {name}__overload_{index} = ({params_expr}) -> {body}")
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
            cleaned = re.sub(r"@[\w:<>]+", "", part).strip()
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
            return self._coerce_string(value)
        if normalised in {"boolean", "bool"}:
            return self._coerce_boolean(value)
        if normalised == "binary":
            return self._coerce_binary(value)
        if normalised == "array":
            return self._coerce_array(value, type_spec.generics, options, ctx)
        if normalised == "object":
            return self._coerce_object(value, type_spec.generics, options, ctx)
        if normalised == "date" or normalised == "datetime":
            return self._coerce_string(value)
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

    @staticmethod
    def _coerce_string(value: Any) -> Optional[str]:
        if value is None:
            return None
        if isinstance(value, str):
            return value
        if isinstance(value, bool):
            return "true" if value else "false"
        return str(value)

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
        if isinstance(value, timedelta):
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
