use serde_json::Map;
use serde_json::Value;

use crate::{as_dataweave_string, output_bool_option, output_option, DwError};

pub(crate) fn render_markdown_output(value: &Value, directive: &str) -> Result<String, DwError> {
    if !output_bool_option(directive, "header", true) {
        return Err(DwError::UnsupportedFeature(
            "Markdown writer requires header=true".to_string(),
        ));
    }
    let rows = match value {
        Value::Object(_) => vec![value],
        Value::Array(items) => items.iter().collect::<Vec<_>>(),
        _ => {
            return Err(DwError::UnsupportedFeature(
                "Markdown writer expects a list or dict value".to_string(),
            ))
        }
    };
    if rows.is_empty() {
        return Ok(String::new());
    }
    if matches!(rows[0], Value::Object(_)) {
        return render_markdown_object_rows(&rows, directive);
    }
    render_markdown_sequence_rows(&rows)
}

fn render_markdown_object_rows(rows: &[&Value], directive: &str) -> Result<String, DwError> {
    let Value::Object(first) = rows[0] else {
        unreachable!();
    };
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
    if columns.is_empty() {
        return Err(DwError::UnsupportedFeature(
            "Markdown writer requires at least one column when writing dictionaries".to_string(),
        ));
    }
    let mut table = Vec::new();
    table.push(
        columns
            .iter()
            .map(|column| markdown_cell_text(&Value::String(column.clone())))
            .collect::<Vec<_>>(),
    );
    for row in rows {
        let Value::Object(map) = row else {
            return Err(DwError::UnsupportedFeature(
                "Markdown writer expects all rows to be dictionaries when the first row is a dictionary"
                    .to_string(),
            ));
        };
        table.push(
            columns
                .iter()
                .map(|column| markdown_cell_text(map.get(column).unwrap_or(&Value::Null)))
                .collect::<Vec<_>>(),
        );
    }
    Ok(render_markdown_table(&table))
}

fn render_markdown_sequence_rows(rows: &[&Value]) -> Result<String, DwError> {
    let materialized = rows
        .iter()
        .map(|row| match row {
            Value::Array(items) => items.iter().collect::<Vec<_>>(),
            value => vec![*value],
        })
        .collect::<Vec<_>>();
    let width = materialized.iter().map(Vec::len).max().unwrap_or(0);
    if width == 0 {
        return Ok(String::new());
    }
    let mut table = Vec::new();
    table.push(
        (1..=width)
            .map(|index| format!("column{index}"))
            .collect::<Vec<_>>(),
    );
    for row in materialized {
        table.push(
            (0..width)
                .map(|index| {
                    row.get(index)
                        .map(|value| markdown_cell_text(value))
                        .unwrap_or_default()
                })
                .collect::<Vec<_>>(),
        );
    }
    Ok(render_markdown_table(&table))
}

fn render_markdown_table(rows: &[Vec<String>]) -> String {
    let width = rows.first().map(Vec::len).unwrap_or(0);
    let widths = (0..width)
        .map(|index| {
            rows.iter()
                .map(|row| row.get(index).map(String::len).unwrap_or(0))
                .max()
                .unwrap_or(0)
                + 2
        })
        .collect::<Vec<_>>();
    let mut output = String::new();
    output.push_str(&render_markdown_table_row(&rows[0], &widths));
    output.push('\n');
    output.push_str(&render_markdown_separator_row(&widths));
    for row in rows.iter().skip(1) {
        output.push('\n');
        output.push_str(&render_markdown_table_row(row, &widths));
    }
    output
}

fn render_markdown_table_row(row: &[String], widths: &[usize]) -> String {
    let cells = widths
        .iter()
        .enumerate()
        .map(|(index, width)| {
            let cell = row.get(index).cloned().unwrap_or_default();
            format!(" {cell:<width$} ")
        })
        .collect::<Vec<_>>();
    format!("|{}|", cells.join("|"))
}

fn render_markdown_separator_row(widths: &[usize]) -> String {
    let cells = widths
        .iter()
        .map(|width| format!(":{}", "-".repeat(*width + 1)))
        .collect::<Vec<_>>();
    format!("|{}|", cells.join("|"))
}

fn markdown_cell_text(value: &Value) -> String {
    let text = match value {
        Value::Null => String::new(),
        Value::String(value) => value.clone(),
        Value::Bool(value) => {
            if *value {
                "True".to_string()
            } else {
                "False".to_string()
            }
        }
        Value::Number(value) => value.to_string(),
        other => as_dataweave_string(other),
    };
    text.replace("\r\n", "\n")
        .replace('\r', "\n")
        .replace('|', "\\|")
        .replace('\n', "<br>")
}

pub(crate) fn read_simple_markdown_table(text: &str, has_header: bool) -> Result<Value, DwError> {
    let rows = text
        .lines()
        .map(str::trim)
        .filter(|line| line.starts_with('|') && line.ends_with('|'))
        .map(|line| {
            line.trim_matches('|')
                .split('|')
                .map(|cell| cell.trim().replace("\\|", "|"))
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    if rows.is_empty() {
        return Ok(Value::Array(Vec::new()));
    }
    let headers = rows[0].clone();
    let data_start = if rows.get(1).is_some_and(|row| {
        row.iter()
            .all(|cell| cell.chars().all(|ch| matches!(ch, '-' | ':')))
    }) {
        2
    } else {
        1
    };
    if has_header {
        Ok(Value::Array(
            rows.into_iter()
                .skip(data_start)
                .map(|cells| {
                    let mut row = Map::new();
                    for (index, header) in headers.iter().enumerate() {
                        row.insert(
                            header.clone(),
                            Value::String(cells.get(index).cloned().unwrap_or_default()),
                        );
                    }
                    Value::Object(row)
                })
                .collect(),
        ))
    } else {
        Ok(Value::Array(
            rows.into_iter()
                .skip(data_start)
                .map(|cells| {
                    Value::Array(
                        (0..headers.len())
                            .map(|index| {
                                Value::String(cells.get(index).cloned().unwrap_or_default())
                            })
                            .collect(),
                    )
                })
                .collect(),
        ))
    }
}
