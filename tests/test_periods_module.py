import json
from datetime import UTC, date, datetime, time

from dwpy import builtins
from dwpy.runtime import DataWeaveRuntime


def test_periods_module_exports_all_documented_functions():
    documented = {
        "between",
        "days",
        "duration",
        "hours",
        "minutes",
        "months",
        "period",
        "seconds",
        "years",
    }
    exports = builtins.resolve_module_exports("dw::core::Periods")
    assert documented.issubset(exports.keys())
    assert all(callable(exports[name]) for name in documented)


def test_periods_functions_match_documented_examples():
    assert str(builtins.builtin_between(date(2010, 12, 12), date(2010, 12, 10))) == "P2D"
    assert str(builtins.builtin_between(date(2011, 12, 11), date(2010, 11, 10))) == "P1Y1M1D"
    assert str(builtins.builtin_between(date(2020, 2, 29), date(2020, 3, 30))) == "P-1M-1D"

    assert str(builtins.builtin_period({})) == "P0D"
    assert str(builtins.builtin_period({"years": 4, "months": 11, "days": 28})) == "P4Y11M28D"
    assert builtins.builtin_period({"years": 4, "months": 11, "days": 28}).months == 11

    assert str(builtins.builtin_duration({})) == "PT0S"
    assert str(builtins.builtin_duration({"days": 4, "hours": 11, "minutes": 28})) == "PT107H28M"
    assert builtins.builtin_duration({"days": 4, "hours": 11, "minutes": 28}).hours == 11.0

    assert str(builtins.builtin_years(4)) == "P4Y"
    assert str(builtins.builtin_months(4)) == "P4M"
    assert str(builtins.builtin_days(4)) == "P4D"
    assert str(builtins.builtin_days(4.555)) == "PT109H19M12S"
    assert str(builtins.builtin_hours(4.555)) == "PT4H33M18S"
    assert str(builtins.builtin_minutes(4.555)) == "PT4M33.3S"
    assert str(builtins.builtin_seconds(4.555)) == "PT4.555S"


def test_periods_runtime_integration_and_arithmetic():
    runtime = DataWeaveRuntime()
    payload = {
        "dt": datetime(2020, 10, 5, 20, 22, 34, 385000, tzinfo=UTC),
        "d": date(2020, 10, 5),
        "t": time(20, 22),
        "startDate": date(2010, 11, 10),
        "endDate": date(2011, 12, 11),
        "midnight": datetime(2020, 10, 5, 0, 0, 0, tzinfo=UTC),
    }
    script = """%dw 2.0
output application/python
import * from dw::core::Periods
---
{
  nextYear: payload.dt + years(1),
  nextMonth: payload.dt + months(1),
  tomorrow: payload.dt + days(1),
  decimalDaysPlusQuarter: payload.midnight + days(0.25),
  nextHour: payload.dt + hours(1),
  nextMinute: payload.dt + minutes(1),
  nextSecond: payload.dt + seconds(1),
  previousHour: payload.dt - hours(1),
  dayAfterDate: payload.d + period({days: 1}),
  yearMonthDayAfterDate: payload.d + period({years: 1, months: 1, days: 1}),
  threeHoursLater: payload.t + hours(3),
  betweenValue: between(payload.endDate, payload.startDate),
  addNegativeYearValue: years(0 - 1) + years(2),
  addNegativeDurationValue: duration({minutes: 1}) + duration({seconds: (0 - 1)}),
  monthsFromPeriod: period({years: 4, months: 11, days: 28}).months,
  hoursFromDuration: duration({days: 4, hours: 11, minutes: 28}).hours
}
"""
    result = runtime.execute(script, payload=payload, render_output=False)

    assert result["nextYear"] == datetime(2021, 10, 5, 20, 22, 34, 385000, tzinfo=UTC)
    assert result["nextMonth"] == datetime(2020, 11, 5, 20, 22, 34, 385000, tzinfo=UTC)
    assert result["tomorrow"] == datetime(2020, 10, 6, 20, 22, 34, 385000, tzinfo=UTC)
    assert result["decimalDaysPlusQuarter"] == datetime(2020, 10, 5, 6, 0, 0, tzinfo=UTC)
    assert result["nextHour"] == datetime(2020, 10, 5, 21, 22, 34, 385000, tzinfo=UTC)
    assert result["nextMinute"] == datetime(2020, 10, 5, 20, 23, 34, 385000, tzinfo=UTC)
    assert result["nextSecond"] == datetime(2020, 10, 5, 20, 22, 35, 385000, tzinfo=UTC)
    assert result["previousHour"] == datetime(2020, 10, 5, 19, 22, 34, 385000, tzinfo=UTC)
    assert result["dayAfterDate"] == date(2020, 10, 6)
    assert result["yearMonthDayAfterDate"] == date(2021, 11, 6)
    assert result["threeHoursLater"] == time(23, 22)
    assert str(result["betweenValue"]) == "P1Y1M1D"
    assert result["addNegativeYearValue"] == 12
    assert result["addNegativeDurationValue"] == 59
    assert result["monthsFromPeriod"] == 11
    assert result["hoursFromDuration"] == 11.0


def test_periods_serialize_in_json_output():
    runtime = DataWeaveRuntime()
    payload = {"dt": datetime(2020, 10, 5, 20, 22, 34, 385000, tzinfo=UTC)}
    script = """%dw 2.0
output application/json
import * from dw::core::Periods
---
{
  periodValue: years(4),
  nextHour: payload.dt + hours(1)
}
"""
    raw = runtime.execute(script, payload=payload)
    parsed = json.loads(raw)
    assert parsed["periodValue"] == "P4Y"
    assert parsed["nextHour"] == "2020-10-05T21:22:34.385000Z"


def test_between_with_pipe_date_literals_matches_docs():
    runtime = DataWeaveRuntime()
    script = """%dw 2.0
import * from dw::core::Periods
output application/json
---
{
   a: between(|2010-12-12|,|2010-12-10|),
   b: between(|2011-12-11|,|2010-11-10|),
   c: between(|2020-02-29|,|2020-03-30|)
}
"""
    raw = runtime.execute(script, payload={})
    parsed = json.loads(raw)
    assert parsed == {
        "a": "P2D",
        "b": "P1Y1M1D",
        "c": "P-1M-1D",
    }
