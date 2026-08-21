use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum DwValue {
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    Array(Vec<DwValue>),
    Object(Vec<(String, DwValue)>),
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

impl TryFrom<Value> for DwValue {
    type Error = DwError;

    fn try_from(value: Value) -> Result<Self, Self::Error> {
        Ok(match value {
            Value::Null => DwValue::Null,
            Value::Bool(value) => DwValue::Bool(value),
            Value::Number(value) => DwValue::Number(
                value
                    .as_f64()
                    .ok_or_else(|| DwError::InvalidJson(value.to_string()))?,
            ),
            Value::String(value) => DwValue::String(value),
            Value::Array(values) => values
                .into_iter()
                .map(DwValue::try_from)
                .collect::<Result<Vec<_>, _>>()?
                .into(),
            Value::Object(values) => values
                .into_iter()
                .map(|(key, value)| Ok((key, DwValue::try_from(value)?)))
                .collect::<Result<Vec<_>, _>>()?
                .into(),
        })
    }
}

impl From<Vec<DwValue>> for DwValue {
    fn from(value: Vec<DwValue>) -> Self {
        DwValue::Array(value)
    }
}

impl From<Vec<(String, DwValue)>> for DwValue {
    fn from(value: Vec<(String, DwValue)>) -> Self {
        DwValue::Object(value)
    }
}
