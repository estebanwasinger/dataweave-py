from __future__ import annotations

import logging
import re
import inspect
import copy
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Callable, Dict, List, Optional, Mapping, Tuple

from . import builtins, parser


Missing = object()
MODULE_BASE_PATH = Path(__file__).resolve().parent / "modules"
LOGGER = logging.getLogger(__name__)


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


@dataclass
class DefinedFunction:
    runtime: "DataWeaveRuntime"
    parameters: List[parser.Parameter]
    body: parser.Expression
    context: EvaluationContext

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
        return self.runtime._evaluate(self.body, body_ctx)


class DataWeaveRuntime:
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
            }
        )

    def execute(
        self, script_source: str, payload: Any, vars: Optional[Dict[str, Any]] = None
    ) -> Any:
        script = parser.parse_script(script_source)
        variables = dict(vars or {})
        context = EvaluationContext(payload=payload, variables=variables, header=script.header)
        if self._enable_module_imports:
            imported = self._resolve_imports(script.header.imports)
            context.variables.update(imported)
        for function_decl in script.header.functions:
            context.variables[function_decl.name] = DefinedFunction(
                runtime=self,
                parameters=function_decl.parameters,
                body=function_decl.body,
                context=context,
            )
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
            return self._evaluate_string_literal(expr.value, ctx)
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

    @staticmethod
    def _func_binary_plus(left: Any, right: Any) -> Any:
        return (left or 0) + (right or 0)

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

    def _call_sequence_lambda(self, function: Callable[..., Any], item: Any, index: int) -> Any:
        return builtins.invoke_lambda(function, item, index)

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
