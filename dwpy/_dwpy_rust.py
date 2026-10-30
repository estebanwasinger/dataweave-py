from __future__ import annotations

import json
from typing import TYPE_CHECKING, Any

from ._python_runtime import DataWeaveRuntime as _PythonDataWeaveRuntime


class RustDataWeaveRuntime:
    """Source-checkout fallback for the compiled PyO3 extension.

    Built wheels provide this module from Rust. This Python version keeps local
    test runs usable before `maturin develop` has produced the native module.
    """

    def __init__(
        self,
        *,
        enable_module_imports: bool = True,
        allow_legacy_fallback: bool = True,
    ) -> None:
        self._runtime = _PythonDataWeaveRuntime(enable_module_imports=enable_module_imports)
        self._allow_legacy_fallback = allow_legacy_fallback
        self._last_execution_engine = "not-run"

    def execute(
        self,
        script_source: str,
        payload: Any,
        vars: dict[str, Any] | None = None,
        *,
        payload_format: str | None = None,
        payload_format_options: dict[str, Any] | None = None,
        render_output: bool = True,
        output_mime_type: Optional[str] = None,
    ) -> Any:
        if not self._allow_legacy_fallback:
            self._last_execution_engine = "rust-unsupported"
            raise NotImplementedError(
                "Rust backend does not support this DataWeave feature and legacy fallback is disabled"
            )
        self._last_execution_engine = "python-legacy"
        return self._runtime.execute(
            script_source,
            payload,
            vars=vars,
            payload_format=payload_format,
            payload_format_options=payload_format_options,
            render_output=render_output,
            output_mime_type=output_mime_type,
        )

    def capabilities(self) -> list[str]:
        return [
            "python-legacy",
            "source-checkout-fallback",
            "workspace-source-backed",
        ]

    def last_execution_engine(self) -> str:
        return self._last_execution_engine

    def execute_smoke_json(self, script_source: str, payload_json: str) -> str:
        payload = json.loads(payload_json)
        if script_source.strip().endswith("payload"):
            return json.dumps(payload, separators=(",", ":"))
        raise NotImplementedError(
            "Rust evaluator does not yet support this DataWeave feature"
        )


if TYPE_CHECKING:
    # Provided by the compiled extension; absent in the source fallback.
    def function_catalog_json() -> str: ...

    def complete_symbols_json(
        script: str, line: int, column: int, module_sources_json: str | None = None
    ) -> str: ...

    def complete_fields_json(
        script: str, line: int, column: int, context_json: str
    ) -> str: ...

    def module_signatures_json(source: str) -> str: ...
