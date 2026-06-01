use serde_json::{Map, Value};

use crate::{as_dataweave_string, numeric_value, DwError};

pub const DW_PERIOD_MARKER: &str = "__dwpy_period";
pub const DW_TEMPORAL_MARKER: &str = "__dwpy_temporal";

#[derive(Debug, Clone, Copy)]
struct Period {
    years: i64,
    months: i64,
    days: f64,
    hours: f64,
    minutes: f64,
    seconds: f64,
    date_based: bool,
}

#[derive(Debug, Clone)]
struct Temporal {
    kind: String,
    value: String,
}

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
    Ok(temporal_value(temporal.kind(), rendered))
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
        "date" => Ok(temporal_value(
            "date",
            format!("{year:04}-{month:02}-{day:02}"),
        )),
        "dateTime" => Ok(temporal_value(
            "datetime",
            format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minutes:02}:{seconds:02}{timezone}"),
        )),
        "localDateTime" => Ok(temporal_value(
            "datetime",
            format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minutes:02}:{seconds:02}"),
        )),
        "localTime" => Ok(temporal_value(
            "time",
            format!("{hour:02}:{minutes:02}:{seconds:02}"),
        )),
        "time" => Ok(temporal_value(
            "time",
            format!("{hour:02}:{minutes:02}:{seconds:02}{timezone}"),
        )),
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
    if let Some(temporal) = temporal_from_value(value) {
        return Some(temporal.value);
    }
    None
}

pub(crate) fn temporal_field_value(value: &Value, field: &str) -> Option<Value> {
    if let Some(period) = period_from_value(value) {
        return period_field_value(period, field);
    }
    let text = special_string_value(value).unwrap_or_else(|| as_dataweave_string(value));
    let parsed = ParsedTemporal::parse(&text)?;
    let number = match field {
        "year" => parsed.year,
        "month" => parsed.month,
        "day" => parsed.day,
        "hour" => parsed.hour?,
        "minutes" => parsed.minute?,
        "seconds" => parsed.second?,
        "milliseconds" => parsed.millisecond,
        "nanoseconds" => parsed.millisecond * 1_000_000,
        "quarter" => (parsed.month - 1) / 3 + 1,
        "dayOfWeek" => day_of_week(parsed.year, parsed.month, parsed.day),
        "dayOfYear" => day_of_year(parsed.year, parsed.month, parsed.day),
        "offsetSeconds" => parsed.offset_seconds.unwrap_or(0),
        _ => return None,
    };
    Some(Value::Number(number.into()))
}

fn period_field_value(period: Period, field: &str) -> Option<Value> {
    let number = match field {
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
    let temporal = temporal_from_value(value)?;
    let millis = temporal_epoch_millis(&temporal)?;
    match unit {
        "nanos" | "nanoseconds" => Some(Value::Number((millis * 1_000_000).into())),
        "millis" | "milliseconds" => Some(Value::Number(millis.into())),
        "seconds" => Some(Value::Number((millis / 1_000).into())),
        "minutes" => Some(Value::Number((millis / 60_000).into())),
        "hours" => Some(Value::Number((millis / 3_600_000).into())),
        "days" => Some(Value::Number((millis / 86_400_000).into())),
        _ => None,
    }
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
    Some(Value::String(combined))
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

struct ParsedTemporal {
    year: i64,
    month: i64,
    day: i64,
    hour: Option<i64>,
    minute: Option<i64>,
    second: Option<i64>,
    millisecond: i64,
    offset_seconds: Option<i64>,
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

impl ParsedTemporal {
    fn parse(value: &str) -> Option<Self> {
        let (year, month, day) = parse_date(value.get(..10)?)?;
        let Some(time_source) = value.split_once('T').map(|(_, time)| time) else {
            return Some(Self {
                year,
                month,
                day,
                hour: None,
                minute: None,
                second: None,
                millisecond: 0,
                offset_seconds: None,
            });
        };
        let (time_source, offset_seconds) = split_time_offset(time_source);
        let mut parts = time_source.split(':');
        let hour = parts.next()?.parse::<i64>().ok()?;
        let minute = parts.next()?.parse::<i64>().ok()?;
        let second_source = parts.next().unwrap_or("0");
        let (second_text, fraction_text) = second_source
            .split_once('.')
            .map_or((second_source, ""), |(second, fraction)| (second, fraction));
        let second = second_text.parse::<i64>().ok()?;
        let millisecond = fraction_text
            .chars()
            .take(3)
            .chain(std::iter::repeat('0'))
            .take(3)
            .collect::<String>()
            .parse::<i64>()
            .ok()
            .unwrap_or(0);
        Some(Self {
            year,
            month,
            day,
            hour: Some(hour),
            minute: Some(minute),
            second: Some(second),
            millisecond,
            offset_seconds,
        })
    }
}

fn split_time_offset(value: &str) -> (&str, Option<i64>) {
    if let Some(time) = value.strip_suffix('Z') {
        return (time, Some(0));
    }
    for sign in ['+', '-'] {
        if let Some(index) = value.rfind(sign) {
            let offset = &value[index..];
            if offset.len() == 6 && offset.as_bytes().get(3) == Some(&b':') {
                let multiplier = if sign == '-' { -1 } else { 1 };
                let hours = offset[1..3].parse::<i64>().ok();
                let minutes = offset[4..6].parse::<i64>().ok();
                if let (Some(hours), Some(minutes)) = (hours, minutes) {
                    return (
                        &value[..index],
                        Some(multiplier * (hours * 3600 + minutes * 60)),
                    );
                }
            }
        }
    }
    (value, None)
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

fn day_of_year(year: i64, month: i64, day: i64) -> i64 {
    (1..month)
        .map(|current| days_in_month(year, current))
        .sum::<i64>()
        + day
}

fn period_value(period: Period) -> Value {
    Value::Object(Map::from_iter([
        (DW_PERIOD_MARKER.to_string(), Value::Bool(true)),
        ("years".to_string(), Value::Number(period.years.into())),
        ("months".to_string(), Value::Number(period.months.into())),
        ("days".to_string(), number_value(period.days)),
        ("hours".to_string(), number_value(period.hours)),
        ("minutes".to_string(), number_value(period.minutes)),
        ("seconds".to_string(), number_value(period.seconds)),
        ("dateBased".to_string(), Value::Bool(period.date_based)),
        ("text".to_string(), Value::String(period_to_string(period))),
    ]))
}

fn temporal_value(kind: &str, value: String) -> Value {
    Value::Object(Map::from_iter([
        (
            DW_TEMPORAL_MARKER.to_string(),
            Value::String(kind.to_string()),
        ),
        ("value".to_string(), Value::String(value)),
    ]))
}

fn period_from_value(value: &Value) -> Option<Period> {
    let map = value.as_object()?;
    if !map.contains_key(DW_PERIOD_MARKER) {
        return None;
    }
    Some(Period {
        years: map.get("years").and_then(Value::as_i64).unwrap_or(0),
        months: map.get("months").and_then(Value::as_i64).unwrap_or(0),
        days: map.get("days").and_then(Value::as_f64).unwrap_or(0.0),
        hours: map.get("hours").and_then(Value::as_f64).unwrap_or(0.0),
        minutes: map.get("minutes").and_then(Value::as_f64).unwrap_or(0.0),
        seconds: map.get("seconds").and_then(Value::as_f64).unwrap_or(0.0),
        date_based: map
            .get("dateBased")
            .and_then(Value::as_bool)
            .unwrap_or(false),
    })
}

fn temporal_from_value(value: &Value) -> Option<Temporal> {
    if let Value::Object(map) = value {
        let kind = map.get(DW_TEMPORAL_MARKER)?.as_str()?.to_string();
        let value = map.get("value")?.as_str()?.to_string();
        return Some(Temporal { kind, value });
    }
    let text = value.as_str()?;
    if text.contains('T') && parse_datetime_parts(text).is_some() {
        return Some(Temporal {
            kind: "datetime".to_string(),
            value: text.to_string(),
        });
    }
    if parse_date(text).is_some() {
        return Some(Temporal {
            kind: "date".to_string(),
            value: text.to_string(),
        });
    }
    if parse_time(text).is_some() {
        return Some(Temporal {
            kind: "time".to_string(),
            value: text.to_string(),
        });
    }
    None
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

fn period_between_temporals(end: &Temporal, start: &Temporal) -> Result<Period, DwError> {
    let end_millis = temporal_epoch_millis(end)
        .ok_or_else(|| DwError::UnsupportedFeature(format!("datetime {}", end.value)))?;
    let start_millis = temporal_epoch_millis(start)
        .ok_or_else(|| DwError::UnsupportedFeature(format!("datetime {}", start.value)))?;
    let mut millis = end_millis - start_millis;
    let sign = if millis < 0 { -1.0 } else { 1.0 };
    millis = millis.abs();
    let total_seconds = millis as f64 / 1_000.0;
    let days = (total_seconds / 86_400.0).floor();
    let remainder = total_seconds - days * 86_400.0;
    let hours = (remainder / 3600.0).floor();
    let remainder = remainder - hours * 3600.0;
    let minutes = (remainder / 60.0).floor();
    let seconds = remainder - minutes * 60.0;
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

fn temporal_epoch_millis(temporal: &Temporal) -> Option<i64> {
    if let Some((date, time, _suffix)) = parse_datetime_parts(&temporal.value) {
        let days = unix_days_from_civil(date.0, date.1, date.2);
        let millis = ((time.0 * 3600.0 + time.1 * 60.0 + time.2) * 1_000.0).round() as i64;
        return Some(days * 86_400_000 + millis);
    }
    if let Some(date) = parse_date(&temporal.value) {
        return Some(unix_days_from_civil(date.0, date.1, date.2) * 86_400_000);
    }
    if let Some((hour, minute, second, suffix)) = parse_time(&temporal.value) {
        let millis = ((hour * 3600.0 + minute * 60.0 + second) * 1_000.0).round() as i64;
        let offset_millis = timezone_offset_seconds(&suffix).unwrap_or(0) * 1_000;
        return Some(millis - offset_millis);
    }
    None
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

fn add_period_to_temporal(temporal: &Temporal, period: Period) -> Result<Value, DwError> {
    match temporal.kind.as_str() {
        "datetime" => {
            let parsed = parse_datetime_parts(&temporal.value).ok_or_else(|| {
                DwError::UnsupportedFeature(format!("datetime {}", temporal.value))
            })?;
            let (date, time, suffix) = parsed;
            let date = add_date_parts(date, period);
            let total_seconds = time.0 * 3600.0 + time.1 * 60.0 + time.2 + total_seconds(period);
            let day_delta = total_seconds.div_euclid(86_400.0).floor() as i64;
            let seconds_of_day = total_seconds.rem_euclid(86_400.0);
            let shifted_date = add_days(date, day_delta);
            let rendered_time = render_time(seconds_of_day, &suffix);
            Ok(temporal_value(
                "datetime",
                format!(
                    "{:04}-{:02}-{:02}T{}",
                    shifted_date.0, shifted_date.1, shifted_date.2, rendered_time
                ),
            ))
        }
        "date" => {
            let date = parse_date(&temporal.value)
                .ok_or_else(|| DwError::UnsupportedFeature(format!("date {}", temporal.value)))?;
            let date = add_days(add_date_parts(date, period), period.days as i64);
            Ok(temporal_value(
                "date",
                format!("{:04}-{:02}-{:02}", date.0, date.1, date.2),
            ))
        }
        "time" => {
            let (hour, minute, second, suffix) = parse_time(&temporal.value)
                .ok_or_else(|| DwError::UnsupportedFeature(format!("time {}", temporal.value)))?;
            let total = hour * 3600.0 + minute * 60.0 + second + total_seconds(period);
            Ok(temporal_value(
                "time",
                render_time(total.rem_euclid(86_400.0), &suffix),
            ))
        }
        _ => Err(DwError::UnsupportedFeature(format!(
            "temporal kind {}",
            temporal.kind
        ))),
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
    temporal_from_value(value)
        .and_then(|temporal| parse_date(&temporal.value))
        .ok_or_else(|| DwError::UnsupportedFeature(format!("date {}", as_dataweave_string(value))))
}

fn period_to_string(period: Period) -> String {
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

fn timezone_offset_seconds(source: &str) -> Option<i64> {
    if source.is_empty() || source == "Z" {
        return Some(0);
    }
    if !is_timezone_offset(source) {
        return None;
    }
    let sign = if source.starts_with('-') { -1 } else { 1 };
    let hours = source[1..3].parse::<i64>().ok()?;
    let minutes = source[4..6].parse::<i64>().ok()?;
    Some(sign * (hours * 3600 + minutes * 60))
}

fn render_time(seconds_of_day: f64, suffix: &str) -> String {
    let hour = (seconds_of_day / 3600.0).floor();
    let minute = ((seconds_of_day - hour * 3600.0) / 60.0).floor();
    let second = seconds_of_day - hour * 3600.0 - minute * 60.0;
    let second_text = if second.fract() == 0.0 {
        format!("{:02}", second as i64)
    } else {
        let whole = second.floor() as i64;
        let mut fraction = format!("{:.6}", second.fract());
        while fraction.ends_with('0') {
            fraction.pop();
        }
        let fraction = fraction.trim_start_matches('0').to_string();
        format!("{whole:02}{fraction}")
    };
    format!(
        "{:02}:{:02}:{}{}",
        hour as i64, minute as i64, second_text, suffix
    )
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
