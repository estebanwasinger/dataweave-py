from __future__ import annotations

import os
from typing import Any, Dict, Literal, Optional

from ._python_runtime import (
    DataWeaveEvaluationError,
    DefinedFunction,
    EvaluationContext,
    ImplicitLambdaCallable,
    LambdaCallable,
    OutputDirective,
    OverloadedFunction,
)
from ._python_runtime import DataWeaveRuntime as PythonDataWeaveRuntime

BackendName = Literal["rust", "python", "wasm", "auto"]


class DataWeaveRuntime:
    """Public runtime facade.

    The default backend is the Rust extension. The legacy Python interpreter is
    still available explicitly, and `auto` can be used during parity work to
    fall back if the extension is unavailable.
    """

    def __init__(
        self,
        *,
        enable_module_imports: bool = True,
        backend: BackendName | None = None,
    ) -> None:
        requested_backend = (
            os.environ.get("DWPY_TEST_BACKEND")
            or backend
            or os.environ.get("DWPY_BACKEND")
        )
        selected_backend = requested_backend or "auto"
        if selected_backend not in {"rust", "python", "wasm", "auto"}:
            raise ValueError(
                "DataWeaveRuntime backend must be one of 'rust', 'python', 'wasm', or 'auto'"
            )

        self.backend: BackendName = selected_backend  # type: ignore[assignment]
        self._enable_module_imports = enable_module_imports
        self._python_runtime: Optional[PythonDataWeaveRuntime] = None
        self._rust_runtime: Optional[Any] = None
        self._wasm_runtime: Optional[Any] = None

        if self.backend == "python":
            self._python_runtime = PythonDataWeaveRuntime(
                enable_module_imports=enable_module_imports
            )
        elif self.backend == "rust":
            self._rust_runtime = self._new_rust_runtime(
                enable_module_imports,
                allow_legacy_fallback=False,
            )
        elif self.backend == "wasm":
            from ._wasm_runtime import WasmDataWeaveRuntime

            self._wasm_runtime = WasmDataWeaveRuntime(
                enable_module_imports=enable_module_imports
            )
            self._rust_runtime = self._wasm_runtime
        else:
            try:
                self._rust_runtime = self._new_rust_runtime(
                    enable_module_imports,
                    allow_legacy_fallback=True,
                )
            except Exception:
                self._python_runtime = PythonDataWeaveRuntime(
                    enable_module_imports=enable_module_imports
                )

    @property
    def active_backend(self) -> BackendName:
        if self._wasm_runtime is not None:
            return "wasm"
        if self._rust_runtime is not None:
            return "rust"
        return "python"

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
        if self._wasm_runtime is not None:
            return self._wasm_runtime.execute(
                script_source,
                payload,
                vars=vars,
                payload_format=payload_format,
                payload_format_options=payload_format_options,
                render_output=render_output,
            )

        if self._rust_runtime is not None:
            return self._rust_runtime.execute(
                script_source,
                payload,
                vars=vars,
                payload_format=payload_format,
                payload_format_options=payload_format_options,
                render_output=render_output,
            )

        return self._legacy_runtime.execute(
            script_source,
            payload,
            vars=vars,
            payload_format=payload_format,
            payload_format_options=payload_format_options,
            render_output=render_output,
        )

    def capabilities(self) -> list[str]:
        if self._wasm_runtime is not None:
            return self._wasm_runtime.capabilities()
        if self._rust_runtime is not None and hasattr(self._rust_runtime, "capabilities"):
            return list(self._rust_runtime.capabilities())
        return ["python-legacy"]

    def __getattr__(self, name: str) -> Any:
        if self._wasm_runtime is not None:
            return getattr(self._wasm_runtime, name)
        return getattr(self._legacy_runtime, name)

    @property
    def _legacy_runtime(self) -> PythonDataWeaveRuntime:
        if self._python_runtime is None:
            self._python_runtime = PythonDataWeaveRuntime(
                enable_module_imports=self._enable_module_imports
            )
        return self._python_runtime

    @staticmethod
    def _new_rust_runtime(
        enable_module_imports: bool,
        *,
        allow_legacy_fallback: bool,
    ) -> Any:
        from ._dwpy_rust import RustDataWeaveRuntime

        return RustDataWeaveRuntime(
            enable_module_imports=enable_module_imports,
            allow_legacy_fallback=allow_legacy_fallback,
        )


__all__ = [
    "DataWeaveRuntime",
    "PythonDataWeaveRuntime",
    "DataWeaveEvaluationError",
    "EvaluationContext",
    "OutputDirective",
    "LambdaCallable",
    "DefinedFunction",
    "OverloadedFunction",
    "ImplicitLambdaCallable",
]
