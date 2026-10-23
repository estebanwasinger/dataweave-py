//! Runtime values. JSON is decoded at the API boundary, never used as an
//! execution value or as the representation of a temporal scalar.
use crate::temporal::{DwDateTime, DwLocalDateTime, DwLocalTime, DwTime};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::{Number, Value as JsonValue};
use std::ops::{Index, IndexMut};
use thiserror::Error;

pub type DwMap<K = String, V = DwValue> = indexmap::IndexMap<K, V>;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DwPeriod {
    pub years: i64,
    pub months: i64,
    pub days: f64,
    pub hours: f64,
    pub minutes: f64,
    pub seconds: f64,
    pub date_based: bool,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub enum DwValue {
    #[default]
    Null,
    Bool(bool),
    Number(Number),
    String(String),
    Array(Vec<DwValue>),
    Object(DwMap),
    TimeZone(chrono::FixedOffset),
    Date(chrono::NaiveDate),
    LocalDateTime(DwLocalDateTime),
    DateTime(DwDateTime),
    LocalTime(DwLocalTime),
    Time(DwTime),
    Period(DwPeriod),
}

#[derive(Debug, Error, PartialEq)]
pub enum DwError {
    #[error("Rust evaluator does not yet support this DataWeave feature: {0}")]
    UnsupportedFeature(String),
    #[error("Invalid JSON value: {0}")]
    InvalidJson(String),
    #[error("DataWeave parse error: {0}")]
    Parse(String),
    #[error(
        "Resource limit exceeded while {operation}: estimated {estimated_bytes} bytes at item {item_index}, limit is {limit_bytes} bytes"
    )]
    ResourceLimit {
        operation: String,
        limit_bytes: usize,
        estimated_bytes: usize,
        item_index: usize,
    },
    #[error("DataWeave output error: {0}")]
    Output(String),
}

impl From<JsonValue> for DwValue {
    /// Decode ordinary JSON without interpreting user-owned marker keys.
    fn from(value: JsonValue) -> Self {
        match value {
            JsonValue::Null => Self::Null,
            JsonValue::Bool(value) => Self::Bool(value),
            JsonValue::Number(value) => Self::Number(value),
            JsonValue::String(value) => Self::String(value),
            JsonValue::Array(values) => Self::Array(values.into_iter().map(Self::from).collect()),
            JsonValue::Object(values) => Self::Object(
                values
                    .into_iter()
                    .map(|(k, v)| (k, Self::from(v)))
                    .collect(),
            ),
        }
    }
}

impl DwValue {
    /// Serialize a runtime value as ordinary JSON. Temporal scalars are ISO strings.
    pub fn to_json(&self) -> JsonValue {
        match self {
            Self::Null => JsonValue::Null,
            Self::Bool(value) => JsonValue::Bool(*value),
            Self::Number(value) => JsonValue::Number(value.clone()),
            Self::String(value) => JsonValue::String(value.clone()),
            Self::Array(values) => JsonValue::Array(values.iter().map(Self::to_json).collect()),
            Self::Object(values) => JsonValue::Object(
                values
                    .iter()
                    .map(|(k, v)| (k.clone(), v.to_json()))
                    .collect(),
            ),
            Self::Period(period) => JsonValue::String(crate::periods::period_to_string(*period)),
            _ => JsonValue::String(self.temporal_text().expect("temporal variant")),
        }
    }

    /// Explicit wire representation for JSON-only language bridges.
    pub fn to_transport_json(&self) -> JsonValue {
        match self {
            Self::Array(values) => {
                JsonValue::Array(values.iter().map(Self::to_transport_json).collect())
            }
            Self::Object(values) => {
                let object: serde_json::Map<String, JsonValue> = values
                    .iter()
                    .map(|(k, v)| (k.clone(), v.to_transport_json()))
                    .collect();
                if object.contains_key("__dwpy_temporal")
                    || object.contains_key("__dwpy_period")
                    || object.contains_key("__dwpy_object")
                {
                    serde_json::json!({"__dwpy_object": object})
                } else {
                    JsonValue::Object(object)
                }
            }
            Self::Period(period) => serde_json::json!({
                "__dwpy_period": true, "years": period.years, "months": period.months,
                "days": period.days, "hours": period.hours, "minutes": period.minutes,
                "seconds": period.seconds, "dateBased": period.date_based,
                "text": crate::periods::period_to_string(*period),
            }),
            Self::TimeZone(_) => self.to_json(),
            _ if self.temporal_type_name().is_some() => {
                let kind = match self {
                    Self::Date(_) => "date",
                    Self::Time(_) | Self::LocalTime(_) => "time",
                    _ => "datetime",
                };
                serde_json::json!({"__dwpy_temporal": kind, "value": self.temporal_text().unwrap()})
            }
            _ => self.to_json(),
        }
    }

    /// Decode only values explicitly supplied by a typed language bridge.
    pub fn from_transport_json(value: JsonValue) -> Result<Self, DwError> {
        match value {
            JsonValue::Array(values) => Ok(Self::Array(
                values
                    .into_iter()
                    .map(Self::from_transport_json)
                    .collect::<Result<_, _>>()?,
            )),
            JsonValue::Object(mut map) => {
                if map.len() == 1 && map.contains_key("__dwpy_object") {
                    let JsonValue::Object(values) = map.remove("__dwpy_object").unwrap() else {
                        return Err(DwError::InvalidJson("invalid escaped bridge object".into()));
                    };
                    return Ok(Self::Object(
                        values
                            .into_iter()
                            .map(|(k, v)| Ok((k, Self::from_transport_json(v)?)))
                            .collect::<Result<_, DwError>>()?,
                    ));
                }
                if let Some(kind) = map.get("__dwpy_temporal").and_then(JsonValue::as_str) {
                    if !matches!(kind, "date" | "datetime" | "time") {
                        return Err(DwError::InvalidJson(format!(
                            "invalid temporal bridge kind '{kind}'"
                        )));
                    }
                    let text = map
                        .get("value")
                        .and_then(JsonValue::as_str)
                        .ok_or_else(|| {
                            DwError::InvalidJson("missing temporal bridge value".into())
                        })?;
                    let parsed = Self::parse_temporal(text)?;
                    let matches_kind = matches!(
                        (kind, &parsed),
                        ("date", Self::Date(_))
                            | ("datetime", Self::DateTime(_) | Self::LocalDateTime(_))
                            | ("time", Self::Time(_) | Self::LocalTime(_))
                    );
                    if !matches_kind {
                        return Err(DwError::InvalidJson(
                            "temporal bridge kind does not match its value".into(),
                        ));
                    }
                    return Ok(parsed);
                }
                if map.get("__dwpy_period").and_then(JsonValue::as_bool) == Some(true) {
                    return Ok(Self::Period(DwPeriod {
                        years: map.get("years").and_then(JsonValue::as_i64).unwrap_or(0),
                        months: map.get("months").and_then(JsonValue::as_i64).unwrap_or(0),
                        days: map.get("days").and_then(JsonValue::as_f64).unwrap_or(0.0),
                        hours: map.get("hours").and_then(JsonValue::as_f64).unwrap_or(0.0),
                        minutes: map
                            .get("minutes")
                            .and_then(JsonValue::as_f64)
                            .unwrap_or(0.0),
                        seconds: map
                            .get("seconds")
                            .and_then(JsonValue::as_f64)
                            .unwrap_or(0.0),
                        date_based: map
                            .get("dateBased")
                            .and_then(JsonValue::as_bool)
                            .unwrap_or(false),
                    }));
                }
                Ok(Self::Object(
                    map.into_iter()
                        .map(|(k, v)| Ok((k, Self::from_transport_json(v)?)))
                        .collect::<Result<_, DwError>>()?,
                ))
            }
            other => Ok(Self::from(other)),
        }
    }

    pub fn is_null(&self) -> bool {
        matches!(self, Self::Null)
    }
    pub fn is_boolean(&self) -> bool {
        matches!(self, Self::Bool(_))
    }
    pub fn is_number(&self) -> bool {
        matches!(self, Self::Number(_))
    }
    pub fn is_string(&self) -> bool {
        matches!(self, Self::String(_))
    }
    pub fn is_array(&self) -> bool {
        matches!(self, Self::Array(_))
    }
    pub fn is_object(&self) -> bool {
        matches!(self, Self::Object(_))
    }
    pub fn as_bool(&self) -> Option<bool> {
        if let Self::Bool(v) = self {
            Some(*v)
        } else {
            None
        }
    }
    pub fn as_str(&self) -> Option<&str> {
        if let Self::String(v) = self {
            Some(v)
        } else {
            None
        }
    }
    pub fn as_f64(&self) -> Option<f64> {
        if let Self::Number(v) = self {
            v.as_f64()
        } else {
            None
        }
    }
    pub fn as_i64(&self) -> Option<i64> {
        if let Self::Number(v) = self {
            v.as_i64()
        } else {
            None
        }
    }
    pub fn as_u64(&self) -> Option<u64> {
        if let Self::Number(v) = self {
            v.as_u64()
        } else {
            None
        }
    }
    pub fn as_array(&self) -> Option<&Vec<Self>> {
        if let Self::Array(v) = self {
            Some(v)
        } else {
            None
        }
    }
    pub fn as_array_mut(&mut self) -> Option<&mut Vec<Self>> {
        if let Self::Array(v) = self {
            Some(v)
        } else {
            None
        }
    }
    pub fn as_object(&self) -> Option<&DwMap> {
        if let Self::Object(v) = self {
            Some(v)
        } else {
            None
        }
    }
    pub fn as_object_mut(&mut self) -> Option<&mut DwMap> {
        if let Self::Object(v) = self {
            Some(v)
        } else {
            None
        }
    }
    pub fn get<I: DwIndex>(&self, index: I) -> Option<&Self> {
        index.get(self)
    }
    pub fn get_mut<I: DwIndex>(&mut self, index: I) -> Option<&mut Self> {
        index.get_mut(self)
    }
    pub fn take(&mut self) -> Self {
        std::mem::take(self)
    }
}

pub trait DwIndex {
    fn get(self, value: &DwValue) -> Option<&DwValue>;
    fn get_mut(self, value: &mut DwValue) -> Option<&mut DwValue>;
}
impl DwIndex for &str {
    fn get(self, value: &DwValue) -> Option<&DwValue> {
        value.as_object()?.get(self)
    }
    fn get_mut(self, value: &mut DwValue) -> Option<&mut DwValue> {
        value.as_object_mut()?.get_mut(self)
    }
}
impl DwIndex for &String {
    fn get(self, value: &DwValue) -> Option<&DwValue> {
        value.get(self.as_str())
    }
    fn get_mut(self, value: &mut DwValue) -> Option<&mut DwValue> {
        value.get_mut(self.as_str())
    }
}
impl DwIndex for usize {
    fn get(self, value: &DwValue) -> Option<&DwValue> {
        value.as_array()?.get(self)
    }
    fn get_mut(self, value: &mut DwValue) -> Option<&mut DwValue> {
        value.as_array_mut()?.get_mut(self)
    }
}
impl<I: DwIndex> Index<I> for DwValue {
    type Output = Self;
    fn index(&self, index: I) -> &Self {
        index.get(self).unwrap_or(&DwValue::Null)
    }
}
impl IndexMut<&str> for DwValue {
    fn index_mut(&mut self, index: &str) -> &mut Self {
        if self.is_null() {
            *self = Self::Object(DwMap::new());
        }
        self.as_object_mut()
            .expect("object index")
            .entry(index.to_string())
            .or_default()
    }
}
impl From<Vec<DwValue>> for DwValue {
    fn from(value: Vec<DwValue>) -> Self {
        Self::Array(value)
    }
}
impl From<Vec<(String, DwValue)>> for DwValue {
    fn from(value: Vec<(String, DwValue)>) -> Self {
        Self::Object(value.into_iter().collect())
    }
}
impl From<String> for DwValue {
    fn from(value: String) -> Self {
        Self::String(value)
    }
}
impl From<&str> for DwValue {
    fn from(value: &str) -> Self {
        Self::String(value.into())
    }
}
impl Serialize for DwValue {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::Null => serializer.serialize_unit(),
            Self::Bool(value) => value.serialize(serializer),
            Self::Number(value) => value.serialize(serializer),
            Self::String(value) => value.serialize(serializer),
            Self::Array(value) => value.serialize(serializer),
            Self::Object(value) => value.serialize(serializer),
            Self::Period(period) => crate::periods::period_to_string(*period).serialize(serializer),
            _ => self
                .temporal_text()
                .expect("temporal variant")
                .serialize(serializer),
        }
    }
}
impl<'de> Deserialize<'de> for DwValue {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        JsonValue::deserialize(deserializer).map(Self::from)
    }
}
impl std::fmt::Display for DwValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.to_json().fmt(f)
    }
}
impl PartialEq<JsonValue> for DwValue {
    fn eq(&self, other: &JsonValue) -> bool {
        self.to_json() == *other
    }
}
