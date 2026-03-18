from __future__ import annotations

import re
from pathlib import Path

from dwpy.runtime import DataWeaveRuntime


def _all_module_native_ids() -> set[str]:
    pattern = re.compile(r'native\((["\'])([^"\']+)\1\)')
    ids: set[str] = set()
    for module_path in (Path(__file__).resolve().parent.parent / "dwpy" / "modules").rglob("*.dwl"):
        content = module_path.read_text()
        ids.update(match.group(2) for match in pattern.finditer(content))
    return ids


def test_native_registry_covers_all_module_native_identifiers() -> None:
    runtime = DataWeaveRuntime()
    module_ids = _all_module_native_ids()
    missing = sorted(module_ids.difference(runtime._native_functions.keys()))  # noqa: SLF001
    assert missing == []


def test_math_and_numbers_native_functions_are_callable_through_imports() -> None:
    runtime = DataWeaveRuntime()
    script = """%dw 2.0
import * from dw::util::Math
import * from dw::core::Numbers
output application/json
---
{
  sinZero: sin(0),
  cosZero: cos(0),
  tanZero: tan(0),
  fromHex: fromRadixNumber("ff", 16),
  toHex: toRadixNumber(255, 16)
}
"""
    result = runtime.execute(script, payload={}, render_output=False)
    assert result == {
        "sinZero": 0.0,
        "cosZero": 1.0,
        "tanZero": 0.0,
        "fromHex": 255,
        "toHex": "ff",
    }


def test_mime_and_core_native_functions_execute() -> None:
    runtime = DataWeaveRuntime()
    script = """%dw 2.0
import * from dw::module::Mime
import read, write, mapObject from dw::Core
output application/json
---
{
  mime: fromString("application/json"),
  mimeText: toString({type: "application", subtype: "json", parameters: {}}),
  handled: isHandledBy({type: "application", subtype: "*", parameters: {}}, {type: "application", subtype: "json", parameters: {}}),
  parsed: read("{\\"a\\": 1}", "application/json").a,
  written: write({a: 1}, "application/json"),
  mapped: mapObject({a: 1, b: 2}, (value, key, index) -> {(upper(key)): value + index})
}
"""
    result = runtime.execute(script, payload={}, render_output=False)
    assert result["mime"]["success"] is True
    assert result["mime"]["result"] == {
        "type": "application",
        "subtype": "json",
        "parameters": {},
    }
    assert result["mimeText"] == "application/json"
    assert result["handled"] is True
    assert result["parsed"] == 1
    assert result["written"] == '{"a":1}'
    assert result["mapped"] == {"A": 1, "B": 3}


def test_runtime_try_and_fail_are_available_via_native_imports() -> None:
    runtime = DataWeaveRuntime()
    script = """%dw 2.0
import try, fail from dw::Runtime
output application/json
---
{
  ok: try(() -> "ok"),
  err: try(() -> fail("boom"))
}
"""
    result = runtime.execute(script, payload={}, render_output=False)
    assert result["ok"] == {"success": True, "result": "ok"}
    assert result["err"]["success"] is False
    assert result["err"]["error"]["kind"] == "DataWeaveEvaluationError"
    assert "boom" in result["err"]["error"]["message"]


def test_crypto_hash_with_supports_requested_algorithms() -> None:
    runtime = DataWeaveRuntime()
    script = """%dw 2.0
import * from dw::Crypto
output application/json
---
{
  md2: hashWith("hello" as Binary, "MD2"),
  md5: hashWith("hello" as Binary, "MD5"),
  sha1: hashWith("hello" as Binary, "SHA-1"),
  sha256: hashWith("hello" as Binary, "SHA-256"),
  sha384: hashWith("hello" as Binary, "SHA-384"),
  sha512: hashWith("hello" as Binary, "SHA-512")
}
"""
    result = runtime.execute(script, payload={}, render_output=False)
    assert result["md2"].hex() == "a9046c73e00331af68917d3804f70655"
    assert result["md5"].hex() == "5d41402abc4b2a76b9719d911017c592"
    assert result["sha1"].hex() == "aaf4c61ddcc5e8a2dabede0f3b482cd9aea9434d"
    assert result["sha256"].hex() == "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"
    assert (
        result["sha384"].hex()
        == "59e1748777448c69de6b800d7a33bbfb9ff1b463e44354c3553bcdb9c666fa90125a3c79f90397bdf5f6a13de828684f"
    )
    assert (
        result["sha512"].hex()
        == "9b71d224bd62f3785d96d46ad3ea3d73319bfbc2890caadae2dff72519673ca72323c3d99ba5c11d7c7acc6e14b8c5da0c4663475c2e5c3adef46f73bcdec043"
    )
