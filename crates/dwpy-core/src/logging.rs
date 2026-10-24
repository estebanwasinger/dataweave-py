//! Execution-scoped logging. No process-wide logger is installed or changed.
use std::{cell::RefCell, fmt, sync::Arc};

use serde::Serialize;

use crate::DwValue;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Debug,
    Info,
    Warn,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct LogEntry {
    pub level: LogLevel,
    pub prefix: Option<String>,
    pub message: String,
}

/// Synchronous sink. Closures can be used directly with `with_logger`.
pub trait Logger: Send + Sync {
    fn log(&self, entry: &LogEntry);
}

impl<F: Fn(&LogEntry) + Send + Sync> Logger for F {
    fn log(&self, entry: &LogEntry) {
        self(entry);
    }
}

#[derive(Clone, Default)]
pub struct Loggers(Vec<Arc<dyn Logger>>);

impl fmt::Debug for Loggers {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Loggers").field(&self.0.len()).finish()
    }
}

impl Loggers {
    pub fn push(&mut self, logger: impl Logger + 'static) {
        self.0.push(Arc::new(logger));
    }

    pub(crate) fn enter(&self) -> LoggingScope {
        LoggingScope(ACTIVE_LOGGERS.with(|active| active.replace(self.clone())))
    }
}

thread_local! {
    static ACTIVE_LOGGERS: RefCell<Loggers> = RefCell::new(Loggers::default());
}

// Execution is synchronous, including lazy writers. Restore on errors, panics,
// and reentrant execution from a logger; never hold the TLS borrow in callbacks.
pub(crate) struct LoggingScope(Loggers);

impl Drop for LoggingScope {
    fn drop(&mut self) {
        ACTIVE_LOGGERS.with(|active| active.replace(std::mem::take(&mut self.0)));
    }
}

pub(crate) fn emit(level: LogLevel, prefix: Option<String>, value: &DwValue) {
    let loggers = ACTIVE_LOGGERS.with(|active| active.borrow().clone());
    if loggers.0.is_empty() {
        return;
    }
    let rendered = match value {
        DwValue::String(text) => text.clone(),
        other => crate::json::render_json_value(
            other,
            crate::json::JsonOutputOptions {
                indent: None,
                ensure_ascii: false,
                sort_keys: false,
                duplicate_key_as_array: false,
            },
        )
        .unwrap_or_else(|_| other.to_string()),
    };
    let message = match &prefix {
        Some(prefix) if !prefix.is_empty() => format!("{prefix} - {rendered}"),
        _ => rendered,
    };
    let entry = LogEntry {
        level,
        prefix,
        message,
    };
    for logger in &loggers.0 {
        logger.log(&entry);
    }
}
