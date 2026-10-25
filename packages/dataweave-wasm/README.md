# @estebanwasinger/dataweave-wasm

## Logging

```js
const {execute, executeWithLogs} = require("@estebanwasinger/dataweave-wasm");

const result = execute({script: "1 to 10 map log($)"}, {
  loggers: [entry => console.log(entry.level, entry.message)],
});
const execution = executeWithLogs({script: 'logInfo("payload", payload)', payload: 42});
// {result: 42, logs: [{level: "info", prefix: "payload", message: "payload - 42"}], logs_truncated: false}
```

Logging functions preserve their return values. Multiple injected callbacks
receive the same records, in evaluation order, after the execution finishes.
`execute` still returns only the result and throws execution errors, after
delivering any logs. Without the `loggers` option it keeps the original silent
behavior. Callback exceptions propagate to the caller.

`executeWithLogs` returns `{result, logs, logs_truncated}` on success or
`{error, logs, logs_truncated}` on failure. Capture is isolated per call and
limited to 1,000 entries / approximately 1 MiB; messages and prefixes are limited
to 16 KiB and 1 KiB. `logs_truncated` indicates omitted or shortened text.
Pass raw request JSON to preserve duplicate keys in inputs and log messages.

Node.js bindings for the strict Rust DataWeave runtime.

Build with `npm run build` after installing `wasm-pack` and the
`wasm32-unknown-unknown` Rust target. The generated `dist/` directory is
included when the package is published to GitHub Packages.

Run the JavaScript API tests after building:

```bash
npm test
```

Run the Rust unit and integration tests compiled to WebAssembly, using
`wasm-bindgen-test` in Node.js:

```bash
npm run test:rust
npm run test:rust -- -- infix
```

These tests cover the WASM entry points, including dependency analysis before
execution. They require the build prerequisites above and do not require Python.
They test the Node.js host; Deno and browser integration need separate checks.

## Debug Adapter Protocol

`DebugAdapter` exposes `dispatch(request)`, `poll(fuel = 200)`, `running`, and
`dispose()`. Requests and returned messages use standard DAP objects. Launch
with inline `source` and the same execution inputs as `execute`; configure the
session with `configurationDone`, then poll cooperatively while `running`.
See [the debugger guide](../../docs/DEBUGGER.md) for the supported protocol,
language subset, inspection APIs, and output event extensions.

## Source completions

```js
const {complete} = require("@estebanwasinger/dataweave-wasm");
const {items} = complete({
  script: "%dw 2.0\nfun multiply(number) = number * 2\n---\nm",
  line: 3,
  column: 1,
});
// multiply(${1:number}), with a function kind and signature detail
```

Positions are zero-based UTF-16. Results contain runtime context roots (`payload`,
`attributes`, `vars`), header functions (one item per signature), header
variables/named inputs, and parameters in the current header
function body. The service does not execute the source and tolerates an incomplete
body. It is a completion API rather than a full LSP transport.

Supply `payload` with `payload_format` (and optional `payload_format_options`) for
field completions using the same Rust input readers as execution. This covers CSV,
JSON, XML, YAML, NDJSON and Markdown tables. For example, completing `payload.` or
`payload[0].` with CSV input suggests its column names. Arrays contribute the union
of record fields; traversal samples up to 10,000 value nodes at a depth of 64.
Scalars and invalid input have no object fields. Invalid input does not block
source-symbol suggestions. `vars` and `attributes` accept structured JSON values.

Callback selectors use those same parsed inputs: `payload map $.` offers the
array elements' fields; `payload map ((item) -> item.)` and nested callbacks are
also supported. Completion follows selectors and known collection shapes without
executing callbacks. Unknown transformed shapes do not produce guessed fields.

## Dynamic function discovery

`getFunctionCatalog()` queries the actual WASM engine's callable descriptors
(names, arities, module exports and signatures). The catalog is discovered
**automatically during the engine build**, using Rust syntax and module source;
there is no handwritten name list or regeneration step. New Rust implementations
require a rebuilt engine, as execution itself does.

`complete()` resolves the supplied document anew on each request. It handles
nested `do` functions, overload shadowing, lexical closures, lambda-valued vars,
wildcard/selective imports, aliases and qualified names. Add `module_sources` to
supply live module documents, for example:

```js
complete({
  script: "%dw 2.0\nimport * from custom::Tools\n---\nnew",
  line: 3, column: 3,
  module_sources: {"custom::Tools": "fun newFunction(value) = value"},
});
```

Changing source or module declarations changes the next response without
restarting or rebuilding the engine. Discovering external module declarations
for tooling does not add external-module execution support to the Rust runtime.
