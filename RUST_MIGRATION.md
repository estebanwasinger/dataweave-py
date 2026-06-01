# Rust Migration Tracker

Last updated: 2026-06-01

## Objective

Migrate the DataWeave execution engine to run completely on Rust while preserving the Python API and bridge.

## Current Progress

Overall Rust migration progress: **100%**

Strict Rust runtime-basic parity: **100%**

Full repository forced-Rust parity: **100%**

Current strict Rust baseline:

- Command: `DWPY_BACKEND=rust UV_CACHE_DIR=.uv-cache uv run --extra dev pytest tests/test_runtime_basic.py --tb=no -q`
- Result: `116 passed`
- Scope: `tests/test_runtime_basic.py` under the Rust-only backend, with Python fallback disabled.

Current full forced-Rust baseline:

- Command: `DWPY_BACKEND=rust UV_CACHE_DIR=.uv-cache uv run --extra dev pytest --tb=short -q`
- Result: `836 passed, 5 xfailed`
- Scope: Full Python suite under the Rust-only backend, with Python fallback disabled.

Default backend baseline:

- Command: `UV_CACHE_DIR=.uv-cache uv run --extra dev pytest`
- Last known result: `836 passed, 5 xfailed`
- Meaning: Python-facing compatibility is green with the staged backend/fallback path.

Rust workspace baseline:

- Command: `cargo test --workspace`
- Last known result: `95 passed`

WASM target baseline:

- Command: `RUSTC=/opt/homebrew/opt/rustup/bin/rustc /opt/homebrew/opt/rustup/bin/cargo build -p dwpy-wasm --target wasm32-unknown-unknown`
- Last known result: passed

## Implemented Rust Coverage

- Rust workspace with `dwpy-core`, `dwpy-python`, and `dwpy-wasm`.
- Python bridge exposed through `dwpy._dwpy_rust`.
- Runtime backend selection for Rust, Python legacy, and auto fallback.
- Multiline header `var`/`fun` declaration grouping, including indented top-level declarations.
- Left-associative chained collection transforms in multiline header expressions.
- `do { ... --- ... }` block evaluation with scoped local header declarations.
- Delimiter-aware script boundary parsing that ignores `---` inside nested delimiters such as `do` blocks.
- Pipe literal parsing for temporal/period values used by the Rust evaluator.
- Format-aware temporal string coercion for core DataWeave date/time tokens.
- Day-based period arithmetic for `Date + |PnD|` / `|PnD| + Date`.
- Header `type` alias registration and alias resolution during Rust coercion.
- XML wildcard child mapping with alias coercions.
- XML attribute path access for repeated child nodes.
- `valueSet` support that collapses repeated XML child values.
- Python bridge support for short `output json` directives.
- Recursive header function execution.
- Recursive `reduce` traversal with default accumulator values.
- Correct top-level `++` precedence around infix string helpers such as `joinBy`.
- Core expression evaluation for literals, selectors, object/array literals, functions, operators, matches, coercions, and collection transforms.
- Format I/O for JSON, CSV, XML, YAML, markdown, plain text, and Python/raw output behavior.
- Output rendering, including compact JSON rendering that preserves duplicate object keys.
- Python bridge normalization for pandas `DataFrame` and `Series` inputs before dispatching to the Rust core.
- Type inference and LSP analysis bridge coverage.
- WASM crate compile/test participation in the Rust workspace.
- Key-value selectors, present/assert-present selectors, selector filters, dynamic multi-value selectors, and dynamic key-value selectors.
- Python bridge reconstruction of Rust duplicate-key object selector results into `dwpy.formats.DWObject`.
- Conditional object-valued expression merging inside object literals.
- Pass-through `log`, `logDebug`, `logInfo`, and `logWarn` built-ins for selector chaining and recursive traversal scripts.
- Postfix selector parsing across indexed function-call results such as `log(item).payload.errors[0].statusCode`.
- Rust bridge mapping for parse errors and unresolved infix reference location text used by Python API tests.
- Rust type-inference support for key-present selectors.
- Native/runtime helpers used by the current suite: `try`, `fail`, and `hashWith` for MD2, MD5, SHA-1, SHA-256, SHA-384, and SHA-512.
- Object module helpers: `entrySet`, `nameSet`, `keySet`, `valueSet`, `mergeWith`, `divideBy`, `takeWhile`, `everyEntry`, and `someEntry`.
- Python bridge conversion for Rust binary markers back into Python `bytes`.
- Rust non-finite numeric marker conversion back into Python floats for paths such as `asin(2)`.
- Period helpers and temporal arithmetic for `years`, `months`, `days`, `hours`, `minutes`, `seconds`, `period`, `duration`, and `between`.
- Python bridge temporal markers for `date`, `datetime`, and `time` round-trips, plus JSON rendering of period/temporal markers as DataWeave strings.

## Rust Core File Organization

`crates/dwpy-core/src/lib.rs` is now a dispatcher and shared utility module, with major behavior split into:

- `builtins.rs`
- `calls.rs`
- `collections.rs`
- `csv.rs`
- `evaluator.rs`
- `functions.rs`
- `json.rs`
- `literals.rs`
- `markdown.rs`
- `matches.rs`
- `mime.rs`
- `operators.rs`
- `output.rs`
- `periods.rs`
- `script.rs`
- `selectors.rs`
- `strings.rs`
- `syntax.rs`
- `type_inference.rs`
- `types.rs`
- `value.rs`
- `xml.rs`
- `yaml.rs`

## Remaining Forced-Rust Failures

There are no remaining failures in the current Python test suite under `DWPY_BACKEND=rust`.

The WASM target build is also verified through the rustup toolchain. The default `cargo` command in this environment points to Homebrew Rust, so the verified command pins both `cargo` and `rustc` to rustup explicitly.

## Next Iteration Candidates

1. Expand Rust-native coverage beyond the current suite for DataWeave modules not represented by tests.
2. Continue shrinking `crates/dwpy-core/src/lib.rs` into parser/runtime modules now that parity is established for the current suite.
3. Consider removing the temporary Python fallback after a stabilization period if no downstream compatibility issues appear.

## Completion Criteria

The migration is complete only when:

- `UV_CACHE_DIR=.uv-cache uv run --extra dev pytest` passes.
- `DWPY_BACKEND=rust UV_CACHE_DIR=.uv-cache uv run --extra dev pytest` passes without Python fallback.
- `cargo test --workspace` passes.
- Public Python APIs remain source-compatible.
- `RUST_MIGRATION.md` shows 100% strict Rust parity with current evidence.
