# Live DataWeave debugging

The Rust engine includes a cooperative debugger and a transport-independent
[Debug Adapter Protocol (DAP)](https://microsoft.github.io/debug-adapter-protocol/overview)
adapter. Native and browser/WASM debugging use the same evaluator and adapter.
LSP remains responsible for editing features such as completion and diagnostics.
Normal `execute` APIs keep their existing execution path.

Implementation progress and remaining work: [Debugger support tracker](DEBUGGER_SUPPORT_TRACKER.md).

## Playground

Use **Debug** in the script header (F5). Without breakpoints it stops before the
first executable header initializer or body expression; with breakpoints it runs
directly to the first hit. Click the editor's glyph gutter to toggle breakpoints.
The floating toolbar and inspector appear only during the session. Continue (F5),
Pause (F6), Into (F11), Over (F10), Out (Shift+F11), Restart, and Stop
(Shift+F5) control the live session. Source and execution inputs are locked while
debugging; auto-run resumes its previous setting afterward without immediately
rerunning the same script.

The inspector shows the selected call frame, parameters, captured bindings,
locals, globals, and the last completed expression result. Arrays and objects
expand in pages of 100 entries. Duplicate object fields retain their order and
individual entries; temporal values keep their native types. Watches are evaluated
in the selected frame. Output is produced when execution completes; logs arrive
while execution advances.

Stepping stops at meaningful expression ranges, including individual object
values and lambda invocations. Repeated invocations on the same source line are
separate stops. Into enters functions and callbacks; Over skips descendants of
the current expression; Out finishes the current function/lambda invocation.
Explicit breakpoints can interrupt Over and Out. Builtins and format readers and
writers are atomic operations.

The browser uses a dedicated worker per session. Bounded polls yield to the
worker's message loop; Stop can terminate the worker even during an atomic call.
Session identities prevent late messages from updating a new session. Browser
execution does not need a debug server, WebSocket, shared memory, or isolation
headers.

## Native DAP client

Launch the **native Rust CLI** with `dw dap`, or run it from this checkout:

```sh
cargo run --quiet -p dwpy-cli -- dap
```

This command uses standard `Content-Length` DAP framing over stdin/stdout. Cargo
and host errors use stderr. It is separate from the Python CLI entry point; a
packaged editor extension and the legacy Python interpreter are not included.

Send `initialize`, then `launch` with either `program` (a UTF-8 file path, loaded
by the native host) or `source` (inline source text). Inline sources may supply
`sourcePath` for editor identity. Launch also accepts the existing execution
inputs: `payload`, `vars`, `attributes`, `properties`, `payload_format`,
`payload_format_options`, and `render_output`. `stopOnEntry` defaults to true.
For JSON data containing duplicate keys, supply the original payload text with
`payload_format: "application/json"`.

After `launch`, the adapter emits `initialized`. Configure breakpoints and the
optional `uncaught` exception filter, then send `configurationDone`. The single
logical thread has ID 1. Inline sources have `sourceReference: 1`; the `source`
request returns the original text. Lines and UTF-16 columns honor the client's
`linesStartAt1` and `columnsStartAt1` initialization settings.

Supported requests: `initialize`, `launch`, `configurationDone`,
`setBreakpoints`, `breakpointLocations`, `setExceptionBreakpoints`, `threads`,
`stackTrace`, `scopes`, `variables`, `evaluate`, `continue`, `pause`, `next`,
`stepIn`, `stepOut`, `exceptionInfo`, `source`, `terminate`, and `disconnect`.
Only implemented optional capabilities are advertised. Breakpoints on lines
without executable expressions return `verified: false` with an explanation.
Frame and variable references expire whenever execution resumes.

Logs are DAP `output` events. Final results also use an `output` event, with a
readable `output` string and an extension in `body.data`:

```json
{"kind":"result","result":"[2,4,6]","typed_transport":true}
```

The `result` uses the existing typed-wire encoding. Log events carry
`data: {kind: "log", entry: {level, prefix, message}}`; capture limits are
signaled by `data: {kind: "logsTruncated"}`. Standard clients may simply display
the textual output and ignore this additional data.

## Embedding

Rust exposes `dwpy_core::debug::{DebugSession, DebugCommand, DebugEvent}`.
Create a session from source, a native `DwValue` payload, `ExecutionContext`, and
`ExecutionOptions`. Call `command`, then `advance(fuel)` until it yields, stops,
completes, or terminates. `frames`, `scopes`, `evaluate`, `locations`, and
`set_breakpoints` support direct integrations. Inspection does not replay code.

`dwpy_core::dap::DebugAdapter` exposes `dispatch(serde_json::Value)`,
`poll(fuel)`, and `is_running()`. Hosts provide framing and scheduling. The WASM
class `DataWeaveDebugAdapter` has the same operations with JSON string inputs
and outputs. The Node package provides a parsed-JSON wrapper:

```js
const { DebugAdapter } = require("@estebanwasinger/dataweave-wasm");
const adapter = new DebugAdapter();
const response = adapter.dispatch({
  seq: 1, type: "request", command: "initialize", arguments: {}
});
// Dispatch launch/configuration requests, then poll while adapter.running.
adapter.dispose();
```

## Language support and limits

The debugger supports variables, named and overloaded functions, generic function
declarations, lexical lambdas, default arguments, computed call targets and
immediately invoked lambdas, `do` and `using` scopes, ordered `match` cases with
bindings/guards, arrays, computed object keys, spreads and conditional entries,
and ordinary string `$(expression)` interpolation. Nested expressions retain
original source ranges, including comments, multiline strings and Unicode.

Selectors include indexing/slices, predicates `[?(...)]`, and dynamic multi/key-value
forms `[*(...)]` / `[&(...)]`. Arithmetic, comparisons, booleans, short-circuiting,
`default`, `if`, coercions and type tests reuse the normal engine. Structured,
union and generic type operands and literal coercion format options retain the
engine's existing behavior; this does not expand its type system or add arbitrary
expression-valued format options.

Collection, object, string, tree and join callbacks are supported, along with
`update`, builtin imports, and ordinary value-only builtins. Infix/prefix forms,
imported function aliases, module aliases and qualified calls share the normal
engine's dispatch; local functions/parameters retain lexical shadowing. String/regex `replace ... with` also
supports callbacks, stepped once per match; regex matching itself is atomic.
Eligible collection pipelines retain normal lazy evaluation, short-circuiting,
and callback/log ordering, including the runtime's existing compiled-to-eager
fallback behavior.
Updates support case bindings (including shorthand `$`), guards, nested property
and array-index paths, upserts, and `update selector with replacement`. Guards and
replacement expressions are live stepping locations; functions called from them
can be entered normally. Case bindings stay scoped to their case. Update paths
use the normal runtime's selector handling. Descendant, metadata and key-value
update-case paths are unsupported by the engine and now raise explicit errors
instead of silently leaving the value unchanged. Debug sessions reject these
paths before execution with a source-located error. Updates are not allowed in
watches.

Named overloads use the normal runtime's declaration order, argument counts,
defaults, and parameter type matcher. Arguments are evaluated once before
dispatch; only the selected body's defaults are evaluated. Recursive calls and
function references retain the overload set. For example, a String overload
receives `"4s"` as a String, but `"4s" as Number` still raises the same coercion
error as normal execution. Use `"4"` when a numeric string is intended.

See the complete current restrictions below. The debugger does not extend the
normal engine's language support or strengthen its existing type matcher.

Watches allow selectors, literals, operators, arrays/objects, conditions, and only
these builtin calls: `sizeOf`, `isEmpty`, `keysOf`, `valuesOf`, `upper`, `lower`,
and `abs`. They reject declarations, user-function calls, logging, time/random,
and other effects. Watches have a bounded continuation budget. Values use bounded
previews and paged expansion; execution retains the default 256 MiB
materialization limit and limits call depth to 1,024 frames. Atomic operations
may take longer than a cooperative poll.

## Collection and callback coverage

Supported operations: `map`, `filter`, `reduce`, `groupBy`, `pluck`, `mapObject`,
`filterObject`, `flatMap`, `distinctBy`, `orderBy`, `then`, `onNull`, `takeWhile`,
`dropWhile`, `some`, `every`, `countCharactersBy`, `everyCharacter`, `mapString`,
`someCharacter`, `substringBy`, `countBy`, `maxBy`, `minBy`, `sumBy`, `firstWith`,
`indexWhere`, `partition`, `splitWhere`, `everyEntry`, `someEntry`, `mapLeafValues`,
`nodeExists`, `filterArrayLeafs`, `filterObjectLeafs`, `join`, `leftJoin`, `outerJoin`.
Callbacks expose item/key/index or tree-path bindings and preserve native early
exit and callback order. Join key selectors run in the engine's nested-loop order.

Eligible compiled pipelines retain their existing laziness, stage indices,
flatMap batching and early termination. Effectful/noneligible expressions retain
the normal eager/fallback behavior. Strings advance one character at a time;
collection/tree traversal yields between bounded continuation tasks.

`readLinesWith`, `writeLinesWith` and `scan` use atomic encoding/regex operations.
`failIf` steps through its scoped predicate with `$` bound to the input. `wait`
retains the engine's current identity behavior and leaves the delay expression
unevaluated. The internal `evaluate_by` helper implements `minBy`/`maxBy`; there
is no standalone `by` builtin in this engine.

Ordinary native value operations (including comparison/sorting), regex work and
format readers/writers remain atomic. Passing function values through native
value operations or data containers remains a limitation described above.

## Unsupported debugger features

These restrictions concern live debugging in this Rust implementation, not the
full MuleSoft language specification. Native and WASM share these restrictions.
Any other syntax not accepted by the source-mapped debug grammar is also rejected.

### Language syntax and values

- `orElse` and `orElseTry` error-handling forms.
- Descendant, metadata and key-value **update-case** selectors. Update cases
  accept property/index paths and upserts; these other paths are also rejected
  by normal execution because their mutation semantics are not implemented.
- Serializing function values as output, placing them in output objects/arrays,
  or passing them to atomic native builtins. Local function values and closures
  used by DataWeave calls are supported.

### Modules, execution, and reflection

- External/custom module imports and stepping into external modules. Supported
  built-in imports use `dw::...`.
- `try`, `eval`, `run`, `evalUrl`, `readUrl`.
- Source/type-reflection operations: `location`, `locationString`,
  `evaluateCompatibilityFlag`, `arrayItem`, `baseTypeOf`, `functionParamTypes`,
  `functionReturnType`, `intersectionItems`, `literalValueOf`, `metadataOf`,
  `nameOf`, `unionItems`, and builtin names matching `is*Type`.

### Watches and debugger operations

- Watches cannot declare variables/functions, create/invoke lambdas, execute
  updates, match bindings or selector/replacement callbacks, call user functions,
  or invoke builtin calls outside `sizeOf`, `isEmpty`, `keysOf`, `valuesOf`, `upper`, `lower`, `abs`.
  This excludes logging, time/random, I/O, and other effects.
- Conditional breakpoints, hit counts, logpoints, function/data/instruction
  breakpoints, reverse stepping, restarting an individual frame, editing
  variables/expressions, memory inspection, and disassembly.
- Attaching to an existing process, multiple execution threads, and a TCP or
  WebSocket transport. Implemented transports are native stdio and worker
  message objects; a host could supply an additional transport.
- Legacy Python-interpreter debugging and a packaged VS Code extension.

Execution uses bounded cooperative polls, but individual atomic builtins and
format readers/writers cannot be paused mid-operation. Playground Stop can
terminate their worker. Materialization and call-depth limits still apply.

## Verification

```sh
cargo test --workspace
bash scripts/test_wasm_rust.sh
npm --prefix packages/dataweave-wasm run build
npm --prefix packages/dataweave-wasm test
UV_CACHE_DIR=.uv-cache uv run --extra dev pytest
```

For the complete Python corpus, follow `OFFICIAL_DOCS_EXAMPLES_STATUS.md` to set
`DWPY_DATAWEAVE_DOCS`. Native stdio, Rust/WASM, and Node/WASM tests execute the
same transcripts in `tests/fixtures/debugger/session.json`,
`tests/fixtures/debugger/m1.json` and `tests/fixtures/debugger/m2.json`.
Differential, source-range, stepping, watch-safety and cancellation coverage lives
in `crates/dwpy-core/tests/debugger_m1.rs` and `debugger_m2.rs`.

In the playground repository run `pnpm test:browser`, `pnpm test`,
`pnpm typecheck`, `pnpm lint`, and `pnpm build`. The browser suite uses port 3015 by default;
set `PLAYWRIGHT_BASE_URL` to reuse another running local instance.
