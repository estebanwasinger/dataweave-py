# DataWeave debugger support tracker

Last updated: **2026-10-24**. Current implementation: **M0–M2 implemented; local validation recorded below**.
Package/release publishing and playground deployment are separate delivery
steps. Usage and current limitations: [DEBUGGER.md](DEBUGGER.md).

## Target and status conventions

**Full language support** means scripts supported by normal Rust execution on a
given host also run under the debugger with matching values, errors, scoping,
evaluation order, laziness, and callback/log order. Executable DataWeave
expressions must have correct locations and support live stepping/inspection.
Native and browser/WASM use the same runtime and DAP adapter.

This is parity with this engine, not a claim of complete MuleSoft compatibility.
Advanced DAP operations, other backends, and editor packaging are tracked
separately; they are not prerequisites for the language-parity milestone.

| Status | Meaning |
| --- | --- |
| Done | Implemented and locally verified for the stated scope |
| Partial | A working subset exists; remaining work is listed |
| Todo | Not implemented in debug execution |
| Deferred | Optional follow-up outside the original delivery scope |
| Policy | Intentional boundary, not missing implementation |

Proposed priorities: **P0** = foundations/common syntax; **P1** = remaining runtime
parity; **P2** = optional tooling. These are sequencing suggestions, not dates.
Completion requires evidence; a single working example does not establish full
support for an area. Record future test names and commit/PR links in the change log.

## Milestones

| ID | Milestone | Status | Exit criteria |
| --- | --- | --- | --- |
| M0 | Live debugger foundation | Done | Source-aware resumable evaluation, shared DAP, native/WASM transports, playground workflow |
| M1 | Core grammar parity | Done | L01–L13 completed with nested/composed regression coverage |
| M2 | Callback/collection parity | Done | Every C item completed without replay or changed laziness |
| M3 | Modules, values, execution, reflection | Partial | Every R item completed where normal execution supports it on that host |
| M4 | Full-support verification | Partial | Q01–Q06 completed; final-source and bundled-WASM gates pass |
| M5 | Optional tooling/protocol extensions | Deferred | Individually scoped E items; separate from language parity |

## Completed foundation and supported baseline

- [x] Source ranges preserved through comments, headers, multiline expressions,
  nesting, and UTF-16 editor columns.
- [x] Explicit continuations, bounded advancement, no replay for inspection;
  normal execution keeps its existing optimized path.
- [x] Entry, Into, Over, Out, Continue, Pause, termination, and source breakpoints.
  Repeated callbacks remain distinct stops; breakpoints interrupt stepping.
- [x] Header variables/functions, default parameters, recursion, lexical closures,
  builtin imports, and `do` scopes.
- [x] Typed and arity-based overloads in declaration order; arguments evaluated
  once, defaults only for the selected overload, recursive/reference dispatch
  retaining the overload set.
- [x] Static objects/arrays, ordinary selectors/indexing/slices, arithmetic,
  comparisons, booleans, short-circuiting, `default`, `if`, simple named-type
  coercions/tests.
- [x] Direct `map`, `filter`, `reduce`: resumable iterators, eligible lazy
  pipelines, empty collections, reduce defaults.
- [x] `update` subset: named bindings/shorthand `$`, guards, property/index paths,
  upserts, scoped replacement evaluation, `update selector with value`.
- [x] Shared DAP lifecycle, source retrieval, one logical thread, breakpoint
  resolution, stack/scopes/variables, evaluation, controls, optional uncaught
  exceptions, termination/disconnect.
- [x] Native `dw dap` Content-Length transport, WASM dispatch/poll bindings,
  Node wrapper, common native/WASM DAP transcript.
- [x] Parameters, locals, captures, globals, `$`/`$$`, reduce accumulators, cached
  last results, paged native values, ordered duplicate fields, stale-reference
  invalidation on resume.
- [x] Selected-frame pure watches with bounded evaluation.
- [x] Playground header Debug button, compact draggable toolbar, breakpoints,
  expression highlighting, frame selection, expandable variables, watches.
- [x] Active-session-only inspector/toolbar, locked source/inputs, auto-run
  suspension/restoration, fresh-session restart, worker Stop, stale-message guards.
- [x] Streamed logs, normal final-output pipeline, retained exception stacks;
  terminal debug errors shown in Output.

Evidence: [runtime tests](../crates/dwpy-core/tests/debugger.rs),
[DAP tests](../crates/dwpy-core/tests/dap.rs),
[native transport tests](../crates/dwpy-cli/tests/dap.rs),
[WASM tests](../crates/dwpy-wasm/tests/wasm.rs),
[Node debugger tests](../packages/dataweave-wasm/test/debugger.test.js).
Playground evidence is in the sibling `v0-dataweave-editor` repository:
`tests/browser/debugger.spec.ts` and `tests/debug-client.test.mjs`.

## M1 — Core grammar parity (completed)

Each item needs parser/AST support, resumable evaluation, original ranges,
inspection behavior, and normal-vs-debug differential tests.

| ID | Priority | Status | Feature and acceptance criteria |
| --- | --- | --- | --- |
| L01 | P0 | Done | `match`: ordered patterns, bindings, guards, defaults, branch/body stops |
| L02 | P0 | Done | `using`: lexical scope, nesting, shadowing, restored outer bindings |
| L03 | P0 | Done | Dynamic/computed object keys: keys and values evaluated once in normal order |
| L04 | P0 | Done | Object spreads/conditional entries: field order, duplicate fields, skipped effects |
| L05 | P0 | Done | Ordinary string `$(expression)` interpolation: nested expressions, effects, Unicode/multiline locations |
| L06 | P1 | Done | Generic function declarations: normal parameter/type behavior with overloads/recursion |
| L07 | P1 | Done | Complex coercions and `is` operands: structured, union, generic types where supported normally |
| L08 | P1 | Done | Literal coercion format/writer options: reuse normal conversions and existing option semantics |
| L09 | P1 | Done | Selector predicates `[?(...)]`: live scopes, repeated stops, null/empty behavior, short-circuiting |
| L10 | P1 | Done | Dynamic multi/key-value selectors `[*(...)]` / `[&(...)]`: normal evaluation order and field semantics |
| L11 | P1 | Done | Update-case engine boundary: property/index mutations remain supported; descendant/metadata/key-value paths explicitly rejected in normal execution and debug preflight, replacing silent no-ops |
| L12 | P1 | Done | Computed call targets/immediately invoked lambdas: captures, arguments, frames |
| L13 | P1 | Done | `replace ... with`: string/regex and callback replacements; regex engine work may remain atomic |

M1 evidence: [differential and stepping tests](../crates/dwpy-core/tests/debugger_m1.rs)
cover nested/composed forms, values/errors/log order, scoped bindings, repeated
predicate stops, watches, Pause/Stop, Unicode/comments and original ranges. The
[shared M1 DAP transcript](../tests/fixtures/debugger/m1.json) runs through native
stdio, Rust/WASM and Node/WASM. Playground tests cover the composed transcript
and each selector invocation without replay.

L07/L08 use the normal engine's type matcher and literal option parser; they do
not add a richer type system or expression-valued format options. L11 completes
the explicit validation requirement: mutation semantics for descendant,
metadata and key-value update-case paths remain engine work. Debug preflight
rejects these paths before any header effects execute.

## M2 — Callback and collection parity (completed)

The following operators now use live continuations or the existing atomic value
implementations, matching the normal Rust engine. Every row inherits the shared
acceptance gates. Eligible lazy pipelines use bounded per-item/per-stage batches.

| ID | Priority | Status | Operator |
| --- | --- | --- | --- |
| C01 | P1 | Done | `groupBy` |
| C02 | P0 | Done | `pluck` |
| C03 | P0 | Done | `mapObject` |
| C04 | P0 | Done | `filterObject` |
| C05 | P0 | Done | `flatMap` |
| C06 | P1 | Done | `distinctBy` |
| C07 | P1 | Done | `orderBy` |
| C08 | P1 | Done | `then` |
| C09 | P1 | Done | `onNull` |
| C10 | P1 | Done | `takeWhile` |
| C11 | P1 | Done | `dropWhile` |
| C12 | P1 | Done | `some` |
| C13 | P1 | Done | `every` |
| C14 | P1 | Done | `countCharactersBy` |
| C15 | P1 | Done | `everyCharacter` |
| C16 | P1 | Done | `mapString` |
| C17 | P1 | Done | `someCharacter` |
| C18 | P1 | Done | `substringBy` |
| C19 | P1 | Done | `countBy` |
| C20 | P1 | Done | `maxBy` |
| C21 | P1 | Done | `minBy` |
| C22 | P1 | Done | `sumBy` |
| C23 | P1 | Done | `firstWith` |
| C24 | P1 | Done | `indexWhere` |
| C25 | P1 | Done | `partition` |
| C26 | P1 | Done | `splitWhere` |
| C27 | P1 | Done | `everyEntry` |
| C28 | P1 | Done | `someEntry` |
| C29 | P1 | Done | `readLinesWith` (atomic encoding/format operation) |
| C30 | P1 | Done | `writeLinesWith` (atomic encoding/format operation) |
| C31 | P1 | Done | `scan` (atomic regex operation) |
| C32 | P1 | Done | `mapLeafValues` |
| C33 | P1 | Done | `nodeExists` |
| C34 | P1 | Done | `filterArrayLeafs` |
| C35 | P1 | Done | `filterObjectLeafs` |
| C36 | P1 | Done | `failIf` |
| C37 | P1 | Done | `wait` (normal engine identity operation; delay expression stays unevaluated) |
| C38 | P1 | Done | `join`, `leftJoin`, `outerJoin`; internal `evaluate_by` is covered by C20/C21 |
| C39 | P1 | Done | Imported aliases and namespaced callback operator variants |

Shared callback acceptance gates:

- [x] Run callbacks through continuations with correct parameters, `$`/`$$`,
  captures, frames, last results, and repeated source locations.
- [x] Compare values/errors and callback/log order, including null/empty inputs,
  early exit, chaining, nesting, and recursion.
- [x] Preserve eligible laziness; never replay execution for debugger state or
  eagerly materialize a stream solely to inspect it.
- [x] Verify Into/Over/Out, explicit breakpoints, Pause/Stop between bounded chunks.
- [x] Cover each operator family through native, WASM, and playground regressions.

Evidence: [M2 runtime tests](../crates/dwpy-core/tests/debugger_m2.rs) compare
values, errors and log order for every operator using infix/prefix forms,
ordinary/null/empty/invalid inputs, and nested compositions. They cover aliases,
qualified names, local-function/parameter shadowing, lexical captures, recursion,
repeated callback breakpoints, Into/Over/Out, Pause/Stop, retained-memory budgets,
and a billion-element range ending through lazy short-circuiting.

The [M2 DAP transcript](../tests/fixtures/debugger/m2.json) and
[expected result](../tests/fixtures/debugger/m2.expected.json) exercise object,
sequence, grouping/sorting, string, tree, join, predicate, encoding, regex and
control-operation families through native stdio, Rust/WASM and Node/WASM.
Playground browser tests cover the composed script plus repeated object callback
stops, inspection, watches, fresh-session restart and Stop.

Engine-specific boundaries are explicit: `wait` currently returns its input
without evaluating its delay; `failIf` evaluates a scoped predicate expression;
encoding and regex operations do not invoke DataWeave callbacks. The tracker
previously called out `by`, but this engine has no standalone `by` builtin: its
internal `evaluate_by` implements `minBy` and `maxBy`. This milestone does not
invent an additional operator. Function values crossing native/data boundaries
remain R02; error recovery, external modules and reflection remain M3.

## M3 — Modules, values, execution, reflection backlog

| ID | Priority | Status | Feature and acceptance criteria |
| --- | --- | --- | --- |
| R01 | P1 | Todo | Custom/external modules: normal loading/import resolution, source identities/ranges, cross-module stepping |
| R02 | P1 | Todo | Function values crossing native/data boundaries: preserve identity/captures through value builtins and arrays/objects/output, matching normal representation |
| R03 | P1 | Todo | `try`, `orElse`, `orElseTry`: live error handling, fallback/callback order, scopes, caught vs uncaught errors |
| R04 | P1 | Todo | `eval`, `run`, `evalUrl`, `readUrl`: define host availability, nested sources/sessions, cancellation; retain normal host/security restrictions |
| R05 | P1 | Todo | `location`, `locationString`, `evaluateCompatibilityFlag`: reflect the live source/context |
| R06 | P1 | Todo | `arrayItem`, `baseTypeOf`, `functionParamTypes`, `functionReturnType`, `intersectionItems`, `literalValueOf`, `metadataOf`, `nameOf`, `unionItems`, `is*Type`: bridge function/value metadata |

## M4 — Verification and hardening backlog

| ID | Priority | Status | Acceptance criteria |
| --- | --- | --- | --- |
| Q01 | P0 | Partial | Expand differential examples into a corpus covering each engine construct/builtin, parser boundaries, errors |
| Q02 | P0 | Partial | Source mapping/stepping for every new form, repeated text, comments, Unicode, multiline/nested/recursive expressions |
| Q03 | P1 | Partial | Expanded feature matrix through identical native stdio and WASM DAP conversations, beyond direct Rust tests |
| Q04 | P1 | Partial | Browser coverage for new syntax/callback families, inspection, exceptions, cancellation/restart, auto-run, stale messages |
| Q05 | P1 | Partial | Measured large-input/deep-nesting/resource/responsiveness/cleanup regressions |
| Q06 | P1 | Todo | Automated support inventory so new normal-engine constructs/builtins receive explicit debugger classifications |

## Intentional boundaries

These do not block full language coverage in script execution.

| Area | Status | Boundary |
| --- | --- | --- |
| Watches | Policy | Read-only and bounded. Calls limited to `sizeOf`, `isEmpty`, `keysOf`, `valuesOf`, `upper`, `lower`, `abs`. User calls, updates, declarations, callbacks, logging, time/random, I/O stay prohibited unless deliberately redesigned |
| Native implementations | Policy | Rust builtins and format readers/writers are atomic; stepping targets DataWeave expressions |
| Pausing atomic work | Partial | Pause between chunks; playground Stop can terminate a busy worker. Cancellation inside atomic operations requires separate implementation |
| Engine limitations | Policy | Reuse normal engine semantics and host capabilities; broader MuleSoft compatibility is engine work |

## M5 — Optional tooling and protocol extensions

All items are **Deferred / P2**. Scope acceptance criteria and advertise capabilities
only when implementing them. These are separate from language-parity work.

- [ ] E01 — Conditional breakpoints and hit counts.
- [ ] E02 — Logpoints preserving ordinary execution/log order.
- [ ] E03 — Function/data/instruction breakpoints where meaningful.
- [ ] E04 — Variable/expression editing and frame restart with a defined state model.
- [ ] E05 — Reverse stepping with explicit effect/replay semantics.
- [ ] E06 — Attach and multiple execution threads, if the execution model supports them.
- [ ] E07 — Additional transports, including TCP/WebSocket where useful.
- [ ] E08 — Memory inspection/disassembly, if meaningful for this interpreter.
- [ ] E09 — Packaged VS Code extension and real-editor integration tests.
- [ ] E10 — Legacy Python-interpreter debugging.

## Completion gates for each implementation item

- [ ] Parse nested syntax in the grammar; retain original locations without
  searching expression text or rewriting source to reconstruct spans.
- [ ] Use bounded live continuations and explicit scopes; reuse normal value
  operations, builtins, formats, and serialization.
- [ ] Verify values, errors, short-circuiting, laziness, effects, stepping,
  breakpoints, and inspection for the changed feature.
- [ ] Add appropriate native/WASM/DAP/browser cases, including invalid syntax and
  unsupported subexpressions failing before execution effects.
- [ ] Rebuild bundled playground WASM/build metadata after engine changes.
- [ ] Run required checks and record results; investigate failures instead of
  weakening assertions to declare completion.
- [ ] Update this tracker and [DEBUGGER.md](DEBUGGER.md) together; record tests and
  commit/PR evidence, preserving unrelated changes.
- [ ] Advertise only implemented DAP capabilities.

## Last local verification

Recorded **2026-10-24**, after M2 callback/collection parity. Historical local results do not
establish that future source changes or published builds pass.

| Check | Last result |
| --- | --- |
| `cargo test --workspace` | 202 passed |
| `bash scripts/test_wasm_rust.sh` | 25 passed |
| Node/WASM package tests | 47 passed |
| `UV_CACHE_DIR=.uv-cache uv run --extra dev pytest` | 1,220 passed, 5 xfailed; corpus supplied via `DWPY_DATAWEAVE_DOCS` |
| Playground unit tests | 22 passed |
| Playground browser tests | 22 passed (12 playground debugger, 3 inline debugger, 7 blog) |
| Playground typecheck and production build | Passed |
| Playground lint | Passed; 21 existing warnings, 0 errors |
| Publishing/deployment | Not performed |

Commands and corpus setup: [verification guide](DEBUGGER.md#verification).
Obtain permission before publishing packages/releases or deploying the playground.

## Change log and future evidence

| Date | Items | Progress | Evidence |
| --- | --- | --- | --- |
| 2026-10-24 | M2 / C01–C39 | Live collection/tree/join cursors, resumable compiled batches, aliases and qualified calls; shared dispatch and callback parameter/precedence fixes | `debugger_m2.rs` (8 tests, all-operator differential matrix), `m2.json` native/WASM/Node conversation, 2 M2 browser tests |
| 2026-10-24 | M1 / L01–L13 | Core grammar continuations; shared lexical boundaries for nested interpolation, comments and generic/comparison syntax; explicit update-path validation | `debugger_m1.rs` (4 tests, differential matrix), `m1.json` native/WASM/Node conversation, 2 M1 browser tests |
| 2026-10-24 | M0 | Live runtime, DAP, inspection, playground workflow implemented locally | Test files linked above |
| 2026-10-24 | Update subset | Guards/bindings, upserts, nested paths, replacement stepping | `differential_updates`, `updates_step_into_replacements_and_restore_bindings`, repeated update breakpoint test, Node/browser update cases |
| 2026-10-24 | Overloads | Typed/arity dispatch, defaults, recursive/reference dispatch; shared temporal-literal detection for union annotations | `differential_overloads_and_recursive_dispatch`, `overload_errors_occur_in_the_selected_body_and_match_normal_execution`, Node/browser overload cases |
| 2026-10-24 | Playground polish | Header Debug, breakpoint-aware entry, smaller toolbar flush with Monaco, active-only inspector, balanced gutter | Browser lifecycle/breakpoint tests and local gutter-spacing check |

Append future rows with item IDs, test names, verification date/results, and
commit/PR links when available. Opening a PR does not publish a package or
authorize a deployment.
