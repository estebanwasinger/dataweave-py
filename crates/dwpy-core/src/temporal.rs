//! Parsed temporal values. Text and tagged JSON are boundary representations.
use crate::{DwError, DwValue};
use chrono::{DateTime, FixedOffset, NaiveDate, NaiveDateTime, NaiveTime, Timelike};

#[derive(Debug, Clone)]
pub struct DwLocalDateTime {
    pub value: NaiveDateTime,
    pub fractional_digits: usize,
}

#[derive(Debug, Clone)]
pub struct DwDateTime {
    pub value: DateTime<FixedOffset>,
    pub fractional_digits: usize,
    pub use_z: bool,
}

#[derive(Debug, Clone)]
pub struct DwLocalTime {
    pub value: NaiveTime,
    pub fractional_digits: usize,
}

#[derive(Debug, Clone)]
pub struct DwTime {
    pub value: NaiveTime,
    pub offset: FixedOffset,
    pub fractional_digits: usize,
    pub use_z: bool,
}

fn precision(text: &str) -> usize {
    text.split_once('.').map_or(0, |(_, tail)| {
        tail.chars().take_while(char::is_ascii_digit).count().min(9)
    })
}

fn time_text(value: NaiveTime, digits: usize) -> String {
    let mut text = value.format("%H:%M:%S").to_string();
    if digits > 0 {
        let fraction = format!("{:09}", value.nanosecond() % 1_000_000_000);
        text.push('.');
        text.push_str(&fraction[..digits.min(9)]);
    }
    text
}

fn offset_text(offset: FixedOffset, use_z: bool) -> String {
    if use_z && offset.local_minus_utc() == 0 {
        "Z".into()
    } else {
        offset.to_string()
    }
}

impl DwValue {
    pub fn parse_temporal(text: &str) -> Result<Self, DwError> {
        let invalid = || DwError::Parse(format!("invalid temporal value '{text}'"));
        if text == "Z" || ((text.starts_with('+') || text.starts_with('-')) && text.len() == 6) {
            let offset = if text == "Z" {
                FixedOffset::east_opt(0).unwrap()
            } else {
                text.parse::<FixedOffset>().map_err(|_| invalid())?
            };
            return Ok(Self::TimeZone(offset));
        }
        if text.contains('T') {
            if let Ok(value) = DateTime::parse_from_rfc3339(text) {
                return Ok(Self::DateTime(DwDateTime {
                    value,
                    fractional_digits: precision(text),
                    use_z: text.ends_with('Z'),
                }));
            }
            let value = NaiveDateTime::parse_from_str(text, "%Y-%m-%dT%H:%M:%S%.f")
                .map_err(|_| invalid())?;
            return Ok(Self::LocalDateTime(DwLocalDateTime {
                value,
                fractional_digits: precision(text),
            }));
        }
        if text.contains(':') {
            let split = text
                .char_indices()
                .skip(1)
                .find(|(_, ch)| matches!(ch, '+' | '-' | 'Z'))
                .map(|(index, _)| index);
            let (clock, zone) =
                split.map_or((text, None), |index| (&text[..index], Some(&text[index..])));
            let value = NaiveTime::parse_from_str(clock, "%H:%M:%S%.f")
                .or_else(|_| NaiveTime::parse_from_str(clock, "%H:%M"))
                .map_err(|_| invalid())?;
            let fractional_digits = precision(clock);
            if let Some(zone) = zone {
                let offset = if zone == "Z" {
                    FixedOffset::east_opt(0).unwrap()
                } else {
                    zone.parse::<FixedOffset>().map_err(|_| invalid())?
                };
                return Ok(Self::Time(DwTime {
                    value,
                    offset,
                    fractional_digits,
                    use_z: zone == "Z",
                }));
            }
            return Ok(Self::LocalTime(DwLocalTime {
                value,
                fractional_digits,
            }));
        }
        NaiveDate::parse_from_str(text, "%Y-%m-%d")
            .map(Self::Date)
            .map_err(|_| invalid())
    }

    pub fn temporal_text(&self) -> Option<String> {
        Some(match self {
            Self::TimeZone(offset) => offset_text(*offset, true),
            Self::Date(date) => date.format("%Y-%m-%d").to_string(),
            Self::LocalDateTime(value) => format!(
                "{}T{}",
                value.value.date().format("%Y-%m-%d"),
                time_text(value.value.time(), value.fractional_digits)
            ),
            Self::DateTime(value) => format!(
                "{}T{}{}",
                value.value.date_naive().format("%Y-%m-%d"),
                time_text(value.value.time(), value.fractional_digits),
                offset_text(*value.value.offset(), value.use_z)
            ),
            Self::LocalTime(value) => time_text(value.value, value.fractional_digits),
            Self::Time(value) => format!(
                "{}{}",
                time_text(value.value, value.fractional_digits),
                offset_text(value.offset, value.use_z)
            ),
            _ => return None,
        })
    }

    pub fn temporal_type_name(&self) -> Option<&'static str> {
        match self {
            Self::TimeZone(_) => Some("TimeZone"),
            Self::Date(_) => Some("Date"),
            Self::LocalDateTime(_) => Some("LocalDateTime"),
            Self::DateTime(_) => Some("DateTime"),
            Self::LocalTime(_) => Some("LocalTime"),
            Self::Time(_) => Some("Time"),
            Self::Period(_) => Some("Period"),
            _ => None,
        }
    }
}

impl PartialEq for DwLocalDateTime {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
}
impl PartialEq for DwDateTime {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
}
impl PartialEq for DwLocalTime {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
}
impl PartialEq for DwTime {
    fn eq(&self, other: &Self) -> bool {
        (
            self.value.num_seconds_from_midnight() as i64 - self.offset.local_minus_utc() as i64,
            self.value.nanosecond(),
        ) == (
            other.value.num_seconds_from_midnight() as i64 - other.offset.local_minus_utc() as i64,
            other.value.nanosecond(),
        )
    }
}
