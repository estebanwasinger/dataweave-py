use serde_json::Map;
use serde_json::Value;

use crate::{as_dataweave_string, output_bool_option, output_option, DwError};

pub(crate) fn render_csv_output(value: &Value, directive: &str) -> Result<String, DwError> {
    let separator = output_option(directive, "separator")
        .and_then(|value| value.chars().next())
        .unwrap_or(',');
    let quote = output_option(directive, "quote")
        .and_then(|value| value.chars().next())
        .unwrap_or('"');
    let include_header = output_bool_option(directive, "header", true);

    let rows = match value {
        Value::Object(_) => vec![value],
        Value::Array(items) => items.iter().collect::<Vec<_>>(),
        _ => {
            return Err(DwError::UnsupportedFeature(
                "CSV writer expects a list or dict value".to_string(),
            ))
        }
    };

    let mut output = String::new();
    if rows.is_empty() {
        return Ok(output);
    }

    if let Value::Object(first) = rows[0] {
        let columns = output_option(directive, "columns")
            .map(|columns| {
                columns
                    .split(',')
                    .map(str::trim)
                    .filter(|column| !column.is_empty())
                    .map(str::to_string)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_else(|| first.keys().cloned().collect::<Vec<_>>());
        if include_header {
            write_csv_text_row(
                &mut output,
                columns.iter().map(String::as_str),
                separator,
                quote,
            );
        }
        for row in rows {
            let Value::Object(map) = row else {
                return Err(DwError::UnsupportedFeature(
                    "CSV writer mixed row types".to_string(),
                ));
            };
            write_csv_row(
                &mut output,
                columns
                    .iter()
                    .map(|column| map.get(column).unwrap_or(&Value::Null)),
                separator,
                quote,
            );
        }
    } else {
        for row in rows {
            match row {
                Value::Array(items) => write_csv_row(&mut output, items.iter(), separator, quote),
                value => write_csv_row(&mut output, std::iter::once(value), separator, quote),
            }
        }
    }
    Ok(output)
}

fn write_csv_row<'a>(
    output: &mut String,
    values: impl Iterator<Item = &'a Value>,
    separator: char,
    quote: char,
) {
    write_csv_text_row(
        output,
        values.map(|value| csv_cell_text(value).into_owned()),
        separator,
        quote,
    );
}

fn write_csv_text_row<S: AsRef<str>>(
    output: &mut String,
    values: impl Iterator<Item = S>,
    separator: char,
    quote: char,
) {
    let mut first = true;
    for value in values {
        if !first {
            output.push(separator);
        }
        first = false;
        write_escaped_csv_cell(output, value.as_ref(), separator, quote);
    }
    output.push('\n');
}

fn csv_cell_text(value: &Value) -> std::borrow::Cow<'_, str> {
    match value {
        Value::Null => std::borrow::Cow::Borrowed(""),
        Value::String(value) => std::borrow::Cow::Borrowed(value),
        Value::Bool(value) => std::borrow::Cow::Owned(value.to_string()),
        Value::Number(value) => std::borrow::Cow::Owned(value.to_string()),
        other => std::borrow::Cow::Owned(
            serde_json::to_string(other).unwrap_or_else(|_| as_dataweave_string(other)),
        ),
    }
}

fn write_escaped_csv_cell(output: &mut String, value: &str, separator: char, quote: char) {
    if value.contains(separator) || value.contains(quote) || value.contains(['\n', '\r']) {
        output.push(quote);
        for ch in value.chars() {
            if ch == quote {
                output.push(quote);
            }
            output.push(ch);
        }
        output.push(quote);
    } else {
        output.push_str(value);
    }
}

pub(crate) fn read_simple_csv(
    text: &str,
    separator: char,
    quote: char,
    has_header: bool,
) -> Result<Value, DwError> {
    let mut lines = text.lines().filter(|line| !line.trim().is_empty());
    let Some(header_line) = lines.next() else {
        return Ok(Value::Array(Vec::new()));
    };
    let first_row = split_simple_delimited_row(header_line, separator, quote);
    if !has_header {
        let rows = std::iter::once(first_row)
            .chain(lines.map(|line| split_simple_delimited_row(line, separator, quote)))
            .map(|cells| Value::Array(cells.into_iter().map(Value::String).collect()))
            .collect::<Vec<_>>();
        return Ok(Value::Array(rows));
    }
    let headers = first_row;
    let rows = lines
        .map(|line| {
            let cells = split_simple_delimited_row(line, separator, quote);
            let mut row = Map::new();
            for (index, header) in headers.iter().enumerate() {
                let value = cells
                    .get(index)
                    .map(|cell| Value::String(cell.clone()))
                    .unwrap_or(Value::Null);
                row.insert(header.clone(), value);
            }
            Value::Object(row)
        })
        .collect::<Vec<_>>();
    Ok(Value::Array(rows))
}

fn split_simple_delimited_row(line: &str, separator: char, quote: char) -> Vec<String> {
    split_simple_delimited_row_with_options(line, separator, quote, true)
}

pub(crate) fn split_simple_delimited_row_preserving_cells(
    line: &str,
    separator: char,
    quote: char,
) -> Vec<String> {
    split_simple_delimited_row_with_options(line, separator, quote, false)
}

fn split_simple_delimited_row_with_options(
    line: &str,
    separator: char,
    quote: char,
    trim_cells: bool,
) -> Vec<String> {
    let mut cells = Vec::new();
    let mut current = String::new();
    let mut chars = line.chars().peekable();
    let mut in_quote = false;
    while let Some(ch) = chars.next() {
        if ch == quote {
            if in_quote && chars.peek() == Some(&quote) {
                chars.next();
                current.push(quote);
            } else {
                in_quote = !in_quote;
            }
            continue;
        }
        if ch == separator && !in_quote {
            cells.push(csv_cell_source(&current, trim_cells));
            current.clear();
            continue;
        }
        current.push(ch);
    }
    cells.push(csv_cell_source(&current, trim_cells));
    cells
}

fn csv_cell_source(cell: &str, trim_cells: bool) -> String {
    if trim_cells {
        cell.trim().to_string()
    } else {
        cell.to_string()
    }
}
