use crate::value::DwMap as Map;
use crate::value::DwValue as Value;
use crate::{as_dataweave_string, DwError};
use std::io::Write;

#[derive(Clone)]
struct CsvWriterOptions {
    separator: char,
    quote: char,
    escape: char,
    header: bool,
    quote_header: bool,
    quote_values: bool,
    line_separator: String,
    columns: Option<Vec<String>>,
}

impl CsvWriterOptions {
    fn from_directive(directive: &str) -> Self {
        let get = |name| directive_option(directive, name);
        let boolean = |name, default| {
            get(name)
                .map(|v| matches!(v.as_str(), "true" | "True" | "1" | "yes"))
                .unwrap_or(default)
        };
        Self {
            separator: get("separator")
                .and_then(|v| v.chars().next())
                .unwrap_or(','),
            quote: get("quote").and_then(|v| v.chars().next()).unwrap_or('"'),
            escape: get("escape").and_then(|v| v.chars().next()).unwrap_or('\\'),
            header: boolean("header", true),
            quote_header: boolean("quoteHeader", false),
            quote_values: boolean("quoteValues", false),
            line_separator: get("lineSeparator")
                .or_else(|| get("newline"))
                .unwrap_or_else(|| "\n".into()),
            columns: get("columns").map(|v| {
                v.split(',')
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .map(str::to_string)
                    .collect()
            }),
        }
    }
    fn from_value(options: &Value) -> Self {
        let map = options.as_object();
        let get = |name: &str| map.and_then(|m| m.get(name)).map(as_dataweave_string);
        let boolean = |name: &str, default| {
            get(name)
                .map(|v| matches!(v.as_str(), "true" | "True" | "1" | "yes"))
                .unwrap_or(default)
        };
        Self {
            separator: get("separator")
                .and_then(|v| v.chars().next())
                .unwrap_or(','),
            quote: get("quote").and_then(|v| v.chars().next()).unwrap_or('"'),
            escape: get("escape").and_then(|v| v.chars().next()).unwrap_or('\\'),
            header: boolean("header", true),
            quote_header: boolean("quoteHeader", false),
            quote_values: boolean("quoteValues", false),
            line_separator: get("lineSeparator")
                .or_else(|| get("newline"))
                .unwrap_or_else(|| "\n".into()),
            columns: get("columns").map(|v| {
                v.split(',')
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .map(str::to_string)
                    .collect()
            }),
        }
    }
}
fn directive_option(directive: &str, name: &str) -> Option<String> {
    let mut chars = directive.chars().peekable();
    while chars.peek().is_some() {
        while chars.peek().is_some_and(|ch| ch.is_whitespace()) {
            chars.next();
        }
        let mut token = String::new();
        let mut quoted = false;
        let mut escaped = false;
        while let Some(&ch) = chars.peek() {
            if !quoted && ch.is_whitespace() {
                break;
            }
            chars.next();
            token.push(ch);
            if ch == '"' && !escaped {
                quoted = !quoted;
            }
            if ch == '\\' && !escaped {
                escaped = true;
            } else {
                escaped = false;
            }
        }
        if let Some((key, value)) = token.split_once('=') {
            if key == name {
                return Some(
                    serde_json::from_str::<String>(value).unwrap_or_else(|_| value.to_string()),
                );
            }
        }
    }
    None
}

pub(crate) struct CsvRecordWriter {
    options: CsvWriterOptions,
    columns: Option<Vec<String>>,
    object_rows: bool,
    started: bool,
}
impl CsvRecordWriter {
    pub(crate) fn new(directive: &str) -> Self {
        Self::with_options(CsvWriterOptions::from_directive(directive))
    }
    fn with_options(options: CsvWriterOptions) -> Self {
        Self {
            options,
            columns: None,
            object_rows: false,
            started: false,
        }
    }
    pub(crate) fn write_record<W: Write>(
        &mut self,
        value: &Value,
        writer: &mut W,
    ) -> Result<(), DwError> {
        let mut output = String::new();
        if !self.started {
            self.started = true;
            if let Value::Object(map) = value {
                self.object_rows = true;
                let columns = self
                    .options
                    .columns
                    .clone()
                    .unwrap_or_else(|| map.keys().cloned().collect());
                if self.options.header {
                    write_row(
                        &mut output,
                        columns.iter().map(String::as_str),
                        &self.options,
                        self.options.quote_header,
                    );
                }
                self.columns = Some(columns);
            }
        }
        if self.object_rows {
            let Value::Object(map) = value else {
                return Err(DwError::UnsupportedFeature(
                    "CSV writer mixed row types".into(),
                ));
            };
            let cells: Vec<String> = self
                .columns
                .as_ref()
                .unwrap()
                .iter()
                .map(|c| csv_cell_text(map.get(c).unwrap_or(&Value::Null)))
                .collect();
            write_row(
                &mut output,
                cells.iter().map(String::as_str),
                &self.options,
                self.options.quote_values,
            );
        } else {
            let cells: Vec<String> = match value {
                Value::Array(items) => items.iter().map(csv_cell_text).collect(),
                other => vec![csv_cell_text(other)],
            };
            write_row(
                &mut output,
                cells.iter().map(String::as_str),
                &self.options,
                self.options.quote_values,
            );
        }
        writer
            .write_all(output.as_bytes())
            .map_err(|e| DwError::Output(e.to_string()))
    }
}
fn csv_cell_text(value: &Value) -> String {
    match value {
        Value::Null => String::new(),
        Value::String(v) => v.clone(),
        Value::Bool(v) => v.to_string(),
        Value::Number(v) => v.to_string(),
        other => serde_json::to_string(other).unwrap_or_else(|_| as_dataweave_string(other)),
    }
}
fn write_row<'a>(
    output: &mut String,
    values: impl Iterator<Item = &'a str>,
    options: &CsvWriterOptions,
    quoted: bool,
) {
    for (index, value) in values.enumerate() {
        if index > 0 {
            output.push(options.separator);
        }
        if quoted {
            output.push(options.quote);
        }
        for ch in value.chars() {
            if ch == options.escape
                || ch == options.quote
                || (!quoted && (ch == options.separator || ch == '\n' || ch == '\r'))
            {
                output.push(options.escape);
            }
            output.push(ch);
        }
        if quoted {
            output.push(options.quote);
        }
    }
    output.push_str(&options.line_separator);
}
fn render(value: &Value, options: CsvWriterOptions) -> Result<String, DwError> {
    let mut bytes = Vec::new();
    let mut writer = CsvRecordWriter::with_options(options);
    match value {
        Value::Object(_) => writer.write_record(value, &mut bytes)?,
        Value::Array(rows) => {
            for row in rows {
                writer.write_record(row, &mut bytes)?;
            }
        }
        _ => {
            return Err(DwError::UnsupportedFeature(
                "CSV writer expects a list or dict value".into(),
            ))
        }
    }
    Ok(String::from_utf8(bytes).expect("CSV is UTF-8"))
}
pub(crate) fn render_csv_output(value: &Value, directive: &str) -> Result<String, DwError> {
    render(value, CsvWriterOptions::from_directive(directive))
}
pub(crate) fn render_csv_with_options(value: &Value, options: &Value) -> Result<String, DwError> {
    render(value, CsvWriterOptions::from_value(options))
}

pub(crate) fn read_csv(
    text: &str,
    options: &Value,
    headerless_objects: bool,
) -> Result<Value, DwError> {
    let map = options.as_object();
    let get = |name: &str| map.and_then(|m| m.get(name)).map(as_dataweave_string);
    let separator = get("separator")
        .and_then(|v| v.chars().next())
        .unwrap_or(',');
    let quote = get("quote").and_then(|v| v.chars().next()).unwrap_or('"');
    let escape = get("escape").and_then(|v| v.chars().next()).unwrap_or('\\');
    let flag = |name, default| {
        get(name)
            .map(|v| matches!(v.as_str(), "true" | "True" | "1" | "yes"))
            .unwrap_or(default)
    };
    let header = flag("header", true);
    let ignore_empty = flag("ignoreEmptyLine", true);
    let mut records: Vec<Vec<String>> = Vec::new();
    let mut row = Vec::new();
    let mut cell = String::new();
    let mut in_quote = false;
    let mut chars = text.chars().peekable();
    let mut has_chars = false;
    let mut non_whitespace_source = false;
    while let Some(ch) = chars.next() {
        if ch != '\n' && ch != '\r' && !ch.is_whitespace() {
            non_whitespace_source = true;
        }
        if ch == escape {
            if let Some(next) = chars.next() {
                cell.push(next);
                has_chars = true;
            } else {
                cell.push(ch);
                has_chars = true;
            }
        } else if ch == quote {
            in_quote = !in_quote;
            has_chars = true;
        } else if ch == separator && !in_quote {
            row.push(std::mem::take(&mut cell));
            has_chars = true;
        } else if (ch == '\n' || ch == '\r') && !in_quote {
            if ch == '\r' && chars.peek() == Some(&'\n') {
                chars.next();
            }
            row.push(std::mem::take(&mut cell));
            if !ignore_empty || has_chars && non_whitespace_source {
                records.push(std::mem::take(&mut row));
            } else {
                row.clear();
            }
            has_chars = false;
            non_whitespace_source = false;
        } else {
            cell.push(ch);
            has_chars = true;
        }
    }
    if has_chars || !row.is_empty() {
        row.push(cell);
        if !ignore_empty || non_whitespace_source {
            records.push(row);
        }
    }
    let headers = if header && !records.is_empty() {
        records.remove(0)
    } else {
        Vec::new()
    };
    let rows = records
        .into_iter()
        .map(|cells| {
            if !header && !headerless_objects {
                Value::Array(cells.into_iter().map(Value::String).collect())
            } else {
                Value::Object(Map::from_iter(cells.into_iter().enumerate().map(
                    |(i, cell)| {
                        (
                            headers
                                .get(i)
                                .cloned()
                                .unwrap_or_else(|| format!("column_{i}")),
                            Value::String(cell),
                        )
                    },
                )))
            }
        })
        .collect();
    Ok(Value::Array(rows))
}
