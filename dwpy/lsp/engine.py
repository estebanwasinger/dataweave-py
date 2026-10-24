from __future__ import annotations

import inspect
import json
import re
from collections.abc import Iterable, Mapping, Sequence
from dataclasses import dataclass, replace
from pathlib import Path
from typing import Any, Dict, List, Optional, Tuple

from .. import builtins, parser
from ..type_inference import (
    TypeInferencer,
    _dw_type_from_rust_descriptor,
    _python_value_to_type,
)
from ..typesystem import (
    ANY,
    NUMBER,
    ArrayType,
    DWType,
    FunctionType,
    IntersectionType,
    ObjectType,
    StringType,
    UnionType,
    union_types,
)

DW_KEYWORDS: tuple[str, ...] = (
    "and",
    "as",
    "case",
    "default",
    "do",
    "else",
    "false",
    "fun",
    "if",
    "import",
    "is",
    "not",
    "null",
    "or",
    "output",
    "true",
    "type",
    "using",
    "var",
)


_SNIPPETS: tuple[tuple[str, str, str], ...] = (
    (
        "dw:script-json",
        "%dw 2.0\noutput application/json\n---\n{\n  $1\n}",
        "Basic DataWeave JSON script template",
    ),
    (
        "dw:map",
        "payload map ((item, index) -> {\n  $1\n})",
        "Map over payload array",
    ),
    (
        "dw:filter",
        "payload filter ((item, index) -> $1)",
        "Filter payload array",
    ),
    (
        "dw:reduce",
        "payload reduce ((item, acc = $1) -> $2)",
        "Reduce payload array",
    ),
)


_MISSING = object()

_ARRAY_MEMBER_FUNCTIONS: tuple[str, ...] = (
    "map",
    "filter",
    "reduce",
    "distinctBy",
    "groupBy",
    "orderBy",
    "contains",
    "joinBy",
    "sizeOf",
    "flatten",
)

_STRING_MEMBER_FUNCTIONS: tuple[str, ...] = (
    "upper",
    "lower",
    "trim",
    "contains",
    "startsWith",
    "endsWith",
    "splitBy",
    "match",
    "matches",
    "sizeOf",
)

_FUNCTION_SNIPPET_OVERRIDES: dict[str, str] = {
    "map": "map ((value, index) -> ${1})",
    "filter": "filter ((value, index) -> ${1})",
    "reduce": "reduce ((value, accumulator = ${1}) -> ${2})",
    "orderBy": "orderBy ((value, index) -> ${1})",
    "groupBy": "groupBy ((value, key) -> ${1})",
}

_HOF_FALLBACK_SIGNATURES: dict[str, tuple[str, ...]] = {
    "map": ("items", "mapper"),
    "reduce": ("items", "reducer"),
}


@dataclass(frozen=True)
class FunctionSignature:
    name: str
    parameters: tuple[str, ...]
    return_type: str | None = None
    documentation: str | None = None
    origin: str = "builtin"

    @property
    def label(self) -> str:
        params = ", ".join(self.parameters)
        if self.return_type:
            return f"{self.name}({params}) -> {self.return_type}"
        return f"{self.name}({params})"


@dataclass(frozen=True)
class EngineCompletionItem:
    label: str
    kind: str
    insert_text: str | None = None
    detail: str | None = None
    documentation: str | None = None
    insert_text_format: str = "plain"
    sort_text: Optional[str] = None
    replacement_range: Optional[Tuple[int, int, int, int]] = None


@dataclass(frozen=True)
class EngineHover:
    contents: str


@dataclass(frozen=True)
class EngineSignature:
    label: str
    documentation: str | None
    parameters: tuple[str, ...]


@dataclass(frozen=True)
class EngineSignatureHelp:
    signatures: tuple[EngineSignature, ...]
    active_signature: int
    active_parameter: int


@dataclass
class _AnalysisContext:
    payload_type: DWType
    vars_type: DWType
    attributes_type: DWType
    payload_value: Any
    vars_value: Any
    attributes_value: Any
    properties_value: Any
    symbol_types: dict[str, DWType]
    local_signatures: dict[str, tuple[FunctionSignature, ...]]
    imported_signatures: dict[str, tuple[FunctionSignature, ...]]
    builtin_signatures: dict[str, tuple[FunctionSignature, ...]]

    def all_function_signatures(self) -> dict[str, tuple[FunctionSignature, ...]]:
        merged: dict[str, tuple[FunctionSignature, ...]] = {}
        for bucket in (self.builtin_signatures, self.imported_signatures, self.local_signatures):
            for name, signatures in bucket.items():
                merged[name] = signatures
        return merged


class DataWeaveLanguageEngine:
    def __init__(self, module_base_path: Path | None = None) -> None:
        self._module_base_path = module_base_path or (Path(__file__).resolve().parent.parent / "modules")
        self._module_catalog_fingerprint = None
        self._module_catalog: Optional[Dict[str, Dict[str, Tuple[FunctionSignature, ...]]]] = None
        self._builtin_signatures = self._build_builtin_catalog()

    def complete(
        self,
        *,
        script: str,
        line: int,
        column: int,
        document_path: str | None = None,
        payload: Any = _MISSING,
        vars: Any = _MISSING,
        attributes: Any = _MISSING,
        properties: Any = _MISSING,
    ) -> list[EngineCompletionItem]:
        script = script or ""
        offset = _line_col_to_offset(script, line, column)
        ctx = self._collect_context(
            script=script,
            document_path=document_path,
            payload=payload,
            vars=vars,
            attributes=attributes,
            properties=properties,
        )
        lambda_symbols = self._infer_lambda_symbols(script, offset, ctx)

        # Lambda symbols should shadow regular locals.
        for name, type_ in lambda_symbols.items():
            ctx.symbol_types[name] = type_

        native_fields = self._native_field_completions(script, line, column, offset, ctx)
        if native_fields:
            return native_fields

        chain_info = _extract_property_chain(script[:offset])
        if chain_info is not None:
            root_expr, prefix = chain_info
            base_type = self._resolve_expression_type(root_expr, ctx)
            field_items = self._complete_object_fields(base_type, prefix)
            return field_items

        property_prefix = _extract_property_key_prefix(script[:offset])
        if property_prefix is not None:
            property_values = ctx.properties_value
            if isinstance(property_values, Mapping):
                return [
                    EngineCompletionItem(
                        label=str(key),
                        kind="property",
                        insert_text=str(key),
                        detail="Configuration property",
                        sort_text=f"1_{key}",
                    )
                    for key in sorted(property_values)
                    if str(key).startswith(property_prefix)
                ]

        native_items = self._native_completions(script, line, column, document_path)
        if native_items is not None:
            qualified = re.search(r"[A-Za-z_][A-Za-z0-9_]*::[A-Za-z0-9_:]*$", script[:offset])
            prefix = qualified.group(0) if qualified else _extract_current_word(script[:offset])
            suffix = re.match(r"[A-Za-z0-9_]*", script[offset:]).group(0)
            replacement_range = (line, max(column - len(prefix), 0), line, column + len(suffix))
            native_items = [replace(item, replacement_range=replacement_range) for item in native_items]
            selector = script[:offset - len(_extract_current_word(script[:offset]))].rstrip().endswith(".")
            extras = [] if qualified or selector else [item for item in self._complete_global(
                prefix=_extract_current_word(script[:offset]), ctx=ctx,
            ) if item.kind in {"keyword", "snippet"}
                and not any(native.label == item.label for native in native_items)]
            return sorted(native_items + extras, key=lambda item: item.sort_text or item.label)

        prefix = _extract_current_word(script[:offset])
        items = self._complete_global(prefix=prefix, ctx=ctx)
        # Accepting a suggestion in front of an existing call keeps its arguments.
        suffix = re.sub(r"^[A-Za-z0-9_]*", "", script[offset:])
        if suffix.lstrip().startswith("("):
            items = [
                replace(item, insert_text=item.label, insert_text_format="plain")
                if item.kind == "function" else item
                for item in items
            ]
        return items

    def _native_field_completions(self, script: str, line: int, column: int,
                                  offset: int, ctx: _AnalysisContext) -> Optional[List[EngineCompletionItem]]:
        try:
            from .._dwpy_rust import complete_fields_json
            context = {name: value for name, value in (
                ("payload", ctx.payload_value), ("vars", ctx.vars_value),
                ("attributes", ctx.attributes_value),
            ) if value is not _MISSING}
            response = json.loads(complete_fields_json(script, max(line, 0), max(column, 0), json.dumps(context)))
        except (ImportError, AttributeError, TypeError, ValueError):
            return None
        if response is None:
            return None
        prefix = re.search(r"[@A-Za-z0-9_]*$", script[:offset]).group(0)
        suffix = re.match(r"[A-Za-z0-9_]*", script[offset:]).group(0)
        return [EngineCompletionItem(
            label=item["label"], kind=item["kind"], insert_text=item["insertText"],
            detail=item["detail"], sort_text=item["sortText"],
            replacement_range=(line, column - len(prefix), line, column + len(suffix)),
        ) for item in response]

    def _native_completions(self, script: str, line: int, column: int,
                            document_path: Optional[str]) -> Optional[List[EngineCompletionItem]]:
        try:
            from .._dwpy_rust import complete_symbols_json
        except (ImportError, AttributeError):
            return None
        modules: Dict[str, str] = {}
        parsed, header = self._safe_parse(script)
        header = header or (parsed.header if parsed is not None else None)
        if header is not None:
            for directive in header.imports:
                raw = directive.raw.strip()
                module = raw.rsplit(" from ", 1)[-1].split(" as ", 1)[0].strip()
                if not re.fullmatch(r"[A-Za-z_][A-Za-z0-9_]*(?:::[A-Za-z_][A-Za-z0-9_]*)*", module):
                    continue
                relative = Path(*module.split("::")).with_suffix(".dwl")
                roots = [Path(document_path).parent] if document_path else []
                roots.append(self._module_base_path)
                for root in roots:
                    candidate = root / relative
                    try:
                        modules[module] = candidate.read_text(encoding="utf-8")
                        break
                    except (OSError, UnicodeError):
                        continue
        response = json.loads(complete_symbols_json(script, max(line, 0), max(column, 0), json.dumps(modules)))
        items: List[EngineCompletionItem] = []
        for item in response:
            insert = item["insertText"]
            items.append(EngineCompletionItem(
                label=item["label"], kind=item["kind"], insert_text=insert,
                detail=item.get("detail"), insert_text_format=item["insertTextFormat"], sort_text=item["sortText"],
            ))
        return items

    def hover(
        self,
        *,
        script: str,
        line: int,
        column: int,
        document_path: str | None = None,
        payload: Any = _MISSING,
        vars: Any = _MISSING,
        attributes: Any = _MISSING,
        properties: Any = _MISSING,
    ) -> EngineHover | None:
        script = script or ""
        offset = _line_col_to_offset(script, line, column)
        ctx = self._collect_context(
            script=script,
            document_path=document_path,
            payload=payload,
            vars=vars,
            attributes=attributes,
            properties=properties,
        )

        chain = _extract_chain_at_offset(script, offset)
        if chain:
            chain_type = self._resolve_expression_type(chain, ctx)
            if chain_type is not ANY:
                return EngineHover(contents=f"`{chain}`: `{chain_type.describe()}`")

        word = _extract_word_at_offset(script, offset)
        if not word:
            return None

        signatures = self._lookup_function_signatures(word, ctx)
        if signatures:
            first = signatures[0]
            docs = first.documentation or ""
            if docs:
                return EngineHover(contents=f"`{first.label}`\n\n{docs}")
            return EngineHover(contents=f"`{first.label}`")

        symbol_type = ctx.symbol_types.get(word)
        if symbol_type is not None:
            return EngineHover(contents=f"`{word}`: `{symbol_type.describe()}`")

        return None

    def signature_help(
        self,
        *,
        script: str,
        line: int,
        column: int,
        document_path: str | None = None,
        payload: Any = _MISSING,
        vars: Any = _MISSING,
        attributes: Any = _MISSING,
        properties: Any = _MISSING,
    ) -> EngineSignatureHelp | None:
        script = script or ""
        offset = _line_col_to_offset(script, line, column)
        ctx = self._collect_context(
            script=script,
            document_path=document_path,
            payload=payload,
            vars=vars,
            attributes=attributes,
            properties=properties,
        )

        call_ctx = _find_active_call(script[:offset])
        if call_ctx is None:
            return None

        fn_name, active_param = call_ctx
        signatures = self._lookup_function_signatures(fn_name, ctx)
        if not signatures:
            return None

        engine_signatures: list[EngineSignature] = []
        for signature in signatures:
            engine_signatures.append(
                EngineSignature(
                    label=signature.label,
                    documentation=signature.documentation,
                    parameters=signature.parameters,
                )
            )

        return EngineSignatureHelp(
            signatures=tuple(engine_signatures),
            active_signature=0,
            active_parameter=min(active_param, max(len(engine_signatures[0].parameters) - 1, 0)),
        )

    def _collect_context(
        self,
        *,
        script: str,
        document_path: str | None,
        payload: Any,
        vars: Any,
        attributes: Any,
        properties: Any,
    ) -> _AnalysisContext:
        sidecar_payload, sidecar_vars, sidecar_attributes, sidecar_properties = self._load_sidecar_context(document_path)

        payload_value = payload
        if payload_value is _MISSING:
            payload_value = sidecar_payload
        vars_value = vars
        if vars_value is _MISSING:
            vars_value = sidecar_vars
        attributes_value = attributes
        if attributes_value is _MISSING:
            attributes_value = sidecar_attributes
        properties_value = properties
        if properties_value is _MISSING:
            properties_value = sidecar_properties

        payload_type = self._value_to_type(payload_value)
        vars_type = self._value_to_type(vars_value)
        attributes_type = self._value_to_type(attributes_value)

        parsed_script, parsed_header = self._safe_parse(script)

        local_signatures: dict[str, tuple[FunctionSignature, ...]] = {}
        imported_signatures: dict[str, tuple[FunctionSignature, ...]] = {}
        symbol_types: dict[str, DWType] = {
            "payload": payload_type,
            "vars": vars_type,
            "attributes": attributes_type,
        }

        if parsed_script is not None:
            try:
                inferencer = TypeInferencer(
                    payload_type=payload_type,
                    vars_type=vars_type,
                    attributes_type=attributes_type,
                )
                inferencer.infer_script(parsed_script)
                if inferencer.context is not None:
                    symbol_types.update(inferencer.context.env)
            except Exception:
                # Completion/hover should stay resilient to parser/type inference failures.
                pass

        header = parsed_header or (parsed_script.header if parsed_script is not None else None)
        if header is not None:
            for function_decl in header.functions:
                signature = _function_signature_from_declaration(function_decl)
                existing = list(local_signatures.get(function_decl.name, ()))
                existing.append(signature)
                local_signatures[function_decl.name] = tuple(existing)
                if function_decl.name not in symbol_types:
                    symbol_types[function_decl.name] = FunctionType(
                        parameter_types=[ANY for _ in function_decl.parameters],
                        return_type=ANY,
                    )

            imported_signatures = self._collect_imported_signatures(header.imports)

        # If parser did not succeed, salvage declared locals with regex fallback.
        if parsed_script is None and parsed_header is None:
            for name in _scan_declared_local_names(script):
                symbol_types.setdefault(name, ANY)

        return _AnalysisContext(
            payload_type=payload_type,
            vars_type=vars_type,
            attributes_type=attributes_type,
            payload_value=payload_value,
            vars_value=vars_value,
            attributes_value=attributes_value,
            properties_value=properties_value,
            symbol_types=symbol_types,
            local_signatures=local_signatures,
            imported_signatures=imported_signatures,
            builtin_signatures=self._builtin_signatures,
        )

    def _safe_parse(self, script: str) -> tuple[parser.Script | None, parser.Header | None]:
        try:
            return parser.parse_script(script), None
        except Exception:
            pass

        # Useful recovery for files being edited before delimiter/body is complete.
        delimiter_span = parser._find_top_level_script_delimiter_span(script)
        if delimiter_span is None and script.strip():
            synthetic = script.rstrip() + "\n---\nnull\n"
            try:
                return parser.parse_script(synthetic), None
            except Exception:
                pass

        header_parser = getattr(parser, "_parse_header", None)
        if callable(header_parser):
            header_source = script
            if delimiter_span is not None:
                header_source = script[:delimiter_span[0]]
            try:
                parsed_header = header_parser(header_source.strip())
                return None, parsed_header
            except Exception:
                pass

        return None, None

    def _collect_imported_signatures(
        self,
        imports: Sequence[parser.ImportDirective],
    ) -> dict[str, tuple[FunctionSignature, ...]]:
        catalog = self._module_function_catalog()
        imported: dict[str, tuple[FunctionSignature, ...]] = {}

        for directive in imports:
            raw = directive.raw.strip()
            if " from " not in raw:
                continue
            names_part, module_part = raw.split(" from ", 1)
            module_name = module_part.strip()
            module_exports = catalog.get(module_name, {})
            if not module_exports:
                continue

            names_expr = names_part.strip()
            if names_expr == "*":
                for exported_name, signatures in module_exports.items():
                    imported[exported_name] = signatures
                continue

            for entry in names_expr.split(","):
                cleaned = entry.strip()
                if not cleaned:
                    continue
                if " as " in cleaned:
                    original, alias = [part.strip() for part in cleaned.split(" as ", 1)]
                else:
                    original = alias = cleaned
                signatures = module_exports.get(original)
                if signatures:
                    aliased = tuple(
                        FunctionSignature(
                            name=alias,
                            parameters=sig.parameters,
                            return_type=sig.return_type,
                            documentation=sig.documentation,
                            origin=sig.origin,
                        )
                        for sig in signatures
                    )
                    imported[alias] = aliased

        return imported

    def _complete_object_fields(self, root_type: DWType, prefix: str) -> list[EngineCompletionItem]:
        fields = sorted(_collect_object_fields(root_type))
        prefix_lower = prefix.lower()
        items: list[EngineCompletionItem] = []
        for field in fields:
            if prefix and not field.lower().startswith(prefix_lower):
                continue
            items.append(
                EngineCompletionItem(
                    label=field,
                    kind="field",
                    insert_text=field,
                    detail="Object field",
                    sort_text=f"1_{field}",
                )
            )
        if items:
            return items

        for function_name in _member_functions_for_type(root_type):
            if prefix and not function_name.lower().startswith(prefix_lower):
                continue
            signatures = self._builtin_signatures.get(function_name, ())
            signature = signatures[0] if signatures else FunctionSignature(name=function_name, parameters=tuple())
            items.append(
                EngineCompletionItem(
                    label=function_name,
                    kind="function",
                    insert_text=_snippet_insert_text(signature),
                    detail=signature.label if signatures else "Function",
                    documentation=signature.documentation,
                    insert_text_format="snippet",
                    sort_text=f"2_{function_name}",
                )
            )
        return items

    def _complete_global(self, *, prefix: str, ctx: _AnalysisContext) -> List[EngineCompletionItem]:
        suggestions: Dict[Tuple[str, str, Optional[str]], EngineCompletionItem] = {}
        prefix_lower = prefix.lower()

        def add_item(item: EngineCompletionItem) -> None:
            key = (item.label, item.kind, item.detail if item.kind == "function" else None)
            existing = suggestions.get(key)
            if existing is None:
                suggestions[key] = item
                return
            # Keep the best ranked candidate.
            existing_rank = existing.sort_text or "9"
            candidate_rank = item.sort_text or "9"
            if candidate_rank < existing_rank:
                suggestions[key] = item

        def matches(name: str) -> bool:
            if not prefix:
                return True
            return name.lower().startswith(prefix_lower)

        for name in sorted(ctx.symbol_types):
            if name in ctx.local_signatures or name in ctx.imported_signatures:
                continue
            if not matches(name):
                continue
            if name in {"payload", "vars", "attributes"}:
                detail = "Runtime context"
                rank = "0"
            else:
                detail = f"{ctx.symbol_types[name].describe()}"
                rank = "1"
            add_item(
                EngineCompletionItem(
                    label=name,
                    kind="variable",
                    insert_text=name,
                    detail=detail,
                    sort_text=f"{rank}_{name}",
                )
            )

        # Local declarations shadow imports, which in turn shadow builtin names.
        seen_names: set[str] = set()
        for rank, bucket in (
            ("2", ctx.local_signatures),
            ("3", ctx.imported_signatures),
            ("4", ctx.builtin_signatures),
        ):
            for name, signatures in sorted(bucket.items()):
                if name in seen_names:
                    continue
                seen_names.add(name)
                if not matches(name):
                    continue
                # Preserve overloads with distinct signatures and argument counts.
                for signature in signatures:
                    add_item(
                        EngineCompletionItem(
                            label=name,
                            kind="function",
                            insert_text=_snippet_insert_text(signature),
                            detail=signature.label,
                            documentation=signature.documentation,
                            insert_text_format="snippet",
                            sort_text=f"{rank}_{name}_{len(signature.parameters):04}",
                        )
                    )

        for keyword in DW_KEYWORDS:
            if not matches(keyword):
                continue
            add_item(
                EngineCompletionItem(
                    label=keyword,
                    kind="keyword",
                    insert_text=keyword,
                    sort_text=f"5_{keyword}",
                )
            )

        for snippet_name, snippet_text, snippet_doc in _SNIPPETS:
            if not matches(snippet_name):
                continue
            add_item(
                EngineCompletionItem(
                    label=snippet_name,
                    kind="snippet",
                    insert_text=snippet_text,
                    detail="Snippet",
                    documentation=snippet_doc,
                    insert_text_format="snippet",
                    sort_text=f"6_{snippet_name}",
                )
            )

        return sorted(suggestions.values(), key=lambda item: item.sort_text or item.label)

    def _resolve_expression_type(self, expression: str, ctx: _AnalysisContext) -> DWType:
        rust_type = self._resolve_expression_type_rust(expression, ctx)
        if rust_type is not None:
            return rust_type

        parts = [part for part in expression.replace("?.", ".").split(".") if part]
        if not parts:
            return ANY

        first_symbol, first_indexes = _split_index_tokens(parts[0])
        current = ctx.symbol_types.get(first_symbol, ANY)
        for index_token in first_indexes:
            current = _resolve_index_type(current, index_token)

        for segment in parts[1:]:
            property_name, indexes = _split_index_tokens(segment)
            if property_name:
                current = _resolve_property_type(current, property_name)
            for index_token in indexes:
                current = _resolve_index_type(current, index_token)
        return current

    def _resolve_expression_type_rust(
        self,
        expression: str,
        ctx: _AnalysisContext,
    ) -> DWType | None:
        payload_value = None if ctx.payload_value is _MISSING else ctx.payload_value
        vars_value = None if ctx.vars_value is _MISSING else ctx.vars_value
        attributes_value = None if ctx.attributes_value is _MISSING else ctx.attributes_value
        properties_value = None if ctx.properties_value is _MISSING else ctx.properties_value
        try:
            from .._dwpy_rust import RustDataWeaveRuntime

            descriptor = RustDataWeaveRuntime().infer_type_descriptor(
                expression,
                payload=payload_value,
                vars=vars_value,
                attributes=attributes_value,
                properties=properties_value,
            )
        except Exception:
            return None
        return _dw_type_from_rust_descriptor(descriptor)

    def _lookup_function_signatures(
        self,
        name: str,
        ctx: _AnalysisContext,
    ) -> tuple[FunctionSignature, ...]:
        if name in ctx.local_signatures:
            return ctx.local_signatures[name]
        if name in ctx.imported_signatures:
            return ctx.imported_signatures[name]
        return ctx.builtin_signatures.get(name, ())

    def _infer_lambda_symbols(
        self,
        script: str,
        offset: int,
        ctx: _AnalysisContext,
    ) -> dict[str, DWType]:
        prefix = script[:offset]
        lambda_match = _find_last_lambda(prefix)
        if lambda_match is None:
            return {}

        lambda_start, params = lambda_match
        if not params:
            return {}

        before_lambda = prefix[:lambda_start]
        op_match = re.search(
            r"([A-Za-z_][A-Za-z0-9_]*(?:\??\.[A-Za-z_][A-Za-z0-9_]*)*)\s+(map|filter|flatMap|groupBy|orderBy|reduce)\s*$",
            before_lambda,
        )
        if op_match is None:
            return {name: ANY for name in params}

        seq_expr, operator = op_match.group(1), op_match.group(2)
        seq_type = self._resolve_expression_type(seq_expr, ctx)
        element_type = _element_type(seq_type)

        typed: dict[str, DWType] = {}
        for index, name in enumerate(params):
            if index == 0:
                typed[name] = element_type
            elif index == 1 and operator in {"map", "filter", "flatMap", "groupBy", "orderBy"}:
                typed[name] = NUMBER
            elif index == 1 and operator == "reduce":
                typed[name] = ANY
            else:
                typed[name] = ANY
        return typed

    def _value_to_type(self, value: Any) -> DWType:
        if value is _MISSING:
            return ANY
        return _python_value_to_type(value)

    def _load_sidecar_context(self, document_path: str | None) -> tuple[Any, Any, Any, Any]:
        if not document_path:
            return _MISSING, _MISSING, _MISSING, _MISSING

        script_path = Path(document_path)
        payload_value = _load_json_file(script_path.with_name(script_path.name + ".payload.json"))
        vars_value = _load_json_file(script_path.with_name(script_path.name + ".vars.json"))
        attributes_value = _load_json_file(
            script_path.with_name(script_path.name + ".attributes.json")
        )
        properties_value = _load_json_file(
            script_path.with_name(script_path.name + ".properties.json")
        )
        return payload_value, vars_value, attributes_value, properties_value

    def _build_builtin_catalog(self) -> Dict[str, Tuple[FunctionSignature, ...]]:
        signatures: Dict[str, Tuple[FunctionSignature, ...]] = {}
        try:
            from .._dwpy_rust import function_catalog_json
            for function in json.loads(function_catalog_json()):
                signatures[function["name"]] = tuple(FunctionSignature(
                    name=function["name"], parameters=tuple(s["parameters"]),
                    return_type=s.get("returnType"), documentation=s.get("documentation"), origin="builtin",
                ) for s in function["signatures"])
        except (ImportError, AttributeError):
            for name, func in sorted(builtins.CORE_FUNCTIONS.items()):
                signature = _signature_from_callable(name=name, func=func, origin="builtin")
                signatures[name] = (signature,)
        signatures["p"] = (
            FunctionSignature(
                name="p",
                parameters=("propertyName",),
                return_type="String | Null",
                documentation="Look up a configuration, system, or environment property.",
                origin="builtin",
            ),
        )
        signatures["Mule::p"] = (
            FunctionSignature(
                name="Mule::p",
                parameters=("propertyName",),
                return_type="String | Null",
                documentation="Look up a configuration, system, or environment property.",
                origin="builtin",
            ),
        )
        signatures["prop"] = signatures["p"]
        for name, parameters in _HOF_FALLBACK_SIGNATURES.items():
            signatures.setdefault(
                name,
                (
                    FunctionSignature(
                        name=name,
                        parameters=parameters,
                        documentation="Higher-order DataWeave operator.",
                        origin="builtin",
                    ),
                ),
            )
        return signatures

    def _module_function_catalog(self) -> Dict[str, Dict[str, Tuple[FunctionSignature, ...]]]:
        files = sorted(self._module_base_path.rglob("*.dwl")) if self._module_base_path.exists() else []
        fingerprint = tuple((str(path), path.stat().st_mtime_ns, path.stat().st_size) for path in files)
        if self._module_catalog is not None and self._module_catalog_fingerprint == fingerprint:
            return self._module_catalog
        self._module_catalog_fingerprint = fingerprint

        catalog: Dict[str, Dict[str, List[FunctionSignature]]] = {}
        try:
            from .._dwpy_rust import function_catalog_json
            for function in json.loads(function_catalog_json()):
                name = function["name"].rsplit("::", 1)[-1]
                for module in function["modules"]:
                    catalog.setdefault(module, {})[name] = [FunctionSignature(
                        name=name, parameters=tuple(s["parameters"]), return_type=s.get("returnType"),
                        documentation=s.get("documentation"), origin=f"module:{module}",
                    ) for s in function["signatures"]]
        except (ImportError, AttributeError):
            pass
        function_pattern = re.compile(
            r"^fun\s+([A-Za-z0-9_]+)(?:\s*<[^>]*>)?\s*\((.*?)\)\s*(?::([^=\n]+))?=",
            re.MULTILINE,
        )

        if self._module_base_path.exists():
            for module_file in self._module_base_path.rglob("*.dwl"):
                module_name = _module_name_from_path(self._module_base_path, module_file)
                raw_text = module_file.read_text(encoding="utf-8", errors="ignore")
                module_functions = catalog.setdefault(module_name, {})

                try:
                    from .._dwpy_rust import module_signatures_json
                    found_names = set()
                    for item in json.loads(module_signatures_json(raw_text)):
                        if item["name"] not in found_names:
                            module_functions[item["name"]] = []
                            found_names.add(item["name"])
                        signature = FunctionSignature(
                            name=item["name"], parameters=tuple(item["parameters"]),
                            return_type=item.get("returnType"), origin=f"module:{module_name}",
                        )
                        module_functions.setdefault(item["name"], []).append(signature)
                    continue
                except (ImportError, AttributeError):
                    pass
                for match in function_pattern.finditer(raw_text):
                    fn_name = match.group(1)
                    params_chunk = match.group(2) or ""
                    return_chunk = (match.group(3) or "").strip() or None
                    params = tuple(_parse_parameter_specs(params_chunk))
                    signature = FunctionSignature(
                        name=fn_name,
                        parameters=params,
                        return_type=return_chunk,
                        origin=f"module:{module_name}",
                    )
                    module_functions.setdefault(fn_name, []).append(signature)

        # Ensure known module exports are always present, even when parser missed the signature.
        for module_name, exported_names in builtins.MODULE_EXPORTS.items():
            module_functions = catalog.setdefault(module_name, {})
            for fn_name in exported_names:
                if fn_name in module_functions:
                    continue
                module_functions[fn_name] = [
                    FunctionSignature(
                        name=fn_name,
                        parameters=tuple(),
                        origin=f"module:{module_name}",
                    )
                ]

        self._module_catalog = {
            module: {name: tuple(overloads) for name, overloads in functions.items()}
            for module, functions in catalog.items()
        }
        return self._module_catalog


def _signature_from_callable(name: str, func: Any, origin: str) -> FunctionSignature:
    parameters: list[str] = []
    try:
        signature = inspect.signature(func)
        for param in signature.parameters.values():
            if param.kind in {
                inspect.Parameter.POSITIONAL_ONLY,
                inspect.Parameter.POSITIONAL_OR_KEYWORD,
            }:
                if param.default is inspect._empty:
                    parameters.append(param.name)
                else:
                    parameters.append(f"{param.name}={_safe_repr(param.default)}")
            elif param.kind == inspect.Parameter.VAR_POSITIONAL:
                parameters.append(f"*{param.name}")
    except (TypeError, ValueError):
        parameters = ["value"]

    docs = inspect.getdoc(func)
    first_line = docs.splitlines()[0].strip() if docs else None
    return FunctionSignature(name=name, parameters=tuple(parameters), documentation=first_line, origin=origin)


def _safe_repr(value: Any) -> str:
    try:
        text = repr(value)
    except Exception:
        return "..."
    if len(text) > 30:
        return text[:27] + "..."
    return text


def _snippet_insert_text(signature: FunctionSignature) -> str:
    override = _FUNCTION_SNIPPET_OVERRIDES.get(signature.name) if signature.origin != "local" else None
    if override is not None:
        return override
    if not signature.parameters:
        return f"{signature.name}()"
    names = [param.split("=", 1)[0].split(":", 1)[0].strip() for param in signature.parameters]
    names = [name.replace("\\", "\\\\").replace("$", "\\$").replace("}", "\\}") for name in names]
    placeholders = [f"${{{idx + 1}:{name}}}" for idx, name in enumerate(names)]
    return f"{signature.name}({', '.join(placeholders)})"


def _line_col_to_offset(source: str, line: int, column: int) -> int:
    if line < 0:
        return 0
    lines = source.splitlines(keepends=True)
    if not lines:
        return 0
    if line >= len(lines):
        return len(source)
    start = sum(len(chunk) for chunk in lines[:line])
    units = 0
    for index, char in enumerate(lines[line]):
        width = 2 if ord(char) > 0xFFFF else 1
        if char in "\r\n" or units + width > max(column, 0):
            return start + index
        units += width
    return start + len(lines[line])


def _extract_current_word(prefix_source: str) -> str:
    match = re.search(r"([A-Za-z_][A-Za-z0-9_]*)$", prefix_source)
    if not match:
        return ""
    return match.group(1)


def _extract_property_chain(prefix_source: str) -> tuple[str, str] | None:
    segment = r"[A-Za-z_][A-Za-z0-9_]*(?:\[[^\]\n]*\])*"
    pattern = re.compile(
        rf"({segment}(?:\??\.{segment})*)\.([A-Za-z_][A-Za-z0-9_]*)?$"
    )
    match = pattern.search(prefix_source)
    if match is None:
        return None
    chain = match.group(1)
    prefix = match.group(2) or ""
    return chain, prefix


def _extract_property_key_prefix(prefix_source: str) -> str | None:
    match = re.search(
        r"\b(?:p|prop|Mule::p|Mule::prop)\(\s*['\"]([^'\"]*)$",
        prefix_source,
    )
    return match.group(1) if match else None


def _extract_chain_at_offset(source: str, offset: int) -> str | None:
    left = source[:offset]
    right = source[offset:]
    segment = r"[A-Za-z_][A-Za-z0-9_]*(?:\[[^\]\n]*\])*"
    chain_expr = rf"({segment}(?:\??\.{segment})*)"

    left_match = re.search(rf"{chain_expr}$", left)
    if left_match:
        chain = left_match.group(1)
        right_match = re.match(rf"((?:[\?\.]?{segment})*)", right)
        if right_match and right_match.group(0):
            chain += right_match.group(0)
    else:
        dot_match = re.search(rf"{chain_expr}\.$", left)
        if not dot_match:
            return None
        right_word = re.match(r"([A-Za-z_][A-Za-z0-9_]*)", right)
        if not right_word:
            return None
        chain = f"{dot_match.group(1)}.{right_word.group(1)}"

    if "." not in chain and "?." not in chain:
        return None
    return chain.replace("?.", ".")


def _extract_word_at_offset(source: str, offset: int) -> str | None:
    if not source:
        return None
    left = source[:offset]
    right = source[offset:]
    left_match = re.search(r"([A-Za-z_][A-Za-z0-9_]*)$", left)
    right_match = re.match(r"([A-Za-z0-9_]*)", right)
    if left_match:
        return left_match.group(1) + (right_match.group(1) if right_match else "")

    right_ident = re.match(r"([A-Za-z_][A-Za-z0-9_]*)", right)
    if right_ident:
        return right_ident.group(1)
    return None


def _resolve_property_type(base_type: DWType, attribute: str) -> DWType:
    if isinstance(base_type, ObjectType):
        info = base_type.get(attribute)
        if info is not None:
            field_type, is_optional, is_repeatable = info
            resolved = field_type
            if is_repeatable:
                resolved = ArrayType(element=resolved)
            if is_optional:
                resolved = union_types(resolved)
            return resolved
        if base_type.open:
            return ANY
        return ANY

    if isinstance(base_type, UnionType):
        resolved = [_resolve_property_type(option, attribute) for option in base_type.options]
        return union_types(*resolved)

    if isinstance(base_type, IntersectionType):
        resolved = [_resolve_property_type(option, attribute) for option in base_type.options]
        return union_types(*resolved)

    if isinstance(base_type, ArrayType):
        return _resolve_property_type(base_type.element, attribute)

    return ANY


def _resolve_index_type(base_type: DWType, index_token: str) -> DWType:
    token = (index_token or "").strip()
    if isinstance(base_type, ArrayType):
        return base_type.element

    if isinstance(base_type, StringType):
        return base_type

    if isinstance(base_type, ObjectType):
        key = _parse_object_index_key(token)
        if key is None:
            return ANY
        return _resolve_property_type(base_type, key)

    if isinstance(base_type, UnionType):
        resolved = [_resolve_index_type(option, token) for option in base_type.options]
        return union_types(*resolved)

    if isinstance(base_type, IntersectionType):
        resolved = [_resolve_index_type(option, token) for option in base_type.options]
        return union_types(*resolved)

    return ANY


def _parse_object_index_key(token: str) -> str | None:
    if not token:
        return None
    if token.startswith('"') and token.endswith('"') and len(token) >= 2:
        return token[1:-1]
    if token.startswith("'") and token.endswith("'") and len(token) >= 2:
        return token[1:-1]
    return None


def _split_index_tokens(segment: str) -> tuple[str, list[str]]:
    text = segment.strip()
    if not text:
        return "", []

    match = re.match(r"([A-Za-z_][A-Za-z0-9_]*)", text)
    if match is None:
        return "", []

    name = match.group(1)
    idx = match.end()
    tokens: list[str] = []

    while idx < len(text):
        if text[idx].isspace():
            idx += 1
            continue
        if text[idx] != "[":
            break
        close_idx = text.find("]", idx + 1)
        if close_idx == -1:
            token = text[idx + 1 :].strip()
            if token:
                tokens.append(token)
            break
        token = text[idx + 1 : close_idx].strip()
        tokens.append(token)
        idx = close_idx + 1

    return name, tokens


def _collect_object_fields(type_: DWType) -> Iterable[str]:
    if isinstance(type_, ObjectType):
        for field_name, _, _, _ in type_.fields:
            yield field_name
        return

    if isinstance(type_, ArrayType):
        yield from _collect_object_fields(type_.element)
        return

    if isinstance(type_, UnionType):
        for option in type_.options:
            yield from _collect_object_fields(option)
        return

    if isinstance(type_, IntersectionType):
        for option in type_.options:
            yield from _collect_object_fields(option)


def _element_type(type_: DWType) -> DWType:
    if isinstance(type_, ArrayType):
        return type_.element
    if isinstance(type_, UnionType):
        return union_types(*[_element_type(option) for option in type_.options])
    if isinstance(type_, IntersectionType):
        return union_types(*[_element_type(option) for option in type_.options])
    return ANY


def _member_functions_for_type(type_: DWType) -> tuple[str, ...]:
    collected: list[str] = []

    def add(names: Sequence[str]) -> None:
        for name in names:
            if name not in builtins.CORE_FUNCTIONS and name not in _FUNCTION_SNIPPET_OVERRIDES:
                continue
            if name not in collected:
                collected.append(name)

    def walk(current: DWType) -> None:
        if isinstance(current, ArrayType):
            add(_ARRAY_MEMBER_FUNCTIONS)
            return
        if isinstance(current, StringType):
            add(_STRING_MEMBER_FUNCTIONS)
            return
        if isinstance(current, UnionType):
            for option in current.options:
                walk(option)
            return
        if isinstance(current, IntersectionType):
            for option in current.options:
                walk(option)

    walk(type_)
    return tuple(collected)


def _load_json_file(path: Path) -> Any:
    try:
        if not path.exists():
            return _MISSING
        raw = path.read_text(encoding="utf-8")
        if not raw.strip():
            return _MISSING
        return json.loads(raw)
    except Exception:
        return _MISSING


def _module_name_from_path(base: Path, module_file: Path) -> str:
    relative = module_file.relative_to(base)
    parts = list(relative.parts)
    parts[-1] = module_file.stem
    return "::".join(parts)


def _find_last_lambda(prefix_source: str) -> tuple[int, list[str]] | None:
    matches: list[tuple[int, list[str]]] = []

    for match in re.finditer(r"\(\(\s*([^)]*?)\s*\)\s*->", prefix_source):
        params = _parse_lambda_parameters(match.group(1))
        if params:
            matches.append((match.start(), params))

    for match in re.finditer(r"(?<!\()\(\s*([A-Za-z_][A-Za-z0-9_]*)\s*\)\s*->", prefix_source):
        matches.append((match.start(), [match.group(1)]))

    if not matches:
        return None
    matches.sort(key=lambda item: item[0])
    return matches[-1]


def _parse_lambda_parameters(params_chunk: str) -> list[str]:
    params: list[str] = []
    for piece in params_chunk.split(","):
        part = piece.strip()
        if not part:
            continue
        name = part.split("=", 1)[0].split(":", 1)[0].strip()
        if re.match(r"^[A-Za-z_][A-Za-z0-9_]*$", name):
            params.append(name)
    return params


def _find_active_call(prefix_source: str) -> tuple[str, int] | None:
    stack: list[tuple[str, int]] = []

    in_string: str | None = None
    escaped = False
    in_line_comment = False
    block_depth = 0

    index = 0
    while index < len(prefix_source):
        char = prefix_source[index]
        nxt = prefix_source[index + 1] if index + 1 < len(prefix_source) else ""

        if in_line_comment:
            if char == "\n":
                in_line_comment = False
            index += 1
            continue

        if block_depth > 0:
            if char == "*" and nxt == "/":
                block_depth -= 1
                index += 2
                continue
            if char == "/" and nxt == "*":
                block_depth += 1
                index += 2
                continue
            index += 1
            continue

        if in_string is not None:
            if escaped:
                escaped = False
                index += 1
                continue
            if char == "\\":
                escaped = True
                index += 1
                continue
            if char == in_string:
                in_string = None
            index += 1
            continue

        if char == "/" and nxt == "/":
            in_line_comment = True
            index += 2
            continue
        if char == "/" and nxt == "*":
            block_depth = 1
            index += 2
            continue
        if char in {"\"", "'"}:
            in_string = char
            index += 1
            continue

        if char == "(":
            fn_name = _extract_function_name_before(prefix_source, index)
            stack.append((fn_name, 0))
            index += 1
            continue

        if char == "," and stack:
            fn_name, comma_count = stack.pop()
            stack.append((fn_name, comma_count + 1))
            index += 1
            continue

        if char == ")" and stack:
            stack.pop()
            index += 1
            continue

        index += 1

    if not stack:
        return None
    fn_name, comma_count = stack[-1]
    if not fn_name:
        return None
    return fn_name, comma_count


def _extract_function_name_before(source: str, open_paren_index: int) -> str:
    probe = source[:open_paren_index]
    match = re.search(r"([A-Za-z_][A-Za-z0-9_]*(?:\??\.[A-Za-z_][A-Za-z0-9_]*)*)\s*$", probe)
    if not match:
        return ""
    chain = match.group(1).replace("?.", ".")
    return chain.split(".")[-1]


def _parse_parameter_specs(params_chunk: str) -> list[str]:
    if not params_chunk.strip():
        return []

    parts: list[str] = []
    current: list[str] = []
    depth = 0

    for char in params_chunk:
        if char == "(":
            depth += 1
        elif char == ")":
            if depth > 0:
                depth -= 1
        elif char == "," and depth == 0:
            piece = "".join(current).strip()
            if piece:
                parts.append(piece)
            current = []
            continue
        current.append(char)

    if current:
        piece = "".join(current).strip()
        if piece:
            parts.append(piece)

    rendered: list[str] = []
    for part in parts:
        cleaned = re.sub(r"@[\w:<>]+(?:\([^)]*\))?", "", part).strip()
        if not cleaned:
            continue
        if ":" in cleaned:
            name, type_spec = [item.strip() for item in cleaned.split(":", 1)]
            if type_spec:
                rendered.append(f"{name}: {type_spec}")
            else:
                rendered.append(name)
        else:
            rendered.append(cleaned)
    return rendered


def _render_type_spec(spec: parser.TypeSpec | None) -> str | None:
    if spec is None:
        return None

    if isinstance(spec, parser.ReferenceTypeSpec):
        if spec.generics:
            inner = ", ".join(
                inner_text
                for inner_text in (_render_type_spec(item) for item in spec.generics)
                if inner_text
            )
            return f"{spec.name}<{inner}>"
        return spec.name

    if isinstance(spec, parser.ObjectTypeSpec):
        fields: list[str] = []
        for name, field_type, is_optional, is_repeatable in spec.fields:
            suffix = "?" if is_optional else "*" if is_repeatable else ""
            rendered = _render_type_spec(field_type) or "Any"
            fields.append(f"{name}{suffix}: {rendered}")
        trailer = ", ..." if spec.is_open else ""
        return "{ " + ", ".join(fields) + trailer + " }"

    if isinstance(spec, parser.UnionTypeSpec):
        return " | ".join(filter(None, (_render_type_spec(option) for option in spec.options)))

    if isinstance(spec, parser.IntersectionTypeSpec):
        return " & ".join(filter(None, (_render_type_spec(option) for option in spec.options)))

    if isinstance(spec, parser.FunctionTypeSpec):
        params = ", ".join(filter(None, (_render_type_spec(option) for option in spec.parameters)))
        return_type = _render_type_spec(spec.return_type) or "Any"
        return f"({params}) -> {return_type}"

    if isinstance(spec, parser.LiteralTypeSpec):
        base = _render_type_spec(spec.base_type) or "Any"
        return f"{spec.value!r} as {base}"

    return str(spec)


def _function_signature_from_declaration(function_decl: parser.FunctionDeclaration) -> FunctionSignature:
    parameters: list[str] = []
    for parameter in function_decl.parameters:
        rendered_type = _render_type_spec(parameter.type_annotation)
        if rendered_type:
            parameters.append(f"{parameter.name}: {rendered_type}")
        else:
            parameters.append(parameter.name)

    return_type = _render_type_spec(function_decl.return_type)
    return FunctionSignature(
        name=function_decl.name,
        parameters=tuple(parameters),
        return_type=return_type,
        origin="local",
    )


def _scan_declared_local_names(script: str) -> list[str]:
    names = set()
    for match in re.finditer(r"(?m)^\s*var\s+([A-Za-z_][A-Za-z0-9_]*)", script):
        names.add(match.group(1))
    for match in re.finditer(r"(?m)^\s*fun\s+([A-Za-z_][A-Za-z0-9_]*)", script):
        names.add(match.group(1))
    return sorted(names)
