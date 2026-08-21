use std::env;
use std::fs;
use std::time::Instant;

use serde::Serialize;
use serde_json::{json, Value};

#[derive(Clone)]
struct Scenario {
    name: &'static str,
    script: &'static str,
    payload: Value,
    payload_format: Option<&'static str>,
    render_output: bool,
    normalize_json_text: bool,
}

#[derive(Serialize)]
struct ScenarioResult {
    scenario: &'static str,
    rust_ms: Vec<f64>,
    normalized_output: Value,
}

#[derive(Serialize)]
struct BenchmarkOutput {
    rust_mode: &'static str,
    warmup: usize,
    iterations: usize,
    results: Vec<ScenarioResult>,
}

fn main() {
    let args = parse_args();
    let scenarios = build_scenarios();
    let results = scenarios
        .iter()
        .map(|scenario| run_scenario(scenario, args.warmup, args.iterations))
        .collect::<Result<Vec<_>, _>>()
        .unwrap_or_else(|err| {
            eprintln!("{err}");
            std::process::exit(1);
        });
    let output = BenchmarkOutput {
        rust_mode: "pure-dwpy-core",
        warmup: args.warmup,
        iterations: args.iterations,
        results,
    };
    let encoded = serde_json::to_string_pretty(&output).unwrap_or_else(|err| {
        eprintln!("{err}");
        std::process::exit(1);
    });
    fs::write(&args.json_output, encoded).unwrap_or_else(|err| {
        eprintln!("failed to write {}: {err}", args.json_output);
        std::process::exit(1);
    });
}

struct Args {
    warmup: usize,
    iterations: usize,
    json_output: String,
}

fn parse_args() -> Args {
    let mut warmup = 20usize;
    let mut iterations = 200usize;
    let mut json_output = None;
    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--warmup" => {
                warmup = args
                    .next()
                    .and_then(|value| value.parse().ok())
                    .unwrap_or_else(|| exit_usage("--warmup expects an integer"));
            }
            "--iterations" => {
                iterations = args
                    .next()
                    .and_then(|value| value.parse().ok())
                    .unwrap_or_else(|| exit_usage("--iterations expects an integer"));
            }
            "--json-output" => {
                json_output = args.next();
            }
            _ => exit_usage(&format!("unknown argument {arg}")),
        }
    }
    let Some(json_output) = json_output else {
        exit_usage("--json-output is required");
    };
    if iterations == 0 {
        exit_usage("--iterations must be > 0");
    }
    Args {
        warmup,
        iterations,
        json_output,
    }
}

fn exit_usage(message: &str) -> ! {
    eprintln!("{message}");
    eprintln!("usage: perf_pure_rust --json-output PATH [--warmup N] [--iterations N]");
    std::process::exit(2);
}

fn run_scenario(
    scenario: &Scenario,
    warmup: usize,
    iterations: usize,
) -> Result<ScenarioResult, String> {
    for _ in 0..warmup {
        execute(scenario)?;
    }
    let mut rust_ms = Vec::with_capacity(iterations);
    for _ in 0..iterations {
        let start = Instant::now();
        execute(scenario)?;
        rust_ms.push(start.elapsed().as_secs_f64() * 1000.0);
    }
    let normalized_output = normalize_output(scenario, execute(scenario)?)?;
    Ok(ScenarioResult {
        scenario: scenario.name,
        rust_ms,
        normalized_output,
    })
}

fn execute(scenario: &Scenario) -> Result<Value, String> {
    let payload =
        dwpy_core::parse_payload_format(scenario.payload.clone(), scenario.payload_format)
            .map_err(|err| format!("{} payload parse failed: {err}", scenario.name))?;
    dwpy_core::execute_json(scenario.script, payload, scenario.render_output)
        .map_err(|err| format!("{} execution failed: {err}", scenario.name))
}

fn normalize_output(scenario: &Scenario, output: Value) -> Result<Value, String> {
    let output = if scenario.normalize_json_text {
        let Value::String(text) = output else {
            return Err(format!(
                "{} expected JSON text output, got {output:?}",
                scenario.name
            ));
        };
        serde_json::from_str(&text)
            .map_err(|err| format!("{} JSON normalization failed: {err}", scenario.name))?
    } else {
        output
    };
    Ok(normalize_temporal_precision(output))
}

fn normalize_temporal_precision(value: Value) -> Value {
    match value {
        Value::String(text) => Value::String(trim_temporal_fraction_zeros(&text)),
        Value::Array(items) => Value::Array(
            items
                .into_iter()
                .map(normalize_temporal_precision)
                .collect(),
        ),
        Value::Object(map) => Value::Object(
            map.into_iter()
                .map(|(key, value)| (key, normalize_temporal_precision(value)))
                .collect(),
        ),
        other => other,
    }
}

fn trim_temporal_fraction_zeros(text: &str) -> String {
    let Some(dot_index) = text.find('.') else {
        return text.to_string();
    };
    let after_dot = &text[dot_index + 1..];
    let digit_count = after_dot
        .chars()
        .take_while(|ch| ch.is_ascii_digit())
        .count();
    if digit_count == 0 {
        return text.to_string();
    }
    let fraction_end = dot_index + 1 + digit_count;
    let suffix = &text[fraction_end..];
    if !(suffix.is_empty()
        || suffix == "Z"
        || (suffix.len() == 6
            && matches!(suffix.as_bytes().first(), Some(b'+') | Some(b'-'))
            && suffix.as_bytes().get(3) == Some(&b':')))
    {
        return text.to_string();
    }
    let fraction = &text[dot_index + 1..fraction_end];
    let trimmed = fraction.trim_end_matches('0');
    if trimmed.is_empty() {
        format!("{}{}", &text[..dot_index], suffix)
    } else {
        format!("{}.{}{}", &text[..dot_index], trimmed, suffix)
    }
}

fn build_scenarios() -> Vec<Scenario> {
    let records = (0..500)
        .map(|index| {
            json!({
                "id": index,
                "name": format!("user-{index}"),
                "department": if index % 3 == 0 { "OPS" } else { "ENG" },
                "active": index % 4 != 0,
                "score": (index % 17) + 1,
                "price": format!("{}", (index % 11) as f64 + 0.5),
                "tags": if index % 2 == 0 { json!(["legacy"]) } else { json!(["core", "rust"]) },
            })
        })
        .collect::<Vec<_>>();
    let table_rows = (0..300)
        .map(|index| {
            json!({
                "name": format!("User {index}"),
                "age": 20 + (index % 30),
                "city": "BA",
            })
        })
        .collect::<Vec<_>>();
    let nested_orders = (0..120)
        .map(|index| {
            json!({
                "id": index,
                "customer": {
                    "name": format!("Customer {index}"),
                    "tier": if index % 2 == 0 { "std" } else { "gold" },
                },
                "items": (0..3).map(|item| {
                    json!({
                        "sku": format!("SKU-{index}-{item}"),
                        "qty": item + 1,
                        "price": 10 + item,
                    })
                }).collect::<Vec<_>>(),
            })
        })
        .collect::<Vec<_>>();
    let aggregation_orders = (0..200)
        .map(|index| {
            json!({
                "id": index,
                "customer": {"name": format!("Customer {index}")},
                "items": (0..5).map(|item| {
                    json!({
                        "sku": format!("SKU-{index}-{item}"),
                        "qty": item + 1,
                        "price": 10 + item,
                    })
                }).collect::<Vec<_>>(),
            })
        })
        .collect::<Vec<_>>();
    let grouped_records = (0..500)
        .map(|index| {
            json!({
                "id": index,
                "department": if index % 3 == 0 { "OPS" } else { "ENG" },
                "score": 500 - (index % 500),
            })
        })
        .collect::<Vec<_>>();
    let object_map_payload = (0..400)
        .map(|index| {
            (
                format!("user{index}"),
                json!({
                    "name": format!("user {index}"),
                    "active": index % 2 == 0,
                }),
            )
        })
        .collect::<serde_json::Map<_, _>>();
    let zip_a = (0..400).map(Value::from).collect::<Vec<_>>();
    let zip_b = (0..400)
        .map(|index| Value::String(format!("value-{index}")))
        .collect::<Vec<_>>();
    let zip_pairs = (0..400)
        .map(|index| json!([index, format!("value-{index}")]))
        .collect::<Vec<_>>();
    let xml_payload = format!(
        "<catalog>{}</catalog>",
        (0..80)
            .map(|index| format!(
                r#"<book id="{index}"><title>Title {index}</title><price>{}</price></book>"#,
                index + 1
            ))
            .collect::<String>()
    );

    vec![
        Scenario {
            name: "range_reduce_10k",
            script: r#"%dw 2.0
output application/python
---
1 to 10000 reduce ((item, accum = 0) -> item + accum)
"#,
            payload: Value::Null,
            payload_format: None,
            render_output: false,
            normalize_json_text: false,
        },
        Scenario {
            name: "range_reduce_100k",
            script: r#"%dw 2.0
output application/python
---
1 to 100000 reduce ((item, accum = 0) -> item + accum)
"#,
            payload: Value::Null,
            payload_format: None,
            render_output: false,
            normalize_json_text: false,
        },
        Scenario {
            name: "range_reduce_1m",
            script: r#"%dw 2.0
output application/python
---
1 to 1000000 reduce ((item, accum = 0) -> item + accum)
"#,
            payload: Value::Null,
            payload_format: None,
            render_output: false,
            normalize_json_text: false,
        },
        Scenario {
            name: "basic_projection",
            script: r#"%dw 2.0
output application/python
---
{
  id: payload.orderId,
  status: upper(payload.status default "pending"),
  city: payload.customer.address.city default "UNKNOWN"
}
"#,
            payload: json!({"orderId": "A123", "customer": {"address": {"city": "BA"}}}),
            payload_format: None,
            render_output: false,
            normalize_json_text: false,
        },
        Scenario {
            name: "array_map_filter_reduce",
            script: r#"%dw 2.0
output application/python
---
{
  activeNames: (payload filter ((item) -> item.active == true)) map ((item) -> upper(item.name)),
  scoreTotal: sum(payload map ((item) -> item.score)),
  flattenedTags: flatten(payload map ((item) -> item.tags))
}
"#,
            payload: Value::Array(records),
            payload_format: None,
            render_output: false,
            normalize_json_text: false,
        },
        Scenario {
            name: "object_helpers",
            script: r#"%dw 2.0
import * from dw::core::Objects
output application/python
---
{
  entries: entrySet(payload.primary),
  names: nameSet(payload.primary),
  merged: mergeWith(payload.primary, payload.patch),
  divided: divideBy(payload.primary, 2),
  taken: takeWhile(payload.primary, (value, key) -> value < 4),
  every: everyEntry(payload.primary, (value, key) -> value < 10),
  some: someEntry(payload.primary, (value, key) -> value > 3)
}
"#,
            payload: json!({"primary": {"a": 1, "b": 2, "c": 5, "d": 8}, "patch": {"b": 7, "e": 9}}),
            payload_format: None,
            render_output: false,
            normalize_json_text: false,
        },
        Scenario {
            name: "xml_selectors",
            script: r#"%dw 2.0
output application/python
---
{
  firstTitle: payload.catalog.book.title,
  titles: payload.catalog.*book map ((book) -> book.title),
  ids: payload.catalog.*book map ((book) -> book.@id)
}
"#,
            payload: Value::String(xml_payload),
            payload_format: Some("application/xml"),
            render_output: false,
            normalize_json_text: false,
        },
        Scenario {
            name: "json_render",
            script: r#"%dw 2.0
output application/json
---
{
  orderCount: sizeOf(payload.orders),
  customers: payload.orders map ((order) -> {
    id: order.id,
    name: upper(order.customer.name),
    firstSku: order.items[0].sku
  })
}
"#,
            payload: json!({"orders": nested_orders}),
            payload_format: None,
            render_output: true,
            normalize_json_text: true,
        },
        Scenario {
            name: "csv_render",
            script: r#"%dw 2.0
output application/csv header=true
---
payload
"#,
            payload: Value::Array(table_rows),
            payload_format: None,
            render_output: true,
            normalize_json_text: false,
        },
        Scenario {
            name: "crypto_hash",
            script: r#"%dw 2.0
import * from dw::Crypto
output application/python
---
hashWith("hello" as Binary, "SHA-256")
"#,
            payload: json!({}),
            payload_format: None,
            render_output: false,
            normalize_json_text: false,
        },
        Scenario {
            name: "period_arithmetic",
            script: r#"%dw 2.0
import * from dw::core::Periods
output application/json
---
{
  periodValue: years(4),
  nextMonth: |2020-10-05T20:22:34.385000Z| + months(1),
  nextHour: |2020-10-05T20:22:34.385000Z| + hours(1),
  betweenValue: between(|2011-12-11|, |2010-11-10|)
}
"#,
            payload: json!({}),
            payload_format: None,
            render_output: true,
            normalize_json_text: true,
        },
        Scenario {
            name: "nested_selectors_aggregation",
            script: r#"%dw 2.0
output application/python
---
payload.orders map ((order) -> {
  id: order.id,
  name: upper(order.customer.name default "UNKNOWN"),
  totalQty: sum(order.items map ((item) -> item.qty)),
  expensiveSkus: (order.items filter ((item) -> item.price > 12)) map ((item) -> item.sku)
})
"#,
            payload: json!({"orders": aggregation_orders}),
            payload_format: None,
            render_output: false,
            normalize_json_text: false,
        },
        Scenario {
            name: "group_order_distinct",
            script: r#"%dw 2.0
output application/python
---
{
  grouped: payload.items groupBy ((item) -> item.department),
  distinct: payload.items distinctBy ((item) -> item.department),
  ordered: payload.items orderBy ((item) -> item.score)
}
"#,
            payload: json!({"items": grouped_records}),
            payload_format: None,
            render_output: false,
            normalize_json_text: false,
        },
        Scenario {
            name: "object_map_filter",
            script: r#"%dw 2.0
output application/python
---
payload mapObject ((value, key) -> if (value.active) {(key): upper(value.name)} else {})
"#,
            payload: Value::Object(object_map_payload),
            payload_format: None,
            render_output: false,
            normalize_json_text: false,
        },
        Scenario {
            name: "zip_unzip",
            script: r#"%dw 2.0
output application/python
---
{
  zipped: zip(payload.a, payload.b),
  unzipped: unzip(payload.pairs)
}
"#,
            payload: json!({"a": zip_a, "b": zip_b, "pairs": zip_pairs}),
            payload_format: None,
            render_output: false,
            normalize_json_text: false,
        },
        Scenario {
            name: "string_functions",
            script: r#"%dw 2.0
output application/python
---
{
  uppered: upper(payload.text),
  split: payload.text splitBy " ",
  starts: payload.text startsWith "alpha",
  containsValue: payload.text contains "beta"
}
"#,
            payload: json!({"text": (0..300).map(|_| "alpha beta gamma").collect::<Vec<_>>().join(" ")}),
            payload_format: None,
            render_output: false,
            normalize_json_text: false,
        },
    ]
}
