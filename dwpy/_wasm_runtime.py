from __future__ import annotations

import atexit
import json
import os
import subprocess
import threading
from datetime import date, datetime, time
from pathlib import Path
from typing import Any, Dict, Optional

from . import parser
from ._python_runtime import DataWeaveEvaluationError
from ._rust_bridge import json_default
from .formats import DWObject


_PACKAGE_ROOT = Path(__file__).resolve().parent.parent / "packages" / "dataweave-wasm"


class _WasmNodeBridge:
    _instance: Optional["_WasmNodeBridge"] = None
    _instance_lock = threading.Lock()

    def __init__(self) -> None:
        node = os.environ.get("DWPY_NODE", "node")
        package_root = Path(
            os.environ.get("DWPY_WASM_PACKAGE", str(_PACKAGE_ROOT))
        ).resolve()
        runner = package_root / "stdio.js"
        if not runner.is_file():
            raise RuntimeError(
                f"WASM bridge runner not found at {runner}; build the package first"
            )

        try:
            self.process = subprocess.Popen(
                [node, str(runner)],
                cwd=str(package_root),
                stdin=subprocess.PIPE,
                stdout=subprocess.PIPE,
                stderr=subprocess.DEVNULL,
                text=True,
                bufsize=1,
            )
        except OSError as error:
            raise RuntimeError(f"Unable to start Node.js WASM bridge: {error}") from error

        self.lock = threading.Lock()

    @classmethod
    def instance(cls) -> "_WasmNodeBridge":
        with cls._instance_lock:
            if cls._instance is None:
                cls._instance = cls()
            return cls._instance

    def execute(self, request: Dict[str, Any]) -> Any:
        encoded = json.dumps(request, separators=(",", ":"), allow_nan=False)
        with self.lock:
            if self.process.poll() is not None:
                raise RuntimeError("Node.js WASM bridge exited unexpectedly")
            assert self.process.stdin is not None
            assert self.process.stdout is not None
            self.process.stdin.write(encoded + "\n")
            self.process.stdin.flush()
            response_line = self.process.stdout.readline()

        if not response_line:
            raise RuntimeError("Node.js WASM bridge closed its output")
        response = json.loads(response_line)
        if not response.get("ok"):
            message = response.get("error", "WASM execution failed")
            if isinstance(message, str) and message.startswith("DataWeave parse error:"):
                parse_message = message.removeprefix("DataWeave parse error:").strip()
                if parse_message.startswith("Failed to parse input as "):
                    raise DataWeaveEvaluationError(parse_message)
                line, column = _parse_error_location(request)
                if line is not None and column is not None:
                    parse_message = f"{parse_message} (line: {line}, column: {column})"
                raise parser.ParseError(parse_message, line, column)
            raise DataWeaveEvaluationError(message)
        return _decode_wasm_value(response.get("result"))

    def close(self) -> None:
        process = self.process
        if process.poll() is None:
            process.terminate()
            try:
                process.wait(timeout=1)
            except subprocess.TimeoutExpired:
                process.kill()


@atexit.register
def _close_wasm_bridge() -> None:
    bridge = _WasmNodeBridge._instance
    if bridge is not None:
        bridge.close()


class WasmDataWeaveRuntime:
    """DataWeave runtime backed by the Rust evaluator compiled to WASM."""

    def __init__(self, *, enable_module_imports: bool = True) -> None:
        self._enable_module_imports = enable_module_imports
        self._bridge = _WasmNodeBridge.instance()

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
        request: Dict[str, Any] = {
            "script": script_source,
            "payload": _encode_wasm_value(payload),
            "vars": _encode_wasm_value(vars or {}),
            "render_output": render_output,
        }
        if payload_format is not None:
            request["payload_format"] = payload_format
        if payload_format_options is not None:
            request["payload_format_options"] = _encode_wasm_value(
                payload_format_options
            )
        return self._bridge.execute(request)

    def capabilities(self) -> list[str]:
        return [
            "rust-core-evaluator",
            "wasm",
            "node-bridge",
        ]

    def last_execution_engine(self) -> str:
        return "rust-core"

    def infer_type_descriptor(
        self,
        script_source: str,
        payload: Any = None,
        vars: Optional[Dict[str, Any]] = None,
    ) -> Any:
        return self._bridge.execute(
            {
                "operation": "analyze",
                "expression": script_source,
                "payload": _encode_wasm_value(payload),
                "vars": _encode_wasm_value(vars or {}),
            }
        )["inferredType"]

    def execute_smoke_json(self, script_source: str, payload_json: str) -> str:
        result = self.execute(
            script_source,
            json.loads(payload_json),
            render_output=False,
        )
        return json.dumps(result, default=json_default, separators=(",", ":"))


def _encode_wasm_value(value: Any) -> Any:
    encoded = json.dumps(value, default=json_default, allow_nan=False)
    return json.loads(encoded)


def _decode_wasm_value(value: Any) -> Any:
    if isinstance(value, list):
        return [_decode_wasm_value(item) for item in value]
    if not isinstance(value, dict):
        return value

    if set(value) == {"__dwpy_binary"}:
        return bytes(value["__dwpy_binary"])
    if set(value) == {"__dwpy_nonfinite"}:
        return {
            "nan": float("nan"),
            "inf": float("inf"),
            "-inf": float("-inf"),
        }[value["__dwpy_nonfinite"]]
    if "__dwpy_temporal" in value:
        kind = value["__dwpy_temporal"]
        raw = value["value"]
        if kind == "datetime":
            return datetime.fromisoformat(raw.replace("Z", "+00:00"))
        if kind == "date":
            return date.fromisoformat(raw)
        if kind == "time":
            return time.fromisoformat(raw.replace("Z", "+00:00"))
    if "__dwpy_period" in value:
        return value.get("text", "")
    if set(value) == {"__dwpy_object_pairs"}:
        result = DWObject()
        for pair in value["__dwpy_object_pairs"]:
            result.add(pair["key"], _decode_wasm_value(pair.get("value")))
        return result
    return {key: _decode_wasm_value(item) for key, item in value.items()}


def _parse_error_location(request: Dict[str, Any]) -> tuple[Optional[int], Optional[int]]:
    source = request.get("script") or request.get("expression")
    if not isinstance(source, str):
        return None, None

    for line_number, line in enumerate(source.splitlines(), start=1):
        duplicate_comma = line.find(",,")
        if duplicate_comma >= 0:
            return line_number, duplicate_comma + 2

    return None, None
