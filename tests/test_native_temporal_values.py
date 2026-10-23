from datetime import date, datetime, time, timedelta, timezone
import json

import pytest

from dwpy.builtins import DWPeriod
from dwpy.runtime import DataWeaveRuntime


@pytest.fixture(params=["rust", "wasm"])
def runtime(request, monkeypatch):
    monkeypatch.delenv("DWPY_TEST_BACKEND", raising=False)
    monkeypatch.delenv("DWPY_BACKEND", raising=False)
    return DataWeaveRuntime(backend=request.param)


def test_native_temporals_cross_the_host_boundary_as_temporal_scalars(runtime):
    value = runtime.execute('''{
      date: "2026-10-26" as Date + |P3D|,
      datetime: |2026-10-26T12:34:56.123456Z|,
      local: |2026-10-26T12:34:56|,
      time: |12:34:56+03:00|,
      localTime: |12:34:56|,
      current: now()
    }''', {}, render_output=False)
    assert value["date"] == date(2026, 10, 29)
    assert value["datetime"] == datetime(2026, 10, 26, 12, 34, 56, 123456, tzinfo=timezone.utc)
    assert value["local"] == datetime(2026, 10, 26, 12, 34, 56)
    assert value["time"] == time(12, 34, 56, tzinfo=timezone(timedelta(hours=3)))
    assert value["localTime"] == time(12, 34, 56)
    assert isinstance(value["current"], datetime)


@pytest.mark.parametrize("root", ["payload", "vars.value", "attributes.value"])
def test_host_dates_and_periods_become_native_values(runtime, root):
    value = {"day": date(2026, 10, 26), "period": DWPeriod(days=3, date_based=True)}
    result = runtime.execute(
        f"{root}.day + {root}.period", value,
        vars={"value": value}, attributes={"value": value}, render_output=False,
    )
    assert result == date(2026, 10, 29)


@pytest.mark.parametrize("marker", [
    {"__dwpy_temporal": "date", "value": "2026-10-29"},
    {"__dwpy_period": True, "text": "P3D", "days": 3},
    {"__dwpy_object": {"__dwpy_temporal": "date", "value": "2026-10-29"}},
])
def test_marker_shaped_user_objects_round_trip_without_becoming_temporals(runtime, marker):
    assert runtime.execute("payload", marker, render_output=False) == marker
    assert runtime.execute("vars.value", {}, vars={"value": marker}, render_output=False) == marker
    assert runtime.execute("attributes.value", {}, attributes={"value": marker}, render_output=False) == marker
    assert runtime.execute("payload is Object", marker, render_output=False) is True
    assert runtime.execute("payload is Date", marker, render_output=False) is False
    assert runtime.execute("payload", json.dumps(marker), payload_format="json", render_output=False) == marker
    assert runtime.execute('read(payload, "json")', json.dumps(marker), render_output=False) == marker


def test_iso_looking_strings_stay_strings(runtime):
    assert runtime.execute('"2026-10-29"', {}, render_output=False) == "2026-10-29"
    assert runtime.execute('typeOf("2026-10-29")', {}, render_output=False) == "String"
    assert runtime.execute('"2026-10-29" ++ "T12:34:56Z"', {}, render_output=False) == "2026-10-29T12:34:56Z"


@pytest.mark.parametrize("source, expected", [
    ("Z", "Z"), ("+00:00", "Z"), ("-03:00", "-03:00"), ("+05:30", "+05:30"),
])
def test_timezone_string_coercion_and_output(runtime, source, expected):
    script = f'''%dw 2.0
import * from dw::util::Coercions
output application/json
---
{{zone: |{source}|, text: toString(|{source}|), coerced: |{source}| as String}}'''
    assert json.loads(runtime.execute(script, {})) == {
        "zone": expected, "text": expected, "coerced": expected,
    }
    assert runtime.execute(f"output text\n---\n|{source}|", {}) == expected


def test_nested_output_and_nanosecond_arithmetic_do_not_leak_markers(runtime):
    script = '''%dw 2.0
output application/json
---
{
    date: "2026-10-26" as Date + |P3D|,
    nested: [|2026-10-26T12:34:56.123456789Z| + |PT1S|],
    nanos: |2026-10-26T12:34:56.123456789Z|.nanoseconds
}'''
    assert json.loads(runtime.execute(script, {})) == {
        "date": "2026-10-29",
        "nested": ["2026-10-26T12:34:57.123456789Z"],
        "nanos": 123456789,
    }


def test_native_type_descriptors_include_host_dates(runtime):
    descriptor = runtime._rust_runtime.infer_type_descriptor("payload", payload=date(2026, 10, 26))
    assert descriptor["kind"] == "Date"


@pytest.mark.parametrize("backend", ["python", "rust"])
@pytest.mark.parametrize("expression", [
    '|2026-10-26| + |P3D|',
    '"2026-10-26" as Date + |P3D|',
])
def test_public_type_inference_preserves_temporal_types(monkeypatch, backend, expression):
    from dwpy.type_inference import infer_script_type
    from dwpy.typesystem import DATE

    monkeypatch.setenv("DWPY_TEST_BACKEND", backend)
    assert infer_script_type(expression) == DATE
