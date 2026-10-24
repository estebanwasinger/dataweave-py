# DataWeave conformance checks

Run all 543 self-contained language cases against the strict Rust engine and
MuleSoft's engine through `dataweave-native==2.12.2`. The reference version is
pinned in the optional `conformance` extra and checked at runtime. No Java or
Mule installation is needed. Build the Rust Python bridge as described in the
main README before running the harness.

```bash
# Live comparison, plus a per-case report for investigation
uv run --extra conformance python -m conformance.dataweave.run --check --report .refs/conformance.json

# Compare Rust against the recorded reference results without loading the reference
uv run python -m conformance.dataweave.run --offline --check
```

The full suite currently has unresolved differences and returns exit 1 with
`--check`. These are the implementation backlog. See [the initial baseline](BASELINE.md)
for counts by language area. Cases are grouped in 11 files, and every case has a
stable `filename-stem/case-name` report ID.

## Working through one case

Select both a file and case name; names can repeat across files:

```bash
uv run --extra conformance python -m conformance.dataweave.run conformance/dataweave/cases/selectors.dwl --case field-of-literal --check
```

The JSON report contains the file, line, stored expectation, reference result,
reference error, Rust mismatch and reproduction command arguments for each case.
Filter `status == "fail"` for implementation work. Entries marked
`reference-error` need reference-engine investigation before judging Rust.
Reports stay in gitignored `.refs/` and are regenerated as work progresses.

Keep the reference expectation when fixing Rust. Rerun the case, then its file,
then the full suite to check for regressions. A passing case prints no failure
and changes to `status == "pass"` in the next report. Every Rust difference
remains a failure.

## Case format and expectations

```dataweave
// case concat-strings
// expect "ab"
%dw 2.0
output application/json
---
'a' ++ 'b'
```

Every body must be a full script beginning with `%dw`; the harness passes it
through without adding headers or interpreting delimiters. Include an explicit
output directive when rendered output is expected. Tests use embedded data and
header variables; no external payload fixtures are bound. `// case` markers at
the start of a line are reserved for separating cases.

Set environment-sensitive options explicitly: numeric formatting cases use
`locale: "en"`, and CSV cases use `lineSeparator='\n'`. Reference expectations
must not depend on the host's locale or default line endings.

An expectation is compact JSON, `error` for reference script rejection, or
`text "escaped output"` for non-JSON output. The text form preserves XML/CSV
line endings and whitespace on a single metadata line. `?` means no verified
reference result is available. For a new case or a reviewed reference change:

```bash
uv run --extra conformance python -m conformance.dataweave.run conformance/dataweave/cases/operators.dwl --case concat-strings --update --check
```

`--update` writes reference results only; it never derives expectations from
Rust. Normal live and offline runs never modify cases. Live mode checks both
the stored expectation against the reference and Rust against that result.
Reference failures preserve existing expectations, including `?`; other verified
results can still be updated in the same run.
Offline mode uses the recorded result and does not establish fresh reference
verification. Updating the pinned package version requires refreshing and
reviewing expectations and the baseline.

## Comparison and limits

The reference's MIME type selects comparison: `application/json` and `+json`
media types use JSON comparison; every other type is stored as escaped text and
compared exactly, including numeric-looking CSV/plain text and line endings.
Offline comparisons retain that choice in the expectation's `text` prefix.
JSON comparison ignores whitespace and equivalent number spelling, preserves
precision, duplicate keys and object/array order, and distinguishes booleans
from numbers. Invalid declared JSON from the reference fails setup; invalid
JSON from Rust is a mismatch. Both engines must return rendered text. The Rust
bridge is required and legacy Python fallback is disabled, independently of
backend environment variables.

The reference binding exposes success or rejection without a structured
syntax/compile/runtime phase, so rejected cases use `error`; messages are
diagnostic and are not compared byte for byte. Unsupported-feature exceptions
and unexpected Rust crashes never count as matching rejections.

Empty/null native responses, malformed response envelopes and failures without
diagnostics appear as `reference-error` and fail `--check`, preserving the stored
expectation. A new case retains `?` until a verified result is available.

Binding/lifecycle exceptions fail setup. Both engines execute in process;
use an external process timeout for potentially unbounded scripts. Native
diagnostics can appear on stderr.

With `--check`, exit 0 means no differences or reference failures, 1 means
differences or reference failures, and 2 means setup failure. Without `--check`,
differences are reported with exit 0 for exploration. An empty case selection
always fails setup. The suite measures its named cases, not total language coverage.

The harness's ordinary pytest tests use fakes and need neither reference-engine
initialization nor network access.
