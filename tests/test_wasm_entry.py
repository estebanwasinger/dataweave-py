from __future__ import annotations

from dwpy.wasm_entry import complete_dataweave, run_dataweave


def test_run_dataweave_executes_core_script() -> None:
    script = """%dw 2.0
output application/python
---
{
  id: payload.id,
  upperName: upper(payload.name)
}
"""
    result = run_dataweave(script=script, payload={"id": 7, "name": "mule"})
    assert result == {"id": 7, "upperName": "MULE"}


def test_run_dataweave_serializes_temporal_and_binary_values() -> None:
    script = """%dw 2.0
import * from dw::Crypto
output application/python
---
{
  asDate: |2024-01-02|,
  asTime: |10:20:30Z|,
  asDateTime: |2024-01-02T10:20:30Z|,
  digest: hashWith("hello" as Binary, "SHA-1")
}
"""
    result = run_dataweave(script=script, payload={})

    assert result["asDate"] == "2024-01-02"
    assert result["asTime"] == "10:20:30Z"
    assert result["asDateTime"] == "2024-01-02T10:20:30Z"
    assert result["digest"] == {
        "__type__": "bytes",
        "base64": "qvTGHdzF6KLavt4PO0gs2a6pQ00=",
    }


def test_run_dataweave_serializes_non_finite_floats() -> None:
    script = """%dw 2.0
import * from dw::util::Math
output application/python
---
{
  value: asin(2)
}
"""
    result = run_dataweave(script=script, payload={})
    assert result["value"] == "nan"


def test_complete_dataweave_returns_dynamic_items() -> None:
    script = """%dw 2.0
---
payload.user.
"""
    result = complete_dataweave(
        script=script,
        line=2,
        column=len("payload.user."),
        payload={"user": {"name": "mule", "id": 7}},
        vars={},
    )
    assert "items" in result
    labels = {item["label"] for item in result["items"]}
    assert "name" in labels
    assert "id" in labels
