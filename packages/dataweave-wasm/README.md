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
