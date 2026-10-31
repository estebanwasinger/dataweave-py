# Initial conformance baseline

Reference: `dataweave-native==2.12.2`. Candidate: compiled Rust bridge at
`f81e7b0`, with legacy fallback disabled. All 543 cases were executed live.

**326 match and 217 differ in Rust.**
Matching cases include successful outputs and matching script rejections.

| Area | Cases | Match | Rust difference |
| --- | ---: | ---: | ---: |
| [functions](dataweave/cases/functions.dwl) | 42 | 27 | 15 |
| [header](dataweave/cases/header.dwl) | 25 | 12 | 13 |
| [literals](dataweave/cases/literals.dwl) | 35 | 21 | 14 |
| [objects](dataweave/cases/objects.dwl) | 40 | 26 | 14 |
| [operators](dataweave/cases/operators.dwl) | 23 | 20 | 3 |
| [patterns](dataweave/cases/patterns.dwl) | 43 | 19 | 24 |
| [precedence](dataweave/cases/precedence.dwl) | 142 | 80 | 62 |
| [selectors](dataweave/cases/selectors.dwl) | 37 | 21 | 16 |
| [syntax-forms](dataweave/cases/syntax-forms.dwl) | 38 | 17 | 21 |
| [types](dataweave/cases/types.dwl) | 80 | 58 | 22 |
| [update](dataweave/cases/update.dwl) | 38 | 25 | 13 |

Cases set numeric formatting locale and CSV line separators explicitly so the
reference results agree across host defaults. The explicit CSV `lineSeparator`
option exposes a Rust writer capability gap in `header/output-csv-properties`;
the reference returns the recorded `1\n`. Live and offline runs agree on this
baseline, including on Windows with a German numeric locale.

The baseline is a starting point, not an automatically updated progress claim.
Generate `.refs/conformance.json` using the command in the README to see current
per-case status, diagnostics and reproduction commands. The full `--check` run
returns exit 1 while the 217 Rust differences remain.

For a bounded first task, `selectors/quoted-interp` exercises interpolation in
a quoted field selector; `selectors/quoted-field` is a passing literal-key
control. All language areas remain available for independent work.
