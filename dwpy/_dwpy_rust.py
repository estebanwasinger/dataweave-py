from __future__ import annotations

import json
from typing import Any, Optional

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
        vars: Optional[dict[str, Any]] = None,
        *,
        payload_format: Optional[str] = None,
        payload_format_options: Optional[dict[str, Any]] = None,
        render_output: bool = True,
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
