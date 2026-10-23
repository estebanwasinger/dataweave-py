# Native temporal values in the Rust evaluator

The evaluator, compiled collection paths, scopes, functions, selectors, and
writers pass `DwValue`. JSON is an external format and bridge representation.

`DwValue` distinguishes `Date`, `LocalDateTime`, `DateTime`, `LocalTime`, `Time`,
`TimeZone`, and `Period`. Calendar dates/times use parsed Chrono values; zoned
values carry a fixed UTC offset. Fractional precision is retained up to nine
digits in the Rust core. A period retains separate years/months and elapsed-time
components, so one calendar month is not approximated as a fixed number of days.

## Rust embedding

Use `execute_value` or a compiled script's `execute_value` /
`execute_value_with_context` methods for typed results. Supply typed variables
and attributes through `ExecutionContext::from_values`.
The `execute_value_to_writer` / `execute_value_to_writer_with_context` sink APIs
also accept native values without a JSON round trip.

The existing `execute_json`, `execute_json_with_context`, and compiled `execute`
adapters still accept and return `serde_json::Value`. Temporal results are ISO
strings, including when output rendering is bypassed. Code that previously
inspected the internal `__dwpy_temporal` / `__dwpy_period` objects should use the
native APIs and match on `DwValue` variants instead. `DwValue::Number` retains
`serde_json::Number` rather than narrowing input numbers to `f64`.

## Host bridges and serialization

Ordinary JSON decoding never interprets marker-shaped user objects as dates.
`DwValue::from_transport_json` and `to_transport_json` are explicit adapters for
the Python bridge and WASM requests with `typed_transport=true`. Only these
adapters interpret or produce temporal tags. User dictionaries containing wire
keys are escaped in an object envelope and restored by the bridge.

Dates/times from the Python host, including variables and attributes, become
native scalars before evaluation. With `render_output=False`, Rust/WASM results
become Python `date`, `datetime`, or `time` values consistently. This also applies
to `now()`, temporal literals, and coercions that previously sometimes returned
strings. Use `.isoformat()` when consuming those unrendered values as text, or
select a rendered output format. Period results retain the existing ISO-string
representation at the Python boundary. Python's temporal types retain
microsecond precision; the Rust core and rendered text/JSON retain nanoseconds.

JSON, NDJSON, YAML, CSV, XML, and plain-text writers format native temporal scalars
without exposing tags. `output text`, `output plain`, and `output text/plain`
render the original date-arithmetic example as `2026-10-29`.

ISO-looking string values remain strings. Coerce them with `as Date`,
`as DateTime`, or the relevant temporal type before arithmetic. Type checks and
`typeOf` distinguish temporal scalars from strings and objects.

Named-zone and daylight-saving behavior is unchanged by this migration. Native
zoned values retain fixed offsets; this change does not add an IANA time-zone
database or expand the previously supported named-zone conversions.
