use serde_json::Value;
use std::env;
use std::fs;
use std::path::Path;
use std::process::ExitCode;

const USAGE: &str = "\
usage:
  dw run <script> [--payload TEXT | --payload-file PATH] [--payload-format MIME]
  dw run --file PATH [--payload TEXT | --payload-file PATH] [--payload-format MIME]";

#[derive(Debug, Default)]
struct RunArgs {
    script: Option<String>,
    script_file: Option<String>,
    payload: Option<String>,
    payload_file: Option<String>,
    payload_format: Option<String>,
}

enum CliError {
    Usage(String),
    Execution(String),
}

fn main() -> ExitCode {
    match run(env::args().skip(1).collect()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(CliError::Usage(message)) => {
            eprintln!("{message}");
            eprintln!("{USAGE}");
            ExitCode::from(2)
        }
        Err(CliError::Execution(message)) => {
            eprintln!("{message}");
            ExitCode::from(1)
        }
    }
}

fn run(args: Vec<String>) -> Result<(), CliError> {
    if args.is_empty() {
        return Err(CliError::Usage("missing command".to_string()));
    }
    if matches!(args[0].as_str(), "-h" | "--help") {
        println!("{USAGE}");
        return Ok(());
    }
    match args[0].as_str() {
        "run" => run_command(&args[1..]),
        other => Err(CliError::Usage(format!("unknown command `{other}`"))),
    }
}

fn run_command(args: &[String]) -> Result<(), CliError> {
    if args.iter().any(|value| matches!(value.as_str(), "-h" | "--help")) {
        println!("{USAGE}");
        return Ok(());
    }

    let parsed = parse_run_args(args)?;
    let script_source = load_script_source(&parsed)?;
    let renders_text = script_renders_text(&script_source);
    let (payload, payload_format, attempt_inline_json) = load_payload(&parsed)?;
    let payload = if let Some(format) = payload_format.as_deref() {
        dwpy_core::parse_payload_format_with_options(Value::String(payload), Some(format), None)
            .map_err(|err| CliError::Execution(err.to_string()))?
    } else if attempt_inline_json {
        parse_inline_json_payload(payload)
    } else {
        Value::String(payload)
    };

    let output = dwpy_core::execute_json(&script_source, payload, true)
        .map_err(|err| CliError::Execution(err.to_string()))?;

    if renders_text {
        match output {
            Value::String(text) => {
                print!("{text}");
                Ok(())
            }
            other => Err(CliError::Execution(format!(
                "expected rendered text output, got {other:?}"
            ))),
        }
    } else {
        let rendered = serde_json::to_string_pretty(&output)
            .map_err(|err| CliError::Execution(err.to_string()))?;
        println!("{rendered}");
        Ok(())
    }
}

fn parse_run_args(args: &[String]) -> Result<RunArgs, CliError> {
    let mut parsed = RunArgs::default();
    let mut index = 0usize;
    while index < args.len() {
        match args[index].as_str() {
            "--file" => {
                index += 1;
                let value = args
                    .get(index)
                    .ok_or_else(|| CliError::Usage("missing value for `--file`".to_string()))?;
                if parsed.script.is_some() || parsed.script_file.is_some() {
                    return Err(CliError::Usage(
                        "provide either an inline script or `--file`, not both".to_string(),
                    ));
                }
                parsed.script_file = Some(value.clone());
            }
            "--payload" => {
                index += 1;
                let value = args
                    .get(index)
                    .ok_or_else(|| CliError::Usage("missing value for `--payload`".to_string()))?;
                if parsed.payload.is_some() || parsed.payload_file.is_some() {
                    return Err(CliError::Usage(
                        "provide either `--payload` or `--payload-file`, not both".to_string(),
                    ));
                }
                parsed.payload = Some(value.clone());
            }
            "--payload-file" => {
                index += 1;
                let value = args.get(index).ok_or_else(|| {
                    CliError::Usage("missing value for `--payload-file`".to_string())
                })?;
                if parsed.payload.is_some() || parsed.payload_file.is_some() {
                    return Err(CliError::Usage(
                        "provide either `--payload` or `--payload-file`, not both".to_string(),
                    ));
                }
                parsed.payload_file = Some(value.clone());
            }
            "--payload-format" => {
                index += 1;
                let value = args.get(index).ok_or_else(|| {
                    CliError::Usage("missing value for `--payload-format`".to_string())
                })?;
                parsed.payload_format = Some(value.clone());
            }
            value if value.starts_with('-') => {
                return Err(CliError::Usage(format!("unknown option `{value}`")));
            }
            value => {
                if parsed.script.is_some() || parsed.script_file.is_some() {
                    return Err(CliError::Usage(
                        "provide exactly one script source".to_string(),
                    ));
                }
                parsed.script = Some(value.to_string());
            }
        }
        index += 1;
    }

    if parsed.script.is_none() && parsed.script_file.is_none() {
        return Err(CliError::Usage(
            "missing script source: pass an inline script or `--file`".to_string(),
        ));
    }

    Ok(parsed)
}

fn load_script_source(args: &RunArgs) -> Result<String, CliError> {
    if let Some(path) = args.script_file.as_deref() {
        return fs::read_to_string(path)
            .map_err(|err| CliError::Execution(format!("failed to read `{path}`: {err}")));
    }
    args.script
        .clone()
        .ok_or_else(|| CliError::Usage("missing script source".to_string()))
}

fn load_payload(args: &RunArgs) -> Result<(String, Option<String>, bool), CliError> {
    if let Some(payload) = args.payload.as_deref() {
        return Ok((payload.to_string(), args.payload_format.clone(), true));
    }

    if let Some(path) = args.payload_file.as_deref() {
        let payload = fs::read_to_string(path)
            .map_err(|err| CliError::Execution(format!("failed to read `{path}`: {err}")))?;
        let format = args
            .payload_format
            .clone()
            .or_else(|| infer_payload_format_from_path(path));
        return Ok((payload, format, false));
    }

    Ok(("{}".to_string(), None, true))
}

fn infer_payload_format_from_path(path: &str) -> Option<String> {
    let extension = Path::new(path)
        .extension()
        .and_then(|value| value.to_str())?
        .to_ascii_lowercase();
    match extension.as_str() {
        "json" => Some("application/json".to_string()),
        "ndjson" | "ldjson" => Some("application/x-ndjson".to_string()),
        "xml" => Some("application/xml".to_string()),
        "csv" => Some("application/csv".to_string()),
        "yaml" | "yml" => Some("application/yaml".to_string()),
        _ => None,
    }
}

fn parse_inline_json_payload(payload: String) -> Value {
    serde_json::from_str(&payload).unwrap_or(Value::String(payload))
}

fn script_renders_text(script: &str) -> bool {
    script_output_directive(script)
        .and_then(|directive| directive.split_whitespace().next().map(str::to_string))
        .is_some_and(|format| !matches!(format.as_str(), "application/python" | "python" | "text/x-python"))
}

fn script_output_directive(script: &str) -> Option<String> {
    let header_source = if let Some((start, _)) = dwpy_core::parse_script_boundary_span(script) {
        &script[..start]
    } else {
        script
    };

    header_source
        .lines()
        .map(str::trim)
        .find_map(|line| line.strip_prefix("output ").map(str::trim))
        .map(str::to_string)
}
