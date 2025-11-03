from __future__ import annotations

from dataclasses import dataclass
from typing import Any, Callable, Dict, List, Optional, Mapping

from . import builtins, parser


Missing = object()


@dataclass
class EvaluationContext:
    payload: Any
    variables: Dict[str, Any]
    header: Optional[parser.Header] = None


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
        body_ctx = EvaluationContext(
            payload=self.payload,
            variables=local_vars,
            header=self.header,
        )
        return self.runtime._evaluate(self.body, body_ctx)


class DataWeaveRuntime:
    def __init__(self) -> None:
        self._builtins: Dict[str, Callable[..., Any]] = dict(builtins.CORE_FUNCTIONS)
        self._builtins.update(
            {
                "_binary_plus": self._func_binary_plus,
                "_binary_times": self._func_binary_times,
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
            }
        )

    def execute(
        self, script_source: str, payload: Any, vars: Optional[Dict[str, Any]] = None
    ) -> Any:
        script = parser.parse_script(script_source)
        variables = dict(vars or {})
        context = EvaluationContext(payload=payload, variables=variables, header=script.header)
        for declaration in script.header.variables:
            value = self._evaluate(declaration.expression, context)
            context.variables[declaration.name] = value
        return self._evaluate(script.body, context)

    def _evaluate(self, expr: parser.Expression, ctx: EvaluationContext) -> Any:
        if isinstance(expr, parser.ObjectLiteral):
            return {key: self._evaluate(value, ctx) for key, value in expr.fields}
        if isinstance(expr, parser.ListLiteral):
            return [self._evaluate(item, ctx) for item in expr.elements]
        if isinstance(expr, parser.StringLiteral):
            return expr.value
        if isinstance(expr, parser.NumberLiteral):
            # Prefer int when possible for friendlier outputs.
            return int(expr.value) if expr.value.is_integer() else expr.value
        if isinstance(expr, parser.BooleanLiteral):
            return expr.value
        if isinstance(expr, parser.NullLiteral):
            return None
        if isinstance(expr, parser.Identifier):
            return self._resolve_identifier(expr.name, ctx)
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
            args = [self._evaluate(argument, ctx) for argument in expr.arguments]
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
        raise TypeError(f"Unsupported expression: {expr!r}")

    def _resolve_identifier(self, name: str, ctx: EvaluationContext) -> Any:
        if name == "payload":
            return ctx.payload
        if name == "vars":
            return ctx.variables
        if name in self._builtins:
            return self._builtins[name]
        if name in ctx.variables:
            return ctx.variables[name]
        raise NameError(f"Unknown identifier '{name}'")

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
    def _func_binary_plus(left: Any, right: Any) -> Any:
        return (left or 0) + (right or 0)

    @staticmethod
    def _func_binary_times(left: Any, right: Any) -> Any:
        return (left or 0) * (right or 0)

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

    def _func_infix_map(self, sequence: Any, function: Callable[..., Any]) -> List[Any]:
        result: List[Any] = []
        for index, item in enumerate(self._to_iterable(sequence)):
            result.append(builtins.invoke_lambda(function, item, index))
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
        result: List[Any] = []
        for index, item in enumerate(self._to_iterable(sequence)):
            if self._is_truthy(builtins.invoke_lambda(function, item, index)):
                result.append(item)
        return result

    def _func_infix_flat_map(self, sequence: Any, function: Callable[..., Any]) -> List[Any]:
        result: List[Any] = []
        for index, item in enumerate(self._to_iterable(sequence)):
            mapped = builtins.invoke_lambda(function, item, index)
            result.extend(self._to_iterable(mapped))
        return result

    def _func_infix_distinct_by(self, sequence: Any, function: Callable[..., Any]) -> List[Any]:
        items = list(self._to_iterable(sequence))
        if function is None:
            return items
        seen = []
        result: List[Any] = []
        for index, item in enumerate(items):
            key = builtins.invoke_lambda(function, item, index)
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

    def _call_sequence_lambda(self, function: Callable[..., Any], item: Any, index: int) -> Any:
        return builtins.invoke_lambda(function, item, index)
