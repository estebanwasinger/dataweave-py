use crate::value::{DwMap as Map, DwValue as Value};

use crate::{as_dataweave_string, numeric_value, DwError};

use crate::value::DwPeriod as Period;
use chrono::{Datelike, Months, NaiveDate, TimeDelta, Timelike};

pub(crate) fn period_function(function_name: &str, value: &Value) -> Result<Value, DwError> {
    let number = numeric_value(value)?;
    match function_name {
        "years" => Ok(period_value(Period {
            years: number as i64,
            months: 0,
            days: 0.0,
            hours: 0.0,
            minutes: 0.0,
            seconds: 0.0,
            date_based: true,
        })),
        "months" => Ok(period_value(Period {
            years: 0,
            months: number as i64,
            days: 0.0,
            hours: 0.0,
            minutes: 0.0,
            seconds: 0.0,
            date_based: true,
        })),
        "days" if number.fract() == 0.0 => Ok(period_value(Period {
            years: 0,
            months: 0,
            days: number,
            hours: 0.0,
            minutes: 0.0,
            seconds: 0.0,
            date_based: true,
        })),
        "days" => Ok(period_value(Period {
            years: 0,
            months: 0,
            days: 0.0,
            hours: number * 24.0,
            minutes: 0.0,
            seconds: 0.0,
            date_based: false,
        })),
        "hours" => Ok(period_value(Period {
            years: 0,
            months: 0,
            days: 0.0,
            hours: number,
            minutes: 0.0,
            seconds: 0.0,
            date_based: false,
        })),
        "minutes" => Ok(period_value(Period {
            years: 0,
            months: 0,
            days: 0.0,
            hours: 0.0,
            minutes: number,
            seconds: 0.0,
            date_based: false,
        })),
        "seconds" => Ok(period_value(Period {
            years: 0,
            months: 0,
            days: 0.0,
            hours: 0.0,
            minutes: 0.0,
            seconds: number,
            date_based: false,
        })),
        _ => Err(DwError::UnsupportedFeature(function_name.to_string())),
    }
}

pub(crate) fn period_literal(source: &str) -> Option<Value> {
    parse_period_literal(source).map(period_value)
}

pub(crate) fn period_from_object(value: &Value, date_based: bool) -> Result<Value, DwError> {
    let map = value.as_object().cloned().unwrap_or_default();
    let number = |name: &str| -> Result<f64, DwError> {
        map.get(name)
            .map(numeric_value)
            .transpose()
            .map(|value| value.unwrap_or(0.0))
    };
    Ok(period_value(if date_based {
        Period {
            years: number("years")? as i64,
            months: number("months")? as i64,
            days: number("days")?,
            hours: 0.0,
            minutes: 0.0,
            seconds: 0.0,
            date_based: true,
        }
    } else {
        Period {
            years: 0,
            months: 0,
            days: number("days")?,
            hours: number("hours")?,
            minutes: number("minutes")?,
            seconds: number("seconds")?,
            date_based: false,
        }
    }))
}

pub(crate) fn between_dates(end: &Value, start: &Value) -> Result<Value, DwError> {
    let end = date_parts(end)?;
    let start = date_parts(start)?;
    Ok(period_value(period_between(end, start)))
}

pub(crate) fn days_between_dates(start: &Value, end: &Value) -> Result<Value, DwError> {
    let start = date_parts(start)?;
    let end = date_parts(end)?;
    Ok(Value::Number(
        (unix_days_from_civil(end.0, end.1, end.2)
            - unix_days_from_civil(start.0, start.1, start.2))
        .into(),
    ))
}

pub(crate) fn is_leap_year_value(value: &Value) -> Result<Value, DwError> {
    let (year, _, _) = date_parts(value)?;
    Ok(Value::Bool(is_leap_year(year)))
}

pub(crate) fn at_beginning_of(function_name: &str, value: &Value) -> Result<Value, DwError> {
    let text = special_string_value(value).unwrap_or_else(|| as_dataweave_string(value));
    let temporal = ParsedTemporalInput::parse(&text)
        .ok_or_else(|| DwError::UnsupportedFeature(format!("{function_name}({text})")))?;
    let rendered = match function_name {
        "atBeginningOfDay" => temporal.render_with_time(temporal.date, "00:00:00", false)?,
        "atBeginningOfHour" => temporal.render_at_beginning_of_hour()?,
        "atBeginningOfMonth" => {
            let date = (temporal.date.0, temporal.date.1, 1);
            temporal.render_with_time(date, "00:00:00", false)?
        }
        "atBeginningOfWeek" => {
            let days_since_sunday =
                day_of_week(temporal.date.0, temporal.date.1, temporal.date.2) % 7;
            let date = add_days(temporal.date, -days_since_sunday);
            temporal.render_with_time(date, "00:00:00", false)?
        }
        "atBeginningOfYear" => {
            let date = (temporal.date.0, 1, 1);
            temporal.render_with_time(date, "00:00:00", temporal.has_zoned_datetime())?
        }
        _ => return Err(DwError::UnsupportedFeature(function_name.to_string())),
    };
    temporal_value(temporal.kind(), rendered)
}

pub(crate) fn temporal_constructor(function_name: &str, value: &Value) -> Result<Value, DwError> {
    let map = value
        .as_object()
        .ok_or_else(|| DwError::UnsupportedFeature(format!("{function_name}({value:?})")))?;
    let year = object_i64(map, "year")?;
    let month = object_i64(map, "month")?;
    let day = object_i64(map, "day")?;
    let hour = object_i64(map, "hour").unwrap_or(0);
    let minutes = object_i64(map, "minutes").unwrap_or(0);
    let seconds = object_i64(map, "seconds").unwrap_or(0);
    let timezone = map
        .get("timeZone")
        .map(|value| special_string_value(value).unwrap_or_else(|| as_dataweave_string(value)))
        .unwrap_or_default();
    match function_name {
        "date" => temporal_value("date", format!("{year:04}-{month:02}-{day:02}")),
        "dateTime" => temporal_value(
            "datetime",
            format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minutes:02}:{seconds:02}{timezone}"),
        ),
        "localDateTime" => temporal_value(
            "datetime",
            format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minutes:02}:{seconds:02}"),
        ),
        "localTime" => temporal_value("time", format!("{hour:02}:{minutes:02}:{seconds:02}")),
        "time" => temporal_value(
            "time",
            format!("{hour:02}:{minutes:02}:{seconds:02}{timezone}"),
        ),
        _ => Err(DwError::UnsupportedFeature(function_name.to_string())),
    }
}

pub(crate) fn evaluate_period_additive(
    left: &Value,
    operator: &str,
    right: &Value,
) -> Result<Option<Value>, DwError> {
    if let (Some(left), Some(right)) = (period_from_value(left), period_from_value(right)) {
        return Ok(Some(combine_periods(left, right, operator)));
    }
    if let (Some(temporal), Some(period)) = (temporal_from_value(left), period_from_value(right)) {
        let period = if operator == "-" {
            negate_period(period)
        } else {
            period
        };
        return Ok(Some(add_period_to_temporal(&temporal, period)?));
    }
    if operator == "-" {
        if let (Some(left_temporal), Some(right_temporal)) =
            (temporal_from_value(left), temporal_from_value(right))
        {
            return Ok(Some(period_value(period_between_temporals(
                &left_temporal,
                &right_temporal,
            )?)));
        }
        if let (Some(period), Some(temporal)) =
            (period_from_value(left), temporal_from_value(right))
        {
            return Ok(Some(add_period_to_temporal(
                &temporal,
                negate_period(period),
            )?));
        }
    }
    if operator == "+" {
        if let (Some(period), Some(temporal)) =
            (period_from_value(left), temporal_from_value(right))
        {
            return Ok(Some(add_period_to_temporal(&temporal, period)?));
        }
    }
    Ok(None)
}

pub(crate) fn special_string_value(value: &Value) -> Option<String> {
    if let Some(period) = period_from_value(value) {
        return Some(period_to_string(period));
    }
    value.temporal_text()
}

pub(crate) fn temporal_field_value(value: &Value, field: &str) -> Option<Value> {
    if let Some(value) = crate::selectors::unwrap_metadata_value(value) {
        return temporal_field_value(&value, field);
    }
    if let Some(period) = period_from_value(value) {
        return period_field_value(period, field);
    }
    let (date, time, offset) = match value {
        Value::Date(value) => (Some(*value), None, 0),
        Value::LocalDateTime(value) => (Some(value.value.date()), Some(value.value.time()), 0),
        Value::DateTime(value) => (
            Some(value.value.date_naive()),
            Some(value.value.time()),
            value.value.offset().local_minus_utc(),
        ),
        Value::LocalTime(value) => (None, Some(value.value), 0),
        Value::Time(value) => (None, Some(value.value), value.offset.local_minus_utc()),
        _ => return None,
    };
    let number = match field {
        "year" => date?.year() as i64,
        "month" => date?.month() as i64,
        "day" => date?.day() as i64,
        "hour" => time?.hour() as i64,
        "minutes" => time?.minute() as i64,
        "seconds" => time?.second() as i64,
        "milliseconds" => time?.nanosecond() as i64 / 1_000_000,
        "nanoseconds" => time?.nanosecond() as i64,
        "quarter" => (date?.month() as i64 - 1) / 3 + 1,
        "dayOfWeek" => date?.weekday().number_from_monday() as i64,
        "dayOfYear" => date?.ordinal() as i64,
        "offsetSeconds" => offset as i64,
        _ => return None,
    };
    Some(Value::Number(number.into()))
}

fn period_field_value(period: Period, field: &str) -> Option<Value> {
    let number = match field {
        "years" => return Some(Value::Number(period.years.into())),
        "months" => return Some(Value::Number(period.months.into())),
        "days" => period.days,
        "hours" => period.hours,
        "minutes" => period.minutes,
        "seconds" | "secs" => period.seconds,
        _ => return None,
    };
    Some(number_value(number))
}

pub(crate) fn period_or_temporal_to_number(value: &Value, unit: &str) -> Option<Value> {
    if let Some(period) = period_from_value(value).or_else(|| parse_period_literal(value.as_str()?))
    {
        return match unit {
            "years" => Some(Value::Number(
                (period.years + period.months.div_euclid(12)).into(),
            )),
            "months" => Some(Value::Number((period.years * 12 + period.months).into())),
            _ => {
                let factor = match unit {
                    "nanos" | "nanoseconds" => 1_000_000_000.0,
                    "millis" | "milliseconds" => 1_000.0,
                    "seconds" => 1.0,
                    "minutes" => 1.0 / 60.0,
                    "hours" => 1.0 / 3600.0,
                    "days" => 1.0 / 86_400.0,
                    _ => return None,
                };
                Some(number_value((total_seconds(period) * factor).trunc()))
            }
        };
    }
    let native = temporal_from_value(value)?;
    let nanos = temporal_epoch_nanos(&native)?;
    let divisor = match unit {
        "nanos" | "nanoseconds" => 1,
        "millis" | "milliseconds" => 1_000_000,
        "seconds" => 1_000_000_000,
        "minutes" => 60_000_000_000,
        "hours" => 3_600_000_000_000,
        "days" => 86_400_000_000_000,
        _ => return None,
    };
    (nanos / divisor)
        .to_string()
        .parse::<serde_json::Number>()
        .ok()
        .map(Value::Number)
}

fn parse_period_literal(source: &str) -> Option<Period> {
    let source = source.strip_prefix('P')?;
    let mut date_part = source;
    let mut time_part = "";
    if let Some((date, time)) = source.split_once('T') {
        date_part = date;
        time_part = time;
    }
    let years = parse_period_component(date_part, 'Y')?.unwrap_or(0.0) as i64;
    let months = parse_period_component(date_part, 'M')?.unwrap_or(0.0) as i64;
    let days = parse_period_component(date_part, 'D')?.unwrap_or(0.0);
    let hours = parse_period_component(time_part, 'H')?.unwrap_or(0.0);
    let minutes = parse_period_component(time_part, 'M')?.unwrap_or(0.0);
    let seconds = parse_period_component(time_part, 'S')?.unwrap_or(0.0);
    Some(Period {
        years,
        months,
        days,
        hours,
        minutes,
        seconds,
        date_based: time_part.is_empty(),
    })
}

fn parse_period_component(source: &str, designator: char) -> Option<Option<f64>> {
    let Some(index) = source.find(designator) else {
        return Some(None);
    };
    let digits_start = source[..index]
        .char_indices()
        .rev()
        .find(|(_, ch)| !(ch.is_ascii_digit() || *ch == '.' || *ch == '-'))
        .map(|(index, ch)| index + ch.len_utf8())
        .unwrap_or(0);
    source[digits_start..index].parse::<f64>().ok().map(Some)
}

pub(crate) fn concatenate_temporals(left: &Value, right: &Value) -> Option<Value> {
    if left.temporal_type_name().is_none() || right.temporal_type_name().is_none() {
        return None;
    }
    let left = TemporalPart::parse_value(left)?;
    let right = TemporalPart::parse_value(right)?;
    let combined = match (&left, &right) {
        (TemporalPart::Date(date), TemporalPart::Time(time))
        | (TemporalPart::Time(time), TemporalPart::Date(date)) => {
            format!("{date}T{}", normalize_time(time, false))
        }
        (TemporalPart::Date(date), TemporalPart::Zone(zone))
        | (TemporalPart::Zone(zone), TemporalPart::Date(date)) => {
            format!("{date}T00:00:00{zone}")
        }
        (TemporalPart::Zone(zone), TemporalPart::DateTime(datetime)) => {
            format!("{datetime}{zone}")
        }
        (TemporalPart::DateTime(datetime), TemporalPart::Zone(zone)) => {
            format!("{datetime}{zone}")
        }
        (TemporalPart::Time(time), TemporalPart::Zone(zone))
        | (TemporalPart::Zone(zone), TemporalPart::Time(time)) => {
            format!("{}{}", normalize_time(time, true), zone)
        }
        _ => return None,
    };
    Value::parse_temporal(&combined).ok()
}

enum TemporalPart {
    Date(String),
    Time(String),
    DateTime(String),
    Zone(String),
}

impl TemporalPart {
    fn parse_value(value: &Value) -> Option<Self> {
        let text = special_string_value(value).unwrap_or_else(|| as_dataweave_string(value));
        Self::parse(&text)
    }

    fn parse(text: &str) -> Option<Self> {
        if is_timezone(text) {
            return Some(Self::Zone(text.to_string()));
        }
        if text.contains('T') && parse_datetime_parts(text).is_some() {
            return Some(Self::DateTime(text.to_string()));
        }
        if parse_date(text).is_some() {
            return Some(Self::Date(text.to_string()));
        }
        if is_time_like(text) {
            return Some(Self::Time(text.to_string()));
        }
        None
    }
}

fn is_time_like(text: &str) -> bool {
    let (time, zone) = split_time_zone(text);
    if !zone.is_empty() && !is_timezone(zone) {
        return false;
    }
    let mut parts = time.split(':');
    let Some(hour) = parts.next() else {
        return false;
    };
    let Some(minute) = parts.next() else {
        return false;
    };
    if hour.len() != 2 || minute.len() != 2 {
        return false;
    }
    let Ok(hour) = hour.parse::<u8>() else {
        return false;
    };
    let Ok(minute) = minute.parse::<u8>() else {
        return false;
    };
    let Some(second) = parts.next() else {
        return parts.next().is_none() && hour <= 23 && minute <= 59;
    };
    let Ok(second) = second.parse::<f64>() else {
        return false;
    };
    parts.next().is_none() && hour <= 23 && minute <= 59 && (0.0..60.0).contains(&second)
}

fn is_timezone(text: &str) -> bool {
    if text == "Z" {
        return true;
    }
    let bytes = text.as_bytes();
    bytes.len() == 6
        && matches!(bytes[0], b'+' | b'-')
        && bytes[3] == b':'
        && bytes[1].is_ascii_digit()
        && bytes[2].is_ascii_digit()
        && bytes[4].is_ascii_digit()
        && bytes[5].is_ascii_digit()
}

fn normalize_time(time: &str, local_only: bool) -> String {
    let (time_without_zone, zone) = split_time_zone(time);
    let mut normalized = if time_without_zone.matches(':').count() == 1 {
        format!("{time_without_zone}:00")
    } else {
        time_without_zone.to_string()
    };
    if !local_only {
        normalized.push_str(zone);
    }
    normalized
}

fn split_time_zone(time: &str) -> (&str, &str) {
    if let Some(stripped) = time.strip_suffix('Z') {
        return (stripped, "Z");
    }
    if time.len() > 6 {
        let split = time.len() - 6;
        let suffix = &time[split..];
        if is_timezone(suffix) {
            return (&time[..split], suffix);
        }
    }
    (time, "")
}

#[derive(Clone, Copy)]
enum ParsedTemporalKind {
    Date,
    DateTime,
    Time,
}

struct ParsedTemporalInput {
    kind: ParsedTemporalKind,
    date: (i64, i64, i64),
    hour: Option<i64>,
    suffix: String,
}

impl ParsedTemporalInput {
    fn parse(value: &str) -> Option<Self> {
        if let Some((date_source, time_source)) = value.split_once('T') {
            let date = parse_date(date_source)?;
            let (time_source, suffix) = split_time_suffix(time_source);
            let hour = parse_hour(time_source)?;
            return Some(Self {
                kind: ParsedTemporalKind::DateTime,
                date,
                hour: Some(hour),
                suffix: suffix.to_string(),
            });
        }
        if let Some(date) = parse_date(value) {
            return Some(Self {
                kind: ParsedTemporalKind::Date,
                date,
                hour: None,
                suffix: String::new(),
            });
        }
        let (time_source, suffix) = split_time_suffix(value);
        let hour = parse_hour(time_source)?;
        Some(Self {
            kind: ParsedTemporalKind::Time,
            date: (1970, 1, 1),
            hour: Some(hour),
            suffix: suffix.to_string(),
        })
    }

    fn kind(&self) -> &str {
        match self.kind {
            ParsedTemporalKind::Date => "date",
            ParsedTemporalKind::DateTime => "datetime",
            ParsedTemporalKind::Time => "time",
        }
    }

    fn has_zoned_datetime(&self) -> bool {
        matches!(self.kind, ParsedTemporalKind::DateTime) && !self.suffix.is_empty()
    }

    fn render_at_beginning_of_hour(&self) -> Result<String, DwError> {
        let hour = self
            .hour
            .ok_or_else(|| DwError::UnsupportedFeature("atBeginningOfHour(date)".to_string()))?;
        let time = format!("{hour:02}:00:00");
        self.render_with_time(self.date, &time, false)
    }

    fn render_with_time(
        &self,
        date: (i64, i64, i64),
        time: &str,
        include_zero_millis: bool,
    ) -> Result<String, DwError> {
        let time = if include_zero_millis {
            format!("{time}.000")
        } else {
            time.to_string()
        };
        match self.kind {
            ParsedTemporalKind::Date => Ok(format!("{:04}-{:02}-{:02}", date.0, date.1, date.2)),
            ParsedTemporalKind::DateTime => Ok(format!(
                "{:04}-{:02}-{:02}T{}{}",
                date.0, date.1, date.2, time, self.suffix
            )),
            ParsedTemporalKind::Time => Ok(format!("{time}{}", self.suffix)),
        }
    }
}

fn object_i64(map: &Map<String, Value>, name: &str) -> Result<i64, DwError> {
    map.get(name)
        .map(numeric_value)
        .transpose()
        .map(|value| value.unwrap_or(0.0) as i64)
}

fn split_time_suffix(value: &str) -> (&str, &str) {
    if let Some(time) = value.strip_suffix('Z') {
        return (time, "Z");
    }
    for sign in ['+', '-'] {
        if let Some(index) = value.rfind(sign) {
            let suffix = &value[index..];
            if suffix.len() == 6 && suffix.as_bytes().get(3) == Some(&b':') {
                return (&value[..index], suffix);
            }
        }
    }
    (value, "")
}

fn parse_hour(value: &str) -> Option<i64> {
    value.split(':').next()?.parse().ok()
}

fn day_of_week(year: i64, month: i64, day: i64) -> i64 {
    (unix_days_from_civil(year, month, day) + 3).rem_euclid(7) + 1
}

fn period_value(period: Period) -> Value {
    Value::Period(period)
}

fn temporal_value(_kind: &str, value: String) -> Result<Value, DwError> {
    Value::parse_temporal(&value)
}

fn period_from_value(value: &Value) -> Option<Period> {
    if let Some(value) = crate::selectors::unwrap_metadata_value(value) {
        return period_from_value(&value);
    }
    match value {
        Value::Period(period) => Some(*period),
        _ => None,
    }
}

pub(crate) fn temporal_from_value(value: &Value) -> Option<Value> {
    if let Some(value) = crate::selectors::unwrap_metadata_value(value) {
        return temporal_from_value(&value);
    }
    match value {
        Value::Date(_)
        | Value::LocalDateTime(_)
        | Value::DateTime(_)
        | Value::LocalTime(_)
        | Value::Time(_) => Some(value.clone()),
        _ => None,
    }
}

fn combine_periods(left: Period, right: Period, operator: &str) -> Value {
    let right = if operator == "-" {
        negate_period(right)
    } else {
        right
    };
    if left.date_based && right.date_based {
        return Value::Number(
            (left.years * 12 + left.months + right.years * 12 + right.months).into(),
        );
    }
    number_value(total_seconds(left) + total_seconds(right))
}

fn period_between_temporals(end: &Value, start: &Value) -> Result<Period, DwError> {
    let end_nanos = temporal_epoch_nanos(end)
        .ok_or_else(|| DwError::UnsupportedFeature(format!("datetime {end}")))?;
    let start_nanos = temporal_epoch_nanos(start)
        .ok_or_else(|| DwError::UnsupportedFeature(format!("datetime {start}")))?;
    let delta = end_nanos - start_nanos;
    let sign = if delta < 0 { -1.0 } else { 1.0 };
    let nanos = delta.abs();
    let whole_seconds = nanos / 1_000_000_000;
    let days = (whole_seconds / 86_400) as f64;
    let hours = ((whole_seconds % 86_400) / 3600) as f64;
    let minutes = ((whole_seconds % 3600) / 60) as f64;
    let seconds = (whole_seconds % 60) as f64 + (nanos % 1_000_000_000) as f64 / 1_000_000_000.0;
    Ok(Period {
        years: 0,
        months: 0,
        days: days * sign,
        hours: hours * sign,
        minutes: minutes * sign,
        seconds: seconds * sign,
        date_based: false,
    })
}

fn temporal_epoch_nanos(value: &Value) -> Option<i128> {
    Some(match value {
        Value::Date(value) => {
            value.and_hms_opt(0, 0, 0)?.and_utc().timestamp() as i128 * 1_000_000_000
        }
        Value::DateTime(value) => {
            value.value.timestamp() as i128 * 1_000_000_000
                + value.value.timestamp_subsec_nanos() as i128
        }
        Value::LocalDateTime(value) => {
            let value = value.value.and_utc();
            value.timestamp() as i128 * 1_000_000_000 + value.timestamp_subsec_nanos() as i128
        }
        Value::LocalTime(value) => {
            value.value.num_seconds_from_midnight() as i128 * 1_000_000_000
                + value.value.nanosecond() as i128
        }
        Value::Time(value) => {
            (value.value.num_seconds_from_midnight() as i128
                - value.offset.local_minus_utc() as i128)
                * 1_000_000_000
                + value.value.nanosecond() as i128
        }
        _ => return None,
    })
}

fn negate_period(period: Period) -> Period {
    Period {
        years: -period.years,
        months: -period.months,
        days: -period.days,
        hours: -period.hours,
        minutes: -period.minutes,
        seconds: -period.seconds,
        date_based: period.date_based,
    }
}

fn add_period_to_temporal(temporal: &Value, period: Period) -> Result<Value, DwError> {
    let overflow =
        || DwError::UnsupportedFeature("temporal arithmetic exceeds the supported range".into());
    let months = period
        .years
        .checked_mul(12)
        .and_then(|years| years.checked_add(period.months))
        .ok_or_else(overflow)?;
    let calendar = |date: NaiveDate| -> Result<NaiveDate, DwError> {
        let amount = u32::try_from(months.unsigned_abs()).map_err(|_| overflow())?;
        if months < 0 {
            date.checked_sub_months(Months::new(amount))
        } else {
            date.checked_add_months(Months::new(amount))
        }
        .ok_or_else(overflow)
    };
    let seconds = total_seconds(period);
    if !seconds.is_finite() || seconds < i64::MIN as f64 || seconds >= i64::MAX as f64 {
        return Err(overflow());
    }
    let delta = TimeDelta::try_seconds(seconds.trunc() as i64)
        .and_then(|whole| {
            whole.checked_add(&TimeDelta::nanoseconds(
                (seconds.fract() * 1_000_000_000.0).round() as i64,
            ))
        })
        .ok_or_else(overflow)?;
    let precision = |time: chrono::NaiveTime| {
        if time.nanosecond() == 0 {
            0
        } else {
            format!("{:09}", time.nanosecond())
                .trim_end_matches('0')
                .len()
        }
    };
    match temporal {
        Value::Date(value) => {
            let date = calendar(*value)?
                .checked_add_signed(TimeDelta::try_days(period.days as i64).ok_or_else(overflow)?)
                .ok_or_else(overflow)?;
            Ok(Value::Date(date))
        }
        Value::LocalDateTime(value) => {
            let next = calendar(value.value.date())?
                .and_time(value.value.time())
                .checked_add_signed(delta)
                .ok_or_else(overflow)?;
            Ok(Value::LocalDateTime(crate::DwLocalDateTime {
                value: next,
                fractional_digits: precision(next.time()),
            }))
        }
        Value::DateTime(value) => {
            use chrono::TimeZone;
            let local = calendar(value.value.date_naive())?
                .and_time(value.value.time())
                .checked_add_signed(delta)
                .ok_or_else(overflow)?;
            let next = value
                .value
                .offset()
                .from_local_datetime(&local)
                .single()
                .ok_or_else(overflow)?;
            Ok(Value::DateTime(crate::DwDateTime {
                value: next,
                fractional_digits: precision(next.time()),
                use_z: value.use_z,
            }))
        }
        Value::LocalTime(value) => {
            let next = value.value.overflowing_add_signed(delta).0;
            Ok(Value::LocalTime(crate::DwLocalTime {
                value: next,
                fractional_digits: precision(next),
            }))
        }
        Value::Time(value) => {
            let next = value.value.overflowing_add_signed(delta).0;
            Ok(Value::Time(crate::DwTime {
                value: next,
                fractional_digits: precision(next),
                offset: value.offset,
                use_z: value.use_z,
            }))
        }
        _ => Err(DwError::UnsupportedFeature(
            "expected a native temporal value".into(),
        )),
    }
}

fn add_date_parts(date: (i64, i64, i64), period: Period) -> (i64, i64, i64) {
    let total_months = date.0 * 12 + (date.1 - 1) + period.years * 12 + period.months;
    let year = total_months.div_euclid(12);
    let month = total_months.rem_euclid(12) + 1;
    let day = date.2.min(days_in_month(year, month));
    (year, month, day)
}

fn add_days(date: (i64, i64, i64), days: i64) -> (i64, i64, i64) {
    civil_from_unix_days(unix_days_from_civil(date.0, date.1, date.2) + days)
}

fn period_between(end: (i64, i64, i64), start: (i64, i64, i64)) -> Period {
    let mut total_months = (end.0 - start.0) * 12 + (end.1 - start.1);
    let mut adjusted = add_date_parts(
        start,
        Period {
            years: 0,
            months: total_months,
            days: 0.0,
            hours: 0.0,
            minutes: 0.0,
            seconds: 0.0,
            date_based: true,
        },
    );
    let mut days = unix_days_from_civil(end.0, end.1, end.2)
        - unix_days_from_civil(adjusted.0, adjusted.1, adjusted.2);
    if total_months > 0 && days < 0 {
        total_months -= 1;
        adjusted = add_date_parts(
            start,
            Period {
                years: 0,
                months: total_months,
                days: 0.0,
                hours: 0.0,
                minutes: 0.0,
                seconds: 0.0,
                date_based: true,
            },
        );
        days = unix_days_from_civil(end.0, end.1, end.2)
            - unix_days_from_civil(adjusted.0, adjusted.1, adjusted.2);
    } else if total_months < 0 && days > 0 {
        total_months += 1;
        days -= days_in_month(end.0, end.1);
    } else if total_months < 0 && days == 0 && start.2 > end.2 {
        days = -1;
    }
    let years = total_months / 12;
    Period {
        years,
        months: total_months - years * 12,
        days: days as f64,
        hours: 0.0,
        minutes: 0.0,
        seconds: 0.0,
        date_based: true,
    }
}

fn date_parts(value: &Value) -> Result<(i64, i64, i64), DwError> {
    parse_date(&as_dataweave_string(value))
        .ok_or_else(|| DwError::UnsupportedFeature(format!("date {}", as_dataweave_string(value))))
}

pub(crate) fn period_to_string(period: Period) -> String {
    if period.date_based {
        let mut output = "P".to_string();
        if period.years != 0 {
            output.push_str(&format!("{}Y", period.years));
        }
        if period.months != 0 {
            output.push_str(&format!("{}M", period.months));
        }
        if period.days != 0.0 || output == "P" {
            output.push_str(&format!("{}D", compact_number(period.days)));
        }
        return output;
    }
    let seconds = total_seconds(period);
    let sign = if seconds < 0.0 { -1.0 } else { 1.0 };
    let seconds = seconds.abs();
    let hours = (seconds / 3600.0).floor();
    let minutes = ((seconds - hours * 3600.0) / 60.0).floor();
    let remaining_seconds = seconds - hours * 3600.0 - minutes * 60.0;
    let mut output = "PT".to_string();
    if hours != 0.0 {
        output.push_str(&format!("{}H", compact_number(hours * sign)));
    }
    if minutes != 0.0 {
        output.push_str(&format!("{}M", compact_number(minutes * sign)));
    }
    if remaining_seconds != 0.0 || output == "PT" {
        output.push_str(&format!("{}S", compact_number(remaining_seconds * sign)));
    }
    output
}

fn total_seconds(period: Period) -> f64 {
    period.days * 86_400.0 + period.hours * 3600.0 + period.minutes * 60.0 + period.seconds
}

fn number_value(value: f64) -> Value {
    if value.fract() == 0.0 {
        Value::Number((value as i64).into())
    } else {
        Value::Number(serde_json::Number::from_f64(value).unwrap_or_else(|| 0.into()))
    }
}

fn compact_number(value: f64) -> String {
    let value = if value.is_finite() {
        (value * 1_000_000_000_000.0).round() / 1_000_000_000_000.0
    } else {
        value
    };
    if value.fract() == 0.0 {
        return (value as i64).to_string();
    }
    let mut output = format!("{value:.12}");
    while output.contains('.') && output.ends_with('0') {
        output.pop();
    }
    if output.ends_with('.') {
        output.pop();
    }
    output
}

fn parse_datetime_parts(source: &str) -> Option<((i64, i64, i64), (f64, f64, f64), String)> {
    let (date, time) = source.split_once('T')?;
    let date = parse_date(date)?;
    let (hour, minute, second, suffix) = parse_time(time)?;
    Some((date, (hour, minute, second), suffix))
}

fn parse_date(source: &str) -> Option<(i64, i64, i64)> {
    let date = source.get(..10)?;
    if date.as_bytes().get(4) != Some(&b'-') || date.as_bytes().get(7) != Some(&b'-') {
        return None;
    }
    Some((
        date[0..4].parse().ok()?,
        date[5..7].parse().ok()?,
        date[8..10].parse().ok()?,
    ))
}

fn parse_time(source: &str) -> Option<(f64, f64, f64, String)> {
    let suffix = if source.ends_with('Z') {
        "Z".to_string()
    } else if source.ends_with("+00:00") {
        "Z".to_string()
    } else if source
        .get(source.len().saturating_sub(6)..)
        .is_some_and(is_timezone_offset)
    {
        source[source.len() - 6..].to_string()
    } else {
        String::new()
    };
    let source = source
        .strip_suffix('Z')
        .or_else(|| source.strip_suffix("+00:00"))
        .or_else(|| source.strip_suffix(&suffix))
        .unwrap_or(source);
    let mut parts = source.split(':');
    let hour = parts.next()?.parse::<f64>().ok()?;
    let minute = parts.next()?.parse::<f64>().ok()?;
    let second = parts.next().unwrap_or("0").parse::<f64>().ok()?;
    Some((hour, minute, second, suffix))
}

fn is_timezone_offset(source: &str) -> bool {
    source.len() == 6
        && matches!(source.as_bytes().first(), Some(b'+') | Some(b'-'))
        && source.as_bytes().get(3) == Some(&b':')
        && source[1..3].chars().all(|ch| ch.is_ascii_digit())
        && source[4..6].chars().all(|ch| ch.is_ascii_digit())
}

fn days_in_month(year: i64, month: i64) -> i64 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap_year(year) => 29,
        2 => 28,
        _ => 30,
    }
}

fn is_leap_year(year: i64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

fn unix_days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let year = year - if month <= 2 { 1 } else { 0 };
    let era = if year >= 0 { year } else { year - 399 }.div_euclid(400);
    let yoe = year - era * 400;
    let month = month + if month > 2 { -3 } else { 9 };
    let doy = (153 * month + 2).div_euclid(5) + day - 1;
    let doe = yoe * 365 + yoe.div_euclid(4) - yoe.div_euclid(100) + doy;
    era * 146_097 + doe - 719_468
}

fn civil_from_unix_days(days: i64) -> (i64, i64, i64) {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 }.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe.div_euclid(1_460) + doe.div_euclid(36_524) - doe.div_euclid(146_096))
        .div_euclid(365);
    let mut year = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe.div_euclid(4) - yoe.div_euclid(100));
    let mp = (5 * doy + 2).div_euclid(153);
    let day = doy - (153 * mp + 2).div_euclid(5) + 1;
    let month = mp + if mp < 10 { 3 } else { -9 };
    if month <= 2 {
        year += 1;
    }
    (year, month, day)
}
