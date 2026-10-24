# DataWeave OSS

A DataWeave data transformation runtime with a Rust-native engine package, python and WASM , providing powerful data transformation capabilities without requiring the JVM.

Install from PyPI:

```bash
uv add dataweave-py
```

or

```bash
pip install dataweave-py
```

Install the terminal command with uv:

```bash
uv tool install dataweave-py
dw run '%dw 2.0\noutput application/json\n---\n{message: upper(payload.name)}' --payload '{"name":"dw"}'
```

## DataWeave Playground

![DataWeave Playground](./playground-image.png)
For the best DataWeave Playground a without payload size limits DataWeave Playground, visit: [https://dataweavelang.org](https://dataweavelang.org)


Optional extras:

```bash
# pandas helpers (DataFrame/Series input normalization)
pip install "dataweave-py[pandas]"

# pydantic helpers
pip install "dataweave-py[pydantic]"

# everything
pip install "dataweave-py[full]"
```

## Overview

DataWeave-Py (`dwpy`) is a Python-facing interpreter for the DataWeave language, originally developed by MuleSoft for data transformation in the Mule runtime. The runtime is migrating to a Rust core while preserving the existing Python API, enabling:

- **Data transformation**: Convert between JSON, XML, CSV and other formats
- **Functional programming**: Leverage map, filter, reduce, and other functional operators
- **Pattern matching**: Use powerful match expressions with guards and bindings
- **Safe navigation**: Handle null values gracefully with null-safe operators
- **Rich built-ins**: Access 100+ built-in functions for strings, numbers, dates, arrays, and objects

## Requirements

- Python 3.10 or higher
- Rust stable toolchain with `cargo`
- Node.js, npm, `wasm-pack`, and the `wasm32-unknown-unknown` target for the WASM backend
- Dependencies managed via [uv](https://github.com/astral-sh/uv) (recommended) or pip

## Rust Engine And Python Bridge

The default runtime path is the Rust engine exposed through the Python package as
`dwpy._dwpy_rust`. The legacy Python interpreter is still available as an
explicit fallback backend.

Build and install the Rust-backed Python bridge into the local virtual
environment:

```bash
uv venv --python 3.12
source .venv/bin/activate
UV_CACHE_DIR=.uv-cache uv run maturin develop --release
```

Run the Python-packaged CLI from the current checkout:

```bash
uv tool install .
dw run '%dw 2.0\noutput application/python\n---\n{message: upper(payload.name)}' --payload '{"name":"dw"}'
```

Or run it without installing globally:

```bash
uvx --from . dw run '%dw 2.0\noutput application/json\n---\npayload' --payload '{"name":"dw"}'
```

Run the standalone native Rust binary:

```bash
cargo run -p dwpy-cli -- run '%dw 2.0\noutput application/json\n---\npayload' --payload '{"name":"dw"}'
```

Rust embedders can compile once and execute repeatedly. Lazy collection
pipelines are enabled by default, and materializing a lazy result is limited to
256 MiB unless the caller supplies a different budget:

```rust
use dwpy_core::{compile, ExecutionOptions};
use serde_json::Value;

let script = compile(
    "%dw 2.0\noutput application/json\n---\n\
     1 to 1000000 reduce ((item, accum = 0) -> item + accum)",
)?;
let result = script.execute(Value::Null, None, &ExecutionOptions::default())?;
assert_eq!(result, Value::String("500000500000".to_string()));

// Render directly to any std::io::Write sink. Streaming sink failures are
// non-atomic and may leave a valid output prefix in the writer.
let mut bytes = Vec::new();
script.execute_to_writer(
    Value::Null,
    None,
    &ExecutionOptions::default(),
    &mut bytes,
)?;
```

The native CLI uses the sink API for streaming JSON, NDJSON, and CSV output.
Expressions outside the compiled lazy subset continue through the compatible
legacy Rust evaluator.

The evaluator uses `DwValue` throughout execution. Dates, local and zoned
date-times, local and zoned times, time zones, and periods have native variants.
Use `execute_value` or `CompiledScript::execute_value_with_context` to retain
these types in a Rust embedding:

```rust
use dwpy_core::{execute_value, DwValue};

let value = execute_value(
    r#""2026-10-26" as Date + |P3D|"#,
    DwValue::Null,
    false,
)?;
assert!(matches!(value, DwValue::Date(_)));
assert_eq!(value.temporal_text().as_deref(), Some("2026-10-29"));
```

The JSON-facing APIs serialize temporal scalars as ISO strings. Only the explicit
Python/WASM typed transport uses tagged JSON to preserve host types. Ordinary
objects with marker-like keys remain objects. With the Rust or WASM Python
backend, `render_output=False` returns temporal results as Python `date`,
`datetime`, or `time` values, including literals, coercions, and `now()`.
Rendered JSON/text output remains text. See [native temporal values](docs/native-temporal-values.md)
for migration notes and bridge details.

## Homebrew

The Rust-native CLI is distributed through a separate Homebrew tap repository:
`estebanwasinger/homebrew-tap`.

The intended user flow is:

```bash
brew tap estebanwasinger/homebrew-tap
brew install dw-cli
dw run "1 to 1"
```

This repository includes:

- `scripts/build_dw_cli_source_archive.sh` to build the deterministic source
  archive used by the formula
- `.github/workflows/release-dw-cli-source.yml` to upload that archive to a
  GitHub release

The release split is intentional:

- `dataweave-py` releases provide the source archive consumed by the formula
- `homebrew-tap` releases provide the bottled Homebrew binaries

Run the Rust backend from Python:

```python
from dwpy import DataWeaveRuntime

runtime = DataWeaveRuntime(backend="rust")
result = runtime.execute(
    "%dw 2.0\noutput application/json\n---\n{message: upper(payload.message)}",
    {"message": "hello from rust"},
)
print(result)
```

Backend selection:

- `DataWeaveRuntime()` or `backend="auto"` uses the Rust bridge first and falls
  back to the legacy Python backend only for explicitly unsupported migration
  gaps.
- `DataWeaveRuntime(backend="rust")` runs strict Rust mode and fails instead of
  falling back.
- `DataWeaveRuntime(backend="python")` uses the legacy Python interpreter.
- `DataWeaveRuntime(backend="wasm")` runs the Rust evaluator compiled to WASM
  through the Node.js bridge.
- `DWPY_BACKEND=rust` forces strict Rust mode for process-wide test runs.

Build a distributable wheel with the Rust extension:

```bash
UV_CACHE_DIR=.uv-cache uv run maturin build --release
```

Run the Rust workspace tests:

```bash
cargo test --workspace
```

Run the WASM crate's Rust tests inside WebAssembly on Node.js:

```bash
bash scripts/test_wasm_rust.sh
```

This requires Node.js, `wasm-pack`, and the `wasm32-unknown-unknown` Rust
target. It runs the WASM crate's unit tests and integration tests through
`wasm-bindgen-test`, including execution, dependency analysis before execution,
NDJSON errors, Unicode text output, and JWT signing. It does not run every
core Rust unit test in WASM or validate Deno/browser host integration.
Pass a test-name filter after `--` to focus a run, for example
`bash scripts/test_wasm_rust.sh -- infix`.

Run the Python suite against the Rust backend:

```bash
DWPY_BACKEND=rust UV_CACHE_DIR=.uv-cache uv run --extra dev pytest
```

Run the default Python package path, which exercises the Python bridge:

```bash
UV_CACHE_DIR=.uv-cache uv run --extra dev pytest
```

Run the Python suite in all three runtime modes—legacy Python, native Rust,
and Rust compiled to WASM:

```bash
bash scripts/test_backends.sh
```

The script also runs the Rust tests inside WASM and the Node.js package tests,
rebuilds the WASM package before its test pass, and preserves the individual
pytest results. It accepts normal pytest arguments for focused
runs, for example:

```bash
bash scripts/test_backends.sh tests/test_runtime_basic.py -k dynamic
```

## Quick Start

### Basic Usage

```python
from dwpy import DataWeaveRuntime

# Create a runtime instance
runtime = DataWeaveRuntime()

# Define a DataWeave script
script = """%dw 2.0
output application/json
---
{
  message: "Hello, " ++ upper(payload.name),
  timestamp: now()
}
"""

# Execute with a payload
payload = {"name": "world"}
result = runtime.execute(script, payload)

print(result)
# Output: {'message': 'Hello, WORLD', 'timestamp': '2025-11-03T...Z'}
```

### Data Transformation Example

```python
from dwpy import DataWeaveRuntime

runtime = DataWeaveRuntime()

# Transform and enrich order data
script = """%dw 2.0
output application/json
---
{
  orderId: payload.id,
  status: upper(payload.status default "pending"),
  total: payload.items reduce ((item, acc = 0) -> 
    acc + (item.price * (item.quantity default 1))
  ),
  itemCount: sizeOf(payload.items)
}
"""

payload = {
    "id": "ORD-123",
    "status": "confirmed",
    "items": [
        {"price": 29.99, "quantity": 2},
        {"price": 15.50, "quantity": 1}
    ]
}

result = runtime.execute(script, payload)
print(result)
# Output: {'orderId': 'ORD-123', 'status': 'CONFIRMED', 'total': 75.48, 'itemCount': 2}
```

### Using Variables

```python
from dwpy import DataWeaveRuntime

runtime = DataWeaveRuntime()

script = """%dw 2.0
output application/json
var requestTime = vars.requestTime default now()
---
{
  user: payload.userId,
  processedAt: requestTime
}
"""

payload = {"userId": "U-456"}
vars = {"requestTime": "2024-05-05T12:00:00Z"}

result = runtime.execute(script, payload, vars=vars)
```

### Pattern Matching

```python
from dwpy import DataWeaveRuntime

runtime = DataWeaveRuntime()

script = """%dw 2.0
output application/json
---
{
  category: payload.price match {
    case var p when p > 100 -> "premium",
    case var p when p > 50 -> "standard",
    else -> "budget"
  }
}
"""

result = runtime.execute(script, {"price": 75})
# Output: {'category': 'standard'}
```

### String Interpolation

```python
from dwpy import DataWeaveRuntime

runtime = DataWeaveRuntime()

# Simple interpolation
script = """%dw 2.0
output application/json
---
{
  greeting: "Hello $(payload.name)!",
  total: "Total: $(payload.price * payload.quantity)",
  status: "Order $(payload.orderId) is $(upper(payload.status))"
}
"""

payload = {
    "name": "Alice",
    "price": 10.5,
    "quantity": 3,
    "orderId": "ORD-123",
    "status": "confirmed"
}

result = runtime.execute(script, payload)
# Output: {
#   'greeting': 'Hello Alice!',
#   'total': 'Total: 31.5',
#   'status': 'Order ORD-123 is CONFIRMED'
# }
```

String interpolation allows you to embed expressions directly within strings using the `$(expression)` syntax. The expression can be:
- Property access: `$(payload.name)`
- Nested properties: `$(payload.user.email)`
- Expressions: `$(payload.price * 1.1)`
- Function calls: `$(upper(payload.status))`
- Any valid DataWeave expression

### Output Formats

The runtime supports these output directives:
- `application/python` (native Python objects)
- `application/json`
- `application/x-ndjson` (also accepts `ndjson` and `application/x-ldjson`)
- `application/csv`
- `application/xml`
- `text/plain`
- `text/markdown`

Format-specific notes:
- JSON readers preserve repeated object keys in source order, so `payload.test.*test2` returns every matching value. Pass the original JSON text with `payload_format="application/json"` (or use `read(text, "application/json")`); a Python `dict` or JavaScript object already parsed by a standard JSON decoder has lost repeated entries.
- XML output renders repeated object keys as repeated sibling elements in source order, including keys preserved from JSON input and object literals. For example, `{root: {item: 1, item: 2}}` produces `<root><item>1</item><item>2</item></root>`.
- The Rust WASM entry points preserve duplicates in raw payload and request JSON. The Node package's `execute` and `analyze` functions accept either request objects or serialized request JSON; the stdio bridge forwards the original request text. For example, `execute('{"script":"payload.*a","payload":{"a":1,"a":2}}')` returns `[1, 2]`. Pass serialized text directly instead of calling `JSON.parse` on duplicate-bearing input first. Rebuild the WASM package after checking out source changes with `npm --prefix packages/dataweave-wasm run build`.
- `application/x-ndjson` maps newline-delimited JSON records to an array. Empty lines are ignored by default; use `skipInvalid=true` to skip malformed records. Writers emit one compact JSON record per line and support `skipNullOn` and `writeAttributes`.
- `output text/plain` accepts strings and temporal values, rendering dates, times, date-times, and periods as ISO text. `text` and `plain` are aliases for `text/plain`.
- To return a string field verbatim, including Markdown and newlines, use `output text/plain --- {content: "hello\nworld"}.content`.
- `output text/markdown` expects a tabular value (`list` or `dict`) and renders a Markdown table.
- `output text/markdown header=false` is rejected because Markdown table rendering requires headers.
- `payload_format="text/markdown"` parses Markdown pipe tables into structured rows (`Array<Object>` by default, or `Array<Array<String>>` with `payload_format_options={"header": False}`).

## Supported Features

### JWT signing (Rust and WASM)

Import `JWT` from `dw::util::jwt::HMAC` or `dw::util::jwt::RSA` to sign tokens:

```dataweave
%dw 2.0
import * from dw::util::jwt::HMAC
output application/json
---
JWT({sub: "example"}, vars.signingKey)
```

HMAC supports `HmacSHA256` (default), `HmacSHA384`, and `HmacSHA512`; RSA supports
`Sha256withRSA` (default), `Sha384withRSA`, and `Sha512withRSA`. Use
`JWT(header, payload, signingKey, algorithm)` to set a header and algorithm.
HMAC keys must contain at least 32, 48, or 64 UTF-8 bytes respectively. RSA keys
must be unencrypted PKCS#1 or PKCS#8 PEM private keys of at least 2048 bits.
The signing algorithm determines the token's `alg` header; `typ` is `JWT`.

Named `input` declarations accept already parsed values supplied through `vars`:
for example, `input key application/json` binds `vars={"key": pem_string}` as
`key` in the script. Input format declarations describe these native values;
parse serialized values explicitly with `read` when needed. JWT modules are
currently available in the native Rust and WASM engines, not the legacy Python
interpreter.

DataWeave-Py currently supports a wide range of DataWeave language features:

### Core Language Features
- ✅ Header directives (`%dw 2.0`, `output`, `var`, `import`)
- ✅ Payload and variable access
- ✅ Object and array literals
- ✅ Field selectors (`.field`, `?.field`, `[index]`)
- ✅ Comments (line `//` and block `/* */`)
- ✅ Default values (`payload.field default "fallback"`)
- ✅ String interpolation (`"Hello $(payload.name)"`)

### Operators
- ✅ Concatenation (`++`)
- ✅ Difference (`--`)
- ✅ Arithmetic (`+`, `-`, `*`, `/`)
- ✅ Comparison (`==`, `!=`, `>`, `<`, `>=`, `<=`)
- ✅ Logical (`and`, `or`, `not`)
- ✅ Range (`to`)
- ✅ Type checks (`content is String`)
- ✅ Field updates (`item update { case content at .content -> ... }`), including conditional branches inside map lambdas

### Control Flow
- ✅ Conditional expressions (`if-else`)
- ✅ Pattern matching (`match-case`)
- ✅ Match guards (`case var x when condition`)

### Collection Operations
- ✅ `map` - Transform elements
- ✅ `filter` - Select elements
- ✅ `reduce` - Aggregate values
- ✅ `flatMap` - Map and flatten
- ✅ `distinctBy` - Remove duplicates
- ✅ `groupBy` - Group by criteria
- ✅ `orderBy` - Sort elements

### Built-in Functions

#### String Functions
`upper`, `lower`, `trim`, `contains`, `startsWith`, `endsWith`, `isBlank`, `splitBy`, `joinBy`, `find`, `match`, `matches`

#### Numeric Functions
`abs`, `ceil`, `floor`, `round`, `pow`, `mod`, `sum`, `avg`, `max`, `min`, `random`, `randomInt`, `isDecimal`, `isInteger`, `isEven`, `isOdd`

#### Array/Object Functions
`sizeOf`, `isEmpty`, `flatten`, `indexOf`, `lastIndexOf`, `distinctBy`, `filterObject`, `keysOf`, `valuesOf`, `entriesOf`, `pluck`, `maxBy`, `minBy`

#### Date Functions
`now`, `isLeapYear`, `daysBetween`

#### Utility Functions
`log`, `logInfo`, `logDebug`, `logWarn`, `logError`, `logWith`

### Injecting loggers

Logging functions return the value they inspect. Inject one or more sinks into
the Python runtime to receive `LogEntry(level, prefix, message)` records:

```python
import logging
from dwpy import DataWeaveRuntime

entries = []
runtime = DataWeaveRuntime(loggers=[entries.append, logging.getLogger("my-app.dataweave")])
result = runtime.execute("1 to 10 map log($)", None)
assert result == list(range(1, 11))
assert entries[0].message == "1"
```

`log(value)` and `log("label", value)` use the `warn` level for compatibility
with the existing Python implementation. The level-specific functions use
`debug`, `info`, `warn`, or `error`; `logWith("debug", "label", value)` selects
a level dynamically (unknown levels default to `info`). Logger callbacks see
all levels; injected Python `logging.Logger` instances apply their own filters.
With `loggers=None`, Python uses standard Python logging. Pass `loggers=[]` to
silence script logs. Configured sinks replace the default sink.

Sinks are isolated by execution, including nested and concurrent runs. Messages
emitted before an execution error are delivered as well. Native Python-extension
and WASM bridges deliver captured entries after evaluation; the Python interpreter
and direct Rust sinks deliver them during evaluation. An exception in a Python or
JavaScript logger callback propagates to the caller.

Rust embedders use `ExecutionContext::with_logger`, repeatedly for multiple sinks:

```rust
use dwpy_core::{compile, ExecutionContext, ExecutionOptions, LogEntry};
use serde_json::Value;

let context = ExecutionContext::new(None, None, None)?
    .with_logger(|entry: &LogEntry| eprintln!("{:?}: {}", entry.level, entry.message));
let output = compile("logInfo(\"value\", payload)")?
    .execute_with_context(Value::from(42), context, &ExecutionOptions::default())?;
# Ok::<(), dwpy_core::DwError>(())
```

Direct Rust and result-only WASM APIs are silent without injected sinks. The
WASM export `run_dataweave_request_with_logs` is opt-in and returns a JSON envelope
with `result` or `error`, plus `logs` and `logs_truncated`; the existing
`run_dataweave_request` contract is unchanged. Captures retain up to 1,000 entries
and approximately 1 MiB of text, with 16 KiB per message and 1 KiB per prefix.
The DataWeave playground displays these records in its **Logs** tab, including
records produced before an error. Each run replaces the previous logs.

## Running Tests

The project includes comprehensive test coverage:

```bash
# Run all tests
pytest

# Run specific test file
pytest tests/test_runtime_basic.py

# Run with verbose output
pytest -v

# Run with coverage
pytest --cov=dwpy
```

## Browser WASM (Pyodide)

The project includes a browser-worker runtime for WASM execution with Pyodide and wheel-based loading.

- Worker bootstrap: `web/pyodide-worker.mjs`
- Python entrypoint: `dwpy.wasm_entry.run_dataweave(...)`
- Full instructions: [`docs/WASM_PYODIDE.md`](docs/WASM_PYODIDE.md)

## Language Server (LSP)

The project now includes a stdio Language Server for DataWeave:

- Command: `dwpy-lsp`
- Module: `dwpy.lsp.server`
- Engine shared with Monaco + WASM completion bridge: `dwpy.lsp.engine`

### Install

Install the LSP extra:

```bash
uv pip install "dataweave-py[lsp]"
```

### Sidecar context files

For structure-aware `payload`/`vars` completion in `.dwl` files, place these JSON files next to the script:

- `<file>.payload.json`
- `<file>.vars.json`

Example for `transform.dwl`:

- `transform.dwl.payload.json`
- `transform.dwl.vars.json`

If sidecars are missing or invalid, the server falls back to script-only inference.

### VS Code client (example)

```json
{
  "languageserver": {
    "dataweave-py": {
      "command": "dwpy-lsp",
      "filetypes": ["dataweave", "dwl"]
    }
  }
}
```

### Neovim client (example)

```lua
require("lspconfig").dwpy_lsp.setup({
  cmd = { "dwpy-lsp" },
  filetypes = { "dataweave", "dwl" },
})
```

## Project Structure

```
dataweave-py/
├── crates/                    # Rust workspace
│   ├── dwpy-core/             # Core Rust value model and engine foundation
│   ├── dwpy-python/           # PyO3 extension exposed as dwpy._dwpy_rust
│   └── dwpy-wasm/             # WASM wrapper foundation
├── dwpy/                      # Main Python package
│   ├── __init__.py           # Package exports
│   ├── parser.py             # DataWeave parser
│   ├── runtime.py            # Runtime backend facade
│   ├── _python_runtime.py    # Legacy Python interpreter backend
│   └── builtins.py           # Built-in functions
├── tests/                     # Test suite
│   ├── test_runtime_basic.py # Core functionality tests
│   ├── test_builtins.py      # Built-in function tests
│   └── fixtures/             # Test data and fixtures
├── runtime-2.11.0-20250825/  # Original JVM runtime reference
├── docs/                      # Documentation
├── pyproject.toml            # Project configuration
└── README.md                 # This file
```

## Development

### Setting Up Development Environment

```bash
# Create virtual environment
uv venv --python 3.12
source .venv/bin/activate

# Install Python development dependencies
UV_CACHE_DIR=.uv-cache uv sync --extra dev

# Build and install the Rust-backed Python bridge in editable mode
UV_CACHE_DIR=.uv-cache uv run maturin develop --release
```

### Running the Test Suite

```bash
# Run all tests
UV_CACHE_DIR=.uv-cache uv run --extra dev pytest

# Force strict Rust backend
DWPY_BACKEND=rust UV_CACHE_DIR=.uv-cache uv run --extra dev pytest

# Run Rust workspace tests
cargo test --workspace
```

### Code Style

The project follows standard Python conventions:
- PEP 8 style guide
- Type hints where appropriate
- Comprehensive docstrings
- Two-space indentation for consistency with Scala codebase

## Comparison with JVM Runtime

DataWeave-Py aims to provide feature parity with the official JVM-based DataWeave runtime. Key differences:

| Feature | JVM Runtime | DataWeave-Py |
|---------|-------------|--------------|
| Language | Scala | Rust core with Python bridge |
| Performance | High (compiled/JIT) | Native Rust engine through PyO3 |
| Startup Time | Slower (JVM warmup) | Fast native extension loading |
| Memory Usage | Higher (JVM overhead) | Lower native runtime footprint |
| Integration | Java/Mule apps | Python apps, Rust crate, future WASM wrapper |
| Module System | Full support | Rust-native support for the current suite |
| Type System | Static typing | Rust-backed inference plus Python API helpers |

## Roadmap

### Current Status (v0.1.0)
- ✅ Core language parser
- ✅ Expression evaluation
- ✅ 60+ built-in functions
- ✅ Pattern matching
- ✅ Collection operators

### Planned Features
- 🔄 Full module system support
- 🔄 Import statements
- 🔄 Custom function definitions
- 🔄 XML/CSV format support
- 🔄 Streaming for large datasets
- 🔄 Type validation
- 🔄 Performance optimizations

## Contributing

Contributions are welcome! Please:

1. Fork the repository
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Write tests for your changes
4. Ensure all tests pass (`pytest`)
5. Commit your changes (`git commit -m 'feat: add amazing feature'`)
6. Push to the branch (`git push origin feature/amazing-feature`)
7. Open a Pull Request

## License

See the original DataWeave runtime license terms. This project is a reference implementation for educational and development purposes.

## Resources

- [DataWeave Documentation](https://docs.mulesoft.com/dataweave/)
- [DataWeave Tutorial](https://developer.mulesoft.com/tutorials-and-howtos/dataweave/)
- [DataWeave Playground](https://dataweave.mulesoft.com/learn/playground)

## Support

For questions, issues, or contributions:
- Open an issue on GitHub
- Check existing documentation in the `docs/` directory
- Review test cases in `tests/` for usage examples

---

**Note**: This is an independent Python implementation and is not officially supported by MuleSoft. For production use cases requiring full DataWeave compatibility, please use the official JVM-based runtime.
